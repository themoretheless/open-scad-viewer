//! Whole-UV-box classification of original SS events against an authored face.
//! Complete Cox-span coverage is required; this is not Boolean material authority.
use crate::{Error,Model,Result,TopoId,face_domain::FaceDomain};
use nurbs_core::{ss_intersection as ss,trim_domain::Location};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
use std::{collections::BTreeSet,sync::Arc};
#[derive(Clone,Copy,Debug)]
pub struct Limits {
    pub tolerance_uv:f64,pub max_trim_pairs:usize,pub max_trim_cells:usize,pub max_trim_domain_cells:usize,
    pub max_boundary_cells:usize,pub max_point_cells:usize,pub max_spans:usize,pub max_records:usize,
}
impl Default for Limits {
    fn default()->Self {Self {tolerance_uv:1e-8,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:100000,
        max_boundary_cells:100000,max_point_cells:100000,max_spans:4096,max_records:4096}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Decision {ExcludedSpan,ExcludedTrim,Retained,RetainedBoundary}
#[derive(Clone,Debug,PartialEq)]
pub struct Record {pub span:usize,pub candidate:usize,pub decision:Decision,pub uv:Option<[[f64;2];2]>,pub winding:Option<i32>,pub cells:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Work {pub trim_pairs:usize,pub trim_cells:usize,pub trim_domain_cells:usize,pub boundary_cells:usize,pub point_cells:usize}
pub struct QualifiedSsTrimmedFaceCensus {
    model:Arc<Model>,face:usize,identity:TopoId,spans:Vec<Arc<ss::QualifiedSsThirdSurfaceBranchCensus>>,
    limits:Limits,records:Vec<Record>,work:Work,
}
fn unproved(message:&str)->Error {Error::new("BREP_SS_TRIMMED_FACE_UNPROVED",message)}
fn need(ok:bool,message:&str)->Result<()> {if ok {Ok(())} else {Err(unproved(message))}}
fn outward(q:&Q,lower:bool)->Result<f64> {
    let mut v=q.to_f64().filter(|v|v.is_finite()).ok_or_else(||unproved("Original third UV enclosure overflows binary64"))?;
    for _ in 0..8 {
        let exact=Q::from_float(v).ok_or_else(||unproved("Original third UV conversion overflows"))?;
        if (lower && exact<=*q) || (!lower && *q<=exact) {return Ok(v);}
        v=if lower {v.next_down()} else {v.next_up()};
    }
    Err(unproved("Original third UV outward conversion is unproved"))
}
impl QualifiedSsTrimmedFaceCensus {
    pub fn model(&self)->&Arc<Model> {&self.model}
    pub fn source_matches(&self,model:&Arc<Model>)->bool {Arc::ptr_eq(&self.model,model)}
    pub fn face(&self)->usize {self.face}
    pub fn identity(&self)->TopoId {self.identity}
    pub fn spans(&self)->&[Arc<ss::QualifiedSsThirdSurfaceBranchCensus>] {&self.spans}
    pub fn records(&self)->&[Record] {&self.records}
    pub fn work(&self)->Work {self.work}
    pub fn point(&self,record:usize)->Option<&Arc<ss::OriginalSsRootPoint>> {
        let r=self.records.get(record)?;Some(self.spans.get(r.span)?.entries().get(r.candidate)?.point())
    }
    pub fn qualify(model:Arc<Model>,face_index:usize,spans:Vec<Arc<ss::QualifiedSsThirdSurfaceBranchCensus>>,limits:Limits)->Result<Self> {
        model.validate()?;
        need(model.persistent_naming_complete() && limits.tolerance_uv.is_finite() && limits.tolerance_uv>0.
            && (1..=100000).contains(&limits.max_trim_pairs) && (1..=100000).contains(&limits.max_trim_cells)
            && (1..=1000000).contains(&limits.max_trim_domain_cells) && (1..=1000000).contains(&limits.max_boundary_cells)
            && (1..=1000000).contains(&limits.max_point_cells) && (1..=4096).contains(&limits.max_spans)
            && (1..=262144).contains(&limits.max_records),"Invalid original face snapshot or bounded trim policy")?;
        let face=model.faces.get(face_index).ok_or_else(||unproved("Third face index is out of range"))?;
        let surface=&face.surface;surface.validate()?;
        let us=(surface.degree_u..surface.control_points.len()).filter(|&i|surface.knots_u[i]<surface.knots_u[i+1]).collect::<Vec<_>>();
        let vs=(surface.degree_v..surface.control_points[0].len()).filter(|&i|surface.knots_v[i]<surface.knots_v[i+1]).collect::<Vec<_>>();
        let count=us.len().checked_mul(vs.len()).ok_or_else(||unproved("Third-face Cox-span count overflows"))?;
        need(count>0 && count<=limits.max_spans && spans.len()==count,"Complete third-face Cox-span coverage is missing or over budget")?;
        let expected:BTreeSet<[usize;2]>=us.iter().flat_map(|u|vs.iter().map(move |v|[*u,*v])).collect();
        let mut seen=BTreeSet::new();let mut records_count=0usize;
        for span in &spans {
            need(Arc::ptr_eq(span.curve(),spans[0].curve()) && span.source().third()==surface
                && expected.contains(&span.source().third_spans()) && seen.insert(span.source().third_spans()),
                "Third-face span is duplicated, foreign or owned by a different SS graph")?;
            records_count=records_count.checked_add(span.entries().len()).ok_or_else(||unproved("Third-face candidate count overflows"))?;
            need(records_count<=limits.max_records,"Third-face candidate coverage exceeds its record budget")?;
        }
        need(seen==expected,"Third-face Cox-span coverage is incomplete")?;
        let mut loops=Vec::new();
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            need(model.loops[wire].coedges.iter().all(|c|c.trim().is_none()),"Restricted or lifted coedges require separate original trim authority")?;
            loops.push(model.loops[wire].coedges.iter().map(|c|c.pcurve.clone()).collect::<Vec<_>>());
        }
        let audit=nurbs_core::trim_region_audit::inspect(&loops,limits.tolerance_uv,limits.max_trim_pairs,limits.max_trim_cells,limits.max_trim_domain_cells)?;
        need(audit.valid==Some(true),"Original face outer/hole region validity is unproved")?;
        let mut work=Work {trim_pairs:audit.pairs,trim_cells:audit.cells,trim_domain_cells:audit.domain_cells,boundary_cells:0,point_cells:0};
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                need(work.boundary_cells<limits.max_boundary_cells,"Original boundary agreement budget exhausted")?;
                let agreement=nurbs_core::curve_surface_agreement::verify(&model.edges[c.edge].curve,&c.pcurve,surface,c.reversed,model.tolerance_mm,
                    (limits.max_boundary_cells-work.boundary_cells).min(100000))?;
                work.boundary_cells+=agreement.cells;
                need(agreement.status==nurbs_core::curve_surface_agreement::Status::WithinTolerance,"Original face boundary lift is unproved")?;
            }
        }
        let domain=FaceDomain::new(&model,face_index,limits.tolerance_uv)?;
        let mut records=Vec::with_capacity(records_count);
        for (span_index,span) in spans.iter().enumerate() {
            for (candidate,entry) in span.entries().iter().enumerate() {
                let mut record=Record {span:span_index,candidate,decision:Decision::ExcludedSpan,uv:None,winding:None,cells:0};
                if let ss::SsThirdSurfaceBranchEntry::Inside(member)=entry {
                    let q=member.third_uv_bounds();
                    let uv=[[outward(&q[0][0],true)?,outward(&q[0][1],false)?],[outward(&q[1][0],true)?,outward(&q[1][1],false)?]];
                    record.uv=Some(uv);
                    record.decision=if domain.on_linear_boundary(uv) {Decision::RetainedBoundary}
                    else if domain.contains_closed_rectangle(uv) {Decision::Retained}
                    else {
                        need(work.point_cells<limits.max_point_cells,"Original face UV classification budget exhausted")?;
                        let result=domain.classify(uv,(limits.max_point_cells-work.point_cells).min(100000))?;
                        work.point_cells+=result.cells;record.cells=result.cells;record.winding=result.winding;
                        match result.location {Location::Inside=>Decision::Retained,Location::Outside=>Decision::ExcludedTrim,
                            Location::Unresolved=>return Err(unproved("Original whole-UV-box trim membership is unresolved"))}
                    };
                }
                records.push(record);
            }
        }
        let identity=model.1.faces[face_index];
        Ok(Self {model,face:face_index,identity,spans,limits,records,work})
    }
    pub fn reverify(&self)->Result<()> {
        for span in &self.spans {span.reverify()?;}
        let fresh=Self::qualify(self.model.clone(),self.face,self.spans.clone(),self.limits)?;
        need(self.identity==fresh.identity && self.records==fresh.records && self.work==fresh.work,"Original trimmed-face snapshot, coverage or proof changed")
    }
}

mod events;
pub use events::{SsFaceEventLimits,SsFaceEvent,SsFaceEventWork,QualifiedOrderedSsFaceEvents};
