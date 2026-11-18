use super::{
    FaceBoundary, FaceLocation, PolyhedralPointLocation, classify_open_point, dot, face_normal,
    nearest_segment_point, plane_axes, point_in_face, prepare_faces, refuse, spend, validate_point,
};
use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSweptBallClearance {
    /// Comparison of the minimum center-path distance with the ball radius.
    /// Equal means contact at radius. Less means a swept ball contains a
    /// boundary point strictly in its interior.
    pub clearance: Ordering,
    pub distance_squared_mm2: Q,
    pub center_mm: [Q; 3],
    pub boundary_mm: [Q; 3],
    pub face: usize,
    pub exact_work: u64,
}

/// Compare a ball swept along a closed segment against the retained solid
/// boundary. Returns a closest witness pair, including contacts between the
/// segment endpoints. Clearance is relative to the boundary on either side.
pub fn exact_polyhedral_swept_ball_clearance(
    solid: &QualifiedClosedSolid,
    centers_mm: &[[Q; 3]; 2],
    radius_mm: &Q,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSweptBallClearance> {
    let radius_squared = squared_radius(radius_mm)?;
    for center in centers_mm {
        validate_point(center)?;
    }
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let [start, end] = centers_mm;
    let mut report = swept_segment(&faces, start, end, &radius_squared, &mut remaining)?;
    report.exact_work = max_exact_work - remaining;
    Ok(report)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSweptPathClearance {
    /// Zero-based path span containing a global minimizing witness.
    pub span: usize,
    /// Closest pair and work for the winning span, excluding preparation.
    pub nearest: ExactPolyhedralSweptBallClearance,
    pub exact_work: u64,
}

/// Compare the entire polyline sweep with retained material boundaries.
/// One face preparation and one budget serve all spans. Among equal
/// distances, the earliest minimizing span wins.
pub fn exact_polyhedral_swept_path_clearance(
    solid: &QualifiedClosedSolid,
    centers_mm: &[[Q; 3]],
    radius_mm: &Q,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSweptPathClearance> {
    let radius_squared = squared_radius(radius_mm)?;
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut report = swept_path(&faces, centers_mm, &radius_squared, &mut remaining)?;
    report.exact_work = max_exact_work - remaining;
    Ok(report)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolyhedralSweptPathRelation {
    Inside,
    InsideTangent,
    CrossesBoundary,
    ZeroRadiusBoundaryContact,
    OutsideTangent,
    Outside,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPolyhedralSweptPathMaterial {
    pub relation: PolyhedralSweptPathRelation,
    /// Global nearest pair with total work, including material classification.
    pub clearance: ExactPolyhedralSweptPathClearance,
}

/// Classify the union of closed balls along a polyline relative to solid
/// material. A clear center path stays on one side of the boundary, so one
/// exact point query determines its side. A zero-radius boundary hit is
/// reported as contact without distinguishing crossing from tangency.
pub fn exact_polyhedral_swept_path_material(
    solid: &QualifiedClosedSolid,
    centers_mm: &[[Q; 3]],
    radius_mm: &Q,
    max_exact_work: u64,
) -> Result<ExactPolyhedralSweptPathMaterial> {
    let radius_squared = squared_radius(radius_mm)?;
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    let mut clearance = swept_path(&faces, centers_mm, &radius_squared, &mut remaining)?;
    spend(&mut remaining, 4)?;
    let relation = match clearance.nearest.clearance {
        Ordering::Less => PolyhedralSweptPathRelation::CrossesBoundary,
        Ordering::Equal if radius_squared.is_zero() => {
            PolyhedralSweptPathRelation::ZeroRadiusBoundaryContact
        }
        comparison => {
            let inside = classify_open_point(&faces, &centers_mm[0], &mut remaining)?
                == PolyhedralPointLocation::Material;
            match (inside, comparison == Ordering::Equal) {
                (true, true) => PolyhedralSweptPathRelation::InsideTangent,
                (true, false) => PolyhedralSweptPathRelation::Inside,
                (false, true) => PolyhedralSweptPathRelation::OutsideTangent,
                (false, false) => PolyhedralSweptPathRelation::Outside,
            }
        }
    };
    clearance.exact_work = max_exact_work - remaining;
    Ok(ExactPolyhedralSweptPathMaterial {
        relation,
        clearance,
    })
}

fn swept_path(
    faces: &BTreeMap<usize, FaceBoundary>,
    centers_mm: &[[Q; 3]],
    radius_squared: &Q,
    remaining: &mut u64,
) -> Result<ExactPolyhedralSweptPathClearance> {
    if centers_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Sweep path requires at least two centers",
        ));
    }
    for center in centers_mm {
        spend(remaining, 1)?;
        validate_point(center)?;
    }
    let mut result: Option<ExactPolyhedralSweptPathClearance> = None;
    for (span, pair) in centers_mm.windows(2).enumerate() {
        let nearest = swept_segment(faces, &pair[0], &pair[1], radius_squared, remaining)?;
        let reaches_boundary = nearest.distance_squared_mm2.is_zero();
        if result
            .as_ref()
            .is_none_or(|old| nearest.distance_squared_mm2 < old.nearest.distance_squared_mm2)
        {
            result = Some(ExactPolyhedralSweptPathClearance {
                span,
                nearest,
                exact_work: 0,
            });
        }
        if reaches_boundary {
            break;
        }
    }
    Ok(result.expect("a sweep path has at least one span"))
}

fn squared_radius(radius_mm: &Q) -> Result<Q> {
    if radius_mm.is_negative() || radius_mm.numer().bits().max(radius_mm.denom().bits()) > 8192 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Expected a nonnegative radius within the exact bit limit",
        ));
    }
    Ok(radius_mm.pow(2))
}

