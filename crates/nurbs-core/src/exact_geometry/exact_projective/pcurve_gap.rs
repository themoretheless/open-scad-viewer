//! Exact continuous gap threshold for two paired surface trim traces.
use super::{
    PcurvePullbackRefusal,
    chart_nonnegative::nonnegative,
    pcurve_agreement::compare_surface_fields,
    pcurve_lift::{trace_cells, validate_inputs},
    pcurve_mobius::{original_span, scale},
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
pub enum PcurveGapOutcome {
    WithinTolerance,
    ExceedsTolerance { parameter: [f64; 2] },
    Refused(PcurvePullbackRefusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveGapReport {
    pub outcome: PcurveGapOutcome,
    pub work_used: u64,
}

/// Decide whether the Euclidean gap between paired traces stays within
/// a finite tolerance at every point of their shared absolute parameter.
/// Uses exact squared-distance polynomials, including equality at isolated
/// extrema. This is a bound for the provided correspondence.
pub fn certify_pcurve_seam_gap(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    tolerance: f64,
    max_work: u64,
) -> Result<PcurveGapReport> {
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "Paired seam gap tolerance must be finite and nonnegative",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut squared = None;
    let result = compare_surface_fields(left, right, false, &mut work, |parameter, a, b, work| {
        if squared.is_none() {
            let tolerance = work.lift(tolerance).map_err(Refusal::from)?;
            squared = Some(
                work.op(&tolerance, &tolerance, |a, b| a * b)
                    .map_err(Refusal::from)?,
            );
        }
        if within(a, b, squared.as_ref().unwrap(), work)? {
            Ok(ControlFlow::Continue(()))
        } else {
            Ok(ControlFlow::Break(parameter))
        }
    })?;
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveGapOutcome::WithinTolerance,
        Ok(ControlFlow::Break(parameter)) => PcurveGapOutcome::ExceedsTolerance { parameter },
        Err(reason) => PcurveGapOutcome::Refused(reason),
    };
    Ok(PcurveGapReport {
        outcome,
        work_used: work.used,
    })
}

/// Check synchronized degree-one edge and trim traces on their whole clock.
pub(crate) fn certify_pcurve_edge_gap(
    surface: &Surface,
    pcurve: &Curve,
    edge: &Curve,
    tolerance: &Q,
    work: &mut Work,
) -> PResult<bool> {
    let axes = validate_inputs(surface, pcurve, work.limit).map_err(|_| Refusal::InvalidOutput)?;
    if pcurve.knots != edge.knots || pcurve.degree != 1 || edge.degree != 1 {
        return Err(Refusal::InvalidOutput);
    }
    let squared = work.op(tolerance, tolerance, |a, b| a * b)?;
    let result = trace_cells(
        surface,
        pcurve,
        &axes,
        &[],
        work,
        |parameter, target, work| {
            let span = edge.knots.partition_point(|&t| t <= parameter[0]) - 1;
            if span >= edge.control_points.len() || edge.knots[span + 1] < parameter[1] {
                return Err(Refusal::UnprovedIdentity.into());
            }
            let original = original_span(edge, span, parameter, work)?;
            Ok(if within(&original, target, &squared, work)? {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            })
        },
    );
    match result {
        Ok(result) => Ok(matches!(result, ControlFlow::Continue(()))),
        Err(PcurvePullbackRefusal::Geometry(reason)) => Err(reason),
        Err(PcurvePullbackRefusal::UnprovedChartDomain) => Err(Refusal::UnprovedIdentity),
    }
}

fn within(a: &[Poly], b: &[Poly], tolerance_squared: &Q, work: &mut Work) -> PResult<bool> {
    let residual = gap_residual(a, b, tolerance_squared, work)?;
    nonnegative(&residual, residual.len().saturating_sub(1), work)
}

pub(super) fn gap_residual(
    a: &[Poly],
    b: &[Poly],
    tolerance_squared: &Q,
    work: &mut Work,
) -> PResult<Poly> {
    let weight = poly::multiply(&a[3], &b[3], work)?;
    let limit = scale(
        &poly::multiply(&weight, &weight, work)?,
        tolerance_squared,
        work,
    )?;
    let mut residual = limit;
    for (x, y) in a[..3].iter().zip(&b[..3]) {
        let left = poly::multiply(x, &b[3], work)?;
        let right = poly::multiply(y, &a[3], work)?;
        let delta = add(&left, &right, true, work)?;
        let square = poly::multiply(&delta, &delta, work)?;
        residual = add(&residual, &square, true, work)?;
    }
    Ok(residual)
}
