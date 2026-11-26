//! Exact tangent-plane comparison along two retained UV traces.
use super::{
    PcurveEdgeAgreement, PcurvePullbackRefusal,
    chart_nonnegative::{contact_counts, sum},
    pcurve_agreement::{compare_surface_fields, compare_trace},
};
use crate::{
    Result,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{PResult, Refusal},
        exact_surface_derivative::{self as tensor, Axis, polynomial as bi},
        rational_factor::polynomial::{self as uni, Poly},
    },
    surface::Surface,
};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
use std::ops::ControlFlow;

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveTangentOutcome {
    Equal,
    PositionMismatch {
        parameter: [f64; 2],
        coordinate: usize,
    },
    Singular {
        parameter: [f64; 2],
        face: usize,
    },
    TangentPlaneMismatch {
        parameter: [f64; 2],
    },
    Refused(PcurvePullbackRefusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveTangentReport {
    pub outcome: PcurveTangentOutcome,
    pub work_used: u64,
}

/// Verify position and regular tangent-plane agreement on the complete
/// shared trace. Normal orientation is free. A normal that vanishes at
/// either endpoint or anywhere inside a piece is reported as singular.
pub fn compare_pcurve_tangent_planes(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    max_work: u64,
) -> Result<PcurveTangentReport> {
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = compare_surface_fields(left, right, true, &mut work, audit_piece)?;
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveTangentOutcome::Equal,
        Ok(ControlFlow::Break(reason)) => reason,
        Err(reason) => PcurveTangentOutcome::Refused(reason),
    };
    Ok(PcurveTangentReport {
        outcome,
        work_used: work.used,
    })
}

fn audit_piece(
    parameter: [f64; 2],
    left: &[Poly],
    right: &[Poly],
    work: &mut Work,
) -> std::result::Result<ControlFlow<PcurveTangentOutcome>, PcurvePullbackRefusal> {
    if let ControlFlow::Break(PcurveEdgeAgreement::Different {
        parameter,
        coordinate,
    }) = compare_trace(parameter, left, right, work)?
    {
        return Ok(ControlFlow::Break(PcurveTangentOutcome::PositionMismatch {
            parameter,
            coordinate,
        }));
    }
    let normals = [&left[4..], &right[4..]];
    for (face, normal) in normals.iter().enumerate() {
        if singular(normal, work)? {
            return Ok(ControlFlow::Break(PcurveTangentOutcome::Singular {
                parameter,
                face,
            }));
        }
    }
    let [a, b] = normals;
    for (i, j) in [(1, 2), (2, 0), (0, 1)] {
        if uni::multiply(&a[i], &b[j], work).map_err(Refusal::from)?
            != uni::multiply(&a[j], &b[i], work).map_err(Refusal::from)?
        {
            return Ok(ControlFlow::Break(
                PcurveTangentOutcome::TangentPlaneMismatch { parameter },
            ));
        }
    }
    Ok(ControlFlow::Continue(()))
}

pub(super) fn singular(normal: &[Poly], work: &mut Work) -> PResult<bool> {
    let mut common = Poly::new();
    for component in normal.iter().filter(|p| !p.is_empty()) {
        common = if common.is_empty() {
            component.clone()
        } else {
            uni::gcd(common, component.clone(), work)?
        };
        if common.len() == 1 {
            return Ok(false);
        }
    }
    Ok(common.is_empty()
        || common[0].is_zero()
        || sum(&common, work)?.is_zero()
        || contact_counts(&common, work)?[0] != 0)
}

pub(super) fn normal_numerators(h: &[tensor::Poly], work: &mut Work) -> PResult<Vec<tensor::Poly>> {
    let partial = |axis, work: &mut Work| -> std::result::Result<Vec<_>, tensor::Refusal> {
        let weight = &h[3];
        let one = Q::one();
        let dw = bi::derivative(weight, axis, &one, work)?;
        h[..3]
            .iter()
            .map(|p| {
                let dp = bi::derivative(p, axis, &one, work)?;
                let a = bi::multiply(&dp, weight, work)?;
                let b = bi::multiply(p, &dw, work)?;
                bi::subtract(a, &b, work)
            })
            .collect()
    };
    let out = (|| {
        let u = partial(Axis::U, work)?;
        let v = partial(Axis::V, work)?;
        [(1, 2), (2, 0), (0, 1)]
            .into_iter()
            .map(|(i, j)| {
                let a = bi::multiply(&u[i], &v[j], work)?;
                let b = bi::multiply(&u[j], &v[i], work)?;
                bi::subtract(a, &b, work)
            })
            .collect::<std::result::Result<Vec<_>, tensor::Refusal>>()
    })();
    out.map_err(super::pcurve_lift::tensor_refusal)
}
