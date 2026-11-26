//! Exact continuous angular tolerance for paired surface tangent planes.
use super::{
    PcurvePullbackRefusal, chart_nonnegative::nonnegative,
    pcurve_agreement::compare_surface_fields, pcurve_mobius::scale, pcurve_tangent::singular,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{PResult, Refusal, add},
        rational_factor::polynomial::{self as poly, Poly},
    },
    surface::Surface,
};
use num_rational::BigRational as Q;
use std::ops::ControlFlow;

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveAngleOutcome {
    WithinTolerance,
    ExceedsTolerance { parameter: [f64; 2] },
    Singular { parameter: [f64; 2], face: usize },
    Refused(PcurvePullbackRefusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveAngleReport {
    pub outcome: PcurveAngleOutcome,
    pub work_used: u64,
}

/// Test the acute angle between tangent planes at every paired parameter.
/// Supply the cosine squared of the permitted angle, in [0, 1].
/// Normal signs are immaterial. The bound includes equality and requires
/// regular planes. Positional tolerance can be checked independently with
/// `certify_pcurve_seam_gap`.
pub fn certify_pcurve_seam_angle(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    min_cosine_squared: f64,
    max_work: u64,
) -> Result<PcurveAngleReport> {
    check(
        (0.0..=1.0).contains(&min_cosine_squared),
        "Seam angle requires a squared cosine in [0, 1]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = compare_surface_fields(left, right, true, &mut work, |parameter, a, b, work| {
        let [a, b] = [&a[4..], &b[4..]];
        for (face, normal) in [a, b].into_iter().enumerate() {
            if singular(normal, work)? {
                return Ok(ControlFlow::Break(PcurveAngleOutcome::Singular {
                    parameter,
                    face,
                }));
            }
        }
        let limit = work.lift(min_cosine_squared).map_err(Refusal::from)?;
        if within(a, b, &limit, work)? {
            Ok(ControlFlow::Continue(()))
        } else {
            Ok(ControlFlow::Break(PcurveAngleOutcome::ExceedsTolerance {
                parameter,
            }))
        }
    })?;
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveAngleOutcome::WithinTolerance,
        Ok(ControlFlow::Break(outcome)) => outcome,
        Err(reason) => PcurveAngleOutcome::Refused(reason),
    };
    Ok(PcurveAngleReport {
        outcome,
        work_used: work.used,
    })
}

fn dot(a: &[Poly], b: &[Poly], work: &mut Work) -> PResult<Poly> {
    a.iter().zip(b).try_fold(Vec::new(), |sum, (a, b)| {
        let product = poly::multiply(a, b, work)?;
        add(&sum, &product, false, work)
    })
}

fn within(a: &[Poly], b: &[Poly], limit: &Q, work: &mut Work) -> PResult<bool> {
    let residual = angle_residual(a, b, limit, work)?;
    nonnegative(&residual, residual.len().saturating_sub(1), work)
}

pub(super) fn angle_residual(a: &[Poly], b: &[Poly], limit: &Q, work: &mut Work) -> PResult<Poly> {
    let product = dot(a, b, work)?;
    let cosine_numerator = poly::multiply(&product, &product, work)?;
    let norm_product = poly::multiply(&dot(a, a, work)?, &dot(b, b, work)?, work)?;
    add(
        &cosine_numerator,
        &scale(&norm_product, limit, work)?,
        true,
        work,
    )
}
