//! Tolerance-qualified chart assembly with explicit topology admission.
use super::{
    AnchorLimits, AnchorTransfer, ChartAssemblyLimits, ChartBoundaryGraph, ChartCalibration,
    IsotopyLimits, Report, double_chart, verify_anchor_winding,
};
use crate::{Result, curve::Curve};
#[derive(Clone, Copy, Debug)]
pub struct ApproximateChartLimits {
    pub isotopy: IsotopyLimits,
    pub max_edges: usize,
    pub max_images: usize,
    /// Shared source and raw-chain winding classification cells.
    pub max_cells: usize,
    pub max_checks: usize,
    pub tolerance_uv: f64,
}
#[derive(Clone, Debug)]
pub struct ApproximateChartAssembly {
    /// Candidate periodic winding equals graph winding off candidate boundary.
    /// This is not pointwise equality with the original, moved source boundary.
    pub candidate_winding_preserved: bool,
    pub transfer: AnchorTransfer,
    pub graph: Option<ChartBoundaryGraph>,
    pub calibration: Option<ChartCalibration>,
    pub reason: Option<&'static str>,
}
/// Assemble tolerance-qualified candidates into local oriented chart cycles.
/// The source-to-candidate topology and anchor winding are recomputed first.
/// Candidate tiles, chart containment, seam cancellation and winding constant
/// are all checked before graph publication. No regular B-rep or 3D claim.
pub fn assemble_approximated_chart(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
) -> Result<ApproximateChartAssembly> {
    assemble_impl(sources, partitions, chart, anchor, limits, true)
}
pub(super) fn assemble_weighted_candidate(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
) -> Result<ApproximateChartAssembly> {
    assemble_impl(sources, partitions, chart, anchor, limits, false)
}
fn assemble_impl(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    unique: bool,
) -> Result<ApproximateChartAssembly> {
    // Exact identity/link budgets belong only to the separate exact admission.
    // The common backend uses edge/image/classification budgets, not these two.
    let common = ChartAssemblyLimits {
        max_segments: 1,
        max_links: 1,
        max_edges: limits.max_edges,
        max_images: limits.max_images,
        max_cells: limits.max_cells,
        tolerance_uv: limits.tolerance_uv,
    };
    double_chart::validate_limits(chart, anchor, common)?;
    let transfer = verify_anchor_winding(
        sources,
        partitions,
        chart,
        anchor,
        AnchorLimits {
            isotopy: limits.isotopy,
            tolerance_uv: limits.tolerance_uv,
            max_images: limits.max_images,
            max_cells: limits.max_cells,
            max_checks: limits.max_checks,
        },
    )?;
    let mut out = ApproximateChartAssembly {
        candidate_winding_preserved: false,
        transfer,
        graph: None,
        calibration: None,
        reason: None,
    };
    if out.transfer.winding.is_none() {
        out.reason = Some("anchor-transfer-unproven");
        return Ok(out);
    }
    // Do not trust Report::candidate_hulls_contained or authored tile labels.
    // Exact subtraction is also required by the already verified connectivity.
    let periods = chart.map(|r| crate::exact_curve_segments::exact_add(r[1], -r[0]));
    let [Some(u), Some(v)] = periods else {
        out.reason = Some("periodic-translation-unproven");
        return Ok(out);
    };
    for partition in partitions {
        for piece in &partition.pieces {
            if (0..2).any(|a| {
                crate::exact_products::sum_sign(&[
                    (piece.translation[a], 1.),
                    (-f64::from(piece.tile[a]), [u, v][a]),
                ]) != std::cmp::Ordering::Equal
            }) {
                out.reason = Some("periodic-translation-unproven");
                return Ok(out);
            }
            if piece
                .curve
                .control_points
                .iter()
                .any(|p| (0..2).any(|a| p[a] < chart[a][0] || chart[a][1] < p[a]))
            {
                out.reason = Some("candidate-chart-unproven");
                return Ok(out);
            }
        }
    }
    // Reuse the actual source query admitted for this anchor, charging its
    // cells to the same source/raw classification allowance in the backend.
    let query = out.transfer.source.as_ref().unwrap().clone();
    let assembled =
        double_chart::assemble_candidate(partitions, chart, anchor, common, unique, || Ok(query))?;
    out.candidate_winding_preserved = assembled.chart_winding_preserved;
    out.graph = assembled.graph;
    out.calibration = assembled.calibration;
    out.reason = assembled.reason;
    Ok(out)
}
