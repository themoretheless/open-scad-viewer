//! Exact ray intersection with a bounded planar rational face.
use super::{
    exact_affine_hull::validate_projection_axes, planar_face_area::rational_interval,
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    trim::trim_domain::{Classification, Location, TrimDomain, exact_loop_joins},
};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlaneRayPoint {
    pub parameter: Q,
    pub point: [Q; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlanarFaceRayOutcome {
    Hit {
        intersection: ExactPlaneRayPoint,
        classification: Classification,
    },
    Miss {
        intersection: ExactPlaneRayPoint,
        classification: Option<Classification>,
    },
    BoundaryUnresolved {
        intersection: ExactPlaneRayPoint,
        classification: Classification,
    },
    Behind {
        parameter: Q,
    },
    Parallel,
    Coplanar,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanarFaceRayReport {
    pub outcome: PlanarFaceRayOutcome,
    pub work_used: u64,
}

/// Cast the half-line origin + t * direction, t >= 0. The plane hit is
/// rational in the original ray parameter. An interval enclosure of its
/// retained coordinates is tested against the exact closed winding domain.
/// Near-boundary classifications remain unresolved. Coplanar rays have
/// their own outcome because their intersection can be an interval.
pub fn cast_projected_face_ray(
    face: &ProjectedTrimmedFace,
    origin: [f64; 3],
    direction: [f64; 3],
    winding_tolerance: f64,
    max_winding_cells: usize,
    max_work: u64,
) -> Result<PlanarFaceRayReport> {
    validate_projection_axes(&face.projection)?;
    check(
        origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|&d| d != 0.)
            && (1..=cad_predicates::MAX_WORK).contains(&max_work)
            && (1..=100_000).contains(&max_winding_cells),
        "Planar ray needs a finite nonzero direction and bounded work",
    )?;
    for contour in &face.loops {
        check(
            exact_loop_joins(contour)? == Some(true),
            "Planar ray needs exact closed contours",
        )?;
    }
    let domain = TrimDomain::new(&face.loops, winding_tolerance)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = match intersect_plane(face, origin, direction, &mut work) {
        Ok(PlaneIntersection::Point(intersection)) => {
            classify(face, &domain, intersection, max_winding_cells)?
        }
        Ok(PlaneIntersection::Behind(parameter)) => PlanarFaceRayOutcome::Behind { parameter },
        Ok(PlaneIntersection::Parallel) => PlanarFaceRayOutcome::Parallel,
        Ok(PlaneIntersection::Coplanar) => PlanarFaceRayOutcome::Coplanar,
        Err(_) => PlanarFaceRayOutcome::ResourceLimit,
    };
    Ok(PlanarFaceRayReport {
        outcome,
        work_used: work.used,
    })
}

pub(super) enum PlaneIntersection {
    Point(ExactPlaneRayPoint),
    Behind(Q),
    Parallel,
    Coplanar,
}

pub(super) fn intersect_plane(
    face: &ProjectedTrimmedFace,
    origin: [f64; 3],
    direction: [f64; 3],
    work: &mut Work,
) -> std::result::Result<PlaneIntersection, Error> {
    let origin = [
        work.lift(origin[0])?,
        work.lift(origin[1])?,
        work.lift(origin[2])?,
    ];
    let direction = [
        work.lift(direction[0])?,
        work.lift(direction[1])?,
        work.lift(direction[2])?,
    ];
    let chart = &face.projection;
    let [i, j] = chart.retained_axes;
    let k = chart.omitted_axis;
    let signed = |point: &[Q; 3], w: &mut Work| {
        let x = w.op(&chart.slopes[0], &point[i], |a, b| a * b)?;
        let y = w.op(&chart.slopes[1], &point[j], |a, b| a * b)?;
        let slope = w.op(&x, &y, |a, b| a + b)?;
        w.op(&point[k], &slope, |a, b| a - b)
    };
    let start = signed(&origin, work)?;
    let delta = signed(&direction, work)?;
    let residual = work.op(&start, &chart.offset, |a, b| a - b)?;
    if delta.is_zero() {
        return Ok(if residual.is_zero() {
            PlaneIntersection::Coplanar
        } else {
            PlaneIntersection::Parallel
        });
    }
    let parameter = work.op(&residual, &delta, |a, b| -a / b)?;
    if parameter.is_negative() {
        return Ok(PlaneIntersection::Behind(parameter));
    }
    let mut point = origin;
    for (coordinate, direction) in point.iter_mut().zip(direction) {
        let shift = work.op(&direction, &parameter, |a, b| a * b)?;
        *coordinate = work.op(coordinate, &shift, |a, b| a + b)?;
    }
    Ok(PlaneIntersection::Point(ExactPlaneRayPoint {
        parameter,
        point,
    }))
}

fn classify(
    face: &ProjectedTrimmedFace,
    domain: &TrimDomain,
    intersection: ExactPlaneRayPoint,
    max_cells: usize,
) -> Result<PlanarFaceRayOutcome> {
    let [u, v] = face
        .projection
        .retained_axes
        .map(|axis| &intersection.point[axis]);
    // Valid curve poles have a coordinate magnitude cap of 1e9. A point
    // beyond that cap cannot hit a region enclosed by those contours.
    let limit = Q::from_integer(1_000_000_000.into());
    if u.abs() > limit || v.abs() > limit {
        return Ok(PlanarFaceRayOutcome::Miss {
            intersection,
            classification: None,
        });
    }
    let [u, v] = [rational_interval(u)?, rational_interval(v)?];
    let classification = domain.classify([[u.lo, u.hi], [v.lo, v.hi]], max_cells)?;
    Ok(match classification.location {
        Location::Inside => PlanarFaceRayOutcome::Hit {
            intersection,
            classification,
        },
        Location::Outside => PlanarFaceRayOutcome::Miss {
            intersection,
            classification: Some(classification),
        },
        Location::Unresolved => PlanarFaceRayOutcome::BoundaryUnresolved {
            intersection,
            classification,
        },
    })
}
