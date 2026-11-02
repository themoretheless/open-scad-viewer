//! One-knot proposal, accepted only after complete nominal parameter identity.
//! Candidate arithmetic carries no exact-construction authority. Its actual
//! emitted binary64 definition is compared independently with the source.
use crate::{Result,check,curve::Curve};
use cad_predicates::{AuthoredScalar,BezierIdentity,Limits,PredicateContext,Reason,SourceArena,ToleranceContext};
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
    let mut values=Vec::new();
    let mut offsets=Vec::new();
    for curve in [source,&candidate] {
        let controls=values.len();
        for (point,&w) in curve.control_points.iter().zip(&curve.weights) {values.extend([point[0],point[1],*point.get(2).unwrap_or(&0.),w]);}
        let knots=values.len();values.extend(&curve.knots);offsets.push((controls,knots));
    }
    let translation=values.len();values.extend([0.;3]);
    values.extend([p as f64,source.control_points[0].len() as f64,n as f64,candidate.control_points.len() as f64,knot]);
    // These are literal source and proposed output definitions. We do not
    // relabel a rounded construction as an exact original-source operation:
    // the full identity is proved independently for the ACTUAL output bits.
    let arena=SourceArena::authored("literal-knot-removal-definitions",1,values.into_iter().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).map_err(|e|crate::input(e.to_string()))?;
    let tolerance=ToleranceContext::default_valid();let remaining=max_work-work.used;
    let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:remaining,..Limits::default()},None);
    let controls=|offset:usize,count:usize|(0..count).map(|i|std::array::from_fn(|axis|arena.leaf(offset+4*i+axis).unwrap())).collect::<Vec<_>>();
    let knots=|offset:usize,count:usize|(0..count).map(|i|arena.leaf(offset+i).unwrap()).collect::<Vec<_>>();
    let a=controls(offsets[0].0,n);let ak=knots(offsets[0].1,source.knots.len());
    let b=controls(offsets[1].0,candidate.control_points.len());let bk=knots(offsets[1].1,candidate.knots.len());
    let t=std::array::from_fn(|axis|arena.leaf(translation+axis).unwrap());
    let decision=cad_predicates::rational_bspline_restriction_identity(&mut ctx,p,&a,&ak,p,&b,&bk,t).map_err(|e|crate::input(e.to_string()))?;
    let used=work.used+decision.work_used;
    match decision.outcome {
        BezierIdentity::Equal=>Ok(Report{curve:candidate,removed:true,refusal:None,work_used:used}),
        BezierIdentity::Different=>Ok(refusal(Refusal::DifferentDefinition,used)),
        BezierIdentity::Indeterminate(Reason::ResourceLimit)=>Ok(refusal(Refusal::ResourceLimit,used)),
        BezierIdentity::Indeterminate(reason)=>Ok(refusal(Refusal::PredicateIndeterminate(reason),used)),
    }
}
