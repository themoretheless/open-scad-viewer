//! Rational XYZ enclosures of exact algebraic intersection points.
use super::{AlgebraicBoundaryClock, ExactSupportLine, Refusal, Work, check};
use crate::Result;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBoundaryEnclosure {
    /// Closed interval in the canonical support clock.
    pub parameter: [Q; 2],
    /// Closed XYZ coordinate bounds. All coordinates use the same clock interval.
    pub xyz: [[Q; 2]; 3],
    /// Exact squared Euclidean diameter of the XYZ box.
    pub diameter_squared: Q,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBoundaryEnclosureReport {
    pub outcome: std::result::Result<AlgebraicBoundaryEnclosure, Refusal>,
    pub work_used: u64,
}

/// Enclose a rational or quadratic point on the support line. The Euclidean
/// diameter of the entire XYZ box is at most the requested absolute tolerance.
/// Zero tolerance is accepted for points with exact rational coordinates.
pub fn enclose_algebraic_planar_boundary_point(
    line: &ExactSupportLine,
    point: &AlgebraicBoundaryClock,
    xyz_tolerance: f64,
    max_work: u64,
) -> Result<AlgebraicBoundaryEnclosureReport> {
    check(
        line.clock_axis < 3
            && line.origin[line.clock_axis].is_zero()
            && line.direction[line.clock_axis].is_one(),
        "Point enclosure requires a canonical support line clock",
    )?;
    check(
        xyz_tolerance.is_finite()
            && xyz_tolerance >= 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Point enclosure needs a finite nonnegative XYZ tolerance and bounded work",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = enclose(line, point, xyz_tolerance, &mut work);
    Ok(AlgebraicBoundaryEnclosureReport {
        outcome,
        work_used: work.used,
    })
}

type Checked<T> = std::result::Result<T, Refusal>;

fn enclose(
    line: &ExactSupportLine,
    point: &AlgebraicBoundaryClock,
    xyz_tolerance: f64,
    work: &mut Work,
) -> Checked<AlgebraicBoundaryEnclosure> {
    let tolerance = work.lift(xyz_tolerance)?;
    let tolerance_squared = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let scale_squared = support_scale_squared(line, work)?;
    let parameter = algebraic_parameter_range(point, &scale_squared, &tolerance_squared, work)?;
    let mut bounds = Vec::with_capacity(3);
    for (origin, direction) in line.origin.iter().zip(&line.direction) {
        let calculate = |t: &Q, work: &mut Work| -> Checked<Q> {
            let delta = work.op(direction, t, |a, b| a * b)?;
            Ok(work.op(origin, &delta, |a, b| a + b)?)
        };
        let lo = calculate(&parameter[0], work)?;
        let hi = calculate(&parameter[1], work)?;
        bounds.push(if direction.is_negative() {
            [hi, lo]
        } else {
            [lo, hi]
        });
    }
    let xyz = bounds.try_into().map_err(|_| Refusal::InvalidOutput)?;
    let diameter_squared = scaled_width_squared(&parameter, &scale_squared, work)?;
    Ok(AlgebraicBoundaryEnclosure {
        parameter,
        xyz,
        diameter_squared,
    })
}

pub(super) fn support_scale_squared(line: &ExactSupportLine, work: &mut Work) -> Checked<Q> {
    let mut scale_squared = Q::zero();
    for direction in &line.direction {
        let square = work.op(direction, direction, |a, b| a * b)?;
        scale_squared = work.op(&scale_squared, &square, |a, b| a + b)?;
    }
    Ok(scale_squared)
}

pub(super) fn algebraic_parameter_range(
    point: &AlgebraicBoundaryClock,
    scale_squared: &Q,
    tolerance_squared: &Q,
    work: &mut Work,
) -> Checked<[Q; 2]> {
    Ok(match point {
        AlgebraicBoundaryClock::Rational(value) => [value.clone(), value.clone()],
        AlgebraicBoundaryClock::Quadratic {
            linear,
            constant,
            upper_root,
        } => quadratic_range(
            linear,
            constant,
            *upper_root,
            scale_squared,
            tolerance_squared,
            work,
        )?,
    })
}

fn quadratic_range(
    linear: &Q,
    constant: &Q,
    upper_root: bool,
    scale_squared: &Q,
    tolerance_squared: &Q,
    work: &mut Work,
) -> Checked<[Q; 2]> {
    let four = Q::from_integer(4.into());
    let linear_squared = work.op(linear, linear, |a, b| a * b)?;
    let offset = work.op(constant, &four, |a, b| a * b)?;
    let discriminant = work.op(&linear_squared, &offset, |a, b| a - b)?;
    if discriminant.is_negative() {
        return Err(Refusal::InvalidOutput);
    }
    let limit_squared = work.op(tolerance_squared, &four, |a, b| a * b)?;
    let [lo, hi] = square_root_range(discriminant, scale_squared, &limit_squared, work)?;
    let two = Q::from_integer(2.into());
    let [lo, hi] = if upper_root { [lo, hi] } else { [-hi, -lo] };
    let map = |x: &Q, work: &mut Work| -> Checked<Q> {
        let difference = work.op(x, linear, |a, b| a - b)?;
        Ok(work.op(&difference, &two, |a, b| a / b)?)
    };
    Ok([map(&lo, work)?, map(&hi, work)?])
}

fn square_root_range(
    value: Q,
    scale_squared: &Q,
    limit_squared: &Q,
    work: &mut Work,
) -> Checked<[Q; 2]> {
    work.charge(1 + (value.numer().bits() + value.denom().bits()) / 64)?;
    let sqrt = Q::new(value.numer().sqrt(), value.denom().sqrt());
    if work.op(&sqrt, &sqrt, |a, b| a * b)? == value {
        return Ok([sqrt.clone(), sqrt]);
    }
    if limit_squared.is_zero() {
        return Err(Refusal::NotRepresentable);
    }
    let two = Q::from_integer(2.into());
    let mut range = [Q::zero(), Q::one().max(value.clone())];
    while scaled_width_squared(&range, scale_squared, work)? > *limit_squared {
        let sum = work.op(&range[0], &range[1], |a, b| a + b)?;
        let middle = work.op(&sum, &two, |a, b| a / b)?;
        let square = work.op(&middle, &middle, |a, b| a * b)?;
        range[usize::from(square > value)] = middle;
    }
    Ok(range)
}

pub(super) fn scaled_width_squared(
    [lo, hi]: &[Q; 2],
    scale_squared: &Q,
    work: &mut Work,
) -> Checked<Q> {
    let width = work.op(hi, lo, |a, b| a - b)?;
    let square = work.op(&width, &width, |a, b| a * b)?;
    Ok(work.op(&square, scale_squared, |a, b| a * b)?)
}
