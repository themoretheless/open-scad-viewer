//! Necessary triple-surface clock constraints, not physical trim authority.
use super::*;
use super::polynomial::SparseSsPolynomial as P;
#[derive(Clone,Debug)]
pub struct QualifiedSsThirdSurfaceCandidates {
    original:QualifiedOriginalSsIdeal,
    third:Surface,
    third_spans:[usize;2],
    axis:usize,
    limits:ExactSsGroebnerLimits,
    polynomial:Vec<Q>,
    work:usize,
}
impl QualifiedSsThirdSurfaceCandidates {
    pub fn original(&self)->&QualifiedOriginalSsIdeal {&self.original}
    pub fn third(&self)->&Surface {&self.third}
    pub fn third_spans(&self)->[usize;2] {self.third_spans}
    pub fn axis(&self)->usize {self.axis}
    pub fn polynomial(&self)->&[Q] {&self.polynomial}
    pub fn work(&self)->usize {self.work}
    pub fn proves_disjoint_on_spans(&self)->bool {self.polynomial.len()==1 && !self.polynomial[0].is_zero()}
    pub fn qualify_candidate_clock(&self,interval:[Q;2],limits:super::super::root_curve::SsAlgebraicClockLimits)->Result<super::super::root_curve::QualifiedSsAlgebraicClock> {
        super::super::root_curve::QualifiedSsAlgebraicClock::qualify(self.polynomial.clone(),interval,limits)
    }
    /// Complete real candidate clocks on a closed source-parameter interval.
    /// Third-face membership is not implied by this scalar root census.
    pub fn isolate_candidate_clocks(&self,domain:[Q;2],limits:super::super::root_curve::SsAlgebraicRootCensusLimits)->Result<super::super::root_curve::QualifiedSsAlgebraicRootCensus> {
        super::super::root_curve::QualifiedSsAlgebraicRootCensus::isolate(self.polynomial.clone(),domain,limits)
    }
    pub fn reverify(&self)->Result<()> {
        self.original.reverify()?;
        let fresh=qualify_ss_third_surface_candidates(&self.original,&self.third,self.third_spans,self.axis,self.limits)?;
        check(self.polynomial==fresh.polynomial && self.work==fresh.work,"Original triple-surface candidate constraint changed")
    }
}
fn lift(p:&ExactSsPolynomial,third:bool,w:&mut Work)->Result<P<7>> {
    let mut out=P::<7>::default();
    for (e,c) in p.terms() {
        let exponent=if third {[0,0,0,0,e[2],e[3],0]} else {[e[0],e[1],e[2],e[3],0,0,0]};
        out.insert(exponent,c.clone(),w)?;
    }
    Ok(out)
}
/// Exact localized elimination of S1=S2=S3 on the supplied original spans.
/// Candidate roots can be extraneous: real third-UV membership is separate.
pub fn qualify_ss_third_surface_candidates(original:&QualifiedOriginalSsIdeal,third:&Surface,third_spans:[usize;2],axis:usize,limits:ExactSsGroebnerLimits)->Result<QualifiedSsThirdSurfaceCandidates> {
    validate_limits(limits.algebra)?;
    let mut sorted=limits.variable_order;sorted.sort();
    check(axis<4 && sorted==[0,1,2,3] && (1..=65536).contains(&limits.max_pairs)
        && (1..=256).contains(&limits.max_basis),"Invalid triple-surface candidate policy or axis")?;
    let source=FiberSources::new(original.source().first(),third)?;
    let spans=original.spans();
    let third_ideal=qualify_original_ss_ideal(&source,[spans[0],spans[1],third_spans[0],third_spans[1]],limits.algebra)?;
    let mut w=Work {used:0,limits:limits.algebra};
    let first:Vec<_>=original.homogeneous()[0].iter().map(|p|lift(p,false,&mut w)).collect::<Result<_>>()?;
    let second_weight=lift(&original.homogeneous()[1][3],false,&mut w)?;
    let last:Vec<_>=third_ideal.homogeneous()[1].iter().map(|p|lift(p,true,&mut w)).collect::<Result<_>>()?;
    let mut equations:Vec<_>=original.equations().iter().map(|p|lift(p,false,&mut w)).collect::<Result<_>>()?;
    for k in 0..3 {
        let a=first[k].product(&last[3],&mut w)?;
        let b=first[3].product(&last[k],&mut w)?;
        equations.push(a.subtract(&b,&mut w)?);
    }
    // All three original positive-weight denominators are nonzero on their
    // real span boxes. Localization removes denominator-zero complex artifacts.
    let denominator=first[3].product(&second_weight,&mut w)?.product(&last[3],&mut w)?;
    let mut inverse=P::<7>::default();inverse.insert([0;7],Q::one(),&mut w)?;
    let product=denominator.scale_monomial([0,0,0,0,0,0,1],&Q::one(),&mut w)?;
    equations.push(inverse.subtract(&product,&mut w)?);
    let mut order=vec![6,4,5];order.extend(limits.variable_order.iter().copied().filter(|k|*k!=axis));order.push(axis);
    let (basis,_)=groebner::basis_for(&equations,order.try_into().unwrap(),limits.max_pairs,limits.max_basis,&mut w)?;
    let candidate=basis.iter().filter(|p|!p.is_zero() && p.terms.keys().all(|e|(0..7).all(|k|k==axis || e[k]==0)))
        .min_by_key(|p|p.terms.keys().map(|e|e[axis]).max().unwrap_or(0))
        .ok_or_else(||crate::input("Triple-surface elimination has no univariate candidate constraint"))?;
    let degree=candidate.terms.keys().map(|e|e[axis]).max().unwrap_or(0);
    check(degree<=64,"Triple-surface candidate degree allowance exceeded")?;
    let mut polynomial=vec![Q::zero();degree+1];
    for (e,c) in &candidate.terms {polynomial[e[axis]]=c.clone();}
    let work=w.used.checked_add(third_ideal.work()).ok_or_else(||crate::resource("Triple-surface work overflow"))?;
    Ok(QualifiedSsThirdSurfaceCandidates {original:original.clone(),third:third.clone(),third_spans,axis,limits,polynomial,work})
}
