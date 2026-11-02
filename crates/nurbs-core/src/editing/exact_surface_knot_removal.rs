//! Atomic exact tensor edit: row geometry AND polynomial denominators.
use crate::{Result,check,surface::{Surface,Axis}};
use super::exact_knot_removal;

pub use super::exact_tensor_edit::Refusal;
#[derive(Clone,Debug)]
pub struct Report {
    /// Unchanged source clone on every refusal, including late-row failures.
    pub surface:Surface,
    pub removed:bool,
    pub refusal:Option<Refusal>,
    pub rows_verified:usize,
    pub work_used:u64,
}
/// Remove one existing interior knot on a clamped nonperiodic axis.
/// Both row geometry and denominator must remain exactly identical.
pub fn remove(source:&Surface,axis:Axis,knot:f64,max_work:u64)->Result<Report>{
    let r=super::exact_tensor_edit::apply(source,axis,knot,max_work,|curve,budget|{
        let p=exact_knot_removal::remove(curve,knot,budget)?;
        Ok(super::exact_tensor_edit::CurveProposal{curve:p.curve,changed:p.removed,refusal:p.refusal,work_used:p.work_used})
    })?;
    Ok(Report{surface:r.surface,removed:r.changed,refusal:r.refusal,rows_verified:r.rows_verified,work_used:r.work_used})
}

/// One bounded pass on a selected tensor axis; does not assert minimality.
#[derive(Clone,Debug)]
pub struct SimplificationReport {
    pub surface:Surface,
    pub removed:usize,
    pub attempts:Vec<(f64,Option<Refusal>)>,
    pub work_used:u64,
    pub completed_pass:bool,
}
/// Try each original interior knot occurrence. Every committed step proves all
/// row laws and denominators; later refusal retains earlier proved edits only.
/// Validation/copying are outside the arithmetic ledger, as in `remove`.
pub fn simplify(source:&Surface,axis:Axis,max_work:u64)->Result<SimplificationReport>{
    source.validate()?;
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Exact tensor edit work must be in1..=1000000")?;
    let (degree,knots,count,periodic)=match axis {
        Axis::U=>(source.degree_u,&source.knots_u,source.control_points.len(),source.periodic_u),
        Axis::V=>(source.degree_v,&source.knots_v,source.control_points[0].len(),source.periodic_v),
    };
    let start=knots[degree];let end=knots[count];
    check(!periodic&&knots[..=degree].iter().all(|&u|u==start)
        &&knots[count..].iter().all(|&u|u==end),"Exact knot removal requires nonperiodic clamped storage")?;
    let original:Vec<_>=knots.iter().copied().filter(|&u|start<u&&u<end).collect();
    let mut report=SimplificationReport{surface:source.clone(),removed:0,attempts:Vec::new(),work_used:0,completed_pass:true};
    for knot in original {
        if report.work_used==max_work {report.completed_pass=false;break;}
        let step=remove(&report.surface,axis,knot,max_work-report.work_used)?;
        report.work_used+=step.work_used;
        let exhausted=matches!(step.refusal,Some(Refusal::ResourceLimit)
            |Some(Refusal::Curve{reason:exact_knot_removal::Refusal::ResourceLimit,..})
            |Some(Refusal::Denominator{decision:cad_predicates::BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit),..}));
        report.attempts.push((knot,step.refusal));
        if step.removed {report.surface=step.surface;report.removed+=1;}
        if exhausted {report.completed_pass=false;break;}
    }
    Ok(report)
}
