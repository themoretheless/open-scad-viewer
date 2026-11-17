use super::{
    FaceBoundary, FaceLocation, dot, face_normal, nearest_segment_point, plane_axes, point_in_face,
    prepare_faces, refuse, spend, validate_point,
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
    let mut result: Option<ExactPolyhedralSweptPathClearance> = None;
    for (span, pair) in centers_mm.windows(2).enumerate() {
        let nearest = swept_segment(&faces, &pair[0], &pair[1], &radius_squared, &mut remaining)?;
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
    let mut report = result.expect("a sweep path has at least one span");
    report.exact_work = max_exact_work - remaining;
    Ok(report)
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

fn swept_segment(
    faces: &BTreeMap<usize, FaceBoundary>,
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
