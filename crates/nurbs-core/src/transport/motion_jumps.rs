use super::*;
use crate::analysis::curve_measure::{
    DirectionalMotionJoinReport, DirectionalMotionLimits, TimedMotionSegment,
    classify_directional_motion_chain, classify_directional_motion_join,
    classify_directional_motion_knot_jumps, classify_timed_motion_knot_jumps,
};

pub(super) const OPS: &[&str] = &["inspect_knot_jumps","inspect_timed_knot_jumps","inspect_motion_join","inspect_motion_chain"];

fn join(r: DirectionalMotionJoinReport) -> Value {
    json!({"positionGap":r.position_gap_bounds,"jumps":r.jump_bounds,"statuses":r.statuses.map(|s| format!("{s:?}"))})
}
pub(super) fn dispatch(v: &Value,op: &str) -> Result<Value> {
    let direction: [f64;3] = field(v,"axis_vector")?;
    let [speed,acceleration,jerk] = field(v,"jump_limits")?;
    let limits = DirectionalMotionLimits{speed,acceleration,jerk};
    if matches!(op,"inspect_knot_jumps" | "inspect_timed_knot_jumps") {
        let curve = field(v,"curve")?;
        let interval = field(v,"domain")?;
        let budget = field(v,"max_knots")?;
        let jumps = if op=="inspect_knot_jumps" {
            classify_directional_motion_knot_jumps(&curve,interval,&direction,limits,budget)?
        } else {
            let segment=TimedMotionSegment{curve:&curve,interval,duration:field(v,"duration")?};
            classify_timed_motion_knot_jumps(segment,&direction,limits,budget)?
        };
        return Ok(json!({"knots":jumps.into_iter().map(|j|json!({"parameter":j.parameter,"jumps":j.jump_bounds,
            "statuses":j.statuses.map(|s|format!("{s:?}"))})).collect::<Vec<_>>()}));
    }
    let curves:Vec<crate::curve::Curve>=field(v,"curves")?;
    let domains:Vec<[f64;2]>=field(v,"domains")?;
    let durations:Vec<f64>=field(v,"durations")?;
    check((1..=32).contains(&curves.len()) && curves.len()==domains.len() && curves.len()==durations.len(),
        "Expected one parameter interval and duration per motion curve")?;
    let segments=curves.iter().zip(domains).zip(durations).map(|((curve,interval),duration)|
        TimedMotionSegment{curve,interval,duration}).collect::<Vec<_>>();
    if op=="inspect_motion_join" {
        let [a,b]=segments.as_slice() else {return Err(input("Motion join requires two curves"));};
        return Ok(join(classify_directional_motion_join([*a,*b],&direction,limits)?));
    }
    let r=classify_directional_motion_chain(&segments,field(v,"closed")?,&direction,field(v,"position_limit")?,limits,field(v,"max_joins")?)?;
    Ok(json!({"status":format!("{:?}",r.status),"joins":r.joins.into_iter().map(|j|json!({"segments":j.segment_indices,
        "positionStatus":format!("{:?}",j.position_status),"join":join(j.join)})).collect::<Vec<_>>()}))
}