fn swept_segment<'a>(
    faces: impl IntoIterator<Item = (&'a usize, &'a FaceBoundary)>,
    start: &[Q; 3],
    end: &[Q; 3],
    radius_squared: &Q,
    remaining: &mut u64,
) -> Result<ExactPolyhedralSweptBallClearance> {
    let start_work = *remaining;
    spend(remaining, 12)?;
    let direction = delta(end, start);
    let mut nearest = None::<ExactPolyhedralSweptBallClearance>;
    for (&face, boundary) in faces {
        spend(remaining, 40)?;
        let normal = face_normal(boundary)?;
        let normal_squared = dot(normal, normal);
        let axes = plane_axes(normal);
        let origin_delta = delta(start, &boundary.origin);
        let residual = dot(normal, &origin_delta);
        let slope = dot(normal, &direction);
        let mut consider = |center: [Q; 3], point: [Q; 3], distance: Q| {
            if nearest
                .as_ref()
                .is_none_or(|old| distance < old.distance_squared_mm2)
            {
                nearest = Some(ExactPolyhedralSweptBallClearance {
                    clearance: Ordering::Equal,
                    distance_squared_mm2: distance,
                    center_mm: center,
                    boundary_mm: point,
                    face,
                    exact_work: 0,
                });
            }
        };
        let on_segment = |t: &Q| Q::zero() <= *t && *t <= Q::one();
        if !slope.is_zero() {
            let t = -&residual / &slope;
            if on_segment(&t) {
                let point = at(start, &direction, &t);
                if point_in_face(&boundary.segments, &point, axes, remaining)?
                    != FaceLocation::Outside
                {
                    consider(point.clone(), point, Q::zero());
                    break;
                }
            }
        }
        for point in [start, end] {
            spend(remaining, 24)?;
            let r = dot(normal, &delta(point, &boundary.origin));
            let scale = &r / &normal_squared;
            let projected = at(point, normal, &-&scale);
            if point_in_face(&boundary.segments, &projected, axes, remaining)?
                != FaceLocation::Outside
            {
                consider(point.clone(), projected, r * scale);
            }
        }
        for [a, b] in &boundary.segments {
            spend(remaining, 216)?;
            for center in [start, end] {
                let (point, squared) = nearest_segment_point(center, a, b);
                consider(center.clone(), point, squared);
            }
            for point in [a, b] {
                let (center, squared) = nearest_segment_point(point, start, end);
                consider(center, point.clone(), squared);
            }
            // The interior minimizer solves the two stationarity equations.
            // For parallel segments an endpoint projection already minimizes.
            let edge = delta(b, a);
            let origin = delta(start, a);
            let dd = dot(&direction, &direction);
            let ee = dot(&edge, &edge);
            let de = dot(&direction, &edge);
            let determinant = &dd * &ee - &de * &de;
            if !determinant.is_zero() {
                let od = dot(&origin, &direction);
                let oe = dot(&origin, &edge);
                let t = (&de * &oe - &ee * &od) / &determinant;
                let u = (&dd * &oe - &de * &od) / determinant;
                if on_segment(&t) && on_segment(&u) {
                    let center = at(start, &direction, &t);
                    let point = at(a, &edge, &u);
                    let gap = delta(&center, &point);
                    consider(center, point, dot(&gap, &gap));
                }
            }
        }
    }
    let mut report =
        nearest.ok_or_else(|| refuse("Sweep clearance requires at least one retained face"))?;
    report.clearance = report.distance_squared_mm2.cmp(radius_squared);
    report.exact_work = start_work - *remaining;
    Ok(report)
}

fn delta(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
    std::array::from_fn(|i| &a[i] - &b[i])
}

