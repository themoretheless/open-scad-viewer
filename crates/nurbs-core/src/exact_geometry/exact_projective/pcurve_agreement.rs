//! Exact comparison of authored world-space and surface-owned edge traces.
use super::{
    PcurvePullbackRefusal,
    pcurve_lift::{trace_cells, trace_fields, validate_inputs},
    pcurve_mobius::original_span,
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
use std::ops::ControlFlow;

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveEdgeAgreement {
    Equal,
    Different {
        /// First common open interval with a nonzero cross-product polynomial.
        parameter: [f64; 2],
        /// The first failing Cartesian coordinate, in XYZ order.
        coordinate: usize,
    },
    Refused(PcurvePullbackRefusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveEdgeAgreementReport {
    pub outcome: PcurveEdgeAgreement,
    pub work_used: u64,
}

/// Compare an authored 3D edge to the complete surface composition S(U(t), V(t))
/// in the same absolute parameter. Compare original homogeneous polynomials on
/// the common trim, edge and UV cell partition, without storing a lifted curve.
/// Interior UV cell crossings still require exactly representable cuts.
pub fn compare_pcurve_edge(
    surface: &Surface,
    pcurve: &Curve,
    edge: &Curve,
    max_work: u64,
) -> Result<PcurveEdgeAgreementReport> {
    let axes = validate_inputs(surface, pcurve, max_work)?;
    edge.validate()?;
    check(
        edge.control_points[0].len() == 3 && edge.domain() == pcurve.domain(),
        "A 3D edge must use the same absolute parameter domain as its UV curve",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let breaks = &edge.knots[edge.degree..=edge.control_points.len()];
    let result = trace_cells(
        surface,
        pcurve,
        &axes,
        breaks,
        &mut work,
        |parameter, target, work| {
            let span = edge.knots.partition_point(|&t| t <= parameter[0]) - 1;
            if span >= edge.control_points.len() || edge.knots[span + 1] < parameter[1] {
                return Err(Refusal::UnprovedIdentity.into());
            }
            let original = original_span(edge, span, parameter, work)?;
            compare_trace(parameter, &original, target, work)
        },
    );
    Ok(agreement_report(result, work.used))
}

fn agreement_report(
    result: std::result::Result<ControlFlow<PcurveEdgeAgreement>, PcurvePullbackRefusal>,
    work_used: u64,
) -> PcurveEdgeAgreementReport {
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveEdgeAgreement::Equal,
        Ok(ControlFlow::Break(mismatch)) => mismatch,
        Err(reason) => PcurveEdgeAgreement::Refused(reason),
    };
    PcurveEdgeAgreementReport { outcome, work_used }
}

pub(super) fn compare_trace(
    parameter: [f64; 2],
    original: &[Poly],
    target: &[Poly],
    work: &mut Work,
) -> std::result::Result<ControlFlow<PcurveEdgeAgreement>, PcurvePullbackRefusal> {
    for coordinate in 0..3 {
        let left =
            poly::multiply(&original[coordinate], &target[3], work).map_err(Refusal::from)?;
        let right =
            poly::multiply(&target[coordinate], &original[3], work).map_err(Refusal::from)?;
        if left != right {
            return Ok(ControlFlow::Break(PcurveEdgeAgreement::Different {
                parameter,
                coordinate,
            }));
        }
    }
    Ok(ControlFlow::Continue(()))
}

/// Compare two surface-owned pcurves in the same original absolute parameter.
/// Their tensor knot grids and their UV curve partitions may differ. Exact
/// polynomial comparison uses the union of the lifted parameter partitions,
/// keeping the original rational expressions on both faces.
pub fn compare_surface_pcurve_seam(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    max_work: u64,
) -> Result<PcurveEdgeAgreementReport> {
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = compare_surface_fields(left, right, false, &mut work, compare_trace)?;
    Ok(agreement_report(result, work.used))
}

pub(super) fn compare_surface_fields<B>(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    include_normal: bool,
    work: &mut Work,
    mut compare: impl FnMut(
        [f64; 2],
        &[Poly],
        &[Poly],
        &mut Work,
    ) -> std::result::Result<ControlFlow<B>, PcurvePullbackRefusal>,
) -> Result<std::result::Result<ControlFlow<B>, PcurvePullbackRefusal>> {
    let left_axes = validate_inputs(left.0, left.1, work.limit)?;
    let right_axes = validate_inputs(right.0, right.1, work.limit)?;
    check(
        left.1.domain() == right.1.domain(),
        "Paired UV traces must use the same absolute parameter domain",
    )?;
    Ok((|| {
        let mut left_pieces = Vec::new();
        let _: ControlFlow<()> = trace_fields(
            left.0,
            left.1,
            &left_axes,
            &[],
            include_normal,
            work,
            |parameter, target, _| {
                left_pieces.push((parameter, target.to_vec()));
                Ok(ControlFlow::Continue(()))
            },
        )?;
        let breaks: Vec<_> = left_pieces
            .iter()
            .map(|(parameter, _)| parameter[1])
            .collect();
        trace_fields(
            right.0,
            right.1,
            &right_axes,
            &breaks,
            include_normal,
            work,
            |parameter, target, work| {
                let index = breaks.partition_point(|&end| end <= parameter[0]);
                let Some((cover, original)) = left_pieces.get(index) else {
                    return Err(Refusal::UnprovedIdentity.into());
                };
                if parameter[0] < cover[0] || parameter[1] > cover[1] {
                    return Err(Refusal::UnprovedIdentity.into());
                }
                if parameter == *cover {
                    compare(parameter, original, target, work)
                } else {
                    let restricted = restrict_trace(original, *cover, parameter, work)?;
                    compare(parameter, &restricted, target, work)
                }
            },
        )
    })())
}

fn restrict_trace(
    original: &[Poly],
    [a, b]: [f64; 2],
    [lo, hi]: [f64; 2],
    work: &mut Work,
) -> PResult<Vec<Poly>> {
    let [a, b, lo, hi] = [work.lift(a)?, work.lift(b)?, work.lift(lo)?, work.lift(hi)?];
    let span = work.op(&b, &a, |a, b| a - b)?;
    let start = work.op(&lo, &a, |a, b| a - b)?;
    let width = work.op(&hi, &lo, |a, b| a - b)?;
    let linear = vec![
        work.op(&start, &span, |a, b| a / b)?,
        work.op(&width, &span, |a, b| a / b)?,
    ];
    original
        .iter()
        .map(|channel| {
            channel.iter().rev().try_fold(Vec::new(), |acc, q| {
                let product = poly::multiply(&acc, &linear, work)?;
                add(&product, &vec![q.clone()], false, work)
            })
        })
        .collect()
}
