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
    let mut contacts = Vec::new();
    let direction: [Q; 3] = std::array::from_fn(|i| &end[i] - &start[i]);
    spend(remaining, 3)?;
    for (&face, boundary) in faces {
        let intervals =
            spatial_segment_region_contacts(boundary, start, end, &direction, remaining)?;
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

fn spatial_segment_region_contacts(
    boundary: &FaceBoundary,
    start: &[Q; 3],
    end: &[Q; 3],
    direction: &[Q; 3],
    remaining: &mut u64,
) -> Result<Vec<[Q; 2]>> {
    use super::section_loops::clip::clip_planar_segment;

    spend(remaining, 24)?;
    let normal = face_normal(boundary)?;
    let delta = std::array::from_fn(|i| &boundary.origin[i] - &start[i]);
    let numerator = dot(normal, &delta);
    let denominator = dot(normal, direction);
    let axes = plane_axes(normal);
    if denominator.is_zero() {
        if !numerator.is_zero() {
            return Ok(Vec::new());
        }
        return clip_planar_segment(
            boundary.segments.iter().map(|[a, b]| (a, b)),
            axes,
            start,
            end,
            remaining,
            |point, remaining| {
                Ok(point_in_face(&boundary.segments, point, axes, remaining)?
                    != FaceLocation::Outside)
            },
        );
    }
    let t = numerator / denominator;
    if t.is_negative() || t > Q::one() {
        return Ok(Vec::new());
    }
    let point = std::array::from_fn(|i| &start[i] + &t * &direction[i]);
    if point_in_face(&boundary.segments, &point, axes, remaining)? == FaceLocation::Outside {
        return Ok(Vec::new());
    }
    Ok(vec![[t.clone(), t]])
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
            *shared = intersect_closed_intervals(shared, &next, &mut remaining)?;
        }
    }
    intersection.exact_work = max_exact_work - remaining;
    Ok(intersection)
}

