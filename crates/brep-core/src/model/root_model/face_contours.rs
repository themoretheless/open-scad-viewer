//! Source-preserving construction of a candidate single face from mixed SS pairs.
use super::*;
#[derive(Clone,Debug)]
pub struct RootFaceContour {
    pub wire: ss::OriginalSsRootWire,
    /// Explicit source surface slot for each world edge.
    pub supports: Vec<usize>,
}
impl RootBrep {
    /// Assemble an open candidate face. UV closure is proved; embedding, hole
    /// nesting and retained material must be qualified independently.
    pub fn face_contours(surface: Surface, outer: RootFaceContour,
        holes: Vec<RootFaceContour>, tolerance_mm: f64) -> Result<Self> {
        if holes.len()>64 {return Err(unproved("Root face hole budget exhausted"));}
        surface.validate()?;
        let mut t=RootTopology {vertices:vec![],edges:vec![],loops:vec![],faces:vec![],
            shells:vec![],bodies:vec![],tolerance_mm};
        for contour in std::iter::once(outer).chain(holes) {
            let edges=contour.wire.edges();
            if contour.supports.len()!=edges.len() {return Err(unproved("Each root face edge needs an explicit support"));}
            let mut coedges=Vec::new();
            for (i,edge) in edges.iter().enumerate() {
                let support=contour.supports[i];
                if support>=2 {return Err(unproved("Invalid root face support"));}
                let source=edge.curve().source();
                let selected=if support==0 {source.first()} else {source.second()};
                if selected!=&surface {return Err(unproved("Root face support belongs to another surface"));}
                let next=(i+1)%edges.len();
                qualify_root_uv_joint(edge.end(),support,edges[next].start(),contour.supports[next])?;
                let mut vertices=[0;2];
                for (k,port) in [edge.start(),edge.end()].into_iter().enumerate() {
                    vertices[k]=if let Some(id)=t.vertices.iter().position(|v|v.point.selector.same_selector(port.point())) {id}
                        else {let id=t.vertices.len();t.vertices.push(brep_topology::Vertex {point:RootVertex {selector:port.point().clone()}});id};
                }
                let id=t.edges.len();
                t.edges.push(brep_topology::Edge {curve:edge.clone(),vertices,degenerate:false});
                coedges.push(brep_topology::Coedge::new(id,false,RootPcurve {world_edge:id,support,reversed:false}));
            }
            t.loops.push(brep_topology::Loop {coedges});
        }
        t.faces.push(brep_topology::Face {surface,outer:0,holes:(1..t.loops.len()).collect()});
        t.shells.push(brep_topology::Shell {faces:vec![brep_topology::FaceUse {face:0,reversed:false}],closed:false});
        Self::from_topology(t)
    }
}
