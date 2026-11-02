//! One-degree inverse Bernstein proposal, accepted by original-source identity.
use crate::{Result,check,curve::Curve};
use crate::core::curve_identity::compare_definitions;
use cad_predicates::{BezierIdentity,Reason};
pub use super::exact_knot_removal::Refusal;
#[derive(Clone,Debug)]
pub struct Report {
    /// Unchanged source on every refusal, never an approximate reduction.
    pub curve:Curve,
    pub reduced:bool,
    pub refusal:Option<Refusal>,
    pub work_used:u64,
}
struct Work {used:u64,limit:u64}
impl Work {
    fn op(&mut self,f:impl FnOnce()->f64)->std::result::Result<f64,Refusal>{
        if self.used==self.limit {return Err(Refusal::ResourceLimit)}
        self.used+=1;let value=f();
        if value.is_finite(){Ok(value)}else{Err(Refusal::NonFiniteProposal)}
    }
}
fn proposal(source:&Curve,breaks:&[f64],work:&mut Work)->std::result::Result<Curve,Refusal>{
    let p=source.degree;let dim=source.control_points[0].len();
    let mut points=Vec::new();let mut weights=Vec::new();
    for span in 0..breaks.len()-1 {
        let first=span*p;let end=first+p;
        if span==0 {points.push(source.control_points[first].clone());weights.push(source.weights[first]);}
        if p>2 {
            let mut previous=Vec::new();
            for axis in 0..dim {previous.push(work.op(||source.control_points[first][axis]*source.weights[first])?);}
            previous.push(source.weights[first]);
            for i in 1..p-1 {
                let mut next=Vec::new();
                for axis in 0..=dim {
                    let current=if axis==dim {source.weights[first+i]}else{work.op(||source.control_points[first+i][axis]*source.weights[first+i])?};
                    // p*A_i = i*D_(i-1)+(p-i)*D_i; do not first round i/p.
                    let a=work.op(||p as f64*current)?;
                    let b=work.op(||i as f64*previous[axis])?;
                    let difference=work.op(||a-b)?;
                    next.push(work.op(||difference/(p-i) as f64)?);
                }
                if next[dim]<=0. {return Err(Refusal::InvalidProposedDefinition)}
                let mut point=Vec::new();for axis in 0..dim {point.push(work.op(||next[axis]/next[dim])?);}
                points.push(point);weights.push(next[dim]);previous=next;
            }
        }
        points.push(source.control_points[end].clone());weights.push(source.weights[end]);
    }
    let mut knots=vec![breaks[0];p];
    for &u in &breaks[1..breaks.len()-1] {knots.extend(std::iter::repeat_n(u,p-1));}
    knots.extend(std::iter::repeat_n(*breaks.last().unwrap(),p));
    let candidate=Curve{degree:p-1,knots,control_points:points,weights,periodic:false};
    if candidate.validate().is_err(){return Err(Refusal::InvalidProposedDefinition)}
    Ok(candidate)
}
/// Lower degree by one for clamped nonperiodic piecewise rational Bezier
/// storage (each interior knot multiplicity equals the source degree2..25).
/// Original complete parameter domain and all span boundaries remain fixed.
/// One shared1..=1000000 arithmetic ledger covers proposal and exact proof.
/// Numeric proposal arithmetic is not treated as exact construction history.
pub fn reduce(source:&Curve,max_work:u64)->Result<Report>{
    source.validate()?;let p=source.degree;let n=source.control_points.len();let domain=source.domain();
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Exact degree work must be in1..=1000000")?;
    check(p>=2&&!source.periodic,"Exact degree reduction requires degree2..25 and nonperiodic storage")?;
    check(source.knots[..=p].iter().all(|&u|u==domain[0])&&source.knots[n..].iter().all(|&u|u==domain[1]),"Exact degree reduction requires clamped storage")?;
    let mut breaks=source.knots.clone();breaks.dedup();
    check(breaks[1..breaks.len()-1].iter().all(|u|source.knots.iter().filter(|v|*v==u).count()==p),"Exact degree reduction requires explicit Bezier joins of multiplicity degree")?;
    check(n==(breaks.len()-1)*p+1,"Inconsistent piecewise Bezier control net")?;
    let mut work=Work{used:0,limit:max_work};
    let refused=|reason,used|Report{curve:source.clone(),reduced:false,refusal:Some(reason),work_used:used};
    let candidate=match proposal(source,&breaks,&mut work){Ok(c)=>c,Err(reason)=>return Ok(refused(reason,work.used))};
    let proof=compare_definitions(source,&candidate,0.,max_work-work.used,false)?;
    let used=work.used+proof.work_used;
    match proof.outcome {
        BezierIdentity::Equal=>Ok(Report{curve:candidate,reduced:true,refusal:None,work_used:used}),
        BezierIdentity::Different=>Ok(refused(Refusal::DifferentDefinition,used)),
        BezierIdentity::Indeterminate(Reason::ResourceLimit)=>Ok(refused(Refusal::ResourceLimit,used)),
        BezierIdentity::Indeterminate(reason)=>Ok(refused(Refusal::PredicateIndeterminate(reason),used)),
    }
}
