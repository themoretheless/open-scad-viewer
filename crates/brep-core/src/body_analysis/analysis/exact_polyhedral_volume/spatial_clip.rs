use super::{nearest_segment_point, refuse, spend, visit_polyhedral_boundary};
use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSegmentClip {
    /// Closed material intervals in the input segment parameter [0, 1].
    pub intervals: Vec<[Q; 2]>,
    pub exact_work: u64,
}

struct FaceBoundary {
    origin: [Q; 3],
    normal: Option<[Q; 3]>,
    segments: Vec<[[Q; 3]; 2]>,
}

/// Clip a nonzero spatial segment against a qualified polyhedral solid.
/// The full supporting line is traversed from outside to classify segments
/// whose endpoints are already inside material. Lines through a boundary
/// edge or vertex, or running along a retained face, are outside this
/// transverse mode.
pub fn exact_polyhedral_segment_clip(
    solid: &QualifiedClosedSolid,
    endpoints_mm: &[[Q; 3]; 2],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSegmentClip> {
    let [start, end] = endpoints_mm;
    if endpoints_mm
        .iter()
        .flatten()
        .any(|v| v.numer().bits().max(v.denom().bits()) > 8192)
        || start == end
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Expected a nonzero clip segment with bounded coordinates",
        ));
    }
    let direction: [Q; 3] = std::array::from_fn(|i| &end[i] - &start[i]);
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut crossings =
        line_crossings(&faces, start, &direction, &mut remaining)?.ok_or_else(|| {
            refuse("Clip line contacts a retained face boundary or lies along a face")
        })?;
    let count = crossings.len();
    if count > 0 {
        spend(
            &mut remaining,
            count as u64 * (4 * u64::from(count.ilog2() + 1) + 5),
        )?;
    }
    crossings.sort_unstable();
    if count % 2 != 0 || crossings.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refuse("Clip line crossings must be distinct and paired"));
    }
    let mut intervals = Vec::new();
    let mut crossings = crossings.into_iter();
    while let Some(lo) = crossings.next() {
        let hi = crossings.next().expect("even number of crossings");
        let lo = lo.max(Q::zero());
        let hi = hi.min(Q::one());
        if lo <= hi {
            intervals.push([lo, hi]);
        }
    }
    Ok(ExactPolyhedralSegmentClip {
        intervals,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyhedralPointLocation {
    Exterior,
    Material,
    Boundary { face: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPointLocation {
    pub location: PolyhedralPointLocation,
    pub exact_work: u64,
}

/// Classify a rational world-space point in qualified polyhedral material.
/// An exact boundary hit returns one incident source face. Interior parity
/// uses a ray with no edge, vertex, or retained coplanar contacts.
pub fn exact_polyhedral_point_location(
    solid: &QualifiedClosedSolid,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPointLocation> {
    validate_point(point_mm)?;
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    for (&face, boundary) in &faces {
        spend(&mut remaining, 12)?;
        let normal = face_normal(boundary)?;
        let delta = std::array::from_fn(|i| &point_mm[i] - &boundary.origin[i]);
        if dot(normal, &delta).is_zero()
            && point_in_face(
                &boundary.segments,
                point_mm,
                plane_axes(normal),
                &mut remaining,
            )? != FaceLocation::Outside
        {
            return Ok(ExactPolyhedralPointLocation {
                location: PolyhedralPointLocation::Boundary { face },
                exact_work: max_exact_work - remaining,
            });
        }
    }
    let location = classify_open_point(&faces, point_mm, &mut remaining)?;
    Ok(ExactPolyhedralPointLocation {
        location,
        exact_work: max_exact_work - remaining,
    })
}

fn classify_open_point(
    faces: &BTreeMap<usize, FaceBoundary>,
    point_mm: &[Q; 3],
    remaining: &mut u64,
) -> Result<PolyhedralPointLocation> {
    // A nonzero plane condition evaluated on (1, k, k²) has at most
    // two roots. Retry degenerate lines while charging the common budget.
    for k in 1_u64.. {
        spend(remaining, 8)?;
        let direction = [1, k, k * k].map(|value| Q::from_integer(value.into()));
        if let Some(crossings) = line_crossings(faces, point_mm, &direction, remaining)? {
            spend(remaining, crossings.len() as u64)?;
            if crossings.len() % 2 != 0 {
                return Err(refuse("Point ray requires paired full-line crossings"));
            }
            let inside = crossings.iter().filter(|t| t.is_positive()).count() % 2 == 1;
            let location = if inside {
                PolyhedralPointLocation::Material
            } else {
                PolyhedralPointLocation::Exterior
            };
            return Ok(location);
        }
    }
    unreachable!("bounded ray search cannot exhaust the integer range")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralBoundaryDistance {
    pub distance_squared_mm2: Q,
    pub point_mm: [Q; 3],
    /// Source face containing the closest point.
    pub face: usize,
    pub exact_work: u64,
}

/// Find a nearest point on the retained boundary of a qualified polyhedral
/// solid. Orthogonal face projections are checked against holes and trims;
/// a missed projection is resolved over all boundary segments of the face.
pub fn exact_polyhedral_boundary_distance(
    solid: &QualifiedClosedSolid,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralBoundaryDistance> {
    validate_point(point_mm)?;
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut result = nearest_boundary(&faces, point_mm, &mut remaining)?;
    result.exact_work = max_exact_work - remaining;
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSignedDistance {
    /// Negative inside material, positive outside, and zero on the boundary.
    /// The unsigned magnitude is the squared Euclidean distance.
    pub signed_distance_squared_mm2: Q,
    pub nearest_mm: [Q; 3],
    pub face: usize,
    pub exact_work: u64,
}

/// Evaluate signed distance to an exactly qualified polyhedral boundary.
/// The same retained faces supply both the distance witness and material parity.
pub fn exact_polyhedral_signed_distance(
    solid: &QualifiedClosedSolid,
    point_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSignedDistance> {
    validate_point(point_mm)?;
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut report = signed_distance_prepared(&faces, point_mm, &mut remaining)?;
    report.exact_work = max_exact_work - remaining;
    Ok(report)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSignedDistanceBatch {
    /// One result per input point in order, with per-point work excluding
    /// shared face preparation.
    pub samples: Vec<ExactPolyhedralSignedDistance>,
    pub exact_work: u64,
}

/// Evaluate signed distance at multiple rational world-space points under
/// one budget. The complete ordered batch is returned only if every query
/// succeeds.
pub fn exact_polyhedral_signed_distance_batch(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralSignedDistanceBatch> {
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut samples = Vec::new();
    for point in points_mm {
        validate_point(point)?;
        samples.push(signed_distance_prepared(&faces, point, &mut remaining)?);
    }
    Ok(ExactPolyhedralSignedDistanceBatch {
        samples,
        exact_work: max_exact_work - remaining,
    })
}

fn signed_distance_prepared(
    faces: &BTreeMap<usize, FaceBoundary>,
    point_mm: &[Q; 3],
    remaining: &mut u64,
) -> Result<ExactPolyhedralSignedDistance> {
    let start_work = *remaining;
    let nearest = nearest_boundary(faces, point_mm, remaining)?;
    let mut squared = nearest.distance_squared_mm2;
    if !squared.is_zero()
        && classify_open_point(faces, point_mm, remaining)? == PolyhedralPointLocation::Material
    {
        squared = -squared;
    }
    Ok(ExactPolyhedralSignedDistance {
        signed_distance_squared_mm2: squared,
        nearest_mm: nearest.point_mm,
        face: nearest.face,
        exact_work: start_work - *remaining,
    })
}

fn validate_point(point_mm: &[Q; 3]) -> Result<()> {
    if point_mm
        .iter()
        .any(|q| q.numer().bits().max(q.denom().bits()) > 8192)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Point coordinates exceed the exact bit limit",
        ));
    }
    Ok(())
}

fn nearest_boundary(
    faces: &BTreeMap<usize, FaceBoundary>,
    point_mm: &[Q; 3],
    remaining: &mut u64,
) -> Result<ExactPolyhedralBoundaryDistance> {
    let mut nearest = None::<ExactPolyhedralBoundaryDistance>;
    for (&face, boundary) in faces {
        spend(remaining, 32)?;
        let normal = face_normal(boundary)?;
        let delta = std::array::from_fn(|i| &point_mm[i] - &boundary.origin[i]);
        let residual = dot(normal, &delta);
        let scale = &residual / dot(normal, normal);
        let plane_distance = &residual * &scale;
        if nearest
            .as_ref()
            .is_some_and(|candidate| candidate.distance_squared_mm2 <= plane_distance)
        {
            continue;
        }
        let projected = std::array::from_fn(|i| &point_mm[i] - &scale * &normal[i]);
        let mut consider = |point_mm, distance_squared_mm2| {
            if nearest
                .as_ref()
                .is_none_or(|candidate| distance_squared_mm2 < candidate.distance_squared_mm2)
            {
                nearest = Some(ExactPolyhedralBoundaryDistance {
                    distance_squared_mm2,
                    point_mm,
                    face,
                    exact_work: 0,
                });
            }
        };
        if point_in_face(
            &boundary.segments,
            &projected,
            plane_axes(normal),
            remaining,
        )? != FaceLocation::Outside
        {
            consider(projected, plane_distance);
        } else {
            for [a, b] in &boundary.segments {
                spend(remaining, 40)?;
                let (closest, squared) = nearest_segment_point(point_mm, a, b);
                consider(closest, squared);
            }
        }
    }
    nearest.ok_or_else(|| refuse("Boundary distance requires at least one face"))
}

fn prepare_faces(
    solid: &QualifiedClosedSolid,
    remaining: &mut u64,
) -> Result<BTreeMap<usize, FaceBoundary>> {
    let mut faces = BTreeMap::<usize, FaceBoundary>::new();
    visit_polyhedral_boundary(solid, remaining, |face, origin, a, b, _, remaining| {
        spend(remaining, 24)?;
        let boundary = faces.entry(face).or_insert_with(|| FaceBoundary {
            origin: origin.clone(),
            normal: None,
            segments: Vec::new(),
        });
        if boundary.normal.is_none() {
            let u: [Q; 3] = std::array::from_fn(|i| &a[i] - &origin[i]);
            let v: [Q; 3] = std::array::from_fn(|i| &b[i] - &origin[i]);
            let normal = [(1, 2), (2, 0), (0, 1)].map(|(j, k)| &u[j] * &v[k] - &u[k] * &v[j]);
            if normal.iter().any(|value| !value.is_zero()) {
                boundary.normal = Some(normal);
            }
        }
        boundary.segments.push([a.clone(), b.clone()]);
        Ok(())
    })?;

    Ok(faces)
}

fn face_normal(face: &FaceBoundary) -> Result<&[Q; 3]> {
    face.normal
        .as_ref()
        .ok_or_else(|| refuse("Clip face has no planar boundary normal"))
}

fn plane_axes(normal: &[Q; 3]) -> [usize; 2] {
    let omitted = normal.iter().position(|value| !value.is_zero()).unwrap();
    [(omitted + 1) % 3, (omitted + 2) % 3]
}

fn line_crossings(
    faces: &BTreeMap<usize, FaceBoundary>,
    start: &[Q; 3],
    direction: &[Q; 3],
    remaining: &mut u64,
) -> Result<Option<Vec<Q>>> {
    let mut crossings = Vec::new();
    for boundary in faces.values() {
        spend(remaining, 24)?;
        let normal = face_normal(boundary)?;
        let denominator = dot(normal, direction);
        let delta = std::array::from_fn(|i| &boundary.origin[i] - &start[i]);
        let numerator = dot(normal, &delta);
        let axes = plane_axes(normal);
        if denominator.is_zero() {
            if numerator.is_zero()
                && coplanar_line_hits_boundary(
                    &boundary.segments,
                    start,
                    direction,
                    axes,
                    remaining,
                )?
            {
                return Ok(None);
            }
            continue;
        }
        let t = numerator / denominator;
        let point = std::array::from_fn(|i| &start[i] + &t * &direction[i]);
        match point_in_face(&boundary.segments, &point, axes, remaining)? {
            FaceLocation::Inside => crossings.push(t),
            FaceLocation::Boundary => return Ok(None),
            FaceLocation::Outside => {}
        }
    }
    Ok(Some(crossings))
}

fn coplanar_line_hits_boundary(
    segments: &[[[Q; 3]; 2]],
    start: &[Q; 3],
    direction: &[Q; 3],
    [u, v]: [usize; 2],
    remaining: &mut u64,
) -> Result<bool> {
    let side =
        |p: &[Q; 3]| &direction[u] * (&p[v] - &start[v]) - &direction[v] * (&p[u] - &start[u]);
    // A full line cannot enter a bounded face without meeting its boundary.
    for [a, b] in segments {
        spend(remaining, 12)?;
        let a = side(a);
        let b = side(b);
        if a.is_zero() || b.is_zero() || a.is_negative() != b.is_negative() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FaceLocation {
    Inside,
    Outside,
    Boundary,
}

fn point_in_face(
    segments: &[[[Q; 3]; 2]],
    point: &[Q; 3],
    [u, v]: [usize; 2],
    remaining: &mut u64,
) -> Result<FaceLocation> {
    let [x, y] = [&point[u], &point[v]];
    let mut inside = false;
    for [a, b] in segments {
        spend(remaining, 12)?;
        let side = (&b[u] - &a[u]) * (y - &a[v]) - (&b[v] - &a[v]) * (x - &a[u]);
        if side.is_zero()
            && std::cmp::min(&a[u], &b[u]) <= x
            && x <= std::cmp::max(&a[u], &b[u])
            && std::cmp::min(&a[v], &b[v]) <= y
            && y <= std::cmp::max(&a[v], &b[v])
        {
            return Ok(FaceLocation::Boundary);
        }
        if (&a[v] > y) != (&b[v] > y) && side.is_positive() == (b[v] > a[v]) {
            inside = !inside;
        }
    }
    Ok(if inside {
        FaceLocation::Inside
    } else {
        FaceLocation::Outside
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyhedralBallRelation {
    Inside,
    InsideTangent,
    CrossesBoundary,
    BoundaryPoint,
    OutsideTangent,
    Outside,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralBallClearance {
    pub relation: PolyhedralBallRelation,
    pub signed_distance_squared_mm2: Q,
    pub nearest_mm: [Q; 3],
    pub face: usize,
    pub exact_work: u64,
}

/// Compare a closed ball with the material of a qualified polyhedral solid.
/// The radius is an exact rational length in millimeters. Tangency is kept
/// distinct from strict clearance and from crossing the material boundary.
pub fn exact_polyhedral_ball_clearance(
    solid: &QualifiedClosedSolid,
    center_mm: &[Q; 3],
    radius_mm: &Q,
    max_exact_work: u64,
) -> Result<ExactPolyhedralBallClearance> {
    use std::cmp::Ordering;

    if radius_mm.is_negative() || radius_mm.numer().bits().max(radius_mm.denom().bits()) > 8192 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Expected a nonnegative radius within the exact bit limit",
        ));
    }
    super::validate_work(max_exact_work)?;
    let mut remaining = max_exact_work;
    spend(&mut remaining, 8)?;
    if remaining == 0 {
        return Err(super::work_exhausted());
    }
    let result = exact_polyhedral_signed_distance(solid, center_mm, remaining)?;
    let signed = result.signed_distance_squared_mm2;
    let radius_squared = radius_mm.pow(2);
    let relation = match (
        signed.signum().cmp(&Q::zero()),
        signed.abs().cmp(&radius_squared),
    ) {
        (Ordering::Equal, Ordering::Equal) => PolyhedralBallRelation::BoundaryPoint,
        (_, Ordering::Less) => PolyhedralBallRelation::CrossesBoundary,
        (Ordering::Less, Ordering::Equal) => PolyhedralBallRelation::InsideTangent,
        (Ordering::Less, Ordering::Greater) => PolyhedralBallRelation::Inside,
        (Ordering::Greater, Ordering::Equal) => PolyhedralBallRelation::OutsideTangent,
        (Ordering::Greater, Ordering::Greater) => PolyhedralBallRelation::Outside,
        (Ordering::Equal, Ordering::Greater) => {
            unreachable!("zero distance cannot exceed a squared radius")
        }
    };
    Ok(ExactPolyhedralBallClearance {
        relation,
        signed_distance_squared_mm2: signed,
        nearest_mm: result.nearest_mm,
        face: result.face,
        exact_work: max_exact_work - remaining + result.exact_work,
    })
}

mod sweep_clearance;
pub use sweep_clearance::{
    ExactPolyhedralSweptBallClearance, ExactPolyhedralSweptPathClearance,
    exact_polyhedral_swept_ball_clearance, exact_polyhedral_swept_path_clearance,
};
