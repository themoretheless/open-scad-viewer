//! Exact Green area of polynomial spans and rational line segments.
use super::{PResult, Refusal, add, differentiated, extraction, poly};
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
    if let Some((curve, span)) = visit_spans(curves, work, |[x, y, w], work| {
        let contribution = if w.len() == 1 {
            polynomial_green(x, y, &w[0], work)?
        } else {
            segment_green(x, y, w, work)?
        };
        twice_area = work.op(&twice_area, &contribution, |a, b| a + b)?;
        Ok(())
    })? {
        return Ok(ExactPlanarAreaOutcome::UnsupportedRationalSpan { curve, span });
    }
    let two = Q::from_integer(2.into());
    Ok(ExactPlanarAreaOutcome::Area(work.op(
        &twice_area,
        &two,
        |a, b| a / b,
    )?))
}

fn visit_spans(
    curves: &[Curve],
    work: &mut Work,
    mut visit: impl FnMut([&[Q]; 3], &mut Work) -> PResult<()>,
) -> PResult<Option<(usize, usize)>> {
    for (index, curve) in curves.iter().enumerate() {
        for span in curve.degree..curve.control_points.len() {
            let [a, b] = [curve.knots[span], curve.knots[span + 1]];
            if a == b {
                continue;
            }
            let a = work.lift(a)?;
            let b = work.lift(b)?;
            let width = work.op(&b, &a, |a, b| a - b)?;
            let h = extraction::span(curve, span, &a, &width, work)?;
            if h[2].len() != 1 && curve.degree != 1 {
                return Ok(Some((index, span)));
            }
            visit([&h[0], &h[1], &h[2]], work)?;
        }
    }
    Ok(None)
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
    let x1 = at_one(x, work)?;
    let y1 = at_one(y, work)?;
    let w1 = at_one(w, work)?;
    if !w[0].is_positive() || !w1.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let a = work.op(x.first().unwrap_or(&zero), &y1, |a, b| a * b)?;
    let b = work.op(y.first().unwrap_or(&zero), &x1, |a, b| a * b)?;
    let cross = work.op(&a, &b, |a, b| a - b)?;
    let denominator = work.op(&w[0], &w1, |a, b| a * b)?;
    Ok(work.op(&cross, &denominator, |a, b| a / b)?)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarMass {
    pub signed_area: Q,
    /// Signed integrals of x and y over the oriented area.
    pub first_moments: [Q; 2],
    /// Absent for zero net signed area.
    pub centroid: Option<[Q; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactPlanarMassOutcome {
    Mass(ExactPlanarMass),
    UnsupportedRationalSpan {
        contour: usize,
        curve: usize,
        span: usize,
    },
    UnprovedClosure {
        contour: usize,
    },
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarMassReport {
    pub outcome: ExactPlanarMassOutcome,
    pub work_used: u64,
}

/// Compute exact signed area, first moments and centroid for closed planar
/// contours made of polynomial spans or degree-one rational spans. Opposite
/// winding subtracts a hole. A zero signed area has no centroid. The input
/// winding defines the integral and must be appropriate for the material.
pub fn exact_planar_boundary_mass(
    loops: &[Vec<Curve>],
    max_work: u64,
) -> Result<ExactPlanarMassReport> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work) && (1..=256).contains(&loops.len()),
        "Exact planar mass needs bounded contours and a valid work budget",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    for (contour, curves) in loops.iter().enumerate() {
        if exact_loop_joins(curves)? != Some(true) {
            return Ok(ExactPlanarMassReport {
                outcome: ExactPlanarMassOutcome::UnprovedClosure { contour },
                work_used: 0,
            });
        }
    }
    let outcome = integrate_mass(loops, &mut work).unwrap_or_else(ExactPlanarMassOutcome::Refused);
    Ok(ExactPlanarMassReport {
        outcome,
        work_used: work.used,
    })
}

fn integrate_mass(loops: &[Vec<Curve>], work: &mut Work) -> PResult<ExactPlanarMassOutcome> {
    let mut total = [Q::zero(), Q::zero(), Q::zero()];
    for (contour, curves) in loops.iter().enumerate() {
        if let Some((curve, span)) = visit_spans(curves, work, |[x, y, w], work| {
            let values = if w.len() == 1 {
                polynomial_mass(x, y, &w[0], work)?
            } else {
                let cross = segment_green(x, y, w, work)?;
                let w1 = at_one(w, work)?;
                let mut values = [cross, Q::zero(), Q::zero()];
                for (k, h) in [x, y].into_iter().enumerate() {
                    let zero = Q::zero();
                    let a = work.op(h.first().unwrap_or(&zero), &w[0], |a, b| a / b)?;
                    let end = at_one(h, work)?;
                    let b = work.op(&end, &w1, |a, b| a / b)?;
                    let mean = work.op(&a, &b, |a, b| (a + b) / Q::from_integer(2.into()))?;
                    values[k + 1] = work.op(&values[0], &mean, |a, b| a * b)?;
                }
                values
            };
            for (sum, value) in total.iter_mut().zip(values) {
                *sum = work.op(sum, &value, |a, b| a + b)?;
            }
            Ok(())
        })? {
            return Ok(ExactPlanarMassOutcome::UnsupportedRationalSpan {
                contour,
                curve,
                span,
            });
        }
    }
    let [area, x, y] = total;
    let signed_area = work.op(&area, &Q::from_integer(2.into()), |a, b| a / b)?;
    let three = Q::from_integer(3.into());
    let first_moments = [
        work.op(&x, &three, |a, b| a / b)?,
        work.op(&y, &three, |a, b| a / b)?,
    ];
    let centroid = if signed_area.is_zero() {
        None
    } else {
        Some([
            work.op(&first_moments[0], &signed_area, |a, b| a / b)?,
            work.op(&first_moments[1], &signed_area, |a, b| a / b)?,
        ])
    };
    Ok(ExactPlanarMassOutcome::Mass(ExactPlanarMass {
        signed_area,
        first_moments,
        centroid,
    }))
}

fn polynomial_mass(x: &[Q], y: &[Q], w: &Q, work: &mut Work) -> PResult<[Q; 3]> {
    if !w.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let x = x.to_vec();
    let y = y.to_vec();
    let dx = differentiated(&x, work)?;
    let dy = differentiated(&y, work)?;
    let a = poly::multiply(&x, &dy, work)?;
    let b = poly::multiply(&y, &dx, work)?;
    let cross = add(&a, &b, true, work)?;
    let w2 = work.op(w, w, |a, b| a * b)?;
    let w3 = work.op(&w2, w, |a, b| a * b)?;
    let mut result = [Q::zero(), Q::zero(), Q::zero()];
    for (output, polynomial, denominator) in [
        (0, cross.clone(), &w2),
        (1, poly::multiply(&x, &cross, work)?, &w3),
        (2, poly::multiply(&y, &cross, work)?, &w3),
    ] {
        let integral = polynomial
            .iter()
            .enumerate()
            .try_fold(Q::zero(), |sum, (i, q)| {
                let denominator = Q::from_integer((i + 1).into());
                let term = work.op(q, &denominator, |a, b| a / b)?;
                work.op(&sum, &term, |a, b| a + b)
            })?;
        result[output] = work.op(&integral, denominator, |a, b| a / b)?;
    }
    Ok(result)
}

fn at_one(a: &[Q], work: &mut Work) -> PResult<Q> {
    Ok(a.iter()
        .try_fold(Q::zero(), |sum, v| work.op(&sum, v, |a, b| a + b))?)
}
