//! Exact source-owned transition faces on the existing generic B-rep topology.
//! Open graph strip only; retained source faces, caps and solid admission are separate.
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use nurbs_core::{
    Error, Result,
    original_graph_transition::{
        BoundaryDefinition, Budget, CurvatureDecision, CurvatureReport, JoinReport, Patch,
    },
    surface_join::Boundary,
};
use std::sync::Arc;
struct RootCurve {
    id: usize,
    points: Vec<[Q; 3]>,
}
#[derive(Clone)]
pub struct Edge {
    points: Vec<[Q; 3]>,
    root: Arc<RootCurve>,
    range: [Q; 2],
}
impl Edge {
    fn original(points: Vec<[Q; 3]>, id: usize) -> Self {
        Self {
            root: Arc::new(RootCurve {
                id,
                points: points.clone(),
            }),
            points,
            range: [Q::zero(), Q::one()],
        }
    }
    pub fn root_id(&self) -> usize {
        self.root.id
    }
    pub fn root_parameters(&self) -> &[Q; 2] {
        &self.range
    }
    pub fn root_points(&self) -> &[[Q; 3]] {
        &self.root.points
    }
    pub fn points(&self) -> &[[Q; 3]] {
        &self.points
    }
}
#[derive(Clone)]
pub struct ParameterLine {
    points: [[Q; 2]; 2],
}
impl ParameterLine {
    pub fn points(&self) -> &[[Q; 2]; 2] {
        &self.points
    }
}
pub type Topology = brep_topology::Model<Edge, Arc<Patch>, ParameterLine, [Q; 3]>;
pub struct GeometryReport {
    pub boundary_uses: usize,
    pub coefficient_comparisons: usize,
    pub open_edges: Vec<usize>,
}
pub struct Strip {
    topology: Topology,
    joins: Vec<JoinReport>,
    fields: Vec<CurvatureReport>,
    geometry: GeometryReport,
}
impl Strip {
    pub fn topology(&self) -> &Topology {
        &self.topology
    }
    pub fn joins(&self) -> &[JoinReport] {
        &self.joins
    }
    pub fn fields(&self) -> &[CurvatureReport] {
        &self.fields
    }
    pub fn geometry(&self) -> &GeometryReport {
        &self.geometry
    }
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_EXACT_GRAPH_STRIP", message))
    }
}
fn line(data: &BoundaryDefinition, reversed: bool) -> ParameterLine {
    let mut points = data.uv.clone();
    if reversed {
        points.reverse();
    }
    ParameterLine { points }
}
/// Local full-boundary geometry and incidence only. Mutable clones never inherit
/// the Strip owner. Closed-body/global external-face validity is not implied.
pub fn inspect_geometry(model: &Topology, budget: &mut Budget) -> Result<GeometryReport> {
    model.validate_topology_with_vertices(|p| {
        require(
            p.iter().all(|q| {
                q.denom().is_positive() && q.numer().bits() <= 8192 && q.denom().bits() <= 8192
            }),
            "Vertex exceeds exact graph bit budget",
        )
    })?;
    let mut report = GeometryReport {
        boundary_uses: 0,
        coefficient_comparisons: 0,
        open_edges: Vec::new(),
    };
    let mut uses = vec![0; model.edges.len()];
    for edge in &model.edges {
        let definition = BoundaryDefinition {
            points: edge.curve.root.points.clone(),
            uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
        };
        let restricted = definition.restricted(edge.curve.range.clone(), budget)?;
        require(
            restricted.points == edge.curve.points,
            "Saved edge lost its original root restriction",
        )?;
        require(
            !edge.degenerate && (2..=33).contains(&edge.curve.points.len()),
            "Unsupported exact graph edge",
        )?;
        require(
            edge.curve.points.first().unwrap() == &model.vertices[edge.vertices[0]].point
                && edge.curve.points.last().unwrap() == &model.vertices[edge.vertices[1]].point,
            "Exact edge endpoints do not own their vertices",
        )?;
    }
    for face in &model.faces {
        require(
            face.holes.is_empty(),
            "Graph strip faces cannot inherit unchecked holes",
        )?;
        for c in &model.loops[face.outer].coedges {
            require(
                c.trim().is_none(),
                "Partial graph coedge clocks require separate qualification",
            )?;
            let mut matched = false;
            for side in [
                Boundary::UMin,
                Boundary::UMax,
                Boundary::VMin,
                Boundary::VMax,
            ] {
                let mut data = face.surface.boundary_definition(side, budget)?;
                let free = usize::from(data.uv[0][0] == data.uv[1][0]);
                let fixed = 1 - free;
                if c.pcurve
                    .points
                    .iter()
                    .any(|p| p[fixed] != data.uv[0][fixed])
                {
                    continue;
                }
                let width = &data.uv[1][free] - &data.uv[0][free];
                let parameters = c
                    .pcurve
                    .points
                    .each_ref()
                    .map(|p| (&p[free] - &data.uv[0][free]) / &width);
                if parameters[0] == parameters[1]
                    || parameters.iter().any(|q| q < &Q::zero() || q > &Q::one())
                {
                    continue;
                }
                let forward = parameters[0] < parameters[1];
                let range = if forward {
                    parameters
                } else {
                    [parameters[1].clone(), parameters[0].clone()]
                };
                data = data.restricted(range, budget)?;
                if !forward {
                    data.points.reverse();
                }
                let edge = &model.edges[c.edge].curve;
                require(
                    edge.points.len() == data.points.len(),
                    "Edge and surface boundary degrees differ",
                )?;
                for (i, p) in data.points.iter().enumerate() {
                    let q = &edge.points[if c.reversed {
                        edge.points.len() - 1 - i
                    } else {
                        i
                    }];
                    report.coefficient_comparisons += 3;
                    require(
                        p == q,
                        "Saved canonical edge does not match its exact face lift",
                    )?;
                }
                matched = true;
                break;
            }
            require(
                matched,
                "Saved parameter line is not an exact original patch-boundary restriction",
            )?;
            report.boundary_uses += 1;
            uses[c.edge] += 1;
        }
    }
    report.open_edges = uses
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| (n == 1).then_some(i))
        .collect();
    Ok(report)
}
/// Fresh source-contact, internal G2, whole-panel 10/mm curvature and exact
/// canonical-boundary checks before publishing an immutable OPEN strip owner.
/// Sharing uses construction identity, never nearest-position edge welding.
pub fn assemble(panels: Vec<Patch>, tolerance_mm: f64, budget: &mut Budget) -> Result<Strip> {
    require(
        !panels.is_empty() && panels.len() <= 64,
        "Choose 1..64 exact transition panels",
    )?;
    require(
        tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&tolerance_mm),
        "Choose a supported B-rep tolerance",
    )?;
    let mut joins = Vec::new();
    let mut fields = Vec::new();
    for p in &panels {
        require(
            p.contact_jets() == [true; 2],
            "Source contact two-jets are unproven",
        )?;
        let field = p.inspect_curvature(10., 100_000, budget)?;
        require(
            field.decision == CurvatureDecision::Within,
            "Exact panel field does not pass 10/mm curvature",
        )?;
        fields.push(field);
    }
    for pair in panels.windows(2) {
        let join = pair[0].inspect_join_g2(&pair[1], 1e-5, 1e-5, budget)?;
        require(
            join.within && join.position_exact,
            "Exact transition internal G2 join is unproven",
        )?;
        joins.push(join);
    }
    let n = panels.len();
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut stations = Vec::new();
    stations.push(panels[0].boundary_definition(Boundary::UMin, budget)?);
    for p in &panels {
        stations.push(p.boundary_definition(Boundary::UMax, budget)?);
    }
    for station in &stations {
        let first = vertices.len();
        vertices.push(brep_topology::Vertex {
            point: station.points[0].clone(),
        });
        vertices.push(brep_topology::Vertex {
            point: station.points.last().unwrap().clone(),
        });
        edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [first, first + 1],
            curve: Edge::original(station.points.clone(), edges.len()),
        });
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    for (i, p) in panels.into_iter().enumerate() {
        let bottom = p.boundary_definition(Boundary::VMin, budget)?;
        let top = p.boundary_definition(Boundary::VMax, budget)?;
        let left = p.boundary_definition(Boundary::UMin, budget)?;
        let right = p.boundary_definition(Boundary::UMax, budget)?;
        let bottom_id = edges.len();
        edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [2 * i, 2 * i + 2],
            curve: Edge::original(bottom.points.clone(), edges.len()),
        });
        let top_id = edges.len();
        edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [2 * i + 1, 2 * i + 3],
            curve: Edge::original(top.points.clone(), edges.len()),
        });
        loops.push(brep_topology::Loop {
            coedges: vec![
                brep_topology::Coedge::new(bottom_id, false, line(&bottom, false)),
                brep_topology::Coedge::new(i + 1, false, line(&right, false)),
                brep_topology::Coedge::new(top_id, true, line(&top, true)),
                brep_topology::Coedge::new(i, true, line(&left, true)),
            ],
        });
        faces.push(brep_topology::Face {
            surface: Arc::new(p),
            outer: i,
            holes: vec![],
        });
    }
    let topology = Topology {
        vertices,
        edges,
        loops,
        faces,
        shells: vec![brep_topology::Shell {
            closed: false,
            faces: (0..n)
                .map(|face| brep_topology::FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
        }],
        bodies: vec![],
        tolerance_mm,
    };
    let geometry = inspect_geometry(&topology, budget)?;
    require(
        geometry.open_edges.len() == 2 * n + 2,
        "Unexpected open transition boundary",
    )?;
    Ok(Strip {
        topology,
        joins,
        fields,
        geometry,
    })
}

