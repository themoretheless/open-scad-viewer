//! One-knot proposal, accepted only after complete nominal parameter identity.
//! Candidate arithmetic carries no exact-construction authority. Its actual
//! emitted binary64 definition is compared independently with the source.
use crate::{Result,check,curve::Curve};
use cad_predicates::{BezierIdentity,Reason};
pub(super) use crate::core::curve_identity::compare_definitions;
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Refusal {
    ResourceLimit,
    NonFiniteProposal,
    InvalidProposedDefinition,
    DifferentDefinition,
    PredicateIndeterminate(Reason),
}
#[derive(Clone,Debug)]
pub struct Report {
    /// On any refusal this is the unchanged source, never an approximate edit.
    pub curve:Curve,
    pub removed:bool,
    pub refusal:Option<Refusal>,
    pub work_used:u64,
}
struct Work {used:u64,limit:u64}
impl Work {
    fn op(&mut self,f:impl FnOnce()->f64)->std::result::Result<f64,Refusal> {
        if self.used==self.limit {return Err(Refusal::ResourceLimit);}
        self.used+=1;let value=f();
        if value.is_finite() {Ok(value)}else{Err(Refusal::NonFiniteProposal)}
    }
}
fn proposal(source:&Curve,knot:f64,index:usize,work:&mut Work)->std::result::Result<Curve,Refusal> {
    let mut c=source.clone();c.knots.remove(index);c.control_points.pop();c.weights.pop();
    let n=c.control_points.len();let p=c.degree;
    let span=c.knots.iter().rposition(|&u|u<=knot).unwrap();
    let multiplicity=c.knots.iter().filter(|&&u|u==knot).count();
    let left=span-p;let right=span-multiplicity;
    for i in 0..=left {c.control_points[i]=source.control_points[i].clone();c.weights[i]=source.weights[i];}
    for i in right..n {c.control_points[i]=source.control_points[i+1].clone();c.weights[i]=source.weights[i+1];}
    let dim=source.control_points[0].len();
    for i in left+1..right {
        let numerator=work.op(||knot-c.knots[i])?;
        let denominator=work.op(||c.knots[i+p]-c.knots[i])?;
        let alpha=work.op(||numerator/denominator)?;
        if alpha<=0.||alpha>1. {return Err(Refusal::InvalidProposedDefinition);}
        let complement=work.op(||1.-alpha)?;let mut h=Vec::with_capacity(dim+1);
        for axis in 0..=dim {
            let current=if axis==dim {source.weights[i]}else{work.op(||source.control_points[i][axis]*source.weights[i])?};
            let previous=if axis==dim {c.weights[i-1]}else{work.op(||c.control_points[i-1][axis]*c.weights[i-1])?};
            let previous=work.op(||complement*previous)?;
            let difference=work.op(||current-previous)?;
            h.push(work.op(||difference/alpha)?);
        }
        if h[dim]<=0. {return Err(Refusal::InvalidProposedDefinition);}
        c.weights[i]=h[dim];
        for axis in 0..dim {c.control_points[i][axis]=work.op(||h[axis]/h[dim])?;}
    }
    if c.validate().is_err() {return Err(Refusal::InvalidProposedDefinition);}
    Ok(c)
}
/// Remove one interior knot, degree1..25, with original absolute parameter
/// domain and positive-weight storage unchanged. Nonperiodic clamped input
/// only. A single proposal is not an exhaustive representability search.
///
/// max_work in1..=MAX_WORK shares charged proposal floating operations and
/// the exact predicate's existing context. Bounded validation, copies and
/// source-arena assembly are outside that arithmetic ledger. No sampled fit,
/// geometric tolerance, source-history assertion or topology certificate.
pub fn remove(source:&Curve,knot:f64,max_work:u64)->Result<Report> {
    source.validate()?;
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Exact knot work must be in1..=1000000")?;
    let domain=source.domain();let n=source.control_points.len();let p=source.degree;
    check(!source.periodic&&source.knots[..=p].iter().all(|&u|u==domain[0])&&source.knots[n..].iter().all(|&u|u==domain[1]),"Exact knot removal requires nonperiodic clamped storage")?;
    check(knot.is_finite()&&domain[0]<knot&&knot<domain[1],"Only finite interior knots can be removed exactly")?;
    let index=source.knots.iter().position(|&u|u==knot).ok_or_else(||crate::input("Exact removal knot is absent"))?;
    let mut work=Work{used:0,limit:max_work};
    let refusal=|why,used|Report{curve:source.clone(),removed:false,refusal:Some(why),work_used:used};
    let candidate=match proposal(source,knot,index,&mut work) {Ok(c)=>c,Err(why)=>return Ok(refusal(why,work.used))};
    let decision=compare_definitions(source,&candidate,knot,max_work-work.used,false)?;
    let used=work.used+decision.work_used;
    match decision.outcome {
        BezierIdentity::Equal=>Ok(Report{curve:candidate,removed:true,refusal:None,work_used:used}),
        BezierIdentity::Different=>Ok(refusal(Refusal::DifferentDefinition,used)),
        BezierIdentity::Indeterminate(Reason::ResourceLimit)=>Ok(refusal(Refusal::ResourceLimit,used)),
        BezierIdentity::Indeterminate(reason)=>Ok(refusal(Refusal::PredicateIndeterminate(reason),used)),
    }
}

/// Result of one bounded pass; completion does not assert minimality.
#[derive(Clone, Debug)]
pub struct SimplificationReport {
    pub curve: Curve,
    pub removed: usize,
    pub attempts: Vec<(f64, Option<Refusal>)>,
    pub work_used: u64,
    pub completed_pass: bool,
}
/// Try each original interior knot occurrence with one shared arithmetic budget.
/// Only independently proved steps are retained. No failed edit is emitted.
/// Validation/copying remain outside the arithmetic ledger, as in `remove`.
pub fn simplify(source: &Curve, max_work: u64) -> Result<SimplificationReport> {
    source.validate()?;
    check((1..=cad_predicates::MAX_WORK).contains(&max_work), "Exact knot work must be in1..=1000000")?;
    let [start, end] = source.domain();
    let n = source.control_points.len();
    check(!source.periodic && source.knots[..=source.degree].iter().all(|&u| u == start)
        && source.knots[n..].iter().all(|&u| u == end), "Exact knot removal requires nonperiodic clamped storage")?;
    let knots: Vec<_> = source.knots.iter().copied().filter(|&u| start < u && u < end).collect();
    let mut report = SimplificationReport { curve: source.clone(), removed: 0,
        attempts: Vec::new(), work_used: 0, completed_pass: true };
    for knot in knots {
        if report.work_used == max_work { report.completed_pass = false; break; }
        let step = remove(&report.curve, knot, max_work - report.work_used)?;
        report.work_used += step.work_used;
        let exhausted = step.refusal == Some(Refusal::ResourceLimit);
        report.attempts.push((knot, step.refusal));
        if step.removed { report.curve = step.curve; report.removed += 1; }
        if exhausted { report.completed_pass = false; break; }
    }
    Ok(report)
}
