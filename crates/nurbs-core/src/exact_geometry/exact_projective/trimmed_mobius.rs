//! Atomic exact chart and trim transfer for a clamped tensor surface.
use super::{
    BezierSpanLayout, PcurvePullback, PcurvePullbackRefusal, Refusal, Report,
    mobius_spline_surface, pullback_mobius_pcurve, pullback_relayout_pcurve,
    relayout_bezier_surface,
};
use crate::{Result, check, curve::Curve, surface::Surface, trim::trim_domain::exact_loop_joins};

const MAX_CURVES: usize = 256;

#[derive(Clone, Debug)]
pub struct ReclockedTrimmedSurface {
    pub surface: Surface,
    pub loops: Vec<Vec<Curve>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrimmedMobiusRefusal {
    Surface(Refusal),
    Curve {
        loop_index: usize,
        curve_index: usize,
        reason: PcurvePullbackRefusal,
    },
    OriginalLoopClosure {
        loop_index: usize,
    },
    TransferredLoopClosure {
        loop_index: usize,
    },
    ResourceLimit,
}

#[derive(Clone, Debug)]
pub struct TrimmedMobiusReport {
    /// Present only if the surface and every ordered trim loop were transferred.
    pub geometry: Option<ReclockedTrimmedSurface>,
    pub refusal: Option<TrimmedMobiusRefusal>,
    pub work_used: u64,
}

/// Reclock a clamped nonperiodic surface together with its exact closed UV
/// loops. Both clock factors are positive, preserving loop direction and
/// winding. Output curves are split at original knots, retaining their
/// absolute curve parameters. One arithmetic budget covers the surface and
/// all pcurves. Endpoint closure is checked separately from trim simplicity
/// and B-rep validity.
pub fn mobius_trimmed_surface(
    source: &Surface,
    loops: &[Vec<Curve>],
    lambda: [f64; 2],
    max_work: u64,
) -> Result<TrimmedMobiusReport> {
    super::mobius::validate_clock(&lambda, max_work)?;
    source.validate()?;
    let domain = [
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
        || mobius_spline_surface(source, lambda, max_work),
        |curve, remaining| pullback_mobius_pcurve(curve, domain, lambda, remaining),
    )
}

/// Relayout a piecewise Bézier surface and all its exact closed UV loops
/// under one shared arithmetic budget. The original grid is taken from the
/// surface's own knot vectors. Boundary curves are cut at internal grid
/// contacts and retain their original curve parameters. Endpoint closure is
/// required after transfer.
pub fn relayout_trimmed_bezier_surface(
    source: &Surface,
    loops: &[Vec<Curve>],
    layouts: [BezierSpanLayout<'_>; 2],
    max_work: u64,
) -> Result<TrimmedMobiusReport> {
    source.validate()?;
    let breaks = [&source.knots_u, &source.knots_v].map(|knots| {
        knots
            .chunk_by(|a, b| a == b)
            .map(|run| run[0])
            .collect::<Vec<_>>()
    });
    transfer_report(
        loops,
        max_work,
        || relayout_bezier_surface(source, layouts, max_work),
        |curve, remaining| {
            pullback_relayout_pcurve(curve, [&breaks[0], &breaks[1]], layouts, remaining)
        },
    )
}

pub(super) fn transfer_report(
    loops: &[Vec<Curve>],
    max_work: u64,
    map_surface: impl FnOnce() -> Result<Report<Surface>>,
    map_curve: impl FnMut(&Curve, u64) -> Result<PcurvePullback>,
) -> Result<TrimmedMobiusReport> {
    check(
        !loops.is_empty()
            && loops.len() <= MAX_CURVES
            && loops
                .iter()
                .all(|curves| (1..=MAX_CURVES).contains(&curves.len()))
            && loops.iter().map(Vec::len).sum::<usize>() <= MAX_CURVES,
        "Chart transfer needs nonempty UV loops with at most 256 total curves",
    )?;
    let mut work_used = 0;
    let out = transfer(loops, max_work, &mut work_used, map_surface, map_curve)?;
    let (geometry, refusal) = match out {
        Ok(geometry) => (Some(geometry), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(TrimmedMobiusReport {
        geometry,
        refusal,
        work_used,
    })
}

fn transfer(
    loops: &[Vec<Curve>],
    max_work: u64,
    used: &mut u64,
    map_surface: impl FnOnce() -> Result<Report<Surface>>,
    mut map_curve: impl FnMut(&Curve, u64) -> Result<PcurvePullback>,
) -> Result<std::result::Result<ReclockedTrimmedSurface, TrimmedMobiusRefusal>> {
    use TrimmedMobiusRefusal as Failure;
    for (loop_index, curves) in loops.iter().enumerate() {
        if exact_loop_joins(curves)? != Some(true) {
            return Ok(Err(Failure::OriginalLoopClosure { loop_index }));
        }
    }

    let report = map_surface()?;
    *used = report.work_used;
    let Some(surface) = report.geometry else {
        return Ok(Err(Failure::Surface(
            report.refusal.unwrap_or(Refusal::InvalidOutput),
        )));
    };
    let mut transferred = Vec::with_capacity(loops.len());
    let mut total_curves = 0;
    for (loop_index, curves) in loops.iter().enumerate() {
        let mut out = Vec::new();
        for (curve_index, curve) in curves.iter().enumerate() {
            let remaining = max_work - *used;
            if remaining == 0 {
                return Ok(Err(Failure::ResourceLimit));
            }
            let report = map_curve(curve, remaining)?;
            *used += report.work_used;
            let Some(pieces) = report.pieces else {
                let reason = report.refusal.unwrap_or(PcurvePullbackRefusal::Geometry(
                    crate::exact_geometry::exact_derivative::Refusal::InvalidOutput,
                ));
                return Ok(Err(Failure::Curve {
                    loop_index,
                    curve_index,
                    reason,
                }));
            };
            total_curves += pieces.len();
            if total_curves > MAX_CURVES {
                return Ok(Err(Failure::ResourceLimit));
            }
            out.extend(pieces);
        }
        if exact_loop_joins(&out)? != Some(true) {
            return Ok(Err(Failure::TransferredLoopClosure { loop_index }));
        }
        transferred.push(out);
    }
    Ok(Ok(ReclockedTrimmedSurface {
        surface,
        loops: transferred,
    }))
}
