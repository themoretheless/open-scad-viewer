//! Exact Green area of polynomial spans and rational line segments.
use super::{PResult, Refusal, extraction};
use crate::{
    Result, check, core::bounded_rational::Work, curve::Curve, trim::trim_domain::exact_loop_joins,
};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactPlanarAreaOutcome {
    Area(Q),
    UnsupportedRationalSpan { curve: usize, span: usize },
    UnprovedClosure,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarAreaReport {
    pub outcome: ExactPlanarAreaOutcome,
    pub work_used: u64,
}

/// Integrate half of x dy - y dx on the exact original span polynomials.
/// Constant homogeneous weights admit arbitrary supported spline degrees.
/// Degree-one rational spans integrate to their endpoint cross product.
/// The returned signed area is exact in the 2D coordinates. Simplicity
/// and material winding are independent properties.
pub fn exact_planar_boundary_area(
    curves: &[Curve],
    max_work: u64,
) -> Result<ExactPlanarAreaReport> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact area needs a valid work budget",
    )?;
    if exact_loop_joins(curves)? != Some(true) {
        return Ok(ExactPlanarAreaReport {
            outcome: ExactPlanarAreaOutcome::UnprovedClosure,
            work_used: 0,
        });
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = integrate(curves, &mut work).unwrap_or_else(ExactPlanarAreaOutcome::Refused);
    Ok(ExactPlanarAreaReport {
        outcome,
        work_used: work.used,
    })
}

fn integrate(curves: &[Curve], work: &mut Work) -> PResult<ExactPlanarAreaOutcome> {
    let mut twice_area = Q::zero();
    for (index, curve) in curves.iter().enumerate() {
        for span in curve.degree..curve.control_points.len() {
            let a = curve.knots[span];
            let b = curve.knots[span + 1];
            if a == b {
                continue;
            }
            let a = work.lift(a)?;
            let b = work.lift(b)?;
            let width = work.op(&b, &a, |a, b| a - b)?;
            let h = extraction::span(curve, span, &a, &width, work)?;
            let [x, y, w] = [&h[0], &h[1], &h[2]];
            let contribution = if w.len() == 1 {
                polynomial_green(x, y, &w[0], work)?
            } else if curve.degree == 1 {
                segment_green(x, y, w, work)?
            } else {
                return Ok(ExactPlanarAreaOutcome::UnsupportedRationalSpan { curve: index, span });
            };
            twice_area = work.op(&twice_area, &contribution, |a, b| a + b)?;
        }
    }
    let two = Q::from_integer(2.into());
    Ok(ExactPlanarAreaOutcome::Area(work.op(
        &twice_area,
        &two,
        |a, b| a / b,
    )?))
}

fn polynomial_green(x: &[Q], y: &[Q], w: &Q, work: &mut Work) -> PResult<Q> {
    if !w.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let mut integral = Q::zero();
    for (i, a) in x.iter().enumerate() {
        for (j, b) in y.iter().enumerate() {
            if i == j || a.is_zero() || b.is_zero() {
                continue;
            }
            // x_i y_j (j-i) t^(i+j-1), integrated on the unit interval.
            let factor = Q::new((j as i64 - i as i64).into(), (i + j).into());
            let xy = work.op(a, b, |a, b| a * b)?;
            let term = work.op(&xy, &factor, |a, b| a * b)?;
            integral = work.op(&integral, &term, |a, b| a + b)?;
        }
    }
    let denominator = work.op(w, w, |a, b| a * b)?;
    Ok(work.op(&integral, &denominator, |a, b| a / b)?)
}

fn segment_green(x: &[Q], y: &[Q], w: &[Q], work: &mut Work) -> PResult<Q> {
    let zero = Q::zero();
    let end = |a: &[Q], work: &mut Work| {
        a.iter()
            .try_fold(Q::zero(), |sum, v| work.op(&sum, v, |a, b| a + b))
    };
    let x1 = end(x, work)?;
    let y1 = end(y, work)?;
    let w1 = end(w, work)?;
    if !w[0].is_positive() || !w1.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let a = work.op(x.first().unwrap_or(&zero), &y1, |a, b| a * b)?;
    let b = work.op(y.first().unwrap_or(&zero), &x1, |a, b| a * b)?;
    let cross = work.op(&a, &b, |a, b| a - b)?;
    let denominator = work.op(&w[0], &w1, |a, b| a * b)?;
    Ok(work.op(&cross, &denominator, |a, b| a / b)?)
}
