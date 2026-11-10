//! Exact source-parameter event identity/order, not world-point tolerance welding.
use super::*;
use std::cmp::Ordering;
#[derive(Clone,Copy,Debug)]
pub struct SsFaceEventLimits {
    pub clock:ss::SsAlgebraicClockLimits,pub max_refinements:usize,pub max_comparisons:usize,
    pub max_clock_queries:usize,pub max_reserved_work:usize,pub max_events:usize,
}
impl Default for SsFaceEventLimits {
    fn default()->Self {Self {clock:Default::default(),max_refinements:256,max_comparisons:1000000,
        max_clock_queries:100000,max_reserved_work:3000000000,max_events:4096}}
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct SsFaceEvent {
    pub canonical_record:usize,pub records:Vec<usize>,pub retained:bool,parameter:[Q;2],
}
impl SsFaceEvent {pub fn parameter_bounds(&self)->&[Q;2] {&self.parameter}}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct SsFaceEventWork {pub comparisons:usize,pub clock_queries:usize,pub reserved_work:usize}
pub struct QualifiedOrderedSsFaceEvents {
    source:Arc<QualifiedSsTrimmedFaceCensus>,limits:SsFaceEventLimits,events:Vec<SsFaceEvent>,work:SsFaceEventWork,
}
fn clock(source:&QualifiedSsTrimmedFaceCensus,record:usize)->Result<&ss::QualifiedSsAlgebraicClock> {
    source.point(record).and_then(|p|p.algebraic_point()).map(|p|p.clock().as_ref()).ok_or_else(||unproved("Ordered SS event lost its exact scalar owner"))
}
fn reserve(w:&mut SsFaceEventWork,l:SsFaceEventLimits)->Result<()> {
    let next=w.reserved_work.checked_add(l.clock.max_operations).ok_or_else(||unproved("SS event work reservation overflows"))?;
    need(w.clock_queries<l.max_clock_queries && next<=l.max_reserved_work,"Exact SS event clock-query/work allowance exhausted")?;
    w.clock_queries+=1;w.reserved_work=next;Ok(())
}
fn separate(a:&[Q;2],b:&[Q;2])->Option<Ordering> {
    if a[1]<b[0] {Some(Ordering::Less)} else if b[1]<a[0] {Some(Ordering::Greater)} else {None}
}
fn compare(a:&ss::QualifiedSsAlgebraicClock,b:&ss::QualifiedSsAlgebraicClock,l:SsFaceEventLimits,w:&mut SsFaceEventWork)->Result<(Ordering,[Q;2],[Q;2])> {
    need(w.comparisons<l.max_comparisons,"Exact SS event comparison allowance exhausted")?;w.comparisons+=1;
    let mut left=a.isolating_interval().clone();let mut right=b.isolating_interval().clone();
    if a.polynomial()==b.polynomial() && left==right {return Ok((Ordering::Equal,left,right));}
    if let Some(order)=separate(&left,&right) {return Ok((order,left,right));}
    reserve(w,l)?;
    if a.equals_clock(b,l.clock)?.0 {return Ok((Ordering::Equal,left,right));}
    let mut n=8.min(l.max_refinements);
    while n>0 {
        reserve(w,l)?;left=a.enclose(n,l.clock)?.bounds;
        reserve(w,l)?;right=b.enclose(n,l.clock)?.bounds;
        if let Some(order)=separate(&left,&right) {return Ok((order,left,right));}
        if n==l.max_refinements {break;}
        n=(n*2).min(l.max_refinements);
    }
    Err(unproved("Distinct exact SS event clocks remain unordered within the allowance"))
}
fn kept(source:&QualifiedSsTrimmedFaceCensus,i:usize)->bool {
    matches!(source.records()[i].decision,Decision::Retained|Decision::RetainedBoundary)
}
impl QualifiedOrderedSsFaceEvents {
    pub fn source(&self)->&Arc<QualifiedSsTrimmedFaceCensus> {&self.source}
    pub fn events(&self)->&[SsFaceEvent] {&self.events}
    pub fn work(&self)->SsFaceEventWork {self.work}
    pub fn point(&self,event:usize)->Option<&Arc<ss::OriginalSsRootPoint>> {
        self.source.point(self.events.get(event)?.canonical_record)
    }
    pub fn qualify(source:Arc<QualifiedSsTrimmedFaceCensus>,limits:SsFaceEventLimits)->Result<Self> {
        need(limits.max_refinements<=1024 && (1..=1000000).contains(&limits.max_comparisons)
            && (1..=100000).contains(&limits.max_clock_queries) && (1..=3000000000usize).contains(&limits.max_reserved_work)
            && (1..=262144).contains(&limits.max_events) && (1..=64).contains(&limits.clock.max_degree)
            && (1..=1000000).contains(&limits.clock.max_operations) && (64..=131072).contains(&limits.clock.max_coefficient_bits),
            "Invalid exact SS event ordering policy")?;
        let mut events:Vec<SsFaceEvent>=Vec::new();let mut work=SsFaceEventWork::default();
        for record in 0..source.records().len() {
            let key=clock(&source,record)?;let mut lo=0;let mut hi=events.len();let mut found=None;
            while lo<hi {
                let mid=lo+(hi-lo)/2;
                match compare(key,clock(&source,events[mid].canonical_record)?,limits,&mut work)?.0 {
                    Ordering::Equal=>{found=Some(mid);break;},Ordering::Less=>hi=mid,Ordering::Greater=>lo=mid+1,
                }
            }
            if let Some(i)=found {events[i].records.push(record);} else {
                need(events.len()<limits.max_events,"Exact SS event count allowance exhausted")?;
                events.insert(lo,SsFaceEvent {canonical_record:record,records:vec![record],retained:false,parameter:key.isolating_interval().clone()});
            }
        }
        for event in &mut events {
            if let Some(i)=event.records.iter().copied().find(|i|kept(&source,*i)) {event.canonical_record=i;event.retained=true;}
            event.parameter=clock(&source,event.canonical_record)?.isolating_interval().clone();
        }
        // Canonical owner selection must preserve strict query-box order too.
        // Intersections only tighten earlier proved bounds, never redefine roots.
        for i in 1..events.len() {
            let (order,a,b)=compare(clock(&source,events[i-1].canonical_record)?,clock(&source,events[i].canonical_record)?,limits,&mut work)?;
            need(order==Ordering::Less,"Canonical SS events lost strict exact parameter order")?;
            for (j,bounds) in [(i-1,a),(i,b)] {
                events[j].parameter[0]=events[j].parameter[0].clone().max(bounds[0].clone());
                events[j].parameter[1]=events[j].parameter[1].clone().min(bounds[1].clone());
            }
        }
        let all:BTreeSet<_>=events.iter().flat_map(|e|e.records.iter().copied()).collect();
        need(events.iter().map(|e|e.records.len()).sum::<usize>()==source.records().len()
            && all.len()==source.records().len() && all.iter().copied().eq(0..source.records().len()),"Canonical SS event lineage omitted a source record")?;
        need(events.iter().all(|e|e.parameter[0]<=e.parameter[1]) && events.windows(2).all(|e|e[0].parameter[1]<e[1].parameter[0]),"Canonical SS event bounds are not strictly separated")?;
        Ok(Self {source,limits,events,work})
    }
    /// Fresh geometry/clock queries have their own allowances; they are not
    /// charged to the already completed ordering ledger. Selector owners stay.
    pub fn bind_port(&self,event:usize,target:Arc<ss::OriginalSsRootCurve>,phase:[i32;4],geometry:ss::OriginalSsCurveLimits)->Result<ss::QualifiedOriginalSsPort> {
        let point=self.point(event).ok_or_else(||unproved("Choose an existing exact SS event"))?;
        let queried=Arc::new(point.with_query_policy(ss::SsAlgebraicPointLimits {
            bisections:self.limits.max_refinements,clock:self.limits.clock,geometry,
        })?);
        ss::bind_original_ss_port(queried,target,phase,geometry)
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;
        let fresh=Self::qualify(self.source.clone(),self.limits)?;
        need(self.events==fresh.events && self.work==fresh.work,"Exact SS event grouping, order or record lineage changed")
    }
}

mod partition;
pub use partition::{SsEventCutSelection,SsEventCutUse,SsEventEdgePartitionLimits,QualifiedSsEventEdgePartition};
