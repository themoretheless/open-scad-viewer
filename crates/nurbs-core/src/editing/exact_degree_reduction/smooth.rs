//! Numeric extraction is only a proposal: final identity uses the original.
use super::{Report,Refusal,Work};
use crate::{Result,check,curve::Curve};
use crate::core::curve_identity::compare_definitions;
use cad_predicates::{BezierIdentity,Reason};

fn insert(source:&Curve,knot:f64,work:&mut Work)->std::result::Result<Curve,Refusal>{
    let p=source.degree;let n=source.control_points.len();let dim=source.control_points[0].len();
    if n==256{return Err(Refusal::InvalidProposedDefinition)}
    let span=source.knots.iter().rposition(|&u|u<=knot).unwrap();
    let multiplicity=source.knots.iter().filter(|&&u|u==knot).count();
    let mut candidate=source.clone();candidate.knots.insert(span+1,knot);
    candidate.control_points.insert(span-p+1,vec![0.;dim]);candidate.weights.insert(span-p+1,1.);
    for i in span-p+1..=span-multiplicity {
        let a=work.op(||knot-source.knots[i])?;
        let b=work.op(||source.knots[i+p]-source.knots[i])?;
        let alpha=work.op(||a/b)?;let complement=work.op(||1.-alpha)?;
        let left=work.op(||complement*source.weights[i-1])?;
        let right=work.op(||alpha*source.weights[i])?;
        let weight=work.op(||left+right)?;
        if weight<=0. {return Err(Refusal::InvalidProposedDefinition)}
        candidate.weights[i]=weight;
        for axis in 0..dim {
            let a=work.op(||left*source.control_points[i-1][axis])?;
            let b=work.op(||right*source.control_points[i][axis])?;
            let c=work.op(||a+b)?;
            candidate.control_points[i][axis]=work.op(||c/weight)?;
        }
    }
    if candidate.validate().is_err(){return Err(Refusal::InvalidProposedDefinition)}
    Ok(candidate)
}
/// Reduce one degree for general clamped nonperiodic smooth knot layouts.
/// All original breaks are extracted numerically, reduced, then compacted to
/// multiplicity max(original-1,0). The actual final definition is independently
/// compared with the original source. Every arithmetic/proof phase shares one
/// ledger. On any failure the entire original definition is returned.
/// This is one candidate search, not an exhaustive representability decision.
pub fn reduce_smooth(source:&Curve,max_work:u64)->Result<Report>{
    source.validate()?;let p=source.degree;let n=source.control_points.len();let domain=source.domain();
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Exact degree work must be in1..=1000000")?;
    check(p>=2&&!source.periodic,"Exact degree reduction requires degree2..25 and nonperiodic storage")?;
    check(source.knots[..=p].iter().all(|&u|u==domain[0])&&source.knots[n..].iter().all(|&u|u==domain[1]),"Exact degree reduction requires clamped storage")?;
    let mut breaks=Vec::<(f64,usize)>::new();
    for &u in &source.knots[p+1..n] {
        if let Some(last)=breaks.last_mut().filter(|last|last.0==u) {last.1+=1;}else{breaks.push((u,1));}
    }
    let refused=|reason,used|Report{curve:source.clone(),reduced:false,refusal:Some(reason),work_used:used};
    let mut work=Work{used:0,limit:max_work};let mut refined=source.clone();
    for &(u,m) in &breaks {
        for _ in m..p {
            refined=match insert(&refined,u,&mut work){Ok(c)=>c,Err(r)=>return Ok(refused(r,work.used))};
        }
    }
    if work.used==max_work{return Ok(refused(Refusal::ResourceLimit,work.used))}
    let step=super::reduce(&refined,max_work-work.used)?;work.used+=step.work_used;
    if !step.reduced{return Ok(refused(step.refusal.unwrap(),work.used))}
    let mut candidate=step.curve;
    for &(u,m) in &breaks {
        for _ in m..p {
            if work.used==max_work{return Ok(refused(Refusal::ResourceLimit,work.used))}
            let step=super::super::exact_knot_removal::remove(&candidate,u,max_work-work.used)?;
            work.used+=step.work_used;
            if !step.removed{return Ok(refused(step.refusal.unwrap(),work.used))}
            candidate=step.curve;
        }
    }
    if work.used==max_work{return Ok(refused(Refusal::ResourceLimit,work.used))}
    let proof=compare_definitions(source,&candidate,0.,max_work-work.used,false)?;work.used+=proof.work_used;
    match proof.outcome {
        BezierIdentity::Equal=>Ok(Report{curve:candidate,reduced:true,refusal:None,work_used:work.used}),
        BezierIdentity::Different=>Ok(refused(Refusal::DifferentDefinition,work.used)),
        BezierIdentity::Indeterminate(Reason::ResourceLimit)=>Ok(refused(Refusal::ResourceLimit,work.used)),
        BezierIdentity::Indeterminate(r)=>Ok(refused(Refusal::PredicateIndeterminate(r),work.used)),
    }
}
