//! Exact line of intersection of two affine support planes.
use super::exact_affine_hull::{PlanarSurfaceProjection, validate_projection_axes};
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSupportLine {
    pub origin: [Q; 3],
    pub direction: [Q; 3],
    /// The line parameter equals the XYZ coordinate on this axis.
    pub clock_axis: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportPlaneIntersection {
    Line(ExactSupportLine),
    Parallel,
    Coincident,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportPlaneIntersectionReport {
    pub outcome: SupportPlaneIntersection,
    pub work_used: u64,
}

/// Intersect two exact affine plane charts. The dominant line component
/// defines the canonical parameter axis, so interchanging the inputs gives
/// the same line clock. Coefficients stay rational through the entire solve.
pub fn intersect_planar_supports(
    left: &PlanarSurfaceProjection,
    right: &PlanarSurfaceProjection,
    max_work: u64,
) -> Result<SupportPlaneIntersectionReport> {
    validate_projection_axes(left)?;
    validate_projection_axes(right)?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Plane intersection needs a valid arithmetic budget",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = solve(left, right, &mut work).unwrap_or(SupportPlaneIntersection::ResourceLimit);
    Ok(SupportPlaneIntersectionReport {
        outcome,
        work_used: work.used,
    })
}

fn normal(projection: &PlanarSurfaceProjection) -> [Q; 3] {
    let mut n = std::array::from_fn(|_| Q::zero());
    for (axis, slope) in projection.retained_axes.into_iter().zip(&projection.slopes) {
        n[axis] = -slope;
    }
    n[projection.omitted_axis] = Q::one();
    n
}

type Exact<T> = std::result::Result<T, Error>;
const COMPLEMENT: [(usize, usize); 3] = [(1, 2), (2, 0), (0, 1)];

fn det(a: &Q, b: &Q, c: &Q, d: &Q, work: &mut Work) -> Exact<Q> {
    let ab = work.op(a, b, |a, b| a * b)?;
    let cd = work.op(c, d, |a, b| a * b)?;
    work.op(&ab, &cd, |a, b| a - b)
}

fn solve(
    left: &PlanarSurfaceProjection,
    right: &PlanarSurfaceProjection,
    work: &mut Work,
) -> Exact<SupportPlaneIntersection> {
    let a = normal(left);
    let b = normal(right);
    let mut direction = std::array::from_fn(|_| Q::zero());
    let mut clock_axis = 0;
    for (axis, (i, j)) in COMPLEMENT.into_iter().enumerate() {
        direction[axis] = det(&a[i], &b[j], &a[j], &b[i], work)?;
        if direction[axis].abs() > direction[clock_axis].abs() {
            clock_axis = axis;
        }
    }
    let pivot = direction[clock_axis].clone();
    if pivot.is_zero() {
        let k = left.omitted_axis;
        let residual = det(&left.offset, &b[k], &right.offset, &a[k], work)?;
        return Ok(if residual.is_zero() {
            SupportPlaneIntersection::Coincident
        } else {
            SupportPlaneIntersection::Parallel
        });
    }
    let (i, j) = COMPLEMENT[clock_axis];
    let u = det(&left.offset, &b[j], &a[j], &right.offset, work)?;
    let v = det(&a[i], &right.offset, &left.offset, &b[i], work)?;
    let mut origin = std::array::from_fn(|_| Q::zero());
    origin[i] = work.op(&u, &pivot, |a, b| a / b)?;
    origin[j] = work.op(&v, &pivot, |a, b| a / b)?;
    for value in &mut direction {
        *value = work.op(value, &pivot, |a, b| a / b)?;
    }
    Ok(SupportPlaneIntersection::Line(ExactSupportLine {
        origin,
        direction,
        clock_axis,
    }))
}