#[derive(Debug)]
pub struct SplitReport {
    pub left_edge: usize,
    pub right_edge: usize,
    pub vertex: usize,
    pub root_id: usize,
    pub root_parameter: Q,
    pub face_uses: usize,
}
impl Strip {
    /// Transactional canonical split. Both neighboring uses inherit the same
    /// exact root endpoint. Surfaces and their existing field/G2 owners remain
    /// unchanged; actual saved root/lift/vertex geometry is freshly rechecked.
    pub fn split_edge(
        &mut self,
        edge: usize,
        parameter: Q,
        budget: &mut Budget,
    ) -> Result<SplitReport> {
        require(
            edge < self.topology.edges.len()
                && parameter.denom().is_positive()
                && parameter > Q::zero()
                && parameter < Q::one()
                && parameter.numer().bits() <= 8192
                && parameter.denom().bits() <= 8192,
            "Choose an interior exact edge parameter",
        )?;
        let old = &self.topology.edges[edge];
        let root_parameter =
            &old.curve.range[0] + (&old.curve.range[1] - &old.curve.range[0]) * &parameter;
        let def = BoundaryDefinition {
            points: old.curve.points.clone(),
            uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
        };
        let left = def.restricted([Q::zero(), parameter.clone()], budget)?;
        let right = def.restricted([parameter.clone(), Q::one()], budget)?;
        require(
            left.points.last() == right.points.first(),
            "Split endpoint differs between child restrictions",
        )?;
        let mut candidate = self.topology.clone();
        let vertex = candidate.vertices.len();
        let right_edge = candidate.edges.len();
        candidate.vertices.push(brep_topology::Vertex {
            point: left.points.last().unwrap().clone(),
        });
        candidate.edges[edge] = brep_topology::Edge {
            degenerate: false,
            vertices: [old.vertices[0], vertex],
            curve: Edge {
                points: left.points,
                root: old.curve.root.clone(),
                range: [old.curve.range[0].clone(), root_parameter.clone()],
            },
        };
        candidate.edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [vertex, old.vertices[1]],
            curve: Edge {
                points: right.points,
                root: old.curve.root.clone(),
                range: [root_parameter.clone(), old.curve.range[1].clone()],
            },
        });
        let mut face_uses = 0;
        for wire in &mut candidate.loops {
            let mut out = Vec::new();
            for c in &wire.coedges {
                if c.edge != edge {
                    out.push(c.clone());
                    continue;
                }
                require(
                    c.trim().is_none(),
                    "Cannot split an unchecked authored coedge trim",
                )?;
                let mut uv = c.pcurve.points.clone();
                if c.reversed {
                    uv.reverse();
                }
                let def = BoundaryDefinition {
                    points: old.curve.points.clone(),
                    uv,
                };
                let left = def.restricted([Q::zero(), parameter.clone()], budget)?;
                let right = def.restricted([parameter.clone(), Q::one()], budget)?;
                let a = brep_topology::Coedge::new(edge, c.reversed, line(&left, c.reversed));
                let b =
                    brep_topology::Coedge::new(right_edge, c.reversed, line(&right, c.reversed));
                if c.reversed {
                    out.extend([b, a]);
                } else {
                    out.extend([a, b]);
                }
                face_uses += 1;
            }
            wire.coedges = out;
        }
        require(
            (1..=2).contains(&face_uses),
            "Split edge must own one or two saved face uses",
        )?;
        let geometry = inspect_geometry(&candidate, budget)?;
        let report = SplitReport {
            left_edge: edge,
            right_edge,
            vertex,
            root_id: old.curve.root.id,
            root_parameter,
            face_uses,
        };
        self.topology = candidate;
        self.geometry = geometry;
        Ok(report)
    }
}

