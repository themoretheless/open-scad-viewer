//! Sufficient exact impossibility proofs, never inferred from failed lifting.
use super::*;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SsThirdSurfaceExclusionKind {EliminationConstraint,Coordinate(usize)}
#[derive(Clone,Debug)]
pub struct QualifiedSsThirdSurfaceExclusion {
    source:Arc<QualifiedSsThirdSurfaceCandidates>,point:Arc<OriginalSsRootPoint>,
    limits:SsThirdSurfaceMembershipLimits,kind:SsThirdSurfaceExclusionKind,
    bounds:[Q;2],allowed:[Q;2],work:usize,clock_work:usize,
}
impl QualifiedSsThirdSurfaceExclusion {
    pub fn source(&self)->&Arc<QualifiedSsThirdSurfaceCandidates> {&self.source}
    pub fn point(&self)->&Arc<OriginalSsRootPoint> {&self.point}
    pub fn kind(&self)->SsThirdSurfaceExclusionKind {self.kind}
    pub fn bounds(&self)->&[Q;2] {&self.bounds}
    pub fn allowed(&self)->&[Q;2] {&self.allowed}
    pub fn work(&self)->usize {self.work}
    pub fn clock_work(&self)->usize {self.clock_work}
    pub fn qualify(source:Arc<QualifiedSsThirdSurfaceCandidates>,point:Arc<OriginalSsRootPoint>,limits:SsThirdSurfaceMembershipLimits)->Result<Self> {
        validate_limits(limits.algebra)?;
        let original=point.curve().original_deflated_span_ideal().ok_or_else(||crate::input("Third-point exclusion requires original deflated graph authority"))?;
        check(original.source().first()==source.original.source().first()
            && original.source().second()==source.original.source().second()
            && original.spans()==source.original.spans() && point.curve().free_axis().index()==source.axis,
            "Third-point exclusion graph or source-axis authority differs")?;
        let domain=point.curve().coordinate_domain();
        check(domain.iter().all(|d|0.<=d[0] && d[0]<=d[1] && d[1]<=1.),
            "Third-point exclusion needs the original graph inside its real Cox spans")?;
        let (coefficients,clock_domain,clock_work)=if let Some(a)=point.algebraic_point() {
            let q=a.clock().enclose(limits.bisections,limits.clock)?;
            (a.clock().polynomial().to_vec(),q.bounds,q.operations)
        } else {
            check(limits.bisections<=1024,"Invalid third-point exclusion clock policy")?;
            let t=Q::from_float(point.binary_parameter().unwrap()).unwrap();
            (vec![-t.clone(),Q::one()],[t.clone(),t],0)
        };
        let mut w=Work {used:0,limits:limits.algebra};let modulus=poly(&coefficients,&mut w)?;
        let image=scalar_image(&source.polynomial,&modulus,&clock_domain,&mut w)?;
        let selected=if image[1]<Q::zero() || Q::zero()<image[0] {
            Some((SsThirdSurfaceExclusionKind::EliminationConstraint,image,[Q::zero(),Q::zero()]))
        } else {
            let mut witness=None;
            for k in 0..6 {
                if k==source.axis || !has_relation(&source.basis,k,source.axis) {continue;}
                // Every real localized triple solution satisfies A(t)*uv+B(t)=0.
                // Nonzero A on the owned clock forces this coordinate value.
                let image=extract(&source.basis,k,source.axis,&modulus,&mut w)?.enclose(&clock_domain,&mut w)?;
                let allowed=if k<4 {[Q::from_float(domain[k][0]).unwrap(),Q::from_float(domain[k][1]).unwrap()]} else {[Q::zero(),Q::one()]};
                if image[1]<allowed[0] || allowed[1]<image[0] {
                    witness=Some((SsThirdSurfaceExclusionKind::Coordinate(k),image,allowed));break;
                }
            }
            witness
        };
        let (kind,bounds,allowed)=selected.ok_or_else(||crate::input("No exact finite third-surface exclusion was established"))?;
        Ok(Self {source,point,limits,kind,bounds,allowed,work:w.used,clock_work})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;self.point.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.point.clone(),self.limits)?;
        check(self.kind==fresh.kind && self.bounds==fresh.bounds && self.allowed==fresh.allowed
            && self.work==fresh.work && self.clock_work==fresh.clock_work,"Original third-point exclusion changed")
    }
}
