//! Lossless root payload transport into the existing mixed BRep pipeline.
use super::*;
impl MixedBrep {
    /// Preserve every original root graph, algebraic owner and topology index.
    /// Conversion is storage transport, not retained-material qualification.
    pub fn from_root(source:&crate::root_model::RootBrep)->Result<Self> {
        let t=source.topology();
        let loops=t.loops.iter().map(|l|brep_topology::Loop {coedges:l.coedges.iter().map(|c|
            brep_topology::Coedge::new(c.edge,c.reversed,MixedPcurve::Root(c.pcurve.clone()))).collect()}).collect();
        Self::from_topology(MixedTopology {
            vertices:t.vertices.iter().map(|v|brep_topology::Vertex {point:MixedVertex::Root(v.point.selector.clone())}).collect(),
            edges:t.edges.iter().map(|e|brep_topology::Edge {vertices:e.vertices,degenerate:e.degenerate,curve:MixedEdge::Root(e.curve.clone())}).collect(),
            loops,faces:t.faces.clone(),shells:t.shells.clone(),bodies:t.bodies.clone(),tolerance_mm:t.tolerance_mm,
        })
    }
    /// Recover pure-root storage only when every payload has root authority.
    /// Rational geometry is never fitted, lifted or silently discarded.
    pub fn to_root(&self)->Result<crate::root_model::RootBrep> {
        let t=&self.topology;
        let vertices=t.vertices.iter().map(|v|match &v.point {
            MixedVertex::Root(selector)=>Ok(brep_topology::Vertex {point:crate::root_model::RootVertex {selector:selector.clone()}}),
            _=>Err(unproved("Mixed rational vertex has no original-root selector")),
        }).collect::<Result<Vec<_>>>()?;
        let edges=t.edges.iter().map(|e|match &e.curve {
            MixedEdge::Root(curve)=>Ok(brep_topology::Edge {vertices:e.vertices,degenerate:e.degenerate,curve:curve.clone()}),
            _=>Err(unproved("Mixed rational edge has no original-root graph")),
        }).collect::<Result<Vec<_>>>()?;
        let mut loops=Vec::new();
        for l in &t.loops {
            let mut coedges=Vec::new();
            for c in &l.coedges {
                let MixedPcurve::Root(pcurve)=&c.pcurve else {return Err(unproved("Mixed rational pcurve has no original-root projection"));};
                if c.trim().is_some() {return Err(unproved("Root transport requires exact owned endpoint ranges"));}
                coedges.push(brep_topology::Coedge::new(c.edge,c.reversed,pcurve.clone()));
            }
            loops.push(brep_topology::Loop {coedges});
        }
        crate::root_model::RootBrep::from_topology(crate::root_model::RootTopology {
            vertices,edges,loops,faces:t.faces.clone(),shells:t.shells.clone(),bodies:t.bodies.clone(),tolerance_mm:t.tolerance_mm,
        })
    }
}