/// Exact source-chart lift bound to the saved canonical edge and actual panel.
/// Recheck `is_current` after edits; this is not an admitted source face.
pub struct SourceBinding {
    edge: usize,
    face: usize,
    side: usize,
    root: Arc<RootCurve>,
    panel: Arc<Patch>,
    contact: nurbs_core::original_graph_transition::SourceContact,
}
impl SourceBinding {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn transition_face(&self) -> usize {
        self.face
    }
    pub fn source_side(&self) -> usize {
        self.side
    }
    pub fn contact(&self) -> &nurbs_core::original_graph_transition::SourceContact {
        &self.contact
    }
    pub fn is_current(&self, strip: &Strip) -> bool {
        let Some(edge) = strip.topology.edges.get(self.edge) else {
            return false;
        };
        let Some(face) = strip.topology.faces.get(self.face) else {
            return false;
        };
        Arc::ptr_eq(&edge.curve.root, &self.root)
            && Arc::ptr_eq(&face.surface, &self.panel)
            && edge.curve.range == *self.contact.root_parameters()
            && edge.curve.points == self.contact.points()
            && self.contact.source() == face.surface.sources()[self.side]
    }
}
impl Strip {
    /// Produce source-UV curves for the ACTUAL stored longitudinal perimeter,
    /// including rational child intervals. Every complete lift is freshly
    /// checked before the collection is returned; no float pcurve is fitted.
    pub fn source_contacts(&self, budget: &mut Budget) -> Result<Vec<SourceBinding>> {
        let mut out = Vec::new();
        for (face_index, face) in self.topology.faces.iter().enumerate() {
            for c in &self.topology.loops[face.outer].coedges {
                let uv = &c.pcurve.points;
                if uv[0][1] != uv[1][1] {
                    continue;
                }
                let side = if uv[0][1] == Q::zero() {
                    0
                } else if uv[0][1] == Q::one() {
                    1
                } else {
                    continue;
                };
                require(
                    self.geometry.open_edges.contains(&c.edge),
                    "Source contact must own an open perimeter edge",
                )?;
                let domain = face.surface.width().domain();
                let a = Q::from_float(domain[0]).unwrap();
                let b = Q::from_float(domain[1]).unwrap();
                let mut range = uv.each_ref().map(|p| (&p[0] - &a) / (&b - &a));
                if c.reversed {
                    range.reverse();
                }
                let full = face.surface.source_contact(side, budget)?;
                let contact = full.restricted(range, budget)?;
                let edge = &self.topology.edges[c.edge].curve;
                require(
                    contact.points() == edge.points && contact.root_parameters() == &edge.range,
                    "Actual saved edge is not the exact source contact restriction",
                )?;
                out.push(SourceBinding {
                    edge: c.edge,
                    face: face_index,
                    side,
                    root: edge.root.clone(),
                    panel: face.surface.clone(),
                    contact,
                });
            }
        }
        Ok(out)
    }
}

