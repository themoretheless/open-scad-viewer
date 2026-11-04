//! Exact source-edge restrictions with endpoint ownership and cut lineage.
use super::*;
#[derive(Clone,Copy,Debug)]
pub struct SsRootEdgePartitionLimits {
    pub max_cuts:usize,pub max_identity_queries:usize,
    pub clock:SsAlgebraicClockLimits,pub geometry:OriginalSsCurveLimits,
}
impl Default for SsRootEdgePartitionLimits {
    fn default()->Self {Self {max_cuts:256,max_identity_queries:768,clock:Default::default(),geometry:Default::default()}}
}
#[derive(Clone,Debug)]
pub struct QualifiedOriginalSsEdgePartition {
    source:OriginalSsRootEdge,cuts:Vec<QualifiedOriginalSsPort>,limits:SsRootEdgePartitionLimits,
    pieces:Vec<OriginalSsRootEdge>,cut_vertices:Vec<usize>,identity_queries:usize,
}
fn same(a:&QualifiedOriginalSsPort,b:&QualifiedOriginalSsPort,l:SsRootEdgePartitionLimits,queries:&mut usize)->Result<bool> {
    if a.point().same_selector(b.point()) {return Ok(true);}
    let x=a.parameter_bounds();let y=b.parameter_bounds();
    if x[0]==x[1] && x==y {return Ok(true);}
    if x[1]<y[0] || y[1]<x[0] {return Ok(false);}
    if *queries==l.max_identity_queries {return Err(crate::resource("SS edge partition identity allowance exhausted"));}
    *queries+=1;a.point().same_source_point_in_charts(b.point(),l.clock,l.geometry)
}
fn before(a:&QualifiedOriginalSsPort,b:&QualifiedOriginalSsPort,forward:bool)->bool {
    if forward {a.parameter_bounds()[1]<b.parameter_bounds()[0]} else {b.parameter_bounds()[1]<a.parameter_bounds()[0]}
}
impl QualifiedOriginalSsEdgePartition {
    pub fn source(&self)->&OriginalSsRootEdge {&self.source}
    pub fn input_cuts(&self)->&[QualifiedOriginalSsPort] {&self.cuts}
    pub fn pieces(&self)->&[OriginalSsRootEdge] {&self.pieces}
    /// Vertex 0 is the original start; vertex pieces.len() the original end.
    pub fn cut_vertices(&self)->&[usize] {&self.cut_vertices}
    pub fn identity_queries(&self)->usize {self.identity_queries}
    /// Cuts must follow SOURCE traversal. Equal endpoint/adjacent cuts are
    /// explicitly aliased; overlapping unproved clocks never become vertices.
    pub fn qualify(source:OriginalSsRootEdge,cuts:Vec<QualifiedOriginalSsPort>,limits:SsRootEdgePartitionLimits)->Result<Self> {
        check((1..=256).contains(&limits.max_cuts) && (1..=768).contains(&limits.max_identity_queries)
            && cuts.len()<=limits.max_cuts,"Invalid or exhausted SS edge partition policy")?;
        check(source.start().phase()==[0;4] && source.end().phase()==[0;4],"SS edge partition requires explicit seam authority for lifted endpoints")?;
        let mut ports=vec![source.start().clone()];let mut mapped=Vec::new();let mut queries=0;let mut saw_end=false;
        for cut in &cuts {
            check(Arc::ptr_eq(cut.target(),source.curve()) && cut.phase()==[0;4],"Cut port must retain the literal original edge graph without inferred phases")?;
            if same(cut,source.start(),limits,&mut queries)? {
                check(ports.len()==1 && !saw_end,"SS start alias occurs after an interior/end cut")?;mapped.push(Some(0));continue;
            }
            if same(cut,source.end(),limits,&mut queries)? {saw_end=true;mapped.push(None);continue;}
            check(!saw_end,"Interior SS cut occurs after the original end")?;
            if ports.len()>1 && same(cut,ports.last().unwrap(),limits,&mut queries)? {
                mapped.push(Some(ports.len()-1));continue;
            }
            check(before(ports.last().unwrap(),cut,source.forward()) && before(cut,source.end(),source.forward()),
                "SS cut is outside, unordered or not strictly separated from an existing endpoint")?;
            ports.push(cut.clone());mapped.push(Some(ports.len()-1));
        }
        ports.push(source.end().clone());let end=ports.len()-1;
        let pieces=ports.windows(2).map(|p|OriginalSsRootEdge::new(p[0].clone(),p[1].clone())).collect::<Result<Vec<_>>>()?;
        check(pieces.iter().all(|e|e.forward()==source.forward() && Arc::ptr_eq(e.curve(),source.curve())),"SS partition changed original graph ownership or direction")?;
        let cut_vertices=mapped.into_iter().map(|v|v.unwrap_or(end)).collect();
        Ok(Self {source,cuts,limits,pieces,cut_vertices,identity_queries:queries})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;for cut in &self.cuts {cut.reverify()?;}
        let fresh=Self::qualify(self.source.clone(),self.cuts.clone(),self.limits)?;
        check(self.cut_vertices==fresh.cut_vertices && self.identity_queries==fresh.identity_queries && self.pieces.len()==fresh.pieces.len(),"SS edge partition coverage or cut lineage changed")?;
        for (a,b) in self.pieces.iter().zip(&fresh.pieces) {
            check(a.forward()==b.forward() && a.start().point().same_selector(b.start().point())
                && a.end().point().same_selector(b.end().point()),"SS partition endpoint selector changed")?;
        }
        Ok(())
    }
}
