//! Atomic affine UV maps for NURBS surfaces and their trimming loops.
use super::{
    PcurvePullback, PcurvePullbackRefusal, TrimmedMobiusReport,
    affine_curve::AffineMap,
    affine_surface_clock,
    pcurve_mobius::{certify_chart, validate_chart},
    trimmed_mobius::transfer_report,
};
use crate::{
    Result, core::bounded_rational::Work, curve::Curve, editing::exact_derivative::Refusal,
    surface::Surface,
};

/// Apply the same independent affine UV maps to a tensor surface and
/// its exact closed trim loops. One reversed axis flips all loop windings
/// in the new UV chart. Curve parameters, knots, weights, and 3D traces
/// are preserved. The result requires exact binary64 storage of the new
/// surface knots and UV poles and is returned as one unit.
pub fn affine_trimmed_surface_clock(
    source: &Surface,
    loops: &[Vec<Curve>],
    endpoints: [[f64; 2]; 2],
    max_work: u64,
) -> Result<TrimmedMobiusReport> {
    source.validate()?;
    let old = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    transfer_report(
        loops,
        max_work,
        || affine_surface_clock(source, endpoints, max_work),
        |curve, budget| map_curve(curve, old, endpoints, budget),
    )
}

fn map_curve(
    curve: &Curve,
    old: [[f64; 2]; 2],
    new: [[f64; 2]; 2],
    budget: u64,
) -> Result<PcurvePullback> {
    validate_chart(curve, old)?;
    let mut work = Work {
        used: 0,
        limit: budget,
    };
    let result = (|| {
        if !certify_chart(curve, old, &mut work)? {
            return Err(PcurvePullbackRefusal::UnprovedChartDomain);
        }
        let u = AffineMap::new(old[0], new[0], &mut work).map_err(curve_refusal)?;
        let v = AffineMap::new(old[1], new[1], &mut work).map_err(curve_refusal)?;
        let mut out = curve.clone();
        for p in &mut out.control_points {
            p[0] = u.store(p[0], &mut work).map_err(curve_refusal)?;
            p[1] = v.store(p[1], &mut work).map_err(curve_refusal)?;
        }
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    let (pieces, refusal) = match result {
        Ok(curve) => (Some(vec![curve]), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurvePullback {
        pieces,
        refusal,
        work_used: work.used,
    })
}

fn curve_refusal(reason: super::Refusal) -> Refusal {
    match reason {
        super::Refusal::ResourceLimit => Refusal::ResourceLimit,
        super::Refusal::NotRepresentable => Refusal::NotRepresentable,
        _ => Refusal::InvalidOutput,
    }
}
