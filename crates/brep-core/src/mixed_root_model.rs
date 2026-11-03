//! Exact mixed rational and implicit SS BRep payloads. This storage never
//! converts an implicit SS edge to fitted poles or grants solid authority.
use crate::{Error, Result, Surface};
use nurbs_core::curve::Curve;
use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub enum MixedVertex {
    Rational([f64; 3]),
    Root(Arc<ss::OriginalSsRootPoint>),
}
#[derive(Clone, Debug)]
pub enum MixedEdge {
    Rational(Curve),
    Root(ss::OriginalSsRootEdge),
}
#[derive(Clone, Debug)]
pub enum MixedPcurve {
    Rational(Curve),
    Root(crate::root_model::RootPcurve),
}
pub type MixedTopology = brep_topology::Model<MixedEdge, Surface, MixedPcurve, MixedVertex>;
#[derive(Clone, Debug)]
pub struct MixedBrep {
    topology: MixedTopology,
}
fn unproved(s: &str) -> Error {
    Error::new("BREP_MIXED_ROOT_UNPROVED", s)
}
impl MixedBrep {
    pub fn topology(&self) -> &MixedTopology {
        &self.topology
    }
    pub fn from_closed_solid(solid: &crate::closed_solid::QualifiedClosedSolid) -> Result<Self> {
        Self::from_legacy(solid.model())
    }
    pub(crate) fn from_legacy(model: &crate::Model) -> Result<Self> {
        model.validate()?;
        let mut loops = Vec::new();
        for wire in &model.loops {
            let mut coedges = Vec::new();
            for c in &wire.coedges {
                let mut new = brep_topology::Coedge::new(
                    c.edge,
                    c.reversed,
                    MixedPcurve::Rational(c.pcurve.clone()),
                );
                if let Some(trim) = c.trim() {
                    new = new.with_trim(trim)?;
                }
                coedges.push(new);
            }
            loops.push(brep_topology::Loop { coedges });
        }
        Self::from_topology(MixedTopology {
            vertices: model
                .vertices
                .iter()
                .map(|v| brep_topology::Vertex {
                    point: MixedVertex::Rational(v.point),
                })
                .collect(),
            edges: model
                .edges
                .iter()
                .map(|e| brep_topology::Edge {
                    degenerate: e.degenerate,
                    vertices: e.vertices,
                    curve: MixedEdge::Rational(e.curve.clone()),
                })
                .collect(),
            loops,
            faces: model.faces.clone(),
            shells: model.shells.clone(),
            bodies: model.bodies.clone(),
            tolerance_mm: model.tolerance_mm,
        })
    }
    pub fn from_topology(topology: MixedTopology) -> Result<Self> {
        topology.validate_topology_with_vertices(|v| match v {
            MixedVertex::Rational(p) if !p.iter().all(|x| x.is_finite()) => {
                Err(unproved("Nonfinite original vertex"))
            }
            MixedVertex::Root(p) if !p.parameter().is_finite() => {
                Err(unproved("Nonfinite root selector"))
            }
            _ => Ok(()),
        })?;
        for e in &topology.edges {
            match &e.curve {
                MixedEdge::Rational(c) => {
                    c.validate()?;
                    if c.control_points.iter().any(|p| p.len() != 3) {
                        return Err(unproved("Rational world edge must be spatial"));
                    }
                }
                MixedEdge::Root(root) => {
                    if e.degenerate {
                        return Err(unproved("Root edge has no collapsed-bridge authority"));
                    }
                    for (i, port) in [root.start(), root.end()].into_iter().enumerate() {
                        match &topology.vertices[e.vertices[i]].point {
                            MixedVertex::Root(p) if port.point().same_selector(p) => (),
                            _ => {
                                return Err(unproved(
                                    "Root edge is not bound to its exact shared vertex",
                                ));
                            }
                        }
                    }
                }
            }
        }
        for face in &topology.faces {
            face.surface.validate()?;
            if face
                .surface
                .control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3)
            {
                return Err(unproved("Mixed BRep face must be spatial"));
            }
            for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for c in &topology.loops[l].coedges {
                    match (&topology.edges[c.edge].curve, &c.pcurve) {
                        (MixedEdge::Rational(_), MixedPcurve::Rational(p)) => {
                            p.validate()?;
                            if p.control_points.iter().any(|p| p.len() != 2) {
                                return Err(unproved(
                                    "Rational pcurve must have two UV coordinates",
                                ));
                            }
                        }
                        (MixedEdge::Root(e), MixedPcurve::Root(p)) => {
                            if c.trim().is_some()
                                || p.world_edge != c.edge
                                || p.reversed != c.reversed
                                || p.support >= 2
                            {
                                return Err(unproved(
                                    "Implicit pcurve has foreign edge/range/direction",
                                ));
                            }
                            let source = e.curve().source();
                            let surface = if p.support == 0 {
                                source.first()
                            } else {
                                source.second()
                            };
                            if surface != &face.surface {
                                return Err(unproved(
                                    "Implicit pcurve belongs to another original surface",
                                ));
                            }
                        }
                        _ => {
                            return Err(unproved(
                                "Rational and root-valued edge projections cannot be interchanged",
                            ));
                        }
                    }
                }
            }
        }
        Ok(Self { topology })
    }
}
mod closed_cut;
pub use closed_cut::{ClosedCutCandidate, ClosedCutLimits, split_closed_contour_candidate};
