//! Reports for measurements of the original rational geometry in model units.
use super::*;
use crate::{curve_measure, surface_measure};

fn length(r: curve_measure::LengthReport) -> Value {
    json!({"valueMm":r.value,"boundsMm":r.bounds,"errorUpperMm":r.error_upper,
        "withinTolerance":r.within_tolerance,"cells":r.cells,"stopReason":format!("{:?}",r.stop_reason)})
}

fn station(r: curve_measure::LengthPoint) -> Value {
    json!({"parameter":r.parameter,"parameterBounds":r.inverse_parameter_bounds,
        "pointMm":r.point,"pointBoundsMm":r.point_bounds,"prefixBoundsMm":r.prefix_bounds,
        "targetBoundsMm":r.target_bounds,"residualUpperMm":r.residual_upper,
        "withinTolerance":r.within_tolerance,"cells":r.cells,"stopReason":format!("{:?}",r.stop_reason)})
}

pub(super) fn measure(op: &str, v: &Value) -> Result<Value> {
    let cells = field(v, "max_cells")?;
    if op == "surface_area_bounds" {
        let r = surface_measure::area(&field(v, "surface")?, field(v, "tolerance_mm2")?, cells)?;
        return Ok(json!({"valueMm2":r.value,"boundsMm2":r.bounds,"errorUpperMm2":r.error_upper,
            "withinTolerance":r.within_tolerance,"cells":r.cells,"stopReason":format!("{:?}",r.stop_reason)}));
    }
    let curve = field(v, "curve")?;
    let tolerance = field(v, "tolerance_mm")?;
    match op {
        "curve_length_bounds" => Ok(length(curve_measure::length(&curve, tolerance, cells)?)),
        "curve_station_at_length" => Ok(station(curve_measure::point_at_length(
            &curve, field(v, "distance_mm")?, tolerance, cells)?)),
        "curve_divide_by_length" => {
            let r = curve_measure::divide_by_length(&curve, field(v, "segments")?, tolerance, cells)?;
            Ok(json!({"total":length(r.total),"points":r.points.into_iter().map(station).collect::<Vec<_>>(),
                "cells":r.cells,"withinTolerance":r.within_tolerance,"stopReason":format!("{:?}",r.stop_reason)}))
        }
        _ => Err(input("Unknown arc measurement")),
    }
}
