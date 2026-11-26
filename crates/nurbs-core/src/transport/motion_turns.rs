use super::*;
use crate::analysis::curve_measure::{
    MotionJoinTurn, classify_motion_chain_turns, classify_motion_join_turn, classify_motion_knot_turns,
};

pub(super) const OPS: &[&str] = &["inspect_knot_turns", "inspect_join_turn", "inspect_chain_turns"];

fn join(report: MotionJoinTurn) -> Value {
    json!({"positionGap":report.position_gap_bounds,"cosine":report.cosine_bounds,"turnStatus":format!("{:?}", report.turn_status)})
}

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let minimum = field(v,"minimum_cosine")?;
    if op == "inspect_knot_turns" {
        let curve = field(v,"curve")?;
        let turns = classify_motion_knot_turns(&curve,field(v,"domain")?,minimum,field(v,"max_knots")?)?;
        return Ok(json!({"knots":turns.into_iter().map(|t|json!({"parameter":t.parameter,
            "cosine":t.cosine_bounds,"status":format!("{:?}",t.status)})).collect::<Vec<_>>()}));
    }
    let curves:Vec<crate::curve::Curve> = field(v,"curves")?;
    let domains:Vec<[f64;2]> = field(v,"domains")?;
    check((1..=32).contains(&curves.len()) && curves.len()==domains.len(),"Expected one parameter interval per curve")?;
    let windows = curves.iter().zip(domains).collect::<Vec<_>>();
    if op=="inspect_join_turn" {
        let [a,b]=windows.as_slice() else { return Err(input("Turn at a join requires two curves")); };
        return Ok(join(classify_motion_join_turn(*a,*b,minimum)?));
    }
    let r=classify_motion_chain_turns(&windows,field(v,"closed")?,minimum,field(v,"position_limit")?,field(v,"max_joins")?)?;
    Ok(json!({"status":format!("{:?}",r.status),"joins":r.joins.into_iter().map(|t|json!({"segments":t.segment_indices,
        "gapStatus":format!("{:?}",t.gap_status),"join":join(t.join)})).collect::<Vec<_>>()}))
}
