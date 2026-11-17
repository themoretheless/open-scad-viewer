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
    let outcome = match integrate_contours::<3>(loops, &mut work)? {
        Ok(total) => mass_from_total(&total, &mut work)
            .map(ExactPlanarMassOutcome::Mass)
            .unwrap_or_else(ExactPlanarMassOutcome::Refused),
        Err(outcome) => outcome,
    };
    Ok(ExactPlanarMassReport {
        outcome,
        work_used: work.used,
    })
}

fn integrate_contours<const N: usize>(
    loops: &[Vec<Curve>],
    work: &mut Work,
) -> Result<std::result::Result<[Q; N], ExactPlanarMassOutcome>> {
    for (contour, curves) in loops.iter().enumerate() {
        if exact_loop_joins(curves)? != Some(true) {
            return Ok(Err(ExactPlanarMassOutcome::UnprovedClosure { contour }));
        }
    }
    let mut total = std::array::from_fn(|_| Q::zero());
    for (contour, curves) in loops.iter().enumerate() {
        let result = visit_spans(curves, work, |[x, y, w], work| {
            let values = if w.len() == 1 {
                polynomial_mass::<N>(x, y, &w[0], work)?
            } else {
                let w1 = at_one(w, work)?;
                if !w[0].is_positive() || !w1.is_positive() {
                    return Err(Refusal::UnprovedDenominator);
                }
                let x = unit_chord(x, &w[0], &w1, work)?;
                let y = unit_chord(y, &w[0], &w1, work)?;
                polynomial_mass::<N>(&x, &y, &Q::from_integer(1.into()), work)?
            };
            for (sum, value) in total.iter_mut().zip(values) {
                *sum = work.op(sum, &value, |a, b| a + b)?;
            }
            Ok(())
        });
        match result {
            Ok(Some((curve, span))) => {
                return Ok(Err(ExactPlanarMassOutcome::UnsupportedRationalSpan {
                    contour,
                    curve,
                    span,
                }));
            }
            Err(reason) => return Ok(Err(ExactPlanarMassOutcome::Refused(reason))),
            Ok(None) => {}
        }
    }
    Ok(Ok(total))
}

fn unit_chord(h: &[Q], w0: &Q, w1: &Q, work: &mut Work) -> PResult<Vec<Q>> {
    let zero = Q::zero();
    let start = work.op(h.first().unwrap_or(&zero), w0, |a, b| a / b)?;
    let end = at_one(h, work)?;
    let end = work.op(&end, w1, |a, b| a / b)?;
    let delta = work.op(&end, &start, |a, b| a - b)?;
    Ok(vec![start, delta])
}

fn mass_from_total(total: &[Q], work: &mut Work) -> PResult<ExactPlanarMass> {
    let signed_area = work.op(&total[0], &Q::from_integer(2.into()), |a, b| a / b)?;
    let three = Q::from_integer(3.into());
    let first_moments = [
        work.op(&total[1], &three, |a, b| a / b)?,
        work.op(&total[2], &three, |a, b| a / b)?,
    ];
    let centroid = if signed_area.is_zero() {
        None
    } else {
        Some([
            work.op(&first_moments[0], &signed_area, |a, b| a / b)?,
            work.op(&first_moments[1], &signed_area, |a, b| a / b)?,
        ])
    };
    Ok(ExactPlanarMass {
        signed_area,
        first_moments,
        centroid,
    })
}

fn polynomial_mass<const N: usize>(x: &[Q], y: &[Q], w: &Q, work: &mut Work) -> PResult<[Q; N]> {
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
    let mut result = std::array::from_fn(|_| Q::zero());
    result[0] = integrate_polynomial(&cross, &w2, work)?;
    let xc = poly::multiply(&x, &cross, work)?;
    let yc = poly::multiply(&y, &cross, work)?;
    result[1] = integrate_polynomial(&xc, &w3, work)?;
    result[2] = integrate_polynomial(&yc, &w3, work)?;
    if N == 6 {
        let w4 = work.op(&w3, w, |a, b| a * b)?;
        for (i, (p, q)) in [(&x, &xc), (&y, &xc), (&y, &yc)].into_iter().enumerate() {
            let term = poly::multiply(p, q, work)?;
            result[i + 3] = integrate_polynomial(&term, &w4, work)?;
        }
    }
    Ok(result)
}

fn integrate_polynomial(poly: &[Q], weight: &Q, work: &mut Work) -> PResult<Q> {
    let integral = poly.iter().enumerate().try_fold(Q::zero(), |sum, (i, q)| {
        let denominator = Q::from_integer((i + 1).into());
        let term = work.op(q, &denominator, |a, b| a / b)?;
        work.op(&sum, &term, |a, b| a + b)
    })?;
    Ok(work.op(&integral, weight, |a, b| a / b)?)
}

fn at_one(a: &[Q], work: &mut Work) -> PResult<Q> {
    Ok(a.iter()
        .try_fold(Q::zero(), |sum, v| work.op(&sum, v, |a, b| a + b))?)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarSecondMoments {
    /// Integrals of [x², xy, y²] about the chart origin.
    pub about_origin: [Q; 3],
    /// The corresponding integrals about the centroid, if net area is nonzero.
    pub about_centroid: Option<[Q; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarInertiaReport {
    pub mass: ExactPlanarMassOutcome,
    pub second_moments: Option<ExactPlanarSecondMoments>,
    pub work_used: u64,
}

/// Integrate exact signed second area moments of supported planar boundaries.
/// Tuple order is x², xy, y². All moments follow the contour winding. The
/// centroidal values use the parallel-axis identity in exact arithmetic.
pub fn exact_planar_boundary_inertia(
    loops: &[Vec<Curve>],
    max_work: u64,
) -> Result<ExactPlanarInertiaReport> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work) && (1..=256).contains(&loops.len()),
        "Exact planar inertia needs bounded contours and a valid work budget",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = match integrate_contours::<6>(loops, &mut work)? {
        Ok(total) => inertia_from_total(&total, &mut work).map_err(ExactPlanarMassOutcome::Refused),
        Err(reason) => Err(reason),
    };
    let (mass, second_moments) = match result {
        Ok((mass, inertia)) => (ExactPlanarMassOutcome::Mass(mass), Some(inertia)),
        Err(reason) => (reason, None),
    };
    Ok(ExactPlanarInertiaReport {
        mass,
        second_moments,
        work_used: work.used,
    })
}

fn inertia_from_total(
    total: &[Q; 6],
    work: &mut Work,
) -> PResult<(ExactPlanarMass, ExactPlanarSecondMoments)> {
    let mass = mass_from_total(total, work)?;
    let four = Q::from_integer(4.into());
    let mut about_origin = std::array::from_fn(|_| Q::zero());
    for (i, value) in about_origin.iter_mut().enumerate() {
        *value = work.op(&total[i + 3], &four, |a, b| a / b)?;
    }
    let about_centroid = if let Some([x, y]) = &mass.centroid {
        let mut shifted = about_origin.clone();
        for (i, (a, b)) in [(x, x), (x, y), (y, y)].into_iter().enumerate() {
            let product = work.op(a, b, |a, b| a * b)?;
            let offset = work.op(&mass.signed_area, &product, |a, b| a * b)?;
            shifted[i] = work.op(&shifted[i], &offset, |a, b| a - b)?;
        }
        Some(shifted)
    } else {
        None
    };
    Ok((
        mass,
        ExactPlanarSecondMoments {
            about_origin,
            about_centroid,
        },
    ))
}