pub struct SourceEndBinding {
    edge: usize,
    face: usize,
    root: Arc<RootCurve>,
    panel: Arc<Patch>,
    contact: nurbs_core::original_graph_transition::SourceEndContact,
}
impl SourceEndBinding {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn transition_face(&self) -> usize {
        self.face
    }
    pub fn contact(&self) -> &nurbs_core::original_graph_transition::SourceEndContact {
        &self.contact
    }
    pub fn is_current(&self, strip: &Strip) -> bool {
        let Some(edge) = strip.topology.edges.get(self.edge) else {
            return false;
        };
        let Some(face) = strip.topology.faces.get(self.face) else {
            return false;
        };
        (if self.contact.at_end() {
            self.face + 1 == strip.topology.faces.len()
        } else {
            self.face == 0
        }) && Arc::ptr_eq(&edge.curve.root, &self.root)
            && Arc::ptr_eq(&face.surface, &self.panel)
            && edge.curve.range == *self.contact.root_parameters()
            && edge.curve.points == self.contact.points()
            && self.contact.source() == face.surface.sources()[self.contact.side()]
    }
}
pub struct EndSplitProposal {
    edge: usize,
    face: usize,
    at_end: bool,
    root: Arc<RootCurve>,
    panel: Arc<Patch>,
    range: [Q; 2],
    parameter: Q,
}
impl EndSplitProposal {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn parameter(&self) -> &Q {
        &self.parameter
    }
    pub fn is_current(&self, strip: &Strip) -> bool {
        let Some(edge) = strip.topology.edges.get(self.edge) else {
            return false;
        };
        let Some(face) = strip.topology.faces.get(self.face) else {
            return false;
        };
        (if self.at_end {
            self.face + 1 == strip.topology.faces.len()
        } else {
            self.face == 0
        }) && Arc::ptr_eq(&edge.curve.root, &self.root)
            && Arc::ptr_eq(&face.surface, &self.panel)
            && edge.curve.range == self.range
    }
}
impl Strip {
    pub fn apply_source_end_split(
        &mut self,
        proposal: &EndSplitProposal,
        budget: &mut Budget,
    ) -> Result<SplitReport> {
        require(
            proposal.is_current(self),
            "Source end split proposal is stale",
        )?;
        self.split_edge(proposal.edge, proposal.parameter.clone(), budget)
    }
    fn end_uses(&self) -> Result<Vec<(usize, usize, bool, [Q; 2])>> {
        let start = Q::from_float(self.topology.faces[0].surface.width().domain()[0]).unwrap();
        let end =
            Q::from_float(self.topology.faces.last().unwrap().surface.width().domain()[1]).unwrap();
        let mut out = Vec::new();
        for (face_index, face) in self.topology.faces.iter().enumerate() {
            for c in &self.topology.loops[face.outer].coedges {
                let uv = &c.pcurve.points;
                if uv[0][0] != uv[1][0] || !self.geometry.open_edges.contains(&c.edge) {
                    continue;
                }
                let at_end = if uv[0][0] == start {
                    false
                } else if uv[0][0] == end {
                    true
                } else {
                    continue;
                };
                let mut range = [uv[0][1].clone(), uv[1][1].clone()];
                if c.reversed {
                    range.reverse();
                }
                require(
                    range == self.topology.edges[c.edge].curve.range,
                    "End source use lost the original canonical V clock",
                )?;
                out.push((c.edge, face_index, at_end, range));
            }
        }
        Ok(out)
    }
    /// Canonical local split proposals at the original source crease V=1/2.
    /// No topology is changed by planning. Apply through transactional split_edge.
    pub fn source_end_splits(&self) -> Result<Vec<EndSplitProposal>> {
        let half = Q::new(1.into(), 2.into());
        let mut out = Vec::new();
        for (edge, face, at_end, r) in self.end_uses()? {
            if r[0] < half && half < r[1] {
                let parameter = (&half - &r[0]) / (&r[1] - &r[0]);
                out.push(EndSplitProposal {
                    edge,
                    face,
                    at_end,
                    root: self.topology.edges[edge].curve.root.clone(),
                    panel: self.topology.faces[face].surface.clone(),
                    range: r,
                    parameter,
                });
            }
        }
        Ok(out)
    }
    /// Every saved end segment must remain within one original source chart.
    /// Returns no partial collection when a crease split or exact G2 is missing.
    pub fn source_end_contacts(&self, budget: &mut Budget) -> Result<Vec<SourceEndBinding>> {
        let half = Q::new(1.into(), 2.into());
        let mut out = Vec::new();
        for (edge_index, face_index, at_end, range) in self.end_uses()? {
            let side = if range[1] <= half {
                0
            } else if range[0] >= half {
                1
            } else {
                return Err(Error::new(
                    "BREP_EXACT_GRAPH_END_SPLIT",
                    "Split the end edge at the original source crease before binding source faces",
                ));
            };
            let face = &self.topology.faces[face_index];
            let contact = face
                .surface
                .source_end_contact(at_end, side, range, budget)?;
            let edge = &self.topology.edges[edge_index].curve;
            require(
                contact.points() == edge.points && contact.root_parameters() == &edge.range,
                "Actual end edge is not the exact original source lift",
            )?;
            out.push(SourceEndBinding {
                edge: edge_index,
                face: face_index,
                root: edge.root.clone(),
                panel: face.surface.clone(),
                contact,
            });
        }
        Ok(out)
    }
}

