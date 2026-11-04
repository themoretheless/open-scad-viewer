//! Original-equation rational-univariate point lifting with real span bounds.
use super::*;
use super::super::super::root_curve::{OriginalSsRootPoint,SsAlgebraicClockLimits};
use std::sync::Arc;
mod rational;
use rational::*;
#[derive(Clone,Copy,Debug)]
pub struct SsThirdSurfaceMembershipLimits {
    pub algebra:ExactSsAlgebraLimits,
    pub clock:SsAlgebraicClockLimits,
    pub bisections:usize,
}
impl Default for SsThirdSurfaceMembershipLimits {
    fn default()->Self {Self {algebra:Default::default(),clock:Default::default(),bisections:48}}
}
#[derive(Clone,Debug)]
pub struct QualifiedSsThirdSurfacePoint {
    source:Arc<QualifiedSsThirdSurfaceCandidates>,point:Arc<OriginalSsRootPoint>,
    limits:SsThirdSurfaceMembershipLimits,bounds:[[Q;2];6],third_uv:[[Q;2];2],work:usize,clock_work:usize,
}
impl QualifiedSsThirdSurfacePoint {
    pub fn point(&self)->&Arc<OriginalSsRootPoint> {&self.point}
    pub fn source(&self)->&Arc<QualifiedSsThirdSurfaceCandidates> {&self.source}
    pub fn local_bounds(&self)->&[[Q;2];6] {&self.bounds}
    pub fn third_uv_bounds(&self)->&[[Q;2];2] {&self.third_uv}
    pub fn work(&self)->usize {self.work}
    pub fn clock_work(&self)->usize {self.clock_work}
    pub fn qualify(source:Arc<QualifiedSsThirdSurfaceCandidates>,point:Arc<OriginalSsRootPoint>,limits:SsThirdSurfaceMembershipLimits)->Result<Self> {
        validate_limits(limits.algebra)?;
        let original=point.curve().original_deflated_span_ideal().ok_or_else(||crate::input("Third-point membership requires original deflated graph authority"))?;
        check(original.source().first()==source.original.source().first()
            && original.source().second()==source.original.source().second()
            && original.spans()==source.original.spans()
            && point.curve().free_axis().index()==source.axis,"Third-point original graph or source-axis authority differs")?;
        let (coefficients,clock_domain,clock_work)=if let Some(a)=point.algebraic_point() {
            let q=a.clock().enclose(limits.bisections,limits.clock)?;
            (a.clock().polynomial().to_vec(),q.bounds,q.operations)
        } else {
            check(limits.bisections<=1024,"Invalid third-point clock policy")?;
            let t=Q::from_float(point.binary_parameter().unwrap()).unwrap();
            (vec![-t.clone(),Q::one()],[t.clone(),t],0)
        };
        let mut w=Work {used:0,limits:limits.algebra};
        let modulus=poly(&coefficients,&mut w)?;
        let mut maps=Vec::new();
        for i in 0..6 {
            maps.push(if i==source.axis {Rat::variable(&mut w)?} else {extract(&source.basis,i,source.axis,&modulus,&mut w)?});
        }
        let mut bounds:Vec<[Q;2]>=Vec::new();
        for map in &maps {bounds.push(map.enclose(&clock_domain,&mut w)?);}
        let domain=point.curve().coordinate_domain();
        for k in 0..6 {
            let allowed=if k<4 {[Q::from_float(domain[k][0]).unwrap().max(Q::zero()),Q::from_float(domain[k][1]).unwrap().min(Q::one())]} else {[Q::zero(),Q::one()]};
            check(allowed[0]<=bounds[k][0] && bounds[k][1]<=allowed[1],"Lifted third point leaves the original SS graph or finite third span")?;
        }
        // Maps are proposals. Exact original-equation identities, not just
        // their Groebner shape, authorize the real point lift.
        for equation in &source.equations {
            let value=evaluate(equation,&maps,&modulus,&mut w)?;
            check(value.n.is_zero(),"Lifted original triple-surface equation is not zero modulo the owned clock")?;
        }
        let mut third_uv=std::array::from_fn(|_|[Q::zero(),Q::zero()]);
        for k in 0..2 {
            let knots=if k==0 {&source.third.knots_u} else {&source.third.knots_v};
            let span=source.third_spans[k];let lo=Q::from_float(knots[span]).unwrap();
            let width=Q::from_float(knots[span+1]).unwrap()-&lo;
            for j in 0..2 {w.charge()?;let x=&lo+&width*&bounds[k+4][j];w.coefficient(&x)?;third_uv[k][j]=x;}
        }
        Ok(Self {source,point,limits,bounds:bounds.try_into().unwrap(),third_uv,work:w.used,clock_work})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;self.point.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.point.clone(),self.limits)?;
        check(self.bounds==fresh.bounds && self.third_uv==fresh.third_uv && self.work==fresh.work
            && self.clock_work==fresh.clock_work,"Original third-surface point membership changed")
    }
}
