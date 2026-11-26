use super::*;
use crate::analysis::curve_measure::{
    last_toolpath_chain_violation, LastToolpathChainViolation, toolpath_chain_clearance_timeline,
    first_toolpath_chain_violation, measure_minimum_toolpath_chain_clearance,
    measure_toolpath_chain_violation_duration, FirstToolpathChainViolation, TimedMotionSegment,
    ToolpathObstacle, ToolpathViolationDuration,
};

fn duration(r: ToolpathViolationDuration) -> Value {
    json!({"duration":r.bounds, "byObstacle":r.by_obstacle, "workUsed":r.work_obstacle_cells})
}

pub(super) fn dispatch(v: &Value, op: &str, obstacles: &[ToolpathObstacle]) -> Result<Value> {
    let curves: Vec<crate::curve::Curve> = field(v, "curves")?;
    let domains: Vec<[f64; 2]> = field(v, "domains")?;
    let durations: Vec<f64> = field(v, "durations")?;
    check((1..=32).contains(&curves.len()) && curves.len() == domains.len() && curves.len() == durations.len(),
        "Expected one parameter interval and duration per motion curve")?;
    let segments = curves.iter().zip(domains).zip(durations)
        .map(|((curve, interval), duration)| TimedMotionSegment { curve, interval, duration })
        .collect::<Vec<_>>();
    let radius = field(v, "radius_mm")?;
    let budget = field(v, "max_cells")?;
    if op == "minimum_chain_gap" {
        let r = measure_minimum_toolpath_chain_clearance(&segments, obstacles, radius, field(v,"gap_tolerance_mm")?, budget)?;
        return Ok(json!({"gap":r.gap_bounds, "withinTolerance":r.within_tolerance,
            "witnessSegment":r.witness_segment, "witnessObstacle":r.witness_obstacle,
            "witnessParameter":r.witness_parameter, "witnessTime":r.witness_time_bounds, "workUsed":r.work_obstacle_cells}));
    }
    let margin = field(v, "margin_mm")?;
    let tol = field(v, "parameter_tolerance")?;
    if op == "first_chain_violation" {
        return Ok(match first_toolpath_chain_violation(&segments, obstacles, radius, margin, tol, budget)? {
            FirstToolpathChainViolation::Clear => json!({"kind":"Clear"}),
            FirstToolpathChainViolation::Possible { earliest_time, earliest_segment } =>
                json!({"kind":"Possible","earliestTime":earliest_time,"earliestSegment":earliest_segment}),
            FirstToolpathChainViolation::Confirmed { time_bounds, earliest_segment, witness_segment, obstacle_index } =>
                json!({"kind":"Confirmed","time":time_bounds,"earliestSegment":earliest_segment,"witnessSegment":witness_segment,"obstacle":obstacle_index}),
        });
    }
    if op == "last_chain_violation" {
        return Ok(match last_toolpath_chain_violation(&segments, obstacles, radius, margin, tol, budget)? {
            LastToolpathChainViolation::Clear => json!({"kind":"Clear"}),
            LastToolpathChainViolation::Possible { latest_time, latest_segment } =>
                json!({"kind":"Possible","latestTime":latest_time,"latestSegment":latest_segment}),
            LastToolpathChainViolation::Confirmed { time_bounds, latest_segment, witness_segment, obstacle_index } =>
                json!({"kind":"Confirmed","time":time_bounds,"latestSegment":latest_segment,"witnessSegment":witness_segment,"obstacle":obstacle_index}),
        });
    }
    if matches!(op, "chain_clearance_timeline" | "longest_chain_violation") {
        let timeline = toolpath_chain_clearance_timeline(&segments, obstacles, radius, margin, tol, budget)?;
        if op == "longest_chain_violation" {
            return Ok(json!({"duration":timeline.longest_violation_duration(),"workUsed":timeline.work_obstacle_cells}));
        }
        let intervals = timeline.intervals.into_iter().map(|cell| json!({
            "segment":cell.segment_index, "domain":cell.parameter_interval, "time":cell.time_endpoint_bounds,
            "status":format!("{:?}",cell.status)
        })).collect::<Vec<_>>();
        return Ok(json!({"intervals":intervals,"workUsed":timeline.work_obstacle_cells}));
    }
    let r = measure_toolpath_chain_violation_duration(&segments, obstacles, radius, margin, tol, budget)?;
    Ok(json!({"total":duration(r.total), "segments":r.segments.into_iter().map(duration).collect::<Vec<_>>()}))
}