#[path = "exact_graph_strip/retained.rs"]
mod retained;
pub use retained::{RetainedRegion, RetainedUse};

#[path = "exact_graph_strip/roof.rs"]
mod roof;
pub use roof::{FaceGeometry, FaceUseGeometry, Roof, RoofTopology, SourceFieldReport, inspect_roof_geometry, inspect_source_support_field};

#[path = "exact_graph_strip/original_boundary.rs"]
mod original_boundary;
pub use original_boundary::{OriginalBoundaryMap, OriginalEdgeLink, OriginalEndpointLink};

#[path = "exact_graph_strip/original_body.rs"]
mod original_body;
pub use original_body::{ClosedShell, ImportedFace};

#[path = "exact_graph_strip/signed_flux.rs"]
mod signed_flux;
pub use signed_flux::SignedFlux;

#[path = "exact_graph_strip/pair_hulls.rs"]
mod pair_hulls;
pub use pair_hulls::{PairHullAudit,PairHullStatus,FacePairHull};

#[path = "exact_graph_strip/transverse_pairs.rs"]
mod transverse_pairs;
pub use transverse_pairs::{TransversePairAudit,TransversePairContact};

#[path = "exact_graph_strip/face_charts.rs"]
mod face_charts;
pub use face_charts::FaceChartAudit;

#[path = "exact_graph_strip/retained_pairs.rs"]
mod retained_pairs;
pub use retained_pairs::RetainedPairAudit;

#[path = "exact_graph_strip/original_pairs.rs"]
mod original_pairs;
pub use original_pairs::OriginalPairAudit;

#[path = "exact_graph_strip/volume.rs"]
mod volume;
pub use volume::{VolumeBounds,VolumeQualification,NativeVolume};

#[path = "exact_graph_strip/normal_radius.rs"]
mod normal_radius;
pub use normal_radius::TransitionNormalRadiusAudit;

#[path = "exact_graph_strip/source_law.rs"]
mod source_law;
pub use source_law::TransitionSourceLawAudit;

#[path = "exact_graph_strip/wall_coverage.rs"]
mod wall_coverage;
pub use wall_coverage::{AlignedWallCoverage,AlignedWallPair,AlignedWallPairDecision};

#[path = "exact_graph_strip/wall_witness.rs"]
mod wall_witness;
pub use wall_witness::AxisMaterialChord;

#[cfg(feature="codec")]
#[path="exact_graph_strip/transport.rs"]
pub mod transport;
