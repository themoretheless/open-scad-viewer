//! Pure edge subdivision of an immutable exact-root BRep snapshot.
//! Existing face regions/shell orientation are preserved, not Boolean-selected.
use super::*;
use std::{collections::BTreeMap,ops::Range};
#[derive(Clone,Debug)]
pub struct RootEdgePartitionRequest {pub edge:usize,pub partition:Arc<ss::QualifiedOriginalSsEdgePartition>}
#[derive(Clone,Copy,Debug)]
pub struct RootBrepPartitionLimits {pub max_requests:usize,pub max_new_vertices:usize,pub max_new_edges:usize,pub max_new_coedges:usize}
impl Default for RootBrepPartitionLimits {
    fn default()->Self {Self {max_requests:256,max_new_vertices:1024,max_new_edges:1024,max_new_coedges:4096}}
}
pub struct QualifiedRootBrepEdgeSubdivision {
    source:Arc<RootBrep>,model:Arc<RootBrep>,requests:Vec<RootEdgePartitionRequest>,limits:RootBrepPartitionLimits,
    edges:Vec<Vec<usize>>,coedges:Vec<Vec<Vec<usize>>>,cut_vertices:Vec<Vec<usize>>,new_vertices:Range<usize>,new_edges:Range<usize>,new_coedges:usize,
}
fn need(ok:bool,message:&str)->Result<()> {if ok {Ok(())} else {Err(unproved(message))}}
fn source_edge_matches(a:&ss::OriginalSsRootEdge,b:&ss::OriginalSsRootEdge)->bool {
    Arc::ptr_eq(a.curve(),b.curve()) && a.forward()==b.forward()
        && a.start().point().same_selector(b.start().point()) && a.end().point().same_selector(b.end().point())
        && a.start().phase()==b.start().phase() && a.end().phase()==b.end().phase()
}
impl QualifiedRootBrepEdgeSubdivision {
    pub fn source(&self)->&Arc<RootBrep> {&self.source}
    pub fn source_matches(&self,source:&Arc<RootBrep>)->bool {Arc::ptr_eq(&self.source,source)}
    pub fn model(&self)->&Arc<RootBrep> {&self.model}
    pub fn edge_children(&self,edge:usize)->Option<&[usize]> {self.edges.get(edge).map(Vec::as_slice)}
    pub fn coedge_children(&self,wire:usize,coedge:usize)->Option<&[usize]> {self.coedges.get(wire)?.get(coedge).map(Vec::as_slice)}
    pub fn input_cut_vertices(&self,request:usize)->Option<&[usize]> {self.cut_vertices.get(request).map(Vec::as_slice)}
    pub fn new_vertices(&self)->Range<usize> {self.new_vertices.clone()}
    pub fn new_edges(&self)->Range<usize> {self.new_edges.clone()}
    pub fn new_coedges(&self)->usize {self.new_coedges}
    pub fn qualify(source:Arc<RootBrep>,requests:Vec<RootEdgePartitionRequest>,limits:RootBrepPartitionLimits)->Result<Self> {
        need((1..=256).contains(&limits.max_requests) && requests.len()<=limits.max_requests
            && (1..=65536).contains(&limits.max_new_vertices) && (1..=65536).contains(&limits.max_new_edges)
            && (1..=262144).contains(&limits.max_new_coedges),"Invalid or exhausted root BRep subdivision policy")?;
        let old=source.topology();let mut selected=BTreeMap::new();
        for (i,request) in requests.iter().enumerate() {
            let edge=old.edges.get(request.edge).ok_or_else(||unproved("Choose an existing original root BRep edge"))?;
            need(selected.insert(request.edge,i).is_none() && source_edge_matches(&edge.curve,request.partition.source()),
                "Duplicated or foreign original edge partition request")?;
        }
        let mut topology=old.clone();let mut edges:Vec<Vec<usize>>=(0..old.edges.len()).map(|i|vec![i]).collect();
        let mut cut_vertices=vec![Vec::new();requests.len()];
        for (edge_index,request_index) in selected {
            let partition=&requests[request_index].partition;let pieces=partition.pieces();
            need(!pieces.is_empty(),"Original root edge partition lost its source interval")?;
            let extra=pieces.len()-1;
            need(topology.vertices.len()-old.vertices.len()+extra<=limits.max_new_vertices
                && topology.edges.len()-old.edges.len()+extra<=limits.max_new_edges,"Root BRep subdivision vertex/edge allowance exhausted")?;
            let mut vertices=vec![old.edges[edge_index].vertices[0]];
            // Pure subdivision: no implicit weld to another source edge or old
            // vertex, even when world images happen to coincide.
            for piece in pieces.iter().take(extra) {
                let v=topology.vertices.len();topology.vertices.push(brep_topology::Vertex {
                    point:RootVertex {selector:piece.end().point().clone()},
                });vertices.push(v);
            }
            vertices.push(old.edges[edge_index].vertices[1]);let mut children=Vec::new();
            for (i,piece) in pieces.iter().enumerate() {
                let edge=brep_topology::Edge {vertices:[vertices[i],vertices[i+1]],curve:piece.clone(),degenerate:false};
                let id=if i==0 {topology.edges[edge_index]=edge;edge_index} else {let id=topology.edges.len();topology.edges.push(edge);id};
                children.push(id);
            }
            cut_vertices[request_index]=partition.cut_vertices().iter().map(|v|vertices[*v]).collect();edges[edge_index]=children;
        }
        let mut coedges=Vec::new();let mut new_coedges=0;
        for (wire_index,wire) in old.loops.iter().enumerate() {
            let mut expanded=Vec::new();let mut map=Vec::new();
            for coedge in &wire.coedges {
                let mut children=edges[coedge.edge].clone();if coedge.reversed {children.reverse();}
                new_coedges+=children.len()-1;
                need(new_coedges<=limits.max_new_coedges,"Root BRep subdivision coedge allowance exhausted")?;
                let mut positions=Vec::new();
                for child in children {
                    let mut c=coedge.clone();c.edge=child;c.pcurve.world_edge=child;
                    positions.push(expanded.len());expanded.push(c);
                }
                map.push(positions);
            }
            topology.loops[wire_index].coedges=expanded;coedges.push(map);
        }
        let new_vertices=old.vertices.len()..topology.vertices.len();let new_edges=old.edges.len()..topology.edges.len();
        let model=Arc::new(RootBrep::from_topology(topology)?);
        Ok(Self {source,model,requests,limits,edges,coedges,cut_vertices,new_vertices,new_edges,new_coedges})
    }
    pub fn reverify(&self)->Result<()> {
        self.source.reverify()?;for r in &self.requests {r.partition.reverify()?;}
        let fresh=Self::qualify(self.source.clone(),self.requests.clone(),self.limits)?;
        need(self.edges==fresh.edges && self.coedges==fresh.coedges && self.cut_vertices==fresh.cut_vertices
            && self.new_vertices==fresh.new_vertices && self.new_edges==fresh.new_edges && self.new_coedges==fresh.new_coedges,
            "Root BRep subdivision coverage or local lineage changed")?;
        let a=self.model.topology();let b=fresh.model.topology();
        need(a.vertices.len()==b.vertices.len() && a.edges.len()==b.edges.len() && a.loops.len()==b.loops.len()
            && a.faces==b.faces && a.shells==b.shells && a.bodies==b.bodies && a.tolerance_mm==b.tolerance_mm,"Root BRep subdivision face/shell/body data changed")?;
        for (a,b) in a.vertices.iter().zip(&b.vertices) {need(a.point.selector.same_selector(&b.point.selector),"Subdivided root BRep vertex selector changed")?;}
        for (a,b) in a.edges.iter().zip(&b.edges) {need(a.vertices==b.vertices && a.degenerate==b.degenerate && source_edge_matches(&a.curve,&b.curve),"Subdivided root BRep edge source or incidence changed")?;}
        for (a,b) in a.loops.iter().zip(&b.loops) {
            need(a.coedges.len()==b.coedges.len(),"Subdivided root BRep loop coverage changed")?;
            for (a,b) in a.coedges.iter().zip(&b.coedges) {need(a.edge==b.edge && a.reversed==b.reversed
                && a.pcurve.world_edge==b.pcurve.world_edge && a.pcurve.support==b.pcurve.support && a.pcurve.reversed==b.pcurve.reversed,
                "Subdivided root BRep pcurve role or direction changed")?;}
        }
        self.model.reverify()
    }
}
