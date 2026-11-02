//! Atomic exact tensor edit: row geometry AND polynomial denominators.
use crate::{Result,check,curve::Curve,surface::{Surface,Axis}};
use super::exact_knot_removal::{self,compare_definitions};
use cad_predicates::{BezierIdentity,Reason};

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Refusal {
    ResourceLimit,
    Curve {row:usize,reason:exact_knot_removal::Refusal},
    Denominator {row:usize,decision:BezierIdentity},
    InvalidOutput,
}
#[derive(Clone,Debug)]
pub struct Report {
    /// Unchanged source clone on every refusal, including late-row failures.
    pub surface:Surface,
    pub removed:bool,
    pub refusal:Option<Refusal>,
    pub rows_verified:usize,
    pub work_used:u64,
}
fn row(source:&Surface,axis:Axis,index:usize)->Curve {
    match axis {
        Axis::U=>Curve{degree:source.degree_u,knots:source.knots_u.clone(),
            control_points:source.control_points.iter().map(|r|r[index].clone()).collect(),
            weights:source.weights.iter().map(|r|r[index]).collect(),periodic:source.periodic_u},
        Axis::V=>Curve{degree:source.degree_v,knots:source.knots_v.clone(),
            control_points:source.control_points[index].clone(),weights:source.weights[index].clone(),periodic:source.periodic_v},
    }
}
/// One existing interior knot along a nonperiodic clamped axis, degree1..25.
/// Unedited tensor basis, UV domain and other-axis periodic storage are kept.
/// One1..=1000000 ledger covers all proposals and both proofs for every row.
/// A refusal is inconclusive about other possible exact representations.
pub fn remove(source:&Surface,axis:Axis,knot:f64,max_work:u64)->Result<Report> {
    source.validate()?;
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Exact surface knot work must be in1..=1000000")?;
    let count=match axis {Axis::U=>source.control_points[0].len(),Axis::V=>source.control_points.len()};
    let mut used=0;let mut rows=Vec::new();
    let refused=|reason,verified,used|Report{surface:source.clone(),removed:false,refusal:Some(reason),rows_verified:verified,work_used:used};
    for index in 0..count {
        if used==max_work {return Ok(refused(Refusal::ResourceLimit,rows.len(),used));}
        let input=row(source,axis,index);
        let proposal=exact_knot_removal::remove(&input,knot,max_work-used)?;
        used+=proposal.work_used;
        if !proposal.removed {return Ok(refused(Refusal::Curve{row:index,reason:proposal.refusal.unwrap()},rows.len(),used));}
        if used==max_work {return Ok(refused(Refusal::ResourceLimit,rows.len(),used));}
        let proof=compare_definitions(&input,&proposal.curve,knot,max_work-used,true)?;
        used+=proof.work_used;
        match proof.outcome {
            BezierIdentity::Equal=>rows.push(proposal.curve),
            BezierIdentity::Indeterminate(Reason::ResourceLimit)=>return Ok(refused(Refusal::ResourceLimit,rows.len(),used)),
            decision=>return Ok(refused(Refusal::Denominator{row:index,decision},rows.len(),used)),
        }
    }
    let mut candidate=source.clone();
    match axis {
        Axis::U=>{
            candidate.knots_u=rows[0].knots.clone();
            candidate.control_points=(0..rows[0].control_points.len()).map(|i|rows.iter().map(|c|c.control_points[i].clone()).collect()).collect();
            candidate.weights=(0..rows[0].weights.len()).map(|i|rows.iter().map(|c|c.weights[i]).collect()).collect();
        },
        Axis::V=>{
            candidate.knots_v=rows[0].knots.clone();
            candidate.control_points=rows.iter().map(|c|c.control_points.clone()).collect();
            candidate.weights=rows.iter().map(|c|c.weights.clone()).collect();
        },
    }
    if candidate.validate().is_err() {return Ok(refused(Refusal::InvalidOutput,rows.len(),used));}
    Ok(Report{surface:candidate,removed:true,refusal:None,rows_verified:rows.len(),work_used:used})
}
