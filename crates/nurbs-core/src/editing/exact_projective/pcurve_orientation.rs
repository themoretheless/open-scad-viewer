//! Compare both affine orientations of paired surface-owned seam traces.
use super::{
    PcurveEdgeAgreement, Refusal, affine_curve_clock, compare_surface_pcurve_seam,
    mobius::validate_clock, pcurve_lift::validate_inputs,
};
use crate::{Result, curve::Curve, surface::Surface};

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveOrientationOutcome {
    Compared(PcurveEdgeAgreement),
    ClockRefused(Refusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveOrientationReport {
    pub forward: PcurveOrientationOutcome,
    pub reversed: PcurveOrientationOutcome,
    pub work_used: u64,
}

/// Test both endpoint-induced affine correspondences between UV traces
/// that may use different parameter domains. Each successful comparison
/// certifies the full spatial trace in the left curve's clock. Both results
/// are kept so a symmetric or constant trace can match twice. The operation
/// spends one shared work budget, with the forward orientation first.
pub fn compare_pcurve_seam_orientations(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    max_work: u64,
) -> Result<PcurveOrientationReport> {
    validate_clock(&[], max_work)?;
    validate_inputs(left.0, left.1, max_work)?;
    validate_inputs(right.0, right.1, max_work)?;
    let mut remaining = max_work;
    let [a, b] = left.1.domain();
    let forward = compare(left, right, [a, b], &mut remaining)?;
    let reversed = compare(left, right, [b, a], &mut remaining)?;
    Ok(PcurveOrientationReport {
        forward,
        reversed,
        work_used: max_work - remaining,
    })
}

fn compare(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    endpoints: [f64; 2],
    remaining: &mut u64,
) -> Result<PcurveOrientationOutcome> {
    use PcurveOrientationOutcome::{ClockRefused, Compared};
    if *remaining == 0 {
        return Ok(ClockRefused(Refusal::ResourceLimit));
    }
    let mapped;
    let curve = if endpoints == right.1.domain() {
        right.1
    } else {
        let report = affine_curve_clock(right.1, endpoints, *remaining)?;
        *remaining -= report.work_used;
        let Some(curve) = report.geometry else {
            return Ok(ClockRefused(
                report.refusal.unwrap_or(Refusal::UnprovedIdentity),
            ));
        };
        mapped = curve;
        &mapped
    };
    if *remaining == 0 {
        return Ok(ClockRefused(Refusal::ResourceLimit));
    }
    let report = compare_surface_pcurve_seam(left, (right.0, curve), *remaining)?;
    *remaining -= report.work_used;
    Ok(Compared(report.outcome))
}
