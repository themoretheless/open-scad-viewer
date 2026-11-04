//! Event-to-source-edge cut selection; no face/material region is discarded.
use super::*;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SsEventCutSelection {RetainedFace,AllCandidates}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SsEventCutUse {ExcludedFace,OutsideEdge,Vertex(usize)}
#[derive(Clone,Copy,Debug)]
pub struct SsEventEdgePartitionLimits {
    pub edge:ss::SsRootEdgePartitionLimits,pub max_event_queries:usize,pub max_reserved_work:usize,
}
impl Default for SsEventEdgePartitionLimits {
    fn default()->Self {Self {edge:Default::default(),max_event_queries:4096,max_reserved_work:32000000}}
}
pub struct QualifiedSsEventEdgePartition {
    events:Arc<QualifiedOrderedSsFaceEvents>,edge:Arc<ss::OriginalSsRootEdge>,selection:SsEventCutSelection,
    limits:SsEventEdgePartitionLimits,partition:ss::QualifiedOriginalSsEdgePartition,uses:Vec<SsEventCutUse>,queries:usize,reserved_work:usize,
}
impl QualifiedSsEventEdgePartition {
    pub fn events(&self)->&Arc<QualifiedOrderedSsFaceEvents> {&self.events}
    pub fn edge(&self)->&Arc<ss::OriginalSsRootEdge> {&self.edge}
    pub fn partition(&self)->&ss::QualifiedOriginalSsEdgePartition {&self.partition}
    pub fn event_uses(&self)->&[SsEventCutUse] {&self.uses}
    pub fn queries(&self)->usize {self.queries}
    pub fn reserved_work(&self)->usize {self.reserved_work}
    pub fn qualify(events:Arc<QualifiedOrderedSsFaceEvents>,edge:Arc<ss::OriginalSsRootEdge>,selection:SsEventCutSelection,limits:SsEventEdgePartitionLimits)->Result<Self> {
        need((1..=262144).contains(&limits.max_event_queries) && (1..=3000000000usize).contains(&limits.max_reserved_work)
            && Arc::ptr_eq(edge.curve(),events.source.spans()[0].curve()) && edge.start().phase()==[0;4] && edge.end().phase()==[0;4],
            "Invalid source edge/event identity, phase or bounded partition policy")?;
        let indices:Vec<_>=if edge.forward() {(0..events.events.len()).collect()} else {(0..events.events.len()).rev().collect()};
        let (low,high)=if edge.forward() {(edge.start().parameter_bounds(),edge.end().parameter_bounds())} else {(edge.end().parameter_bounds(),edge.start().parameter_bounds())};
        let mut uses=vec![None;events.events.len()];let mut cuts=Vec::new();let mut selected=Vec::new();let mut queries=0;let mut reserved_work=0usize;
        let amount=events.limits.clock.max_operations.checked_mul(2).ok_or_else(||unproved("SS event port reservation overflows"))?;
        for i in indices {
            if selection==SsEventCutSelection::RetainedFace && !events.events[i].retained {uses[i]=Some(SsEventCutUse::ExcludedFace);continue;}
            let next=reserved_work.checked_add(amount).ok_or_else(||unproved("SS event partition reservation overflows"))?;
            need(queries<limits.max_event_queries && next<=limits.max_reserved_work,"SS event partition query/reservation allowance exhausted")?;
            queries+=1;reserved_work=next;
            let port=events.bind_port(i,edge.curve().clone(),[0;4],limits.edge.geometry)?;
            if port.parameter_bounds()[1]<low[0] || high[1]<port.parameter_bounds()[0] {uses[i]=Some(SsEventCutUse::OutsideEdge);continue;}
            need(cuts.len()<limits.edge.max_cuts,"SS event partition cut allowance exhausted")?;
            cuts.push(port);selected.push(i);
        }
        let partition=ss::QualifiedOriginalSsEdgePartition::qualify((*edge).clone(),cuts,limits.edge)?;
        for (i,v) in selected.iter().zip(partition.cut_vertices()) {uses[*i]=Some(SsEventCutUse::Vertex(*v));}
        let uses=uses.into_iter().map(|u|u.ok_or_else(||unproved("SS event partition omitted an event decision"))).collect::<Result<_>>()?;
        Ok(Self {events,edge,selection,limits,partition,uses,queries,reserved_work})
    }
    pub fn reverify(&self)->Result<()> {
        self.events.reverify()?;self.edge.reverify()?;self.partition.reverify()?;
        let fresh=Self::qualify(self.events.clone(),self.edge.clone(),self.selection,self.limits)?;
        need(self.uses==fresh.uses && self.queries==fresh.queries && self.reserved_work==fresh.reserved_work
            && self.partition.cut_vertices()==fresh.partition.cut_vertices() && self.partition.pieces().len()==fresh.partition.pieces().len(),
            "SS event-to-edge partition source, decisions or lineage changed")
    }
}
