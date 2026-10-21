//! Combined UV boundary assembly and continuous supporting-surface admission.
use super::{
    ApproximateChartAssembly, ApproximateChartLimits, ChartAssembly, ChartAssemblyLimits,
    ChartBoundaryGraph, Report, SurfaceCorrespondence, SurfaceCorrespondenceLimits,
    assemble_approximated_chart, assemble_periodic_chart, verify_surface_correspondence,
};
use crate::{Result, curve::Curve, surface::Surface};
#[derive(Clone, Copy, Debug)]
pub enum SurfaceUvLimits {
    Exact(ChartAssemblyLimits),
    Approximate(ApproximateChartLimits),
}
#[derive(Clone, Copy, Debug)]
pub struct SurfaceChartLimits {
    /// Independent surface deviation/rectangle budget. UV assembly has its
    /// own explicit allowances; no shared-budget claim is made between stages.
    pub correspondence: SurfaceCorrespondenceLimits,
    pub uv: SurfaceUvLimits,
}
#[derive(Clone, Debug)]
pub enum SurfaceUvAssembly {
    Exact(ChartAssembly),
    Approximate(ApproximateChartAssembly),
}
impl SurfaceUvAssembly {
    fn graph(&self) -> Option<&ChartBoundaryGraph> {
        match self {
            Self::Exact(r) => r.graph.as_ref(),
            Self::Approximate(r) => r.graph.as_ref(),
        }
    }
    fn accepted(&self) -> bool {
        match self {
            Self::Exact(r) => r.chart_winding_preserved,
            Self::Approximate(r) => r.candidate_winding_preserved,
        }
    }
    fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Exact(r) => r.reason,
            Self::Approximate(r) => r.reason,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceChartStatus {
    Admitted,
    SurfaceUnproven,
    UvUnproven,
    NonwrappedConnector,
}
#[derive(Clone, Debug)]
pub struct SurfaceChartAssembly {
    pub status: SurfaceChartStatus,
    pub support: SurfaceCorrespondence,
    /// Diagnostics retain the exact vs approximate semantics. A UV-only graph
    /// here is not the admitted surface result; use graph() for that result.
    pub uv: Option<SurfaceUvAssembly>,
    pub reason: Option<&'static str>,
}
impl SurfaceChartAssembly {
    /// Published only after all UV, surface and connector-axis checks pass.
    pub fn graph(&self) -> Option<&ChartBoundaryGraph> {
        (self.status == SurfaceChartStatus::Admitted)
            .then(|| self.uv.as_ref().and_then(SurfaceUvAssembly::graph))
            .flatten()
    }
}
/// Assemble periodic trim on the actual supporting surface. Exact admission
/// retains source winding equality; approximate admission retains candidate
/// winding plus the independently verified source-to-candidate isotopy.
/// Source/candidate world deviation and seam identity are checked in both
/// modes. Generated periodic connectors on a nonwrapped axis are refused.
/// Admitted does not assert regular B-rep faces or surface injectivity.
pub fn assemble_on_surface(
    surface: &Surface,
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    wrapped: [bool; 2],
    anchor: [f64; 2],
    limits: SurfaceChartLimits,
) -> Result<SurfaceChartAssembly> {
    let support = verify_surface_correspondence(
        surface,
        sources,
        partitions,
        chart,
        wrapped,
        limits.correspondence,
    )?;
    let mut out = SurfaceChartAssembly {
        status: SurfaceChartStatus::SurfaceUnproven,
        reason: support.reason,
        support,
        uv: None,
    };
    if !out.support.within_world_tolerance {
        return Ok(out);
    }
    let uv = match limits.uv {
        SurfaceUvLimits::Exact(limits) => SurfaceUvAssembly::Exact(assemble_periodic_chart(
            sources, partitions, chart, anchor, limits,
        )?),
        SurfaceUvLimits::Approximate(limits) => SurfaceUvAssembly::Approximate(
            assemble_approximated_chart(sources, partitions, chart, anchor, limits)?,
        ),
    };
    let admitted = uv.accepted() && uv.graph().is_some();
    out.reason = uv.reason();
    out.uv = Some(uv);
    if !admitted {
        out.status = SurfaceChartStatus::UvUnproven;
        return Ok(out);
    }
    let graph = out.uv.as_ref().unwrap().graph().unwrap();
    let nonwrapped = graph
        .edges
        .iter()
        .filter(|e| e.source.is_none())
        .any(|edge| {
            (0..2).any(|a| {
                !wrapped[a]
                    && (0..2).any(|side| {
                        edge.curve
                            .control_points
                            .iter()
                            .all(|p| p[a] == chart[a][side])
                    })
            })
        });
    if nonwrapped {
        out.status = SurfaceChartStatus::NonwrappedConnector;
        out.reason = Some("nonwrapped-periodic-connector");
        return Ok(out);
    }
    out.status = SurfaceChartStatus::Admitted;
    out.reason = None;
    Ok(out)
}
