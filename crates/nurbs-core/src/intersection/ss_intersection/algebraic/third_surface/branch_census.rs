//! All-or-nothing real intersection classification on one owned SS graph.
use super::*;
use super::super::super::root_curve::{OriginalSsRootCurve,OriginalSsRootPoint,OriginalSsAlgebraicPoint,SsAlgebraicPointLimits,SsAlgebraicRootCensusLimits,QualifiedSsAlgebraicRootCensus};
use std::sync::Arc;
#[derive(Clone,Copy,Debug)]
pub struct SsThirdSurfaceBranchLimits {
    pub roots:SsAlgebraicRootCensusLimits,
    pub point:SsThirdSurfaceMembershipLimits,
    pub max_queries:usize,
    pub max_reserved_work:usize,
}
impl Default for SsThirdSurfaceBranchLimits {
    fn default()->Self {Self {roots:Default::default(),point:Default::default(),max_queries:192,max_reserved_work:16000000}}
}
#[derive(Clone,Debug)]
pub enum SsThirdSurfaceBranchEntry {
    Inside(Arc<QualifiedSsThirdSurfacePoint>),
    Outside(Arc<QualifiedSsThirdSurfaceExclusion>),
}
impl SsThirdSurfaceBranchEntry {
    pub fn point(&self)->&Arc<OriginalSsRootPoint> {match self {Self::Inside(p)=>p.point(),Self::Outside(p)=>p.point()}}
    pub fn is_inside(&self)->bool {matches!(self,Self::Inside(_))}
    fn source(&self)->&Arc<QualifiedSsThirdSurfaceCandidates> {match self {Self::Inside(p)=>p.source(),Self::Outside(p)=>p.source()}}
    fn same_evidence(&self,b:&Self)->bool {
        match (self,b) {
            (Self::Inside(a),Self::Inside(b))=>a.local_bounds()==b.local_bounds() && a.third_uv_bounds()==b.third_uv_bounds() && a.work()==b.work() && a.clock_work()==b.clock_work(),
            (Self::Outside(a),Self::Outside(b))=>a.kind()==b.kind() && a.bounds()==b.bounds() && a.allowed()==b.allowed() && a.work()==b.work() && a.clock_work()==b.clock_work(),
            _=>false,
        }
    }
}
#[derive(Clone,Debug)]
pub struct QualifiedSsThirdSurfaceBranchCensus {
    source:Arc<QualifiedSsThirdSurfaceCandidates>,curve:Arc<OriginalSsRootCurve>,
    limits:SsThirdSurfaceBranchLimits,roots:Arc<QualifiedSsAlgebraicRootCensus>,
    entries:Vec<SsThirdSurfaceBranchEntry>,queries:usize,reserved_work:usize,
}
fn reserve(queries:&mut usize,work:&mut usize,amount:usize,limits:SsThirdSurfaceBranchLimits)->Result<()> {
    let next=work.checked_add(amount).ok_or_else(||crate::resource("Third-branch work reservation overflow"))?;
    if *queries==limits.max_queries || next>limits.max_reserved_work {
        return Err(crate::resource("Complete third-branch query/work reservation exhausted"));
    }
    *queries+=1;*work=next;Ok(())
}
impl QualifiedSsThirdSurfaceBranchCensus {
    pub fn source(&self)->&Arc<QualifiedSsThirdSurfaceCandidates> {&self.source}
    pub fn curve(&self)->&Arc<OriginalSsRootCurve> {&self.curve}
    pub fn root_census(&self)->&Arc<QualifiedSsAlgebraicRootCensus> {&self.roots}
    pub fn entries(&self)->&[SsThirdSurfaceBranchEntry] {&self.entries}
    pub fn queries(&self)->usize {self.queries}
    pub fn reserved_work(&self)->usize {self.reserved_work}
    pub fn qualify(source:Arc<QualifiedSsThirdSurfaceCandidates>,curve:Arc<OriginalSsRootCurve>,limits:SsThirdSurfaceBranchLimits)->Result<Self> {
        check((1..=192).contains(&limits.max_queries) && (1..=3000000000usize).contains(&limits.max_reserved_work),"Invalid complete third-branch resource policy")?;
        validate_limits(limits.point.algebra)?;
        let original=curve.original_deflated_span_ideal().ok_or_else(||crate::input("Third-branch census requires original deflated graph authority"))?;
        check(original.source().first()==source.original.source().first() && original.source().second()==source.original.source().second()
            && original.spans()==source.original.spans() && curve.free_axis().index()==source.axis
            && curve.coordinate_domain().iter().all(|d|0.<=d[0] && d[0]<=d[1] && d[1]<=1.),"Third-branch original source span/axis/domain authority differs")?;
        let domain=curve.parameter_domain();
        let domain=[Q::from_float(domain[0]).unwrap(),Q::from_float(domain[1]).unwrap()];
        // Reserve the entire scalar census allowance before any work, including
        // the empty-list path. Failed attempts consume their full reservation.
        if limits.roots.clock.max_operations>limits.max_reserved_work {return Err(crate::resource("Third-branch scalar census reservation exhausted"));}
        let mut reserved_work=limits.roots.clock.max_operations;
        let roots=Arc::new(source.isolate_candidate_clocks(domain,limits.roots)?);
        let mut queries=0;let mut entries=Vec::new();
        let attempt=limits.point.algebra.max_work.checked_add(limits.point.clock.max_operations)
            .ok_or_else(||crate::resource("Third-branch point reservation overflow"))?;
        for root in roots.roots() {
            reserve(&mut queries,&mut reserved_work,limits.point.clock.max_operations,limits)?;
            let point=Arc::new(OriginalSsRootPoint::from_algebraic(OriginalSsAlgebraicPoint::new(curve.clone(),Arc::new(root.clone()))?,
                SsAlgebraicPointLimits {bisections:0,clock:limits.point.clock,..Default::default()})?);
            reserve(&mut queries,&mut reserved_work,attempt,limits)?;
            let entry=match QualifiedSsThirdSurfaceExclusion::qualify(source.clone(),point.clone(),limits.point) {
                Ok(proof)=>SsThirdSurfaceBranchEntry::Outside(Arc::new(proof)),
                Err(error) if error.code==crate::INVALID_INPUT=> {
                    reserve(&mut queries,&mut reserved_work,attempt,limits)?;
                    SsThirdSurfaceBranchEntry::Inside(Arc::new(QualifiedSsThirdSurfacePoint::qualify(source.clone(),point,limits.point)?))
                },
                Err(error)=>return Err(error),
            };
            entries.push(entry);
        }
        check(entries.len()==roots.roots().len(),"Complete third-branch census omitted a scalar candidate")?;
        Ok(Self {source,curve,limits,roots,entries,queries,reserved_work})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;self.curve.reverify()?;self.roots.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.curve.clone(),self.limits)?;
        check(self.queries==fresh.queries && self.reserved_work==fresh.reserved_work && self.entries.len()==fresh.entries.len() && self.entries.len()==self.roots.roots().len()
            && self.roots.polynomial()==fresh.roots.polynomial() && self.roots.domain()==fresh.roots.domain(),"Complete third-branch coverage changed")?;
        for ((a,b),root) in self.entries.iter().zip(&fresh.entries).zip(self.roots.roots()) {
            let clock=a.point().algebraic_point().ok_or_else(||crate::input("Third-branch candidate lost its exact clock"))?.clock();
            check(Arc::ptr_eq(a.source(),&self.source) && Arc::ptr_eq(a.point().curve(),&self.curve)
                && clock.polynomial()==root.polynomial() && clock.isolating_interval()==root.isolating_interval()
                && a.same_evidence(b),"Third-branch candidate owner, selector or evidence changed")?;
        }
        Ok(())
    }
}
