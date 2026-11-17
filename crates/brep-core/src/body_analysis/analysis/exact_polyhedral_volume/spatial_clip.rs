use super::{
    bound_root_sum, enclosure_width, nearest_segment_point, refuse, spend,
    visit_polyhedral_boundary,
};
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
    ExactPolyhedralSweptPathMaterial, PolyhedralSweptFaceContactRange,
    PolyhedralSweptFaceContactRanges, PolyhedralSweptPathContact, PolyhedralSweptPathFirstContact,
    PolyhedralSweptPathRelation, bounded_polyhedral_swept_face_contact_ranges,
    bounded_polyhedral_swept_path_first_contact, exact_polyhedral_swept_ball_clearance,
    exact_polyhedral_swept_path_clearance, exact_polyhedral_swept_path_material,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathFaceContacts {
    pub span: usize,
    pub face: usize,
    /// Closed local parameter intervals, including isolated intersections.
    pub intervals: Vec<[Q; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathBoundaryContacts {
    /// Nonempty contacts in path order, then by source face index.
    pub contacts: Vec<PolyhedralPathFaceContacts>,
    pub exact_work: u64,
}

/// Compute the full set of zero-radius contacts between a spatial polyline
/// and the boundary of a qualified polyhedral solid. Coplanar overlaps are
/// clipped against the retained face, so holes can split the contact set.
pub fn exact_polyhedral_path_boundary_contacts(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathBoundaryContacts> {
    let mut remaining = max_exact_work;
    let faces = prepare_path_faces(solid, points_mm, &mut remaining)?;
    let mut contacts = Vec::new();
    for (span, pair) in points_mm.windows(2).enumerate() {
        contacts.extend(span_face_contacts(
            &faces,
            span,
            &pair[0],
            &pair[1],
            &mut remaining,
        )?);
    }
    Ok(ExactPolyhedralPathBoundaryContacts {
        contacts,
        exact_work: max_exact_work - remaining,
    })
}

fn prepare_path_faces(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    remaining: &mut u64,
) -> Result<BTreeMap<usize, FaceBoundary>> {
    if points_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Spatial path requires at least two points",
        ));
    }
    let faces = prepare_faces(solid, remaining)?;
    for point in points_mm {
        spend(remaining, 1)?;
        validate_point(point)?;
    }
    Ok(faces)
}

