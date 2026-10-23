//! Exact source restrictions on the existing generic indexed topology.
//! Only a privately admitted Body constructs this immutable projection.
use crate::{
    source_boundary_fragment::Fragment, source_collapsed_boundary::CollapsedBoundary,
    source_edge_restriction::Restriction, source_volume::Body,
};
use nurbs_core::{Error, Result, surface::Surface};
use std::collections::BTreeMap;
use value_codec::Value;
#[derive(Clone)]
pub enum Carrier {
    Edge(Restriction),
    Pole(CollapsedBoundary),
}
#[derive(Clone, Copy)]
pub struct Vertex {
    pub source_id: usize,
    /// An enclosure, never an authored point or rounded root.
    pub bounds: [[f64; 2]; 3],
}
pub type Topology = brep_topology::Model<Carrier, Surface, Fragment, Vertex>;
pub struct SourceModel {
    topology: Topology,
    definition: Value,
}
impl SourceModel {
    pub fn topology(&self) -> &Topology {
        &self.topology
    }
    /// Restore this original Body definition through native admission, not this projection.
    pub fn definition(&self) -> &Value {
        &self.definition
    }
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_BODY_TOPOLOGY", message)
}
/// Transfer topology, original root restrictions and directed pcurves unchanged.
/// This does not produce the coordinate-based Model, STEP or a mesh, and does
/// not authorize edited copies of the returned generic topology as geometry.
pub fn convert(
    body: &Body,
    tolerance_mm: f64,
    max_spans: usize,
    max_endpoints: usize,
) -> Result<SourceModel> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(error("Choose a finite positive topology tolerance"));
    }
    let shell = body.geometry().shell();
    let network = crate::source_boundary_network::inspect(shell, max_spans, max_endpoints)?;
    let ids: BTreeMap<_, _> = network
        .vertices
        .iter()
        .enumerate()
        .map(|(i, v)| (v.id, i))
        .collect();
    let vertices = network
        .vertices
        .iter()
        .map(|v| brep_topology::Vertex {
            point: Vertex {
                source_id: v.id,
                bounds: v.bounds,
            },
        })
        .collect();
    let mut uses = BTreeMap::new();
    let mut edges = Vec::with_capacity(shell.edges().len() + shell.poles().len());
    for (index, shared) in shell.edges().iter().enumerate() {
        let addresses = shell.uses()[index];
        let first = addresses[0];
        let mut source_ids = shell.vertices()[first.face][first.wire][first.edge];
        if shared.reversed()[0] {
            source_ids.reverse();
        }
        for (slot, a) in addresses.into_iter().enumerate() {
            let mut other = shell.vertices()[a.face][a.wire][a.edge];
            if shared.reversed()[slot] {
                other.reverse();
            }
            if other != source_ids {
                return Err(error("Conflicting canonical vertex ownership"));
            }
            uses.insert((a.face, a.wire, a.edge), (index, shared.reversed()[slot]));
        }
        edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: source_ids.map(|id| ids[&id]),
            curve: Carrier::Edge(body.edge_restriction(index)?),
        });
    }
    for (a, pole) in shell.poles() {
        let source_ids = shell.vertices()[a.face][a.wire][a.edge];
        if source_ids[0] != source_ids[1] {
            return Err(error("Pole must own one vertex"));
        }
        uses.insert((a.face, a.wire, a.edge), (edges.len(), false));
        edges.push(brep_topology::Edge {
            degenerate: true,
            vertices: source_ids.map(|id| ids[&id]),
            curve: Carrier::Pole(pole.clone()),
        });
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    for (face, wires) in shell.faces().iter().enumerate() {
        let outer = loops.len();
        for (wire, source) in wires.iter().enumerate() {
            let coedges = source
                .edges()
                .iter()
                .enumerate()
                .map(|(edge, fragment)| {
                    let (index, reversed) = uses[&(face, wire, edge)];
                    brep_topology::Coedge {
                        edge: index,
                        reversed,
                        pcurve: fragment.clone(),
                    }
                })
                .collect();
            loops.push(brep_topology::Loop { coedges });
        }
        faces.push(brep_topology::Face {
            surface: wires[0].edges()[0].surface().clone(),
            outer,
            holes: (outer + 1..loops.len()).collect(),
        });
    }
    let face_uses = (0..faces.len())
        .map(|face| brep_topology::FaceUse {
            face,
            reversed: body.reverse_orientation()
                ^ (shell.regions().unwrap()[face].chart_winding() < 0),
        })
        .collect();
    let topology = Topology {
        vertices,
        edges,
        loops,
        faces,
        shells: vec![brep_topology::Shell {
            faces: face_uses,
            closed: true,
        }],
        bodies: vec![brep_topology::Body {
            outer_shell: 0,
            inner_shells: vec![],
        }],
        tolerance_mm,
    };
    topology
        .validate_topology_with_vertices(|v| {
            let Some(&index) = ids.get(&v.source_id) else {
                return Err(brep_topology::Error::new(
                    "BREP_SOURCE_BODY_TOPOLOGY",
                    "Unknown source vertex",
                ));
            };
            if network.vertices[index].bounds != v.bounds {
                return Err(brep_topology::Error::new(
                    "BREP_SOURCE_BODY_TOPOLOGY",
                    "Changed source vertex enclosure",
                ));
            }
            Ok(())
        })
        .map_err(|e| error(&e.message))?;
    Ok(SourceModel {
        topology,
        definition: body.definition()?,
    })
}
