//! Atomic affine UV maps for NURBS surfaces and their trimming loops.
use super::{
    PcurvePullback, PcurvePullbackRefusal, Report, TrimmedMobiusReport,
    affine_curve::AffineMap,
    affine_surface_clock,
    mobius::validate_clock,
    pcurve_mobius::{certify_chart, validate_chart},
    trimmed_mobius::transfer_report,
};
use crate::{
    Result,
    core::{bounded_rational::Work, surface_transpose::transpose},
    curve::Curve,
    editing::exact_derivative::Refusal,
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
    let old = domain(source);
    transfer_report(
        loops,
        max_work,
        || affine_surface_clock(source, endpoints, max_work),
        |curve, budget| {
            map_curve(curve, old, budget, |out, work| {
                let u = AffineMap::new(old[0], endpoints[0], work).map_err(curve_refusal)?;
                let v = AffineMap::new(old[1], endpoints[1], work).map_err(curve_refusal)?;
                for p in &mut out.control_points {
                    p[0] = u.store(p[0], work).map_err(curve_refusal)?;
                    p[1] = v.store(p[1], work).map_err(curve_refusal)?;
                }
                Ok(())
            })
        },
    )
}

/// Exchange the U and V axes of a surface together with its exact closed
/// trimming loops. Original curve parameters, knot vectors and weights are
/// retained. The exchange flips chart winding and the surface normal
/// orientation while preserving the corresponding three-dimensional traces.
pub fn transpose_trimmed_surface(
    source: &Surface,
    loops: &[Vec<Curve>],
    max_work: u64,
) -> Result<TrimmedMobiusReport> {
    validate_clock(&[], max_work)?;
    source.validate()?;
    let old = domain(source);
    transfer_report(
        loops,
        max_work,
        || {
            Ok(Report {
                geometry: Some(transpose(source)?),
                refusal: None,
                work_used: 0,
            })
        },
        |curve, budget| {
            map_curve(curve, old, budget, |out, _| {
                for p in &mut out.control_points {
                    p.swap(0, 1);
                }
                Ok(())
            })
        },
    )
}

fn domain(source: &Surface) -> [[f64; 2]; 2] {
    [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ]
}

fn map_curve(
    curve: &Curve,
    old: [[f64; 2]; 2],
    budget: u64,
    transform: impl FnOnce(&mut Curve, &mut Work) -> std::result::Result<(), Refusal>,
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
        let mut out = curve.clone();
        transform(&mut out, &mut work)?;
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