fn span_face_contacts(
    faces: &BTreeMap<usize, FaceBoundary>,
    span: usize,
    start: &[Q; 3],
    end: &[Q; 3],
    remaining: &mut u64,
) -> Result<Vec<PolyhedralPathFaceContacts>> {
    use super::section_loops::clip::clip_planar_segment;

    let mut contacts = Vec::new();
    let direction: [Q; 3] = std::array::from_fn(|i| &end[i] - &start[i]);
    spend(remaining, 3)?;
    for (&face, boundary) in faces {
        spend(remaining, 24)?;
        let normal = face_normal(boundary)?;
        let delta = std::array::from_fn(|i| &boundary.origin[i] - &start[i]);
        let numerator = dot(normal, &delta);
        let denominator = dot(normal, &direction);
        let axes = plane_axes(normal);
        let intervals = if denominator.is_zero() {
            if !numerator.is_zero() {
                continue;
            }
            clip_planar_segment(
                boundary.segments.iter().map(|[a, b]| (a, b)),
                axes,
                start,
                end,
                remaining,
                |point, remaining| {
                    Ok(point_in_face(&boundary.segments, point, axes, remaining)?
                        != FaceLocation::Outside)
                },
            )?
        } else {
            let t = numerator / denominator;
            if t.is_negative() || t > Q::one() {
                continue;
            }
            let point = std::array::from_fn(|i| &start[i] + &t * &direction[i]);
            if point_in_face(&boundary.segments, &point, axes, remaining)? == FaceLocation::Outside
            {
                continue;
            }
            vec![[t.clone(), t]]
        };
        if !intervals.is_empty() {
            contacts.push(PolyhedralPathFaceContacts {
                span,
                face,
                intervals,
            });
        }
    }
    Ok(contacts)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathMaterialClip {
    /// Sorted, disjoint closed intervals of local parameters for each span.
    pub span_intervals: Vec<Vec<[Q; 2]>>,
    pub exact_work: u64,
}

/// Clip a spatial polyline to the closed material of a qualified polyhedral
/// solid. Intervals include boundary overlaps and isolated tangencies.
pub fn exact_polyhedral_path_material_clip(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathMaterialClip> {
    let (span_intervals, exact_work) =
        partition_path(solid, points_mm, max_exact_work, |intervals, part| {
            if part.location != PolyhedralPathLocation::Exterior {
                append_closed_interval(intervals, part.parameters);
            }
        })?;
    Ok(ExactPolyhedralPathMaterialClip {
        span_intervals,
        exact_work,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralInteriorInterval {
    pub parameters: [Q; 2],
    /// Whether the start and end parameters, respectively, are strictly inside.
    pub endpoints_included: [bool; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathInteriorClip {
    /// Strict interior intervals in local parameter order, one list per span.
    pub span_intervals: Vec<Vec<PolyhedralInteriorInterval>>,
    pub exact_work: u64,
}

/// Clip a spatial polyline to the strict interior of a qualified polyhedral
/// solid. Boundary runs and isolated contacts are excluded from the result.
pub fn exact_polyhedral_path_interior_clip(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathInteriorClip> {
    let (span_intervals, exact_work) =
        partition_path(solid, points_mm, max_exact_work, |intervals, part| {
            if part.location == PolyhedralPathLocation::Interior {
                intervals.push(PolyhedralInteriorInterval {
                    parameters: part.parameters,
                    endpoints_included: part.endpoints_included,
                });
            }
        })?;
    Ok(ExactPolyhedralPathInteriorClip {
        span_intervals,
        exact_work,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyhedralPathLocation {
    Interior,
    Boundary,
    Exterior,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathPart {
    pub location: PolyhedralPathLocation,
    pub parameters: [Q; 2],
    /// Whether the start and end parameters, respectively, belong to this part.
    pub endpoints_included: [bool; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathPartition {
    /// Disjoint parts covering [0, 1] in order for each local span.
    pub spans: Vec<Vec<PolyhedralPathPart>>,
    pub exact_work: u64,
}

/// Partition each polyline span into strict interior, boundary and exterior
/// parts. Isolated contacts remain distinct zero-width boundary parts.
pub fn exact_polyhedral_path_partition(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathPartition> {
    let (spans, exact_work) = partition_path(solid, points_mm, max_exact_work, Vec::push)?;
    Ok(ExactPolyhedralPathPartition { spans, exact_work })
}

fn partition_path<T>(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
    mut visit: impl FnMut(&mut Vec<T>, PolyhedralPathPart),
) -> Result<(Vec<Vec<T>>, u64)> {
    let mut remaining = max_exact_work;
    let faces = prepare_path_faces(solid, points_mm, &mut remaining)?;
    let mut span_intervals = Vec::new();
    for (span, pair) in points_mm.windows(2).enumerate() {
        let [start, end] = [&pair[0], &pair[1]];
        let contacts = span_face_contacts(&faces, span, start, end, &mut remaining)?
            .into_iter()
            .flat_map(|contact| contact.intervals)
            .collect();
        let boundary = merge_closed_intervals(contacts, &mut remaining)?;

        let mut intervals = Vec::new();
        let mut cursor = Q::zero();
        let mut start_included = true;
        let direction: [Q; 3] = std::array::from_fn(|i| &end[i] - &start[i]);
        let two = Q::from_integer(2.into());
        for contact in boundary.into_iter().map(Some).chain([None]) {
            let end_included = contact.is_none();
            let lo = contact
                .as_ref()
                .map_or_else(Q::one, |interval| interval[0].clone());
            if cursor < lo {
                spend(&mut remaining, 8)?;
                let t = (&cursor + &lo) / &two;
                let point = std::array::from_fn(|i| &start[i] + &t * &direction[i]);
                let location = match classify_open_point(&faces, &point, &mut remaining)? {
                    PolyhedralPointLocation::Material => PolyhedralPathLocation::Interior,
                    PolyhedralPointLocation::Exterior => PolyhedralPathLocation::Exterior,
                    PolyhedralPointLocation::Boundary { .. } => {
                        return Err(refuse("Path gap intersects boundary"));
                    }
                };
                visit(
                    &mut intervals,
                    PolyhedralPathPart {
                        location,
                        parameters: [cursor.clone(), lo],
                        endpoints_included: [start_included, end_included],
                    },
                );
            }
            if let Some([lo, hi]) = contact {
                cursor = hi.clone();
                start_included = false;
                visit(
                    &mut intervals,
                    PolyhedralPathPart {
                        location: PolyhedralPathLocation::Boundary,
                        parameters: [lo, hi],
                        endpoints_included: [true, true],
                    },
                );
            }
        }
        span_intervals.push(intervals);
    }
    Ok((span_intervals, max_exact_work - remaining))
}

fn merge_closed_intervals(mut intervals: Vec<[Q; 2]>, remaining: &mut u64) -> Result<Vec<[Q; 2]>> {
    let count = intervals.len() as u64;
    spend(
        remaining,
        count.saturating_mul(8 * (u64::from(count.checked_ilog2().unwrap_or(0)) + 1)),
    )?;
    intervals.sort_unstable();
    spend(remaining, count.saturating_mul(4))?;
    let mut merged = Vec::new();
    for interval in intervals {
        append_closed_interval(&mut merged, interval);
    }
    Ok(merged)
}

fn append_closed_interval(intervals: &mut Vec<[Q; 2]>, interval: [Q; 2]) {
    if let Some(last) = intervals.last_mut()
        && last[1] >= interval[0]
    {
        if interval[1] > last[1] {
            last[1] = interval[1].clone();
        }
    } else {
        intervals.push(interval);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPolyhedralPathMaterialLength {
    /// Inclusive bounds in millimeters for distance traveled in closed material.
    pub length_bounds_mm: [Q; 2],
    pub exact_work: u64,
}

/// Bound the length of a polyline restricted to a qualified polyhedral solid.
/// Boundary travel contributes to length, and repeated traversals count separately.
pub fn bounded_polyhedral_path_material_length(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    tolerance_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralPathMaterialLength> {
    let tolerance = enclosure_width(tolerance_mm)?;
    let clip = exact_polyhedral_path_material_clip(solid, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - clip.exact_work;
    let mut squares = Vec::new();
    for (pair, intervals) in points_mm.windows(2).zip(clip.span_intervals) {
        spend(&mut remaining, 8)?;
        let mut fraction = Q::zero();
        for [lo, hi] in intervals {
            spend(&mut remaining, 4)?;
            fraction += hi - lo;
        }
        if fraction.is_positive() {
            let direction = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
            let square = dot(&direction, &direction);
            if square.is_positive() {
                squares.push(fraction.pow(2) * square);
            }
        }
    }
    let length_bounds_mm = bound_root_sum(squares, tolerance, &mut remaining)?;
    Ok(BoundedPolyhedralPathMaterialLength {
        length_bounds_mm,
        exact_work: max_exact_work - remaining,
    })
}

/// A connected interval of exterior parameters within a single path span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralExteriorInterval {
    pub parameters: [Q; 2],
    /// Whether the start and end parameters, respectively, are exterior.
    pub endpoints_included: [bool; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathExteriorClip {
    /// Exterior intervals for each path span, ordered by local parameter.
    pub span_intervals: Vec<Vec<PolyhedralExteriorInterval>>,
    pub exact_work: u64,
}

/// Clip a spatial polyline to the exterior of a qualified polyhedral solid.
/// Boundary points are excluded, so a tangency splits an exterior interval.
pub fn exact_polyhedral_path_exterior_clip(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathExteriorClip> {
    let material = exact_polyhedral_path_material_clip(solid, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - material.exact_work;
    let mut span_intervals = Vec::with_capacity(material.span_intervals.len());
    for intervals in material.span_intervals {
        spend(&mut remaining, 4)?;
        let mut exterior = Vec::new();
        let mut start = Q::zero();
        let mut include_start = true;
        for [lo, hi] in intervals {
            spend(&mut remaining, 4)?;
            if start < lo {
                exterior.push(PolyhedralExteriorInterval {
                    parameters: [start, lo],
                    endpoints_included: [include_start, false],
                });
            }
            start = hi;
            include_start = false;
        }
        if start < Q::one() {
            exterior.push(PolyhedralExteriorInterval {
                parameters: [start, Q::one()],
                endpoints_included: [include_start, true],
            });
        }
        span_intervals.push(exterior);
    }
    Ok(ExactPolyhedralPathExteriorClip {
        span_intervals,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathBoundaryEvent {
    /// Closed range of global parameters, measured as span index plus local parameter.
    /// A single parameter denotes an isolated contact.
    pub parameters: [Q; 2],
    /// Interior or exterior immediately before contact. None at path start.
    pub before: Option<PolyhedralPathLocation>,
    /// Interior or exterior immediately after contact. None at path end.
    pub after: Option<PolyhedralPathLocation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathBoundaryEvents {
    pub events: Vec<PolyhedralPathBoundaryEvent>,
    pub exact_work: u64,
}

/// Report connected contacts with the boundary along an ordered polyline.
/// Boundary ranges meeting at a shared path vertex form a single event.
pub fn exact_polyhedral_path_boundary_events(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathBoundaryEvents> {
    let partition = exact_polyhedral_path_partition(solid, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - partition.exact_work;
    let mut events = Vec::new();
    let mut previous = None;
    let mut pending: Option<PolyhedralPathBoundaryEvent> = None;
    for (span, parts) in partition.spans.into_iter().enumerate() {
        let origin = Q::from_integer(span.into());
        for part in parts {
            spend(&mut remaining, 8)?;
            if part.location == PolyhedralPathLocation::Boundary {
                let [lo, hi] = part.parameters;
                let end = &origin + hi;
                if let Some(event) = &mut pending {
                    event.parameters[1] = end;
                } else {
                    pending = Some(PolyhedralPathBoundaryEvent {
                        parameters: [&origin + lo, end],
                        before: previous,
                        after: None,
                    });
                }
            } else {
                previous = Some(part.location);
                if let Some(mut event) = pending.take() {
                    event.after = previous;
                    events.push(event);
                }
            }
        }
    }
    events.extend(pending);
    Ok(ExactPolyhedralPathBoundaryEvents {
        events,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPolyhedralPathLengthBreakdown {
    pub interior_bounds_mm: [Q; 2],
    pub boundary_bounds_mm: [Q; 2],
    pub exterior_bounds_mm: [Q; 2],
    pub exact_work: u64,
}

/// Enclose the distance traveled in each location. The requested absolute
/// tolerance bounds the width of each enclosure independently.
pub fn bounded_polyhedral_path_length_breakdown(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    tolerance_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralPathLengthBreakdown> {
    let tolerance = enclosure_width(tolerance_mm)?;
    let partition = exact_polyhedral_path_partition(solid, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - partition.exact_work;
    let [mut interior, mut boundary, mut exterior] = [Vec::new(), Vec::new(), Vec::new()];
    for (pair, parts) in points_mm.windows(2).zip(partition.spans) {
        spend(&mut remaining, 12)?;
        let mut fractions = [Q::zero(), Q::zero(), Q::zero()];
        for part in parts {
            spend(&mut remaining, 4)?;
            let [lo, hi] = part.parameters;
            let index = match part.location {
                PolyhedralPathLocation::Interior => 0,
                PolyhedralPathLocation::Boundary => 1,
                PolyhedralPathLocation::Exterior => 2,
            };
            fractions[index] += hi - lo;
        }
        let direction = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        let length_squared = dot(&direction, &direction);
        if length_squared.is_zero() {
            continue;
        }
        for (fraction, squares) in
            fractions
                .into_iter()
                .zip([&mut interior, &mut boundary, &mut exterior])
        {
            if fraction.is_positive() {
                spend(&mut remaining, 4)?;
                squares.push(fraction.pow(2) * &length_squared);
            }
        }
    }
    Ok(BoundedPolyhedralPathLengthBreakdown {
        interior_bounds_mm: bound_root_sum(interior, tolerance.clone(), &mut remaining)?,
        boundary_bounds_mm: bound_root_sum(boundary, tolerance.clone(), &mut remaining)?,
        exterior_bounds_mm: bound_root_sum(exterior, tolerance, &mut remaining)?,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralMaterialPathComponent {
    /// Closed interval in span-index-plus-local-parameter coordinates.
    pub parameters: [Q; 2],
    /// Exact vertices in millimeters. A point-only component has one vertex.
    pub points_mm: Vec<[Q; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralMaterialPathComponents {
    pub components: Vec<PolyhedralMaterialPathComponent>,
    pub exact_work: u64,
}

/// Reconstruct closed material portions as exact spatial polylines in path
/// order. Components are connected in the input parameter domain.
pub fn exact_polyhedral_material_path_components(
    solid: &QualifiedClosedSolid,
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralMaterialPathComponents> {
    let clip = exact_polyhedral_path_material_clip(solid, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - clip.exact_work;
    let mut components: Vec<PolyhedralMaterialPathComponent> = Vec::new();
    for (span, (pair, intervals)) in points_mm.windows(2).zip(clip.span_intervals).enumerate() {
        spend(&mut remaining, 4)?;
        let origin = Q::from_integer(span.into());
        let direction: [Q; 3] = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        let evaluate = |t: &Q| std::array::from_fn(|i| &pair[0][i] + t * &direction[i]);
        for [lo, hi] in intervals {
            spend(&mut remaining, 16)?;
            let parameters = [&origin + &lo, &origin + &hi];
            let end = evaluate(&hi);
            if let Some(component) = components.last_mut()
                && component.parameters[1] == parameters[0]
            {
                component.parameters[1] = parameters[1].clone();
                if component.points_mm.last() != Some(&end) {
                    component.points_mm.push(end);
                }
            } else {
                let start = evaluate(&lo);
                let points_mm = if start == end {
                    vec![start]
                } else {
                    vec![start, end]
                };
                components.push(PolyhedralMaterialPathComponent {
                    parameters,
                    points_mm,
                });
            }
        }
    }
    Ok(ExactPolyhedralMaterialPathComponents {
        components,
        exact_work: max_exact_work - remaining,
    })
}

/// Clip a spatial polyline to the closed material shared by all input solids.
/// Touching solids can yield point-only or boundary-only intervals.
pub fn exact_polyhedral_path_common_material_clip(
    solids: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathMaterialClip> {
    let Some((first, rest)) = solids.split_first() else {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Material intersection requires at least one solid",
        ));
    };
    let mut intersection = exact_polyhedral_path_material_clip(first, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - intersection.exact_work;
    for solid in rest {
        if remaining == 0 {
            return Err(super::work_exhausted());
        }
        let clip = exact_polyhedral_path_material_clip(solid, points_mm, remaining)?;
        remaining -= clip.exact_work;
        for (shared, next) in intersection
            .span_intervals
            .iter_mut()
            .zip(clip.span_intervals)
        {
            spend(&mut remaining, 1)?;
            let mut overlap = Vec::new();
            let mut left = shared.iter().peekable();
            let mut right = next.iter().peekable();
            while let (Some([a_lo, a_hi]), Some([b_lo, b_hi])) =
                (left.peek().copied(), right.peek().copied())
            {
                spend(&mut remaining, 8)?;
                let lo = a_lo.max(b_lo);
                let hi = a_hi.min(b_hi);
                if lo <= hi {
                    overlap.push([lo.clone(), hi.clone()]);
                }
                match a_hi.cmp(b_hi) {
                    std::cmp::Ordering::Less => {
                        left.next();
                    }
                    std::cmp::Ordering::Greater => {
                        right.next();
                    }
                    std::cmp::Ordering::Equal => {
                        left.next();
                        right.next();
                    }
                }
            }
            *shared = overlap;
        }
    }
    intersection.exact_work = max_exact_work - remaining;
    Ok(intersection)
}

/// Clip to the union of the closed material of all input solids.
/// Closed intervals that meet at a shared path parameter are merged.
pub fn exact_polyhedral_path_union_material_clip(
    solids: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathMaterialClip> {
    let Some((first, rest)) = solids.split_first() else {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Material union requires at least one solid",
        ));
    };
    let mut union = exact_polyhedral_path_material_clip(first, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - union.exact_work;
    for solid in rest {
        if remaining == 0 {
            return Err(super::work_exhausted());
        }
        let clip = exact_polyhedral_path_material_clip(solid, points_mm, remaining)?;
        remaining -= clip.exact_work;
        for (combined, next) in union.span_intervals.iter_mut().zip(clip.span_intervals) {
            spend(&mut remaining, 1 + next.len() as u64)?;
            combined.extend(next);
        }
    }
    if !rest.is_empty() {
        for combined in &mut union.span_intervals {
            *combined = merge_closed_intervals(std::mem::take(combined), &mut remaining)?;
        }
    }
    union.exact_work = max_exact_work - remaining;
    Ok(union)
}
