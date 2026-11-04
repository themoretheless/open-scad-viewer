//! BRep storage for exact implicit SS world edges and their paired UV curves.
//! Combinatorial closure and source binding are checked here. This model does
//! not acquire material/embedding authority from storage or a fitted Curve.
use crate::{Error, Result, Surface};
use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct RootVertex {
    pub selector: Arc<ss::OriginalSsRootPoint>,
}
#[derive(Clone, Debug)]
pub struct RootPcurve {
    pub world_edge: usize,
    pub support: usize,
    /// The face-local UV traversal follows the coedge direction.
    pub reversed: bool,
}
pub type RootTopology =
    brep_topology::Model<ss::OriginalSsRootEdge, Surface, RootPcurve, RootVertex>;
#[derive(Clone, Debug)]
pub struct RootBrep {
    topology: RootTopology,
}
fn unproved(message: &str) -> Error {
    Error::new("BREP_ROOT_BINDING_UNPROVED", message)
}
impl RootBrep {
    pub fn topology(&self) -> &RootTopology {
        &self.topology
    }
    pub fn from_topology(topology: RootTopology) -> Result<Self> {
        topology.validate_topology_with_vertices(|v| {
            v.selector.validate_selector().map_err(|_| unproved("BRep vertex leaves its original root selector"))?;
            Ok(())
        })?;
        for edge in &topology.edges {
            if edge.degenerate {
                return Err(unproved(
                    "A collapsed pcurve bridge needs its own original pole authority",
                ));
            }
            for (i, port) in [edge.curve.start(), edge.curve.end()]
                .into_iter()
                .enumerate()
            {
                if !port
                    .point()
                    .same_selector(&topology.vertices[edge.vertices[i]].point.selector)
                {
                    return Err(unproved(
                        "BRep edge endpoint differs from its shared original root vertex",
                    ));
                }
            }
        }
        for face in &topology.faces {
            face.surface.validate()?;
            for id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &topology.loops[id].coedges {
                    if coedge.trim().is_some()
                        || coedge.pcurve.world_edge != coedge.edge
                        || coedge.pcurve.support >= 2
                        || coedge.pcurve.reversed != coedge.reversed
                    {
                        return Err(unproved(
                            "Root pcurve range/direction must be owned by its exact world edge",
                        ));
                    }
                    let root = topology.edges[coedge.edge].curve.curve();
                    let surface = if coedge.pcurve.support == 0 {
                        root.source().first()
                    } else {
                        root.source().second()
                    };
                    if &face.surface != surface {
                        return Err(unproved(
                            "Root pcurve projection belongs to another original face surface",
                        ));
                    }
                }
            }
        }
        Ok(Self { topology })
    }
    /// Candidate two-sided finite source patch topology. Both surface loops
    /// project the SAME root edges; face reversal establishes opposite shell
    /// incidence. Interior/holes/Jordan/material checks remain independent.
    pub fn paired_contours(
        outer: &ss::OriginalSsRootWire,
        holes: &[ss::OriginalSsRootWire],
        tolerance_mm: f64,
    ) -> Result<Self> {
        if holes.len() > 64 {
            return Err(unproved("Root face hole allowance exhausted"));
        }
        let source = outer.edges()[0].curve().source();
        let mut topology = RootTopology {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm,
        };
        let mut boundaries = Vec::new();
        for wire in std::iter::once(outer).chain(holes) {
            let mut ids = Vec::new();
            for edge in wire.edges() {
                let s = edge.curve().source();
                if source.first() != s.first() || source.second() != s.second() {
                    return Err(unproved(
                        "Paired contours must belong to the same ordered original carriers",
                    ));
                }
                let mut vertices = [0; 2];
                for (i, port) in [edge.start(), edge.end()].into_iter().enumerate() {
                    let point = port.point();
                    vertices[i] = if let Some(id) = topology
                        .vertices
                        .iter()
                        .position(|v| v.point.selector.same_selector(point))
                    {
                        id
                    } else {
                        let id = topology.vertices.len();
                        topology.vertices.push(brep_topology::Vertex {
                            point: RootVertex {
                                selector: point.clone(),
                            },
                        });
                        id
                    };
                }
                let id = topology.edges.len();
                topology.edges.push(brep_topology::Edge {
                    degenerate: false,
                    vertices,
                    curve: edge.clone(),
                });
                ids.push(id);
            }
            boundaries.push(ids);
        }
        for support in 0..2 {
            let mut loops = Vec::new();
            for ids in &boundaries {
                let id = topology.loops.len();
                loops.push(id);
                topology.loops.push(brep_topology::Loop {
                    coedges: ids
                        .iter()
                        .map(|&edge| {
                            brep_topology::Coedge::new(
                                edge,
                                false,
                                RootPcurve {
                                    world_edge: edge,
                                    support,
                                    reversed: false,
                                },
                            )
                        })
                        .collect(),
                });
            }
            topology.faces.push(brep_topology::Face {
                surface: if support == 0 {
                    source.first().clone()
                } else {
                    source.second().clone()
                },
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
        }
        topology.shells.push(brep_topology::Shell {
            faces: vec![
                brep_topology::FaceUse {
                    face: 0,
                    reversed: false,
                },
                brep_topology::FaceUse {
                    face: 1,
                    reversed: true,
                },
            ],
            closed: true,
        });
        topology.bodies.push(brep_topology::Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        Self::from_topology(topology)
    }
    /// Replay all opaque original roots, endpoint bindings and incidence. This
    /// still proves neither root-image embedding nor valid retained material.
    pub fn reverify(&self) -> Result<()> {
        for edge in &self.topology.edges {
            edge.curve.reverify()?;
        }
        Self::from_topology(self.topology.clone()).map(|_| ())
    }
    pub fn pcurve_bounds(
        &self,
        face: usize,
        loop_in_face: usize,
        coedge: usize,
        limits: ss::OriginalSsCurveLimits,
    ) -> Result<[[f64; 2]; 2]> {
        let face = self
            .topology
            .faces
            .get(face)
            .ok_or_else(|| unproved("Root pcurve face is absent"))?;
        let loops = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        let id = *loops
            .get(loop_in_face)
            .ok_or_else(|| unproved("Root pcurve loop is absent"))?;
        let coedge = self.topology.loops[id]
            .coedges
            .get(coedge)
            .ok_or_else(|| unproved("Root pcurve use is absent"))?;
        let bounds = self.topology.edges[coedge.edge].curve.enclose(limits)?;
        Ok([
            bounds.uv[2 * coedge.pcurve.support],
            bounds.uv[2 * coedge.pcurve.support + 1],
        ])
    }
}

mod partition;
pub use partition::{RootEdgePartitionRequest,RootBrepPartitionLimits,QualifiedRootBrepEdgeSubdivision};

mod uv_closure;
pub use uv_closure::{QualifiedRootUvClosure,qualify_root_uv_joint};

mod face_contours;
pub use face_contours::RootFaceContour;

mod winding;
pub use winding::QualifiedRootLoopWinding;

mod face_split;
pub use face_split::RootFaceChordSplit;

mod hole_assignment;
pub use hole_assignment::{RootHoleAssignmentLimits,QualifiedRootHoleAssignment};

mod chord_boundary;
pub use chord_boundary::{RootChordBoundaryLimits,QualifiedRootChordBoundary};
