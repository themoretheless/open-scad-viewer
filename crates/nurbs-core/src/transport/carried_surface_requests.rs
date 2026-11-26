use super::*;

pub(super) fn spring(v: &Value) -> Result<Value> {
    let report = crate::profile_spring::approximate(
        &field(v, "profile")?,
        crate::profile_spring::Request {
            center: field(v, "center")?,
            radius: field(v, "radius")?,
            height: field(v, "height")?,
            turns: field(v, "turns")?,
            phase_degrees: field(v, "phase_degrees")?,
            tolerance: field(v, "tolerance_mm")?,
            max_spans: field(v, "max_spans")?,
        },
    )?;
    Ok(json!({
        "patches":encode(report.patches)?,
        "spans":report.spans,
        "realArithmeticErrorEstimateMm":report.real_arithmetic_error_estimate,
        "withinRealArithmeticEstimate":report.stop == crate::profile_spring::Stop::EstimateReached,
        "roundingCertified":false,
        "solidTopologyCertified":false
    }))
}

pub(super) fn envelope(v: &Value) -> Result<Value> {
    let audit = crate::sphere_envelope::approximate_with_domain_audit(
        &field(v, "path")?,
        &field(v, "radius_curve")?,
        field(v, "normal")?,
        crate::sphere_envelope::Limits {
            sampled_tolerance: field(v, "tolerance_mm")?,
            max_patches: field(v, "max_patches")?,
        },
        field(v, "max_domain_cells")?,
    )?;
    let mut patches = Vec::new();
    let mut path_intervals = Vec::new();
    let mut deviation = None;
    let mut within_tolerance = false;
    if let Some(report) = audit.approximation {
        deviation = Some(report.sampled_control_deviation);
        within_tolerance = report.within_sampled_tolerance;
        for patch in report.patches {
            patches.push(patch.surface);
            path_intervals.push(patch.path_interval);
        }
    }
    Ok(json!({
        "patches":encode(patches)?,
        "pathIntervals":path_intervals,
        "sampledControlDeviationMm":deviation,
        "withinSampledTolerance":within_tolerance,
        "spanwiseCharacteristicCirclesProven":audit.domain.spanwise_characteristic_circles_proven,
        "angularChartProven":audit.domain.angular_chart_proven,
        "domainCells":audit.domain.cells,
        "unresolvedIntervals":audit.domain.unresolved,
        "continuousApproximationBound":false,
        "roundingCertified":false,
        "solidTopologyCertified":false
    }))
}
