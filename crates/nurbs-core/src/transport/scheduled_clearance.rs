use super::*;
use crate::analysis::curve_measure::{
    measure_scheduled_clearance_with_holds, measure_scheduled_curve_clearance,
    partition_scheduled_clearance_with_holds, partition_scheduled_curve_clearance, TimedMotionSegment,
};

pub(super) fn dispatch(v: &Value, op: &str, curves: [&crate::curve::Curve; 2]) -> Result<Value> {
    let windows: [[f64; 2]; 2] = field(v, "windows")?;
    let durations: [f64; 2] = field(v, "durations")?;
    let segments = [0, 1].map(|i| TimedMotionSegment { curve: curves[i], interval: windows[i], duration: durations[i] });
    let starts = field(v, "start_times")?;
    let radii = field(v, "radii_mm")?;
    let budget = field(v, "max_cells")?;
    if op == "minimum_scheduled_gap" {
        let r = measure_scheduled_curve_clearance(segments, starts, radii, field(v, "gap_tolerance_mm")?, budget)?;
        return Ok(match r {
            None => json!({"kind":"NoOverlap"}),
            Some(r) => json!({"kind":"Measured","gap":r.minimum.gap_bounds,"witnessTime":r.witness_time_bounds,
                "witnessProgress":r.minimum.witness_parameter,"workUsed":r.minimum.work_cells,"stopReason":format!("{:?}",r.minimum.stop_reason)}),
        });
    }
    if op == "minimum_held_gap" {
        let r = measure_scheduled_clearance_with_holds(segments, starts, radii, field(v,"gap_tolerance_mm")?, budget)?;
        return Ok(json!({"kind":"Measured","gap":r.gap_bounds,"witnessTime":r.witness_time_bounds,
            "workUsed":r.work_cells,"stopReason":format!("{:?}",r.stop_reason)}));
    }
    let tol = field(v, "progress_tolerance")?;
    let r = if op == "held_contact_intervals" {
        partition_scheduled_clearance_with_holds(segments, starts, radii, tol, budget)?
    } else {
        match partition_scheduled_curve_clearance(segments, starts, radii, tol, budget)? {
            Some(r) => r,
            None => return Ok(json!({"kind":"NoOverlap"})),
        }
    };
    let cells = r.cells.into_iter().map(|c| json!({"time":c.time_endpoint_bounds,
        "gap":c.gap_bounds,"status":format!("{:?}",c.status)})).collect::<Vec<_>>();
    Ok(json!({"kind":"Measured","cells":cells,"workUsed":r.work_cells}))
}