fn at(origin: &[Q; 3], direction: &[Q; 3], t: &Q) -> [Q; 3] {
    std::array::from_fn(|i| &origin[i] + t * &direction[i])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSweptPathContact {
    /// Zero-based span, with the earlier span chosen at a shared vertex.
    pub span: usize,
    /// Outward bounds for the first local contact parameter in [0, 1].
    pub parameter_bounds: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSweptPathFirstContact {
    /// No contact means the swept balls never meet the retained boundary.
    pub contact: Option<PolyhedralSweptPathContact>,
    pub exact_work: u64,
}

/// Enclose the first time a closed swept ball meets a retained boundary.
/// The search uses exact prefix minimum distance, so even a single instant
/// of tangency counts. Parameter width is local to the first contacting span.
pub fn bounded_polyhedral_swept_path_first_contact(
    solid: &QualifiedClosedSolid,
    centers_mm: &[[Q; 3]],
    radius_mm: &Q,
    max_parameter_width: f64,
    max_exact_work: u64,
) -> Result<PolyhedralSweptPathFirstContact> {
    let radius_squared = squared_radius(radius_mm)?;
    let width = super::super::enclosure_width(max_parameter_width)?;
    if centers_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Sweep path requires at least two centers",
        ));
    }
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    for center in centers_mm {
        spend(&mut remaining, 1)?;
        validate_point(center)?;
    }
    let initial = super::nearest_boundary(&faces, &centers_mm[0], &mut remaining)?;
    let mut contact = if initial.distance_squared_mm2 <= radius_squared {
        Some(PolyhedralSweptPathContact {
            span: 0,
            parameter_bounds: [Q::zero(), Q::zero()],
        })
    } else {
        None
    };
    if contact.is_none() {
        for (span, pair) in centers_mm.windows(2).enumerate() {
            let [start, end] = [&pair[0], &pair[1]];
            let sweep = swept_segment(&faces, start, end, &radius_squared, &mut remaining)?;
            if sweep.clearance == Ordering::Greater {
                continue;
            }
            let parameter_bounds = first_parameter_from_clear_start(
                &faces,
                start,
                end,
                &radius_squared,
                &width,
                &mut remaining,
            )?;
            contact = Some(PolyhedralSweptPathContact {
                span,
                parameter_bounds,
            });
            break;
        }
    }
    Ok(PolyhedralSweptPathFirstContact {
        contact,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSweptFaceContactRange {
    pub span: usize,
    pub face: usize,
    pub first_parameter_bounds: [Q; 2],
    pub last_parameter_bounds: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyhedralSweptFaceContactRanges {
    /// Ordered by path span, then source face index.
    pub contacts: Vec<PolyhedralSweptFaceContactRange>,
    pub exact_work: u64,
}

/// Enclose the first and last contact parameters of each retained face on
/// each span. A face can have disjoint contacts, for instance across a hole;
/// the two enclosures describe the extremes of that contact set.
pub fn bounded_polyhedral_swept_face_contact_ranges(
    solid: &QualifiedClosedSolid,
    centers_mm: &[[Q; 3]],
    radius_mm: &Q,
    max_parameter_width: f64,
    max_exact_work: u64,
) -> Result<PolyhedralSweptFaceContactRanges> {
    let radius_squared = squared_radius(radius_mm)?;
    let width = super::super::enclosure_width(max_parameter_width)?;
    if centers_mm.len() < 2 {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Sweep path requires at least two centers",
        ));
    }
    let mut remaining = max_exact_work;
    let faces = prepare_faces(solid, &mut remaining)?;
    for center in centers_mm {
        spend(&mut remaining, 1)?;
        validate_point(center)?;
    }

    let mut contacts = Vec::new();
    for (span, pair) in centers_mm.windows(2).enumerate() {
        let (start, end) = (&pair[0], &pair[1]);
        for (face, boundary) in &faces {
            let single = std::iter::once((face, boundary));
            let sweep = swept_segment(single.clone(), start, end, &radius_squared, &mut remaining)?;
            if sweep.clearance == Ordering::Greater {
                continue;
            }
            let mut bounds = |a: &[Q; 3], b: &[Q; 3]| -> Result<[Q; 2]> {
                if swept_segment(single.clone(), a, a, &radius_squared, &mut remaining)?.clearance
                    != Ordering::Greater
                {
                    return Ok([Q::zero(), Q::zero()]);
                }
                first_parameter_from_clear_start(
                    single.clone(),
                    a,
                    b,
                    &radius_squared,
                    &width,
                    &mut remaining,
                )
            };
            let first_parameter_bounds = bounds(start, end)?;
            let [reverse_lo, reverse_hi] = bounds(end, start)?;
            let last_parameter_bounds = [Q::one() - reverse_hi, Q::one() - reverse_lo];
            contacts.push(PolyhedralSweptFaceContactRange {
                span,
                face: *face,
                first_parameter_bounds,
                last_parameter_bounds,
            });
        }
    }
    Ok(PolyhedralSweptFaceContactRanges {
        contacts,
        exact_work: max_exact_work - remaining,
    })
}

fn first_parameter_from_clear_start<'a>(
    faces: impl IntoIterator<Item = (&'a usize, &'a FaceBoundary)> + Clone,
    start: &[Q; 3],
    end: &[Q; 3],
    radius_squared: &Q,
    width: &Q,
    remaining: &mut u64,
) -> Result<[Q; 2]> {
    let direction = delta(end, start);
    let (mut lo, mut hi) = (Q::zero(), Q::one());
    let two = Q::from_integer(2.into());
    while &hi - &lo > *width {
        spend(remaining, 16)?;
        let mid = (&lo + &hi) / &two;
        let center = at(start, &direction, &mid);
        let prefix = swept_segment(faces.clone(), start, &center, radius_squared, remaining)?;
        if prefix.clearance == Ordering::Greater {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok([lo, hi])
}
