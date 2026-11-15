//! Exact comparison of authored world-space and surface-owned edge traces.
use super::{
    PcurvePullbackRefusal,
    pcurve_lift::{trace_cells, validate_inputs},
    pcurve_mobius::original_span,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    editing::{exact_derivative::Refusal, rational_factor::polynomial as poly},
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
            for coordinate in 0..3 {
                let left = poly::multiply(&original[coordinate], &target[3], work)
                    .map_err(Refusal::from)?;
                let right = poly::multiply(&target[coordinate], &original[3], work)
                    .map_err(Refusal::from)?;
                if left != right {
                    return Ok(ControlFlow::Break(PcurveEdgeAgreement::Different {
                        parameter,
                        coordinate,
                    }));
                }
            }
            Ok(ControlFlow::Continue(()))
        },
    );
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveEdgeAgreement::Equal,
        Ok(ControlFlow::Break(mismatch)) => mismatch,
        Err(reason) => PcurveEdgeAgreement::Refused(reason),
    };
    Ok(PcurveEdgeAgreementReport {
        outcome,
        work_used: work.used,
    })
}