fn intersect_closed_intervals(
    first: &[[Q; 2]],
    second: &[[Q; 2]],
    remaining: &mut u64,
) -> Result<Vec<[Q; 2]>> {
    let mut overlap = Vec::new();
    let mut left = first.iter().peekable();
    let mut right = second.iter().peekable();
    while let (Some([a_lo, a_hi]), Some([b_lo, b_hi])) =
        (left.peek().copied(), right.peek().copied())
    {
        spend(remaining, 8)?;
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
    Ok(overlap)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathDifferenceInterval {
    pub parameters: [Q; 2],
    /// Whether the start and end parameters belong to the difference.
    pub endpoints_included: [bool; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathMaterialDifference {
    /// Ordered intervals of remaining material for each local span.
    pub span_intervals: Vec<Vec<PolyhedralPathDifferenceInterval>>,
    pub exact_work: u64,
}

/// Remove points belonging to any cutter's closed material from the stock's
/// closed material along a polyline. An empty cutter list retains all stock.
pub fn exact_polyhedral_path_material_difference(
    stock: &QualifiedClosedSolid,
    cutters: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathMaterialDifference> {
    let stock = exact_polyhedral_path_material_clip(stock, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - stock.exact_work;
    let removed = if cutters.is_empty() {
        vec![Vec::new(); stock.span_intervals.len()]
    } else {
        if remaining == 0 {
            return Err(super::work_exhausted());
        }
        let clip = exact_polyhedral_path_union_material_clip(cutters, points_mm, remaining)?;
        remaining -= clip.exact_work;
        clip.span_intervals
    };
    let mut span_intervals = Vec::with_capacity(stock.span_intervals.len());
    for (stock, removed) in stock.span_intervals.into_iter().zip(removed) {
        spend(&mut remaining, 4)?;
        let mut kept = Vec::new();
        let mut cuts = removed.into_iter().peekable();
        'stock: for [lo, hi] in stock {
            spend(&mut remaining, 4)?;
            let mut start = lo;
            let mut include_start = true;
            while let Some([cut_lo, cut_hi]) = cuts.peek() {
                spend(&mut remaining, 8)?;
                if cut_lo > &hi {
                    break;
                }
                if cut_hi < &start {
                    cuts.next();
                    continue;
                }
                if &start < cut_lo {
                    kept.push(PolyhedralPathDifferenceInterval {
                        parameters: [start, cut_lo.clone()],
                        endpoints_included: [include_start, false],
                    });
                }
                if cut_hi >= &hi {
                    continue 'stock;
                }
                start = cut_hi.clone();
                include_start = false;
                cuts.next();
            }
            kept.push(PolyhedralPathDifferenceInterval {
                parameters: [start, hi],
                endpoints_included: [include_start, true],
            });
        }
        span_intervals.push(kept);
    }
    Ok(ExactPolyhedralPathMaterialDifference {
        span_intervals,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathCoveragePart {
    pub parameters: [Q; 2],
    pub endpoints_included: [bool; 2],
    /// Sorted indices into the input solid list, possibly empty.
    pub solids: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathCoverage {
    /// Ordered, disjoint parts covering the full parameter domain of each span.
    pub spans: Vec<Vec<PolyhedralPathCoveragePart>>,
    pub exact_work: u64,
}

/// Partition a path by which solids contain each point in their closed
/// material. Shared boundary points can have a different membership from
/// both adjacent open intervals.
pub fn exact_polyhedral_path_coverage(
    solids: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathCoverage> {
    use std::collections::BTreeSet;

    if solids.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Path coverage requires at least one solid",
        ));
    }
    let mut remaining = max_exact_work;
    let mut spans = Vec::new();
    for (index, solid) in solids.iter().enumerate() {
        if remaining == 0 {
            return Err(super::work_exhausted());
        }
        let clip = exact_polyhedral_path_material_clip(solid, points_mm, remaining)?;
        remaining -= clip.exact_work;
        spans.resize_with(
            clip.span_intervals.len(),
            BTreeMap::<Q, [Vec<usize>; 2]>::new,
        );
        for (events, intervals) in spans.iter_mut().zip(clip.span_intervals) {
            spend(&mut remaining, 1)?;
            for [lo, hi] in intervals {
                let depth = u64::from((events.len() + 2).ilog2()) + 1;
                spend(&mut remaining, 16 * depth)?;
                events.entry(lo).or_default()[0].push(index);
                events.entry(hi).or_default()[1].push(index);
            }
        }
    }

    let mut coverage = Vec::with_capacity(spans.len());
    for mut events in spans {
        let depth = u64::from((events.len() + 2).ilog2()) + 1;
        spend(&mut remaining, 16 * depth)?;
        events.entry(Q::zero()).or_default();
        events.entry(Q::one()).or_default();
        let mut active = BTreeSet::new();
        let mut parts = Vec::new();
        let mut previous = None;
        let active_depth = u64::from(solids.len().ilog2()) + 1;
        for (at, [starts, ends]) in events {
            if let Some(lo) = previous {
                spend(&mut remaining, 4 + active.len() as u64)?;
                append_coverage_part(&mut parts, [lo, at.clone()], [false, false], &active);
            }
            spend(
                &mut remaining,
                active_depth.saturating_mul((starts.len() + ends.len()) as u64),
            )?;
            active.extend(starts);
            spend(&mut remaining, 4 + active.len() as u64)?;
            append_coverage_part(&mut parts, [at.clone(), at.clone()], [true, true], &active);
            for index in ends {
                active.remove(&index);
            }
            previous = Some(at);
        }
        coverage.push(parts);
    }
    Ok(ExactPolyhedralPathCoverage {
        spans: coverage,
        exact_work: max_exact_work - remaining,
    })
}

fn append_coverage_part(
    parts: &mut Vec<PolyhedralPathCoveragePart>,
    parameters: [Q; 2],
    endpoints_included: [bool; 2],
    active: &std::collections::BTreeSet<usize>,
) {
    if let Some(last) = parts.last_mut()
        && last.solids.iter().eq(active)
    {
        if parameters[1] > last.parameters[1] {
            last.endpoints_included[1] = endpoints_included[1];
            last.parameters[1] = parameters[1].clone();
        } else {
            last.endpoints_included[1] |= endpoints_included[1];
        }
    } else {
        parts.push(PolyhedralPathCoveragePart {
            parameters,
            endpoints_included,
            solids: active.iter().copied().collect(),
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathCoverageLength {
    pub solid_count: usize,
    /// Inclusive bounds in millimeters for the distance with this count.
    pub length_bounds_mm: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPolyhedralPathCoverageLengths {
    /// Counts with positive traveled length in ascending order. Zero counts
    /// describe free travel. Isolated contacts contribute no distance.
    pub lengths: Vec<PolyhedralPathCoverageLength>,
    pub exact_work: u64,
}

/// Enclose traveled length by the number of solids containing each path
/// point in their closed material. The tolerance applies to each count.
pub fn bounded_polyhedral_path_coverage_lengths(
    solids: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    tolerance_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralPathCoverageLengths> {
    let tolerance = enclosure_width(tolerance_mm)?;
    let coverage = exact_polyhedral_path_coverage(solids, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - coverage.exact_work;
    let map_cost = 8 * (u64::from(solids.len().ilog2()) + 1);
    let mut squares: BTreeMap<usize, Vec<Q>> = BTreeMap::new();
    for (pair, parts) in points_mm.windows(2).zip(coverage.spans) {
        spend(&mut remaining, 12)?;
        let direction = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        let length_squared = dot(&direction, &direction);
        if length_squared.is_zero() {
            continue;
        }
        let mut fractions = BTreeMap::<usize, Q>::new();
        for part in parts {
            spend(&mut remaining, 4)?;
            let [lo, hi] = part.parameters;
            if lo < hi {
                spend(&mut remaining, map_cost)?;
                *fractions.entry(part.solids.len()).or_default() += hi - lo;
            }
        }
        for (solid_count, fraction) in fractions {
            spend(&mut remaining, map_cost)?;
            squares
                .entry(solid_count)
                .or_default()
                .push(fraction.pow(2) * &length_squared);
        }
    }
    let mut lengths = Vec::with_capacity(squares.len());
    for (solid_count, terms) in squares {
        lengths.push(PolyhedralPathCoverageLength {
            solid_count,
            length_bounds_mm: bound_root_sum(terms, tolerance.clone(), &mut remaining)?,
        });
    }
    Ok(BoundedPolyhedralPathCoverageLengths {
        lengths,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralPathCoverageComponent {
    /// Global endpoints in span-index-plus-local-parameter coordinates.
    pub parameters: [Q; 2],
    pub endpoints_included: [bool; 2],
    /// Exact vertices of the component closure, including excluded ends.
    pub points_mm: Vec<[Q; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralPathCoverageComponents {
    pub components: Vec<PolyhedralPathCoverageComponent>,
    pub exact_work: u64,
}

/// Reconstruct connected path components whose closed material coverage
/// count falls in the inclusive range. Connectivity uses path parameters.
pub fn exact_polyhedral_path_coverage_components(
    solids: &[&QualifiedClosedSolid],
    points_mm: &[[Q; 3]],
    count: std::ops::RangeInclusive<usize>,
    max_exact_work: u64,
) -> Result<ExactPolyhedralPathCoverageComponents> {
    if count.is_empty() || *count.end() > solids.len() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Coverage count range must be within the solid count",
        ));
    }
    let coverage = exact_polyhedral_path_coverage(solids, points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - coverage.exact_work;
    let mut components: Vec<PolyhedralPathCoverageComponent> = Vec::new();
    for (span, (pair, parts)) in points_mm.windows(2).zip(coverage.spans).enumerate() {
        spend(&mut remaining, 4)?;
        let origin = Q::from_integer(span.into());
        let direction: [Q; 3] = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        let evaluate = |t: &Q| std::array::from_fn(|i| &pair[0][i] + t * &direction[i]);
        for part in parts {
            spend(&mut remaining, 2)?;
            if !count.contains(&part.solids.len()) {
                continue;
            }
            spend(&mut remaining, 16)?;
            let [lo, hi] = part.parameters;
            let parameters = [&origin + &lo, &origin + &hi];
            let [include_start, include_end] = part.endpoints_included;
            let end = evaluate(&hi);
            if let Some(component) = components.last_mut()
                && component.parameters[1] == parameters[0]
                && (component.endpoints_included[1] || include_start)
            {
                component.endpoints_included[1] = include_end
                    || (component.parameters[1] == parameters[1]
                        && component.endpoints_included[1]);
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
                components.push(PolyhedralPathCoverageComponent {
                    parameters,
                    endpoints_included: part.endpoints_included,
                    points_mm,
                });
            }
        }
    }
    Ok(ExactPolyhedralPathCoverageComponents {
        components,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSpatialPolylineSimplification {
    pub points_mm: Vec<[Q; 3]>,
    /// Original vertex index for each retained point. Consecutive duplicate
    /// positions are represented by their first occurrence.
    pub source_vertices: Vec<usize>,
    pub exact_work: u64,
}

/// Remove consecutive duplicate vertices and vertices strictly between
/// their neighbors on a straight segment. Preserve the oriented trace,
/// including reversals and the anchor of a closed path.
pub fn simplify_exact_spatial_polyline(
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactSpatialPolylineSimplification> {
    super::validate_work(max_exact_work)?;
    if points_mm.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Polyline requires at least one point",
        ));
    }
    let mut remaining = max_exact_work;
    let mut source_vertices: Vec<usize> = Vec::new();
    for (index, point) in points_mm.iter().enumerate() {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
        if source_vertices
            .last()
            .is_some_and(|&last| points_mm[last] == *point)
        {
            continue;
        }
        while let [.., before, last] = source_vertices.as_slice() {
            spend(&mut remaining, 20)?;
            let a = &points_mm[*before];
            let b = &points_mm[*last];
            let Some(axis) = (0..3).find(|&i| a[i] != point[i]) else {
                break;
            };
            let span = &point[axis] - &a[axis];
            let offset = &b[axis] - &a[axis];
            let t = offset / span;
            if t <= Q::zero()
                || t >= Q::one()
                || (0..3).any(|i| &b[i] - &a[i] != &t * (&point[i] - &a[i]))
            {
                break;
            }
            source_vertices.pop();
        }
        source_vertices.push(index);
    }
    let points_mm = source_vertices
        .iter()
        .map(|&index| points_mm[index].clone())
        .collect();
    Ok(ExactSpatialPolylineSimplification {
        points_mm,
        source_vertices,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialPolylineProjectionHit {
    /// Global parameter interval. A stationary span can attain the minimum
    /// throughout its interval. A singleton input has parameter zero.
    pub parameters: [Q; 2],
    pub point_mm: [Q; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSpatialPolylineProjection {
    pub distance_squared_mm2: Q,
    /// Minimizers in path order, merged at common span endpoints.
    pub hits: Vec<SpatialPolylineProjectionHit>,
    pub exact_work: u64,
}

/// Project a rational point onto a rational spatial polyline using exact
/// arithmetic. Distinct visits to the same closest point remain distinct.
pub fn project_point_to_exact_spatial_polyline(
    points_mm: &[[Q; 3]],
    query_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactSpatialPolylineProjection> {
    super::validate_work(max_exact_work)?;
    let mut remaining = max_exact_work;
    spend(&mut remaining, 4)?;
    validate_point(query_mm)?;
    if points_mm.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Polyline requires at least one point",
        ));
    }
    for point in points_mm {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let first = &points_mm[0];
    let mut distance_squared_mm2: Q = first
        .iter()
        .zip(query_mm)
        .map(|(p, q)| (p - q).pow(2))
        .sum();
    let mut hits: Vec<SpatialPolylineProjectionHit> = vec![SpatialPolylineProjectionHit {
        parameters: [Q::zero(), Q::zero()],
        point_mm: first.clone(),
    }];
    for (span, pair) in points_mm.windows(2).enumerate() {
        spend(&mut remaining, 32)?;
        let (point_mm, distance, t) = super::project_segment_point(query_mm, &pair[0], &pair[1]);
        if distance > distance_squared_mm2 {
            continue;
        }
        if distance < distance_squared_mm2 {
            distance_squared_mm2 = distance;
            hits.clear();
        }
        let origin = Q::from_integer(span.into());
        let [lo, hi] = match t {
            Some(t) => {
                let at = origin + t;
                [at.clone(), at]
            }
            None => [origin.clone(), origin + Q::one()],
        };
        if let Some(last) = hits.last_mut()
            && last.parameters[1] == lo
        {
            last.parameters[1] = hi;
        } else {
            hits.push(SpatialPolylineProjectionHit {
                parameters: [lo, hi],
                point_mm,
            });
        }
    }
    Ok(ExactSpatialPolylineProjection {
        distance_squared_mm2,
        hits,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpatialPolylineIntersection {
    pub spans: [usize; 2],
    /// Local endpoint parameters for each span, in first-span traversal
    /// order. The second pair can descend. A stationary span has [0, 1].
    pub parameters: [[Q; 2]; 2],
    /// Coincident spatial endpoints. Equal endpoints denote point contact.
    pub points_mm: [[Q; 3]; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSpatialPolylineIntersections {
    /// One record per intersecting span pair, in lexicographic span order.
    pub intersections: Vec<SpatialPolylineIntersection>,
    pub exact_work: u64,
}

/// Intersect two rational spatial polylines. Shared vertices can appear
/// in multiple records, preserving their incident spans.
pub fn intersect_exact_spatial_polylines(
    first_mm: &[[Q; 3]],
    second_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactSpatialPolylineIntersections> {
    super::validate_work(max_exact_work)?;
    if first_mm.len() < 2 || second_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Each intersection path requires two or more points",
        ));
    }
    let mut remaining = max_exact_work;
    for point in first_mm.iter().chain(second_mm) {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let mut intersections = Vec::new();
    for (i, a) in first_mm.windows(2).enumerate() {
        for (j, b) in second_mm.windows(2).enumerate() {
            spend(&mut remaining, 64)?;
            if let Some((parameters, points_mm)) = intersect_spatial_segments(a, b) {
                intersections.push(SpatialPolylineIntersection {
                    spans: [i, j],
                    parameters,
                    points_mm,
                });
            }
        }
    }
    Ok(ExactSpatialPolylineIntersections {
        intersections,
        exact_work: max_exact_work - remaining,
    })
}

fn intersect_spatial_segments(a: &[[Q; 3]], b: &[[Q; 3]]) -> Option<([[Q; 2]; 2], [[Q; 3]; 2])> {
    let unit = Q::zero()..=Q::one();
    let (u, v, offset): ([Q; 3], [Q; 3], [Q; 3]) = (
        std::array::from_fn(|i| &a[1][i] - &a[0][i]),
        std::array::from_fn(|i| &b[1][i] - &b[0][i]),
        std::array::from_fn(|i| &b[0][i] - &a[0][i]),
    );
    let point_on_a = |t: &Q| std::array::from_fn(|i| &a[0][i] + t * &u[i]);
    let point_on_b = |t: &Q| std::array::from_fn(|i| &b[0][i] + t * &v[i]);
    let a_axis = u.iter().position(|coord| !coord.is_zero());
    let b_axis = v.iter().position(|coord| !coord.is_zero());
    let (Some(axis), Some(b_axis)) = (a_axis, b_axis) else {
        let (point, target, swapped) = if a_axis.is_none() {
            (&a[0], b, false)
        } else {
            (&b[0], a, true)
        };
        let (_, distance, t) = super::project_segment_point(point, &target[0], &target[1]);
        let span = || [Q::zero(), Q::one()];
        let projected = t.map_or_else(span, |t| [t.clone(), t]);
        let parameters = if swapped {
            [projected, span()]
        } else {
            [span(), projected]
        };
        return distance
            .is_zero()
            .then(|| (parameters, [point.clone(), point.clone()]));
    };
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let determinant = &u[j] * &v[i] - &u[i] * &v[j];
        if !determinant.is_zero() {
            let ta = (&offset[j] * &v[i] - &offset[i] * &v[j]) / &determinant;
            let tb = (&u[i] * &offset[j] - &u[j] * &offset[i]) / determinant;
            if !unit.contains(&ta) || !unit.contains(&tb) {
                return None;
            }
            let point = point_on_a(&ta);
            return (point == point_on_b(&tb))
                .then(|| ([[ta.clone(), ta], [tb.clone(), tb]], [point.clone(), point]));
        }
    }
    let start = &offset[axis] / &u[axis];
    if point_on_a(&start) != b[0] {
        return None;
    }
    let end = (&b[1][axis] - &a[0][axis]) / &u[axis];
    let lo = start.clone().min(end.clone()).max(Q::zero());
    let hi = start.max(end).min(Q::one());
    if lo > hi {
        return None;
    }
    let points = [point_on_a(&lo), point_on_a(&hi)];
    let on_b = points
        .each_ref()
        .map(|q| (&q[b_axis] - &b[0][b_axis]) / &v[b_axis]);
    Some(([[lo, hi], on_b], points))
}

/// Find spatial self-intersections and retraced intervals, using original
/// span indices. Stationary spans are ignored when deciding adjacency.
/// The shared vertex of adjacent spans, including the seam of a closed path,
/// is omitted when it is their only intersection.
pub fn self_intersections_exact_spatial_polyline(
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactSpatialPolylineIntersections> {
    super::validate_work(max_exact_work)?;
    if points_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Self-intersection path requires at least two points",
        ));
    }
    let mut remaining = max_exact_work;
    for point in points_mm {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let mut spans = Vec::new();
    for (index, pair) in points_mm.windows(2).enumerate() {
        spend(&mut remaining, 4)?;
        if pair[0] != pair[1] {
            spans.push((index, pair));
        }
    }
    let closed = points_mm.first() == points_mm.last();
    let mut intersections = Vec::new();
    for (i, &(first, a)) in spans.iter().enumerate() {
        for (j, &(second, b)) in spans.iter().enumerate().skip(i + 1) {
            spend(&mut remaining, 64)?;
            let adjacent = j == i + 1 || (closed && i == 0 && j + 1 == spans.len());
            if let Some((parameters, points_mm)) = intersect_spatial_segments(a, b)
                && (!adjacent || points_mm[0] != points_mm[1])
            {
                intersections.push(SpatialPolylineIntersection {
                    spans: [first, second],
                    parameters,
                    points_mm,
                });
            }
        }
    }
    Ok(ExactSpatialPolylineIntersections {
        intersections,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSpatialPolylineDistance {
    pub distance_squared_mm2: Q,
    pub spans: [usize; 2],
    pub parameters: [Q; 2],
    pub points_mm: [[Q; 3]; 2],
    pub exact_work: u64,
}

/// Find the exact squared distance and a closest pair on two spatial
/// polylines. Ties favor the first span pair, then the earliest local
/// parameters. A repeated vertex can represent a point as a stationary span.
pub fn distance_between_exact_spatial_polylines(
    first_mm: &[[Q; 3]],
    second_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactSpatialPolylineDistance> {
    super::validate_work(max_exact_work)?;
    if first_mm.len() < 2 || second_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Each distance path requires two or more points",
        ));
    }
    let mut remaining = max_exact_work;
    for point in first_mm.iter().chain(second_mm) {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let mut best: Option<ExactSpatialPolylineDistance> = None;
    for (i, a) in first_mm.windows(2).enumerate() {
        for (j, b) in second_mm.windows(2).enumerate() {
            spend(&mut remaining, 160)?;
            let (distance_squared_mm2, parameters, points_mm) = closest_spatial_segments(a, b);
            if best
                .as_ref()
                .is_none_or(|entry| distance_squared_mm2 < entry.distance_squared_mm2)
            {
                best = Some(ExactSpatialPolylineDistance {
                    distance_squared_mm2,
                    spans: [i, j],
                    parameters,
                    points_mm,
                    exact_work: 0,
                });
            }
        }
    }
    let mut best = best.expect("validated paths have at least one span pair");
    best.exact_work = max_exact_work - remaining;
    Ok(best)
}

fn closest_spatial_segments(a: &[[Q; 3]], b: &[[Q; 3]]) -> (Q, [Q; 2], [[Q; 3]; 2]) {
    let mut best = None;
    let mut offer = |distance: Q, parameters: [Q; 2], points: [[Q; 3]; 2]| {
        if best
            .as_ref()
            .is_none_or(|(current, at, _)| (&distance, &parameters) < (current, at))
        {
            best = Some((distance, parameters, points));
        }
    };
    // A minimum on the boundary of the parameter square is an endpoint
    // projection. Rank-deficient interior minima also reach this boundary.
    for index in 0..2 {
        let at = Q::from_integer(index.into());
        let (closest, distance, t) = super::project_segment_point(&a[index], &b[0], &b[1]);
        offer(
            distance,
            [at.clone(), t.unwrap_or_default()],
            [a[index].clone(), closest],
        );
        let (closest, distance, t) = super::project_segment_point(&b[index], &a[0], &a[1]);
        offer(
            distance,
            [t.unwrap_or_default(), at],
            [closest, b[index].clone()],
        );
    }

    let u = std::array::from_fn(|i| &a[1][i] - &a[0][i]);
    let v = std::array::from_fn(|i| &b[1][i] - &b[0][i]);
    let uu = dot(&u, &u);
    let vv = dot(&v, &v);
    let uv = dot(&u, &v);
    let determinant = &uu * &vv - uv.pow(2);
    if determinant.is_positive() {
        let offset = std::array::from_fn(|i| &b[0][i] - &a[0][i]);
        let du = dot(&u, &offset);
        let dv = dot(&v, &offset);
        let t = (&vv * &du - &uv * &dv) / &determinant;
        let s = (&uv * &du - &uu * &dv) / determinant;
        let unit = Q::zero()..=Q::one();
        if unit.contains(&t) && unit.contains(&s) {
            let first: [Q; 3] = std::array::from_fn(|i| &a[0][i] + &t * &u[i]);
            let second: [Q; 3] = std::array::from_fn(|i| &b[0][i] + &s * &v[i]);
            let distance = first.iter().zip(&second).map(|(p, q)| (p - q).pow(2)).sum();
            offer(distance, [t, s], [first, second]);
        }
    }
    best.expect("endpoint projections always exist")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedSpatialPolylineSimplification {
    pub points_mm: Vec<[Q; 3]>,
    pub source_vertices: Vec<usize>,
    /// Squared error bound for an order-preserving continuous traversal.
    pub deviation_squared_mm2: Q,
    pub exact_work: u64,
}

/// Simplify a spatial polyline within a rational distance tolerance. A chord
/// is accepted only if its clamped projections of the source vertices are
/// monotone and within tolerance. Interpolating those projections along
/// each original span proves a continuous bound in both directions.
/// Topology preservation requires a separate test.
pub fn simplify_bounded_spatial_polyline(
    points_mm: &[[Q; 3]],
    tolerance_mm: &Q,
    max_exact_work: u64,
) -> Result<BoundedSpatialPolylineSimplification> {
    super::validate_work(max_exact_work)?;
    if points_mm.is_empty()
        || tolerance_mm.is_negative()
        || tolerance_mm.numer().bits().max(tolerance_mm.denom().bits()) > 8192
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Expected a nonempty path and a bounded nonnegative tolerance",
        ));
    }
    let mut remaining = max_exact_work;
    for point in points_mm {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let tolerance_squared = tolerance_mm.pow(2);
    let mut source_vertices = vec![0];
    let mut deviation_squared_mm2 = Q::zero();
    let mut pending = Vec::new();
    if points_mm.len() > 1 {
        pending.push([0, points_mm.len() - 1]);
    }
    while let Some([first, last]) = pending.pop() {
        spend(&mut remaining, 4)?;
        let a = &points_mm[first];
        let b = &points_mm[last];
        let mut previous = Q::zero();
        let mut worst = Q::zero();
        let mut furthest = None;
        let mut reversal = None;
        for (offset, point) in points_mm[first + 1..last].iter().enumerate() {
            spend(&mut remaining, 32)?;
            let (_, distance, t) = super::project_segment_point(point, a, b);
            let t = t.unwrap_or_default();
            let index = first + 1 + offset;
            if t < previous {
                reversal = Some(index);
                break;
            }
            previous = t;
            if distance > worst {
                worst = distance;
                furthest = Some(index);
            }
        }
        if let Some(split) = reversal.or(furthest.filter(|_| a == b || worst > tolerance_squared)) {
            pending.push([split, last]);
            pending.push([first, split]);
        } else {
            deviation_squared_mm2 = deviation_squared_mm2.max(worst);
            source_vertices.push(last);
        }
    }
    let points_mm = source_vertices
        .iter()
        .map(|&index| points_mm[index].clone())
        .collect();
    Ok(BoundedSpatialPolylineSimplification {
        points_mm,
        source_vertices,
        deviation_squared_mm2,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodedSpatialPolylineEdge {
    pub source_span: usize,
    pub parameters: [Q; 2],
    /// Start and end node indices in traversal order.
    pub nodes: [usize; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactNodedSpatialPolyline {
    /// Unique exact positions, indexed in order of first visit.
    pub nodes_mm: Vec<[Q; 3]>,
    /// Directed occurrences in source order, including retracing and stationary spans.
    pub edges: Vec<NodedSpatialPolylineEdge>,
    pub exact_work: u64,
}

/// Split a spatial polyline at self-crossings, nonlocal touches and both
/// ends of every overlap. Coincident positions share a node.
pub fn node_exact_spatial_polyline(
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactNodedSpatialPolyline> {
    let report = self_intersections_exact_spatial_polyline(points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - report.exact_work;
    spend(&mut remaining, 4 * (points_mm.len() as u64 - 1))?;
    let mut cuts = vec![vec![Q::zero(), Q::one()]; points_mm.len() - 1];
    for hit in report.intersections {
        spend(&mut remaining, 4)?;
        for (span, parameters) in hit.spans.into_iter().zip(hit.parameters) {
            cuts[span].extend(parameters);
        }
    }
    let mut nodes_mm = Vec::new();
    let mut indices = BTreeMap::new();
    let mut edges = Vec::new();
    for (source_span, (pair, mut parameters)) in points_mm.windows(2).zip(cuts).enumerate() {
        let count = parameters.len() as u64;
        spend(
            &mut remaining,
            count.saturating_mul(8 * u64::from(count.ilog2() + 1)),
        )?;
        parameters.sort_unstable();
        parameters.dedup();
        let direction: [Q; 3] = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        let mut previous = None;
        for at in parameters {
            let depth = u64::from((indices.len() + 1).ilog2()) + 1;
            spend(&mut remaining, 16 + 8 * depth)?;
            let point = std::array::from_fn(|i| &pair[0][i] + &at * &direction[i]);
            let node = *indices.entry(point.clone()).or_insert_with(|| {
                let index = nodes_mm.len();
                nodes_mm.push(point);
                index
            });
            if let Some((lo, start)) = previous {
                edges.push(NodedSpatialPolylineEdge {
                    source_span,
                    parameters: [lo, at.clone()],
                    nodes: [start, node],
                });
            }
            previous = Some((at, node));
        }
    }
    Ok(ExactNodedSpatialPolyline {
        nodes_mm,
        edges,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineArea {
    /// Oriented area vector in square millimeters, reversed by contour reversal.
    pub area_vector_mm2: [Q; 3],
    pub area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Certify that an explicitly closed spatial polyline encloses a simple
/// planar region and compute its exact area vector and squared area.
pub fn exact_planar_spatial_polyline_area(
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineArea> {
    super::validate_work(max_exact_work)?;
    if points_mm.len() < 4 || points_mm.first() != points_mm.last() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Area requires an explicitly closed contour",
        ));
    }
    let mut remaining = max_exact_work;
    for point in points_mm {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let origin = &points_mm[0];
    let mut double_area = [Q::zero(), Q::zero(), Q::zero()];
    for pair in points_mm.windows(2) {
        spend(&mut remaining, 16)?;
        let a: [Q; 3] = std::array::from_fn(|i| &pair[0][i] - &origin[i]);
        let b: [Q; 3] = std::array::from_fn(|i| &pair[1][i] - &origin[i]);
        for (k, (i, j)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            double_area[k] += &a[i] * &b[j] - &a[j] * &b[i];
        }
    }
    if double_area.iter().all(Zero::is_zero) {
        return Err(refuse("Contour has zero oriented area"));
    }
    for point in points_mm {
        spend(&mut remaining, 12)?;
        let offset = std::array::from_fn(|i| &point[i] - &origin[i]);
        if !dot(&offset, &double_area).is_zero() {
            return Err(refuse("Contour is not planar"));
        }
    }
    if remaining == 0 {
        return Err(super::work_exhausted());
    }
    let topology = self_intersections_exact_spatial_polyline(points_mm, remaining)?;
    remaining -= topology.exact_work;
    if !topology.intersections.is_empty() {
        return Err(refuse("Contour has self-intersections"));
    }
    spend(&mut remaining, 12)?;
    let two = Q::from_integer(2.into());
    let area_vector_mm2 = double_area.map(|coord| coord / &two);
    Ok(ExactPlanarPolylineArea {
        area_squared_mm4: dot(&area_vector_mm2, &area_vector_mm2),
        area_vector_mm2,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineCentroid {
    pub centroid_mm: [Q; 3],
    pub area: ExactPlanarPolylineArea,
    pub exact_work: u64,
}

/// Compute the area centroid of a simple planar spatial polyline.
/// Signed triangle contributions account for concavity and orientation.
pub fn exact_planar_spatial_polyline_centroid(
    points_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineCentroid> {
    let area = exact_planar_spatial_polyline_area(points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - area.exact_work;
    spend(&mut remaining, 16)?;
    let [i, j] = plane_axes(&area.area_vector_mm2);
    let origin = &points_mm[0];
    let mut moment = [Q::zero(), Q::zero(), Q::zero()];
    for pair in points_mm.windows(2) {
        spend(&mut remaining, 20)?;
        let a: [Q; 3] = std::array::from_fn(|k| &pair[0][k] - &origin[k]);
        let b: [Q; 3] = std::array::from_fn(|k| &pair[1][k] - &origin[k]);
        let twice_area = &a[i] * &b[j] - &a[j] * &b[i];
        for k in 0..3 {
            moment[k] += (&a[k] + &b[k]) * &twice_area;
        }
    }
    // plane_axes may return either orientation. Use the corresponding
    // oriented component of the area vector in the triangle denominator.
    let normal_axis = 3 - i - j;
    let sign = if (i + 1) % 3 == j { 6 } else { -6 };
    let denominator = &area.area_vector_mm2[normal_axis] * Q::from_integer(sign.into());
    let centroid_mm = std::array::from_fn(|k| &origin[k] + &moment[k] / &denominator);
    Ok(ExactPlanarPolylineCentroid {
        centroid_mm,
        area,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanarPolylinePointLocation {
    Interior,
    Boundary,
    Exterior,
    OffPlane,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylinePointLocations {
    /// Locations in query input order.
    pub locations: Vec<PlanarPolylinePointLocation>,
    pub exact_work: u64,
}

/// Classify a batch of rational points relative to a simple planar contour.
/// The support and contour are certified once for the whole batch.
pub fn locate_points_in_exact_planar_spatial_polyline(
    points_mm: &[[Q; 3]],
    queries_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylinePointLocations> {
    let area = exact_planar_spatial_polyline_area(points_mm, max_exact_work)?;
    let mut remaining = max_exact_work - area.exact_work;
    let axes = plane_axes(&area.area_vector_mm2);
    spend(&mut remaining, 8 * (points_mm.len() as u64 - 1))?;
    let boundary: Vec<_> = points_mm
        .windows(2)
        .map(|pair| [pair[0].clone(), pair[1].clone()])
        .collect();
    let mut locations = Vec::new();
    for query in queries_mm {
        spend(&mut remaining, 16)?;
        validate_point(query)?;
        let offset = std::array::from_fn(|i| &query[i] - &points_mm[0][i]);
        let location = if dot(&offset, &area.area_vector_mm2).is_zero() {
            match point_in_face(&boundary, query, axes, &mut remaining)? {
                FaceLocation::Inside => PlanarPolylinePointLocation::Interior,
                FaceLocation::Boundary => PlanarPolylinePointLocation::Boundary,
                FaceLocation::Outside => PlanarPolylinePointLocation::Exterior,
            }
        } else {
            PlanarPolylinePointLocation::OffPlane
        };
        locations.push(location);
    }
    Ok(ExactPlanarPolylinePointLocations {
        locations,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylinePathClip {
    /// Closed local parameter intervals per path span, including single points.
    pub span_intervals: Vec<Vec<[Q; 2]>>,
    pub exact_work: u64,
}

/// Intersect a spatial path with the closed planar region of a simple
/// contour. Coplanar spans can produce intervals; transverse spans can
/// produce isolated points. Both include the contour boundary.
pub fn clip_spatial_path_to_exact_planar_polyline(
    contour_mm: &[[Q; 3]],
    path_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylinePathClip> {
    clip_path_to_planar_region(contour_mm, &[], path_mm, max_exact_work)
}

/// Clip to the closed material of a planar polygon with disjoint holes.
/// Hole interiors are excluded and their boundaries are retained.
pub fn clip_spatial_path_to_exact_planar_polyline_region(
    outer_mm: &[[Q; 3]],
    holes_mm: &[&[[Q; 3]]],
    path_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylinePathClip> {
    clip_path_to_planar_region(outer_mm, holes_mm, path_mm, max_exact_work)
}

fn clip_path_to_planar_region(
    outer_mm: &[[Q; 3]],
    holes_mm: &[&[[Q; 3]]],
    path_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylinePathClip> {
    if path_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Clip path requires at least two vertices",
        ));
    }
    let mut remaining = max_exact_work;
    let boundary = prepare_planar_polyline_region(outer_mm, holes_mm, &mut remaining)?;
    for point in path_mm {
        spend(&mut remaining, 4)?;
        validate_point(point)?;
    }
    let mut span_intervals = Vec::new();
    for pair in path_mm.windows(2) {
        spend(&mut remaining, 3)?;
        let direction = std::array::from_fn(|i| &pair[1][i] - &pair[0][i]);
        span_intervals.push(spatial_segment_region_contacts(
            &boundary,
            &pair[0],
            &pair[1],
            &direction,
            &mut remaining,
        )?);
    }
    Ok(ExactPlanarPolylinePathClip {
        span_intervals,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineRegionProjection {
    pub closest_mm: [Q; 3],
    pub distance_squared_mm2: Q,
    /// Location of the returned closest point on the region.
    pub location: PlanarPolylinePointLocation,
    pub exact_work: u64,
}

/// Find a closest point on a filled simple planar contour. If the
/// orthogonal projection lies outside, use the nearest boundary feature.
/// Boundary ties favor the first occurrence in contour order.
pub fn project_point_to_exact_planar_polyline_region(
    contour_mm: &[[Q; 3]],
    query_mm: &[Q; 3],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineRegionProjection> {
    let area = exact_planar_spatial_polyline_area(contour_mm, max_exact_work)?;
    let mut remaining = max_exact_work - area.exact_work;
    spend(&mut remaining, 32 + 8 * (contour_mm.len() as u64 - 1))?;
    validate_point(query_mm)?;
    let normal = &area.area_vector_mm2;
    let offset = std::array::from_fn(|i| &query_mm[i] - &contour_mm[0][i]);
    let height = dot(&offset, normal);
    let scale = &height / &area.area_squared_mm4;
    let projected = std::array::from_fn(|i| &query_mm[i] - &scale * &normal[i]);
    let boundary: Vec<_> = contour_mm
        .windows(2)
        .map(|pair| [pair[0].clone(), pair[1].clone()])
        .collect();
    let location = point_in_face(&boundary, &projected, plane_axes(normal), &mut remaining)?;
    let (closest_mm, distance_squared_mm2, location) = match location {
        FaceLocation::Inside => (
            projected,
            height * scale,
            PlanarPolylinePointLocation::Interior,
        ),
        FaceLocation::Boundary => (
            projected,
            height * scale,
            PlanarPolylinePointLocation::Boundary,
        ),
        FaceLocation::Outside => {
            let mut best = None;
            for [a, b] in &boundary {
                spend(&mut remaining, 32)?;
                let (point, distance) = nearest_segment_point(query_mm, a, b);
                if best.as_ref().is_none_or(|(_, squared)| &distance < squared) {
                    best = Some((point, distance));
                }
            }
            let (point, distance) = best.expect("certified contour has boundary segments");
            (point, distance, PlanarPolylinePointLocation::Boundary)
        }
    };
    Ok(ExactPlanarPolylineRegionProjection {
        closest_mm,
        distance_squared_mm2,
        location,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineTriangulation {
    /// Positive-area triangles in contour orientation, indexing the input.
    /// Consecutive duplicates use their first occurrence.
    pub triangles: Vec<[usize; 3]>,
    pub exact_work: u64,
}

/// Triangulate a simple planar polygon using exact orientation and visibility
/// predicates. Collinear boundary vertices participate in the triangulation.
pub fn triangulate_exact_planar_spatial_polyline(
    contour_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineTriangulation> {
    let area = exact_planar_spatial_polyline_area(contour_mm, max_exact_work)?;
    let mut remaining = max_exact_work - area.exact_work;
    let [u, v] = plane_axes(&area.area_vector_mm2);
    let positive = area.area_vector_mm2[3 - u - v].is_positive();
    let side = |a: usize, b: usize, c: usize| {
        let [a, b, c] = [&contour_mm[a], &contour_mm[b], &contour_mm[c]];
        (&b[u] - &a[u]) * (&c[v] - &a[v]) - (&b[v] - &a[v]) * (&c[u] - &a[u])
    };
    let is_forward = |value: &Q| !value.is_zero() && value.is_positive() == positive;
    let mut ring = vec![0];
    for index in 1..contour_mm.len() - 1 {
        spend(&mut remaining, 4)?;
        if contour_mm[index] != contour_mm[index - 1] {
            ring.push(index);
        }
    }
    if contour_mm[*ring.last().unwrap()] == contour_mm[0] && ring.len() > 1 {
        ring.pop();
    }
    let mut triangles = Vec::new();
    while ring.len() > 3 {
        let mut ear = None;
        for index in 0..ring.len() {
            let triangle = [
                ring[(index + ring.len() - 1) % ring.len()],
                ring[index],
                ring[(index + 1) % ring.len()],
            ];
            spend(&mut remaining, 12)?;
            if !is_forward(&side(triangle[0], triangle[1], triangle[2])) {
                continue;
            }
            let mut blocked = false;
            for &point in &ring {
                spend(&mut remaining, 24)?;
                if triangle.contains(&point) {
                    continue;
                }
                if [(0, 1), (1, 2), (2, 0)].into_iter().all(|(a, b)| {
                    let signed = side(triangle[a], triangle[b], point);
                    signed.is_zero() || is_forward(&signed)
                }) {
                    blocked = true;
                    break;
                }
            }
            if !blocked {
                ear = Some((index, triangle));
                break;
            }
        }
        let (index, triangle) = ear.ok_or_else(|| refuse("No valid triangulation ear"))?;
        spend(&mut remaining, (ring.len() - index) as u64)?;
        ring.remove(index);
        triangles.push(triangle);
    }
    spend(&mut remaining, 12)?;
    let [a, b, c] =
        <[usize; 3]>::try_from(ring).map_err(|_| refuse("Expected three terminal vertices"))?;
    if !is_forward(&side(a, b, c)) {
        return Err(refuse("Degenerate terminal triangle"));
    }
    triangles.push([a, b, c]);
    Ok(ExactPlanarPolylineTriangulation {
        triangles,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineSecondMoments {
    pub centroid_mm: [Q; 3],
    pub area_squared_mm4: Q,
    /// Area-average products of coordinate offsets from the centroid.
    /// Multiplying by area gives the central second moment integrals.
    pub normalized_central_moments_mm2: [[Q; 3]; 3],
    pub exact_work: u64,
}

/// Integrate central second moments of a simple planar profile at uniform
/// area density. Normalization by area preserves exact rational values.
pub fn exact_planar_spatial_polyline_second_moments(
    contour_mm: &[[Q; 3]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineSecondMoments> {
    let properties = exact_planar_spatial_polyline_centroid(contour_mm, max_exact_work)?;
    let mut remaining = max_exact_work - properties.exact_work;
    let normalized_central_moments_mm2 = planar_region_central_moments(
        contour_mm,
        &[],
        &properties.centroid_mm,
        &properties.area.area_vector_mm2,
        &mut remaining,
    )?;
    Ok(ExactPlanarPolylineSecondMoments {
        centroid_mm: properties.centroid_mm,
        area_squared_mm4: properties.area.area_squared_mm4,
        normalized_central_moments_mm2,
        exact_work: max_exact_work - remaining,
    })
}

/// Integrate the area-normalized central second moments of a planar profile
/// with disjoint holes. Hole order and orientation do not affect the tensor.
pub fn exact_planar_spatial_polyline_region_second_moments(
    outer_mm: &[[Q; 3]],
    holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineSecondMoments> {
    if holes_mm.is_empty() {
        return exact_planar_spatial_polyline_second_moments(outer_mm, max_exact_work);
    }
    let properties = exact_planar_spatial_polyline_region_mass(outer_mm, holes_mm, max_exact_work)?;
    let mut remaining = max_exact_work - properties.exact_work;
    let normalized_central_moments_mm2 = planar_region_central_moments(
        outer_mm,
        holes_mm,
        &properties.centroid_mm,
        &properties.area_vector_mm2,
        &mut remaining,
    )?;
    Ok(ExactPlanarPolylineSecondMoments {
        centroid_mm: properties.centroid_mm,
        area_squared_mm4: properties.area_squared_mm4,
        normalized_central_moments_mm2,
        exact_work: max_exact_work - remaining,
    })
}

fn planar_region_central_moments(
    outer: &[[Q; 3]],
    holes: &[&[[Q; 3]]],
    centroid: &[Q; 3],
    area_vector: &[Q; 3],
    remaining: &mut u64,
) -> Result<[[Q; 3]; 3]> {
    let [u, v] = plane_axes(area_vector);
    let normal_area = &area_vector[3 - u - v];
    let denominator = normal_area.abs() * Q::from_integer(24.into());
    let zero_matrix = || std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    let mut net: [[Q; 3]; 3] = zero_matrix();
    for (index, contour) in std::iter::once(outer)
        .chain(holes.iter().copied())
        .enumerate()
    {
        spend(remaining, 24)?;
        let mut projected_area = Q::zero();
        let mut moments: [[Q; 3]; 3] = zero_matrix();
        for pair in contour.windows(2) {
            spend(remaining, 96)?;
            let a: [Q; 3] = std::array::from_fn(|i| &pair[0][i] - &centroid[i]);
            let b: [Q; 3] = std::array::from_fn(|i| &pair[1][i] - &centroid[i]);
            let twice_area = &a[u] * &b[v] - &a[v] * &b[u];
            projected_area += &twice_area;
            for i in 0..3 {
                for j in i..3 {
                    let diagonal = &a[i] * &a[j] + &b[i] * &b[j];
                    let mixed = &a[i] * &b[j] + &b[i] * &a[j];
                    moments[i][j] += (&diagonal + &diagonal + mixed) * &twice_area;
                }
            }
        }
        let add = projected_area.is_positive() == (index == 0);
        for (net_row, row) in net.iter_mut().zip(moments) {
            for (total, value) in net_row.iter_mut().zip(row) {
                if add {
                    *total += value;
                } else {
                    *total -= value;
                }
            }
        }
    }
    spend(remaining, 16)?;
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|j| &net[i.min(j)][i.max(j)] / &denominator)
    }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPlanarPolylinePrincipalMoments {
    /// Inclusive bounds for the smaller and larger area-normalized moments
    /// in the profile plane. The normal direction has a zero moment.
    pub principal_bounds_mm2: [[Q; 2]; 2],
    pub centroid_mm: [Q; 3],
    pub area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Bound the two in-plane eigenvalues of a simple profile's normalized
/// central second moment tensor. Each enclosure has width at most tolerance.
pub fn bounded_planar_spatial_polyline_principal_moments(
    contour_mm: &[[Q; 3]],
    tolerance_mm2: f64,
    max_exact_work: u64,
) -> Result<BoundedPlanarPolylinePrincipalMoments> {
    let tolerance = enclosure_width(tolerance_mm2)?;
    let report = exact_planar_spatial_polyline_second_moments(contour_mm, max_exact_work)?;
    let mut remaining = max_exact_work - report.exact_work;
    spend(&mut remaining, 64)?;
    let tensor = &report.normalized_central_moments_mm2;
    let trace: Q = (0..3).map(|i| &tensor[i][i]).sum();
    let squared_trace: Q = tensor
        .iter()
        .flatten()
        .map(|component| component.pow(2))
        .sum();
    let two = Q::from_integer(2.into());
    // The normal eigenvalue is zero. Thus 2*tr(M²)-tr(M)² is the
    // squared gap between the two in-plane eigenvalues.
    let discriminant = &two * squared_trace - trace.pow(2);
    if discriminant.is_negative() {
        return Err(refuse("Invalid exact second moment spectrum"));
    }
    let [lo, hi] = bound_root_sum(vec![discriminant], &two * tolerance, &mut remaining)?;
    let principal_bounds_mm2 = [
        [
            ((&trace - &hi) / &two).max(Q::zero()),
            (&trace - &lo) / &two,
        ],
        [(&trace + lo) / &two, (trace + hi) / two],
    ];
    Ok(BoundedPlanarPolylinePrincipalMoments {
        principal_bounds_mm2,
        centroid_mm: report.centroid_mm,
        area_squared_mm4: report.area_squared_mm4,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineRegionMass {
    pub centroid_mm: [Q; 3],
    /// Net area vector oriented by the outer contour.
    pub area_vector_mm2: [Q; 3],
    pub area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Compute area and centroid for a planar region with strictly interior,
/// pairwise disjoint holes. Hole winding is immaterial.
pub fn exact_planar_spatial_polyline_region_mass(
    outer_mm: &[[Q; 3]],
    holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineRegionMass> {
    let outer = exact_planar_spatial_polyline_centroid(outer_mm, max_exact_work)?;
    let mut remaining = max_exact_work - outer.exact_work;
    let normal = &outer.area.area_vector_mm2;
    let axes = plane_axes(normal);
    let axis = 3 - axes[0] - axes[1];
    let mut weight = normal[axis].abs();
    let mut moment = outer.centroid_mm.map(|coord| coord * &weight);
    let segments = |points: &[[Q; 3]], remaining: &mut u64| -> Result<Vec<[[Q; 3]; 2]>> {
        spend(remaining, 8 * (points.len() as u64 - 1))?;
        Ok(points
            .windows(2)
            .map(|pair| [pair[0].clone(), pair[1].clone()])
            .collect())
    };
    let mut boundaries = vec![(outer_mm, segments(outer_mm, &mut remaining)?)];
    for &hole in holes_mm {
        if remaining == 0 {
            return Err(super::work_exhausted());
        }
        let properties = exact_planar_spatial_polyline_centroid(hole, remaining)?;
        remaining -= properties.exact_work;
        for point in hole {
            spend(&mut remaining, 16)?;
            let offset = std::array::from_fn(|i| &point[i] - &outer_mm[0][i]);
            if !dot(&offset, normal).is_zero() {
                return Err(refuse("Hole does not lie on the outer support"));
            }
        }
        let edges = segments(hole, &mut remaining)?;
        for (index, (other, other_edges)) in boundaries.iter().enumerate() {
            for edge in &edges {
                for other_edge in other_edges {
                    spend(&mut remaining, 64)?;
                    if intersect_spatial_segments(edge, other_edge).is_some() {
                        return Err(refuse("Profile contours intersect"));
                    }
                }
            }
            let hole_location = point_in_face(other_edges, &hole[0], axes, &mut remaining)?;
            if index == 0 {
                if hole_location != FaceLocation::Inside {
                    return Err(refuse("Hole must lie strictly inside the exterior contour"));
                }
            } else if hole_location != FaceLocation::Outside
                || point_in_face(&edges, &other[0], axes, &mut remaining)? != FaceLocation::Outside
            {
                return Err(refuse("Profile holes must be disjoint and unnested"));
            }
        }
        spend(&mut remaining, 16)?;
        let hole_weight = properties.area.area_vector_mm2[axis].abs();
        for (sum, coord) in moment.iter_mut().zip(properties.centroid_mm) {
            *sum -= coord * &hole_weight;
        }
        weight -= hole_weight;
        boundaries.push((hole, edges));
    }
    spend(&mut remaining, 24)?;
    if !weight.is_positive() {
        return Err(refuse("Expected positive net profile area"));
    }
    let centroid_mm = moment.map(|component| component / &weight);
    let area_vector_mm2 = normal
        .each_ref()
        .map(|component| component * &weight / normal[axis].abs());
    Ok(ExactPlanarPolylineRegionMass {
        centroid_mm,
        area_squared_mm4: dot(&area_vector_mm2, &area_vector_mm2),
        area_vector_mm2,
        exact_work: max_exact_work - remaining,
    })
}

fn prepare_planar_polyline_region(
    outer_mm: &[[Q; 3]],
    holes_mm: &[&[[Q; 3]]],
    remaining: &mut u64,
) -> Result<FaceBoundary> {
    if *remaining == 0 {
        return Err(super::work_exhausted());
    }
    let (normal, used) = if holes_mm.is_empty() {
        let area = exact_planar_spatial_polyline_area(outer_mm, *remaining)?;
        (area.area_vector_mm2, area.exact_work)
    } else {
        let mass = exact_planar_spatial_polyline_region_mass(outer_mm, holes_mm, *remaining)?;
        (mass.area_vector_mm2, mass.exact_work)
    };
    *remaining -= used;
    let mut segments = Vec::new();
    for contour in std::iter::once(outer_mm).chain(holes_mm.iter().copied()) {
        spend(remaining, 8 * (contour.len() as u64 - 1))?;
        segments.extend(
            contour
                .windows(2)
                .map(|pair| [pair[0].clone(), pair[1].clone()]),
        );
    }
    Ok(FaceBoundary {
        origin: outer_mm[0].clone(),
        normal: Some(normal),
        segments,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarPolylineRegionIntersection {
    /// Disjoint closed segments in order of the first nonzero line axis.
    /// Repeated endpoints represent isolated contacts.
    pub segments_mm: Vec<[[Q; 3]; 2]>,
    pub exact_work: u64,
}

/// Intersect two closed planar profiles on nonparallel supports. Both
/// profiles can contain disjoint interior holes.
pub fn intersect_transverse_exact_planar_polyline_regions(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactPlanarPolylineRegionIntersection> {
    super::validate_work(max_exact_work)?;
    let mut remaining = max_exact_work;
    let a = prepare_planar_polyline_region(first_outer_mm, first_holes_mm, &mut remaining)?;
    let b = prepare_planar_polyline_region(second_outer_mm, second_holes_mm, &mut remaining)?;
    spend(&mut remaining, 64)?;
    let na = face_normal(&a)?;
    let nb = face_normal(&b)?;
    let cross = [(1, 2), (2, 0), (0, 1)].map(|(i, j)| &na[i] * &nb[j] - &na[j] * &nb[i]);
    let Some(axis) = cross.iter().position(|c| !c.is_zero()) else {
        return Err(refuse("Profile supports must be nonparallel"));
    };
    let [u, v] = [(axis + 1) % 3, (axis + 2) % 3];
    let da = dot(na, &a.origin);
    let db = dot(nb, &b.origin);
    let mut base = [Q::zero(), Q::zero(), Q::zero()];
    base[u] = (&da * &nb[v] - &db * &na[v]) / &cross[axis];
    base[v] = (&na[u] * db - &nb[u] * da) / &cross[axis];
    let direction = cross.each_ref().map(|c| c / &cross[axis]);
    spend(&mut remaining, 4 * first_outer_mm.len() as u64)?;
    let coordinates = first_outer_mm.iter().map(|point| &point[axis]);
    let lo = coordinates.clone().min().expect("certified outer contour");
    let hi = coordinates.max().expect("certified outer contour");
    let [start, end] = [lo, hi].map(|t| std::array::from_fn(|i| &base[i] + t * &direction[i]));
    let span = std::array::from_fn(|i| &end[i] - &start[i]);
    let left = spatial_segment_region_contacts(&a, &start, &end, &span, &mut remaining)?;
    let right = spatial_segment_region_contacts(&b, &start, &end, &span, &mut remaining)?;
    let intervals = intersect_closed_intervals(&left, &right, &mut remaining)?;
    spend(&mut remaining, 16 * intervals.len() as u64)?;
    let segments_mm = intervals
        .into_iter()
        .map(|bounds| bounds.map(|t| std::array::from_fn(|i| &start[i] + &t * &span[i])))
        .collect();
    Ok(ExactPlanarPolylineRegionIntersection {
        segments_mm,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineRegionOverlapArea {
    /// Area vector oriented by the first profile.
    pub area_vector_mm2: [Q; 3],
    pub area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Measure exact common area of two coplanar profiles with disjoint holes.
/// Boundary-only intersection gives zero area.
pub fn exact_coplanar_planar_polyline_region_overlap_area(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineRegionOverlapArea> {
    let (area, _, _) = coplanar_profile_overlap(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        max_exact_work,
        false,
    )?;
    Ok(area)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineRegionOverlapMass {
    /// Absent when the common region has zero area.
    pub centroid_mm: Option<[Q; 3]>,
    pub area_vector_mm2: [Q; 3],
    pub area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Measure the exact area and centroid of the common material of two
/// coplanar regions with holes. Area orientation follows the first profile.
pub fn exact_coplanar_planar_polyline_region_overlap_mass(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineRegionOverlapMass> {
    let (area, centroid_mm, _) = coplanar_profile_overlap(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        max_exact_work,
        true,
    )?;
    Ok(ExactCoplanarPolylineRegionOverlapMass {
        centroid_mm,
        area_vector_mm2: area.area_vector_mm2,
        area_squared_mm4: area.area_squared_mm4,
        exact_work: area.exact_work,
    })
}

fn coplanar_profile_overlap(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
    measure_centroid: bool,
) -> Result<(
    ExactCoplanarPolylineRegionOverlapArea,
    Option<[Q; 3]>,
    [Q; 2],
)> {
    let a =
        exact_planar_spatial_polyline_region_mass(first_outer_mm, first_holes_mm, max_exact_work)?;
    let mut remaining = max_exact_work - a.exact_work;
    if remaining == 0 {
        return Err(super::work_exhausted());
    }
    let b = exact_planar_spatial_polyline_region_mass(second_outer_mm, second_holes_mm, remaining)?;
    remaining -= b.exact_work;
    let normal = &a.area_vector_mm2;
    let axes = plane_axes(normal);
    for point in second_outer_mm {
        spend(&mut remaining, 16)?;
        let offset = std::array::from_fn(|i| &point[i] - &first_outer_mm[0][i]);
        if !dot(&offset, normal).is_zero() {
            return Err(refuse("Profile supports must coincide"));
        }
    }
    let first = projected_profile_triangles(first_outer_mm, first_holes_mm, axes, &mut remaining)?;
    let second =
        projected_profile_triangles(second_outer_mm, second_holes_mm, axes, &mut remaining)?;
    let mut twice_area = Q::zero();
    let mut first_moment = [Q::zero(), Q::zero()];
    for (a_hole, a) in &first {
        for (b_hole, b) in &second {
            let polygon = projected_triangle_overlap(a, b, &mut remaining)?;
            let mut signed_area = Q::zero();
            let mut moment = [Q::zero(), Q::zero()];
            for (p, q) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
                spend(&mut remaining, 8)?;
                let cross = &p[0] * &q[1] - &p[1] * &q[0];
                if measure_centroid {
                    spend(&mut remaining, 12)?;
                    for i in 0..2 {
                        moment[i] += (&p[i] + &q[i]) * &cross;
                    }
                }
                signed_area += cross;
            }
            if signed_area.is_zero() {
                continue;
            }
            if a_hole == b_hole {
                twice_area += signed_area.abs();
            } else {
                twice_area -= signed_area.abs();
            }
            if measure_centroid {
                spend(&mut remaining, 4)?;
                let add = signed_area.is_positive() == (a_hole == b_hole);
                for (total, value) in first_moment.iter_mut().zip(moment) {
                    if add {
                        *total += value;
                    } else {
                        *total -= value;
                    }
                }
            }
        }
    }
    spend(&mut remaining, 16)?;
    if twice_area.is_negative() {
        return Err(refuse("Negative exact profile overlap area"));
    }
    let [u, v] = axes;
    let axis = 3 - u - v;
    let centroid = if measure_centroid && twice_area.is_positive() {
        spend(&mut remaining, 24)?;
        let denominator = &twice_area * Q::from_integer(3.into());
        let mut point = [Q::zero(), Q::zero(), Q::zero()];
        point[u] = &first_moment[0] / &denominator;
        point[v] = &first_moment[1] / &denominator;
        let origin = &first_outer_mm[0];
        point[axis] = &origin[axis]
            - (&normal[u] * (&point[u] - &origin[u]) + &normal[v] * (&point[v] - &origin[v]))
                / &normal[axis];
        Some(point)
    } else {
        None
    };
    let scale = twice_area / (normal[3 - axes[0] - axes[1]].abs() * Q::from_integer(2.into()));
    let area_vector_mm2 = normal.each_ref().map(|component| component * &scale);
    Ok((
        ExactCoplanarPolylineRegionOverlapArea {
            area_squared_mm4: dot(&area_vector_mm2, &area_vector_mm2),
            area_vector_mm2,
            exact_work: max_exact_work - remaining,
        },
        centroid,
        [a.area_squared_mm4, b.area_squared_mm4],
    ))
}

type ProjectedTriangle = [[Q; 2]; 3];

fn projected_profile_triangles(
    outer: &[[Q; 3]],
    holes: &[&[[Q; 3]]],
    [u, v]: [usize; 2],
    remaining: &mut u64,
) -> Result<Vec<(bool, ProjectedTriangle)>> {
    let mut triangles = Vec::new();
    for (index, contour) in std::iter::once(outer)
        .chain(holes.iter().copied())
        .enumerate()
    {
        if *remaining == 0 {
            return Err(super::work_exhausted());
        }
        let report = triangulate_exact_planar_spatial_polyline(contour, *remaining)?;
        *remaining -= report.exact_work;
        for indices in report.triangles {
            spend(remaining, 12)?;
            triangles.push((
                index != 0,
                indices.map(|i| [contour[i][u].clone(), contour[i][v].clone()]),
            ));
        }
    }
    Ok(triangles)
}

fn projected_triangle_overlap(
    a: &ProjectedTriangle,
    b: &ProjectedTriangle,
    remaining: &mut u64,
) -> Result<Vec<[Q; 2]>> {
    spend(remaining, 12)?;
    let side = |a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]| {
        (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
    };
    let positive = side(&b[0], &b[1], &b[2]).is_positive();
    let inside = |s: &Q| s.is_zero() || s.is_positive() == positive;
    let mut polygon = a.to_vec();
    for edge in 0..3 {
        let start = &b[edge];
        let end = &b[(edge + 1) % 3];
        let Some(mut previous) = polygon.last() else {
            return Ok(Vec::new());
        };
        spend(remaining, 8)?;
        let mut previous_side = side(start, end, previous);
        let mut clipped = Vec::new();
        for point in &polygon {
            spend(remaining, 12)?;
            let current_side = side(start, end, point);
            if inside(&previous_side) != inside(&current_side)
                && !previous_side.is_zero()
                && !current_side.is_zero()
            {
                spend(remaining, 16)?;
                let t = &previous_side / (&previous_side - &current_side);
                clipped.push(std::array::from_fn(|i| {
                    &previous[i] + &t * (&point[i] - &previous[i])
                }));
            }
            if inside(&current_side) {
                clipped.push(point.clone());
            }
            previous = point;
            previous_side = current_side;
        }
        polygon = clipped;
    }
    Ok(polygon)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoplanarPolylineRegionRelation {
    Disjoint,
    TouchingAtPoints,
    TouchingAlongEdges,
    PartialOverlap,
    FirstInsideSecond,
    SecondInsideFirst,
    Equal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineRegionRelation {
    pub relation: CoplanarPolylineRegionRelation,
    pub overlap_area_squared_mm4: Q,
    pub exact_work: u64,
}

/// Classify two filled coplanar profiles. Containment includes shared
/// boundaries. Touching variants apply only when common area is zero.
pub fn classify_exact_coplanar_planar_polyline_regions(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineRegionRelation> {
    use CoplanarPolylineRegionRelation::*;

    let (area, _, [first_area, second_area]) = coplanar_profile_overlap(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        max_exact_work,
        false,
    )?;
    let mut remaining = max_exact_work - area.exact_work;
    spend(&mut remaining, 4)?;
    let overlap = area.area_squared_mm4;
    let relation = if overlap.is_zero() {
        let first = std::iter::once(first_outer_mm).chain(first_holes_mm.iter().copied());
        let second = std::iter::once(second_outer_mm).chain(second_holes_mm.iter().copied());
        let mut relation = Disjoint;
        'contours: for a in first {
            for b in second.clone() {
                for pa in a.windows(2) {
                    for pb in b.windows(2) {
                        spend(&mut remaining, 64)?;
                        if let Some((_, points)) = intersect_spatial_segments(pa, pb) {
                            if points[0] != points[1] {
                                relation = TouchingAlongEdges;
                                break 'contours;
                            }
                            relation = TouchingAtPoints;
                        }
                    }
                }
            }
        }
        relation
    } else {
        // A nonempty difference of regular closed polygonal regions
        // contains a positive-area subset of the larger region.
        match (overlap == first_area, overlap == second_area) {
            (true, true) => Equal,
            (true, false) => FirstInsideSecond,
            (false, true) => SecondInsideFirst,
            (false, false) => PartialOverlap,
        }
    };
    Ok(ExactCoplanarPolylineRegionRelation {
        relation,
        overlap_area_squared_mm4: overlap,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineRegionOverlapCells {
    /// Convex three- or four-vertex cells in first-profile winding.
    /// Interiors are disjoint. The union is the closure of the area overlap.
    pub cells_mm: Vec<Vec<[Q; 3]>>,
    pub exact_work: u64,
}

/// Construct the positive-area intersection of two profiles on a common
/// plane. The exact cells retain holes and disconnected material.
pub fn decompose_exact_coplanar_planar_polyline_region_overlap(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineRegionOverlapCells> {
    let report = decompose_exact_coplanar_planar_polyline_region_boolean(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        CoplanarPolylineBoolean::Intersection,
        max_exact_work,
    )?;
    Ok(ExactCoplanarPolylineRegionOverlapCells {
        cells_mm: report.cells_mm,
        exact_work: report.exact_work,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoplanarPolylineBoolean {
    Intersection,
    Union,
    Difference,
    SymmetricDifference,
}

impl CoplanarPolylineBoolean {
    fn contains(self, [first, second]: [bool; 2]) -> bool {
        match self {
            Self::Intersection => first && second,
            Self::Union => first || second,
            Self::Difference => first && !second,
            Self::SymmetricDifference => first != second,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineRegionBooleanCells {
    /// Convex cells with disjoint interiors, oriented like the first profile.
    pub cells_mm: Vec<Vec<[Q; 3]>>,
    pub exact_work: u64,
}

/// Decompose a regularized Boolean result into exact convex cells. The
/// result is the closure of the selected area, discarding isolated
/// zero-area remnants. Difference subtracts the second from the first.
pub fn decompose_exact_coplanar_planar_polyline_region_boolean(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    operation: CoplanarPolylineBoolean,
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineRegionBooleanCells> {
    let (report, _) = planar_profile_boolean_cells(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        operation,
        max_exact_work,
    )?;
    Ok(report)
}

fn planar_profile_boolean_cells(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    operation: CoplanarPolylineBoolean,
    max_exact_work: u64,
) -> Result<(ExactCoplanarPolylineRegionBooleanCells, [usize; 2])> {
    super::validate_work(max_exact_work)?;
    let mut remaining = max_exact_work;
    let first = prepare_planar_polyline_region(first_outer_mm, first_holes_mm, &mut remaining)?;
    let second = prepare_planar_polyline_region(second_outer_mm, second_holes_mm, &mut remaining)?;
    let normal = face_normal(&first)?;
    let [u, v] = plane_axes(normal);
    let axis = 3 - u - v;
    let mut cuts = Vec::new();
    for [a, b] in first.segments.iter().chain(&second.segments) {
        spend(&mut remaining, 16)?;
        let offset = std::array::from_fn(|i| &a[i] - &first.origin[i]);
        if !dot(&offset, normal).is_zero() {
            return Err(refuse("Profile supports must coincide"));
        }
        cuts.extend([a[u].clone(), b[u].clone()]);
    }
    for a in &first.segments {
        for b in &second.segments {
            spend(&mut remaining, 64)?;
            if let Some((_, points)) = intersect_spatial_segments(a, b) {
                cuts.extend(points.map(|point| point[u].clone()));
            }
        }
    }
    let count = cuts.len() as u64;
    spend(
        &mut remaining,
        count.saturating_mul(8 * u64::from(count.ilog2() + 1)),
    )?;
    cuts.sort_unstable();
    cuts.dedup();
    let two = Q::from_integer(2.into());
    let lift = |x: &Q, y: &Q| {
        let mut point = [Q::zero(), Q::zero(), Q::zero()];
        point[u] = x.clone();
        point[v] = y.clone();
        point[axis] = &first.origin[axis]
            - (&normal[u] * (x - &first.origin[u]) + &normal[v] * (y - &first.origin[v]))
                / &normal[axis];
        point
    };
    let mut cells_mm = Vec::new();
    for window in cuts.windows(2) {
        spend(&mut remaining, 4)?;
        let lo = &window[0];
        let hi = &window[1];
        let mid = (lo + hi) / &two;
        let bands = profile_boolean_slice_bands(
            [&first, &second],
            [u, v],
            &mid,
            operation,
            &mut remaining,
        )?;
        for [lower, upper] in bands {
            spend(&mut remaining, 64)?;
            let mut cell = vec![
                lift(lo, &profile_edge_ordinate(lower, [u, v], lo)),
                lift(hi, &profile_edge_ordinate(lower, [u, v], hi)),
                lift(hi, &profile_edge_ordinate(upper, [u, v], hi)),
                lift(lo, &profile_edge_ordinate(upper, [u, v], lo)),
            ];
            cell.dedup();
            if cell.first() == cell.last() {
                cell.pop();
            }
            if normal[axis].is_negative() {
                cell.reverse();
            }
            cells_mm.push(cell);
        }
    }
    Ok((
        ExactCoplanarPolylineRegionBooleanCells {
            cells_mm,
            exact_work: max_exact_work - remaining,
        },
        [u, v],
    ))
}

type PlanarProfileEdge = [[Q; 3]; 2];

fn profile_boolean_slice_bands<'a>(
    faces: [&'a FaceBoundary; 2],
    axes: [usize; 2],
    at: &Q,
    operation: CoplanarPolylineBoolean,
    remaining: &mut u64,
) -> Result<Vec<[&'a PlanarProfileEdge; 2]>> {
    let [u, _] = axes;
    let mut crossings = Vec::new();
    for (index, face) in faces.into_iter().enumerate() {
        for edge in &face.segments {
            spend(remaining, 4)?;
            if (&edge[0][u] < at) != (&edge[1][u] < at) {
                spend(remaining, 12)?;
                crossings.push((profile_edge_ordinate(edge, axes, at), edge, index));
            }
        }
    }
    let count = crossings.len() as u64;
    spend(
        remaining,
        count.saturating_mul(8 * (u64::from(count.checked_ilog2().unwrap_or(0)) + 1)),
    )?;
    crossings.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
    let mut inside = [false; 2];
    let mut lower = None;
    let mut bands = Vec::new();
    for group in crossings.chunk_by(|a, b| a.0 == b.0) {
        let before = operation.contains(inside);
        for (_, _, index) in group {
            spend(remaining, 4)?;
            inside[*index] = !inside[*index];
        }
        let edge = group[0].1;
        match (before, operation.contains(inside)) {
            (false, true) => lower = Some(edge),
            (true, false) => {
                bands.push([lower.take().expect("open selected band"), edge]);
            }
            _ => {}
        }
    }
    if inside != [false; 2] {
        return Err(refuse("Unexpected open planar slice"));
    }
    Ok(bands)
}

fn profile_edge_ordinate([a, b]: &[[Q; 3]; 2], [u, v]: [usize; 2], at: &Q) -> Q {
    &a[v] + (at - &a[u]) * (&b[v] - &a[v]) / (&b[u] - &a[u])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactCoplanarPolylineBooleanBoundary {
    /// Directed exact boundary edges with interior seams removed.
    /// Junctions use identical coordinates; rings can meet at a vertex.
    pub edges_mm: Vec<[[Q; 3]; 2]>,
    pub exact_work: u64,
}

/// Extract the oriented boundary of a regularized planar Boolean result.
/// Partially shared cell edges cancel over their common subintervals.
pub fn boundary_of_exact_coplanar_planar_polyline_region_boolean(
    first_outer_mm: &[[Q; 3]],
    first_holes_mm: &[&[[Q; 3]]],
    second_outer_mm: &[[Q; 3]],
    second_holes_mm: &[&[[Q; 3]]],
    operation: CoplanarPolylineBoolean,
    max_exact_work: u64,
) -> Result<ExactCoplanarPolylineBooleanBoundary> {
    let (report, [u, v]) = planar_profile_boolean_cells(
        first_outer_mm,
        first_holes_mm,
        second_outer_mm,
        second_holes_mm,
        operation,
        max_exact_work,
    )?;
    let mut remaining = max_exact_work - report.exact_work;
    let mut seams = BTreeMap::<[Q; 2], ([Q; 3], i32)>::new();
    let mut edges_mm = Vec::new();
    for cell in report.cells_mm {
        for (a, b) in cell.iter().zip(cell.iter().cycle().skip(1)) {
            let depth = u64::from((seams.len() + 2).ilog2()) + 1;
            spend(&mut remaining, 16 * depth)?;
            let start = seams
                .entry([a[u].clone(), a[v].clone()])
                .or_insert_with(|| (a.clone(), 0));
            if a[u] != b[u] {
                edges_mm.push([a.clone(), b.clone()]);
                continue;
            }
            // Accumulate signed edge multiplicity over the seam ordinate.
            // A descending edge starts at the upper endpoint.
            start.1 += 1;
            seams
                .entry([b[u].clone(), b[v].clone()])
                .or_insert_with(|| (b.clone(), 0))
                .1 -= 1;
        }
    }
    let mut previous: Option<[Q; 3]> = None;
    let mut winding = 0;
    for (_, (point, delta)) in seams {
        spend(&mut remaining, 8)?;
        if let Some(prior) = previous {
            if prior[u] != point[u] && winding != 0 {
                return Err(refuse("Unbalanced planar cell seam"));
            }
            match winding {
                0 => {}
                1 => edges_mm.push([prior, point.clone()]),
                -1 => edges_mm.push([point.clone(), prior]),
                _ => return Err(refuse("Overlapping planar cell seam edges")),
            }
        }
        winding += delta;
        previous = Some(point);
    }
    if winding != 0 {
        return Err(refuse("Unbalanced planar cell seam"));
    }
    Ok(ExactCoplanarPolylineBooleanBoundary {
        edges_mm,
        exact_work: max_exact_work - remaining,
    })
}
