use super::*;
use crate::geodesic::{GeodesicStop, trace_geodesic_report};

pub(super) fn trace(v: &Value) -> Result<Value> {
    let surface: crate::surface::Surface = field(v, "surface")?;
    let trace = trace_geodesic_report(
        &surface,
        field(v, "start_uv")?,
        field(v, "direction_uv")?,
        field(v, "length")?,
        field(v, "max_steps")?,
    )?;
    let curve = if field::<bool>(v, "return_field")? && trace.uv_polyline.len() >= 2 {
        let points = trace.uv_polyline.iter()
            .map(|&[u, w]| surface.evaluate_validated(u, w).map(|jet| jet.point.to_vec()))
            .collect::<Result<Vec<_>>>()?;
        Some(crate::curve::Curve::from_polyline(points)?)
    } else {
        None
    };
    let stop = match trace.stop {
        GeodesicStop::LengthReached => "length_reached",
        GeodesicStop::BoundaryHit => "boundary_hit",
        GeodesicStop::BudgetExhausted => "budget_exhausted",
        GeodesicStop::DegenerateMetric => "degenerate_metric",
    };
    Ok(json!({
        "curve":curve,
        "stop":stop,
        "uvPolyline":trace.uv_polyline,
        "lengthAchievedMm":trace.length_achieved,
        "maxSampledGeodesicCurvature":trace.max_residual_geodesic_curvature,
        "continuousBound":false,
    }))
}
