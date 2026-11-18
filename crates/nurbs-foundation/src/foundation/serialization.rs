//! Host evidence and projection adapters for the native foundation kernels.
use super::*;
use value_codec::{Value, json};
pub(super) fn tolerance_evidence(context: &ToleranceContext) -> Value {
    let spatial = context.spatial_bounds();
    json!({
        "toleranceIdentity": context.spec_identity(),
        "linearAbsoluteMm": spatial.absolute_mm,
        "linearRelative": spatial.relative,
        "parametricFloor": context.parametric_bounds().floor,
        "maxEntityErrorMm": context.entity_error_bounds().maximum_mm
    })
}

pub fn project_curve(
    curve: &Curve,
    point: &[f64],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(value_codec::Serialize::to_value(&project_curve_report(
        curve, point, tolerance,
    )?))
}

pub fn project_surface(
    surface: &Surface,
    point: [f64; 3],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(value_codec::Serialize::to_value(&project_surface_report(
        surface, point, tolerance,
    )?))
}
