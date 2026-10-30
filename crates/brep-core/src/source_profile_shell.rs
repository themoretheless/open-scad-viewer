//! Closed oriented incidence for exact profile/transition expression faces.
//! Global embedding, volume, radius and wall admission remain separate.
#[path = "source_profile_shell_tangency.rs"]
mod tangency;
pub use tangency::{TransitionNeighborTangency,TransitionNeighborTangencyReport};
#[path = "source_profile_shell_geometry.rs"]
mod geometry;
pub use geometry::EmbeddedExpressionGeometry;
#[path = "source_profile_shell_flux.rs"]
mod flux;
pub use flux::MaterialFluxReport;
#[path = "source_profile_shell_gap.rs"]
mod gap;
pub use gap::{ExpressionFaceClearance, ExpressionGapReport};
#[path = "source_profile_shell_segment.rs"]
mod segment;
pub use segment::{
    ExpressionEndpointNormalReport, ExpressionLineReport, ExpressionMaterialInterval,
    ExpressionMaterialLine, ExpressionMaterialLineQualification, ExpressionMaterialLineReport,
    ExpressionSegmentCandidate, ExpressionSegmentReport, ExpressionTrimmedLineReport,
};
#[path = "source_profile_shell_volume.rs"]
mod volume;
pub use volume::{
    ExpressionVolumeLimits, ExpressionVolumeQualification, ExpressionVolumeReport,
    OrientedExpressionVolume,
};
#[path = "source_profile_shell_wall.rs"]
mod wall;
use super::*;
use crate::source_boundary_fragment::Fragment;
use crate::source_shell_incidence::Address;
use std::collections::{BTreeMap, BTreeSet};
pub use wall::{
    ExpressionAlignedChordMinimum, ExpressionWallCell, ExpressionWallPendingReason,
    ExpressionWallQualification, ExpressionWallReport,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellFaceKind {
    Original(usize),
    Retained(usize),
    Transition,
}
pub enum ShellUse {
    Source(Fragment),
    Transition(usize),
}
pub struct ShellFace {
    kind: ShellFaceKind,
    loops: Vec<Vec<ShellUse>>,
}
impl ShellFace {
    pub fn kind(&self) -> ShellFaceKind {
        self.kind
    }
    pub fn loops(&self) -> &[Vec<ShellUse>] {
        &self.loops
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPairKind {
    Original(usize),
    Outer(usize),
    Rail(usize),
    Section(usize),
}
pub struct ShellPair {
    uses: [Address; 2],
    reversed: [bool; 2],
    kind: ShellPairKind,
}
impl ShellPair {
    pub fn uses(&self) -> [Address; 2] {
        self.uses
    }
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
    pub fn kind(&self) -> ShellPairKind {
        self.kind
    }
}
pub struct ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    assembly: &'x OuterEdgeAssembly<'o, 'i, 'f, 'a, 's, 'p, 'b>,
    faces: Vec<ShellFace>,
    pairs: Vec<ShellPair>,
    poles: Vec<Address>,
    vertices: usize,
    vertex_ids: BTreeMap<(usize, usize, usize), usize>,
    components: usize,
}
fn original_support_derivatives(
    source: &nurbs_core::surface::Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
    source.validate()?;
    let natural = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    if max_spans > 100000
        || (0..2).any(|k| {
            !domain[k].iter().all(|v| v.is_finite())
                || domain[k][0] > domain[k][1]
                || domain[k][0] < natural[k][0]
                || domain[k][1] > natural[k][1]
        })
    {
        return Err(Error::new(
            "BREP_PROFILE_SHELL",
            "Keep original derivatives in the natural domain and bounded work",
        ));
    }
    let mut out = [[[0.; 2]; 3]; 2];
    let mut spent = 0;
    for axis in 0..2 {
        if spent == max_spans {
            return Ok((None, spent));
        }
        let (bounds, used) = nurbs_core::normal_alignment::derivative_path_bounds(
            source,
            domain,
            axis,
            max_spans - spent,
        )?;
        spent += used;
        let Some(bounds) = bounds else {
            return Ok((None, spent));
        };
        out[axis] = bounds.components();
    }
    Ok((Some(out), spent))
}

pub struct ShellAssemblyReport<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub shell: Option<ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
    pub uses: usize,
    pub reason: &'static str,
}
fn key(a: Address) -> (usize, usize, usize) {
    (a.face, a.wire, a.edge)
}
fn locate(faces: &[ShellFace], ids: &[usize], fragment: &Fragment) -> Option<Address> {
    let mut found = None;
    for &face in ids {
        for (wire, loop_) in faces[face].loops.iter().enumerate() {
            for (edge, use_) in loop_.iter().enumerate() {
                if matches!(use_,ShellUse::Source(f) if f.same_definition(fragment)) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(Address { face, wire, edge });
                }
            }
        }
    }
    found
}
fn component_count(n: usize, pairs: &[[Address; 2]]) -> usize {
    let mut seen = vec![false; n];
    let mut count = 0;
    for start in 0..n {
        if seen[start] {
            continue;
        }
        count += 1;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(face) = stack.pop() {
            for pair in pairs {
                for side in 0..2 {
                    if pair[side].face == face && !seen[pair[1 - side].face] {
                        seen[pair[1 - side].face] = true;
                        stack.push(pair[1 - side].face);
                    }
                }
            }
        }
    }
    count
}
fn qualify_vertex_link(edges: &[(usize, usize)]) -> std::result::Result<(), &'static str> {
    if edges.is_empty() {
        return Err("profile-shell-empty-vertex-link");
    }
    let mut degrees = BTreeMap::new();
    for &(a, b) in edges {
        *degrees.entry(a).or_insert(0) += 1;
        *degrees.entry(b).or_insert(0) += 1;
    }
    if degrees.values().any(|d| *d != 2) {
        return Err("profile-shell-nonmanifold-vertex");
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![*degrees.keys().next().unwrap()];
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        for &(a, b) in edges {
            if a == node {
                stack.push(b);
            }
            if b == node {
                stack.push(a);
            }
        }
    }
    if seen.len() != degrees.len() {
        return Err("profile-shell-disconnected-vertex-link");
    }
    Ok(())
}
struct Union {
    parents: Vec<usize>,
}
impl Union {
    fn root(&mut self, mut n: usize) -> usize {
        while self.parents[n] != n {
            let p = self.parents[n];
            self.parents[n] = self.parents[p];
            n = self.parents[n];
        }
        n
    }
    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        self.parents[b] = a;
    }
}


#[cfg(test)]
#[path = "source_profile_shell/tests/mod.rs"]
mod tests;

#[path = "source_profile_shell_contacts.rs"]
mod contacts;
pub use contacts::{ShellSupportPair, ShellSupportReport, SupportPairReport, SupportProduct};
#[path = "source_profile_shell_original_contact.rs"]
mod original_contact;
pub use original_contact::{
    OriginalAdditionalContact, OriginalAdditionalProof, OriginalAdditionalReport,
    OriginalContactReport, OriginalFiberContact, OriginalFiberProof, OriginalFiberReport,
    OriginalHullContact, OriginalHullReport, OriginalPlanarContact, OriginalPoleContact,
    OriginalPoleProof, OriginalPoleReport,
};
#[path = "source_profile_shell_audit.rs"]
mod audit;
pub use audit::{AuditedContactPair, ContactAuditLimits, PairContactProof, ShellContactAudit};
#[path = "source_profile_shell_transition_contact.rs"]
mod transition_contact;
pub use transition_contact::{TransitionPlaneContact, TransitionPlaneReport};

#[path = "source_profile_shell_ruled_contact.rs"]
mod ruled_contact;
pub use ruled_contact::{
    ProfileRuledContact, ProfileRuledContactReport, ProfileRuledImage, ProfileRuledImageReport,
    TransitionRuledContact, TransitionRuledReport,
};

#[path = "source_profile_shell_section_contact.rs"]
mod section_contact;
pub use section_contact::{TransitionSectionContact, TransitionSectionContactReport};

pub use original_contact::{ProfileHullContact, ProfileHullReport};

pub use original_contact::{
    OriginalBoundaryContact, OriginalBoundaryProof, OriginalBoundaryReport,
};

#[path = "source_profile_shell_xy_contact.rs"]
mod xy_contact;
pub use xy_contact::{TransitionXyHullContact, TransitionXyHullReport};

pub use xy_contact::{TransitionXyBoundsContact, TransitionXyBoundsReport};

#[path = "source_profile_shell_vertex_contact.rs"]
mod vertex_contact;
pub use vertex_contact::{ProfileVertexContact, ProfileVertexReport};

#[path = "source_profile_shell_coordinate_contact.rs"]
mod coordinate_contact;
pub use coordinate_contact::{TransitionCoordinateContact, TransitionCoordinateReport};

#[path = "source_profile_shell_boundary_contact.rs"]
mod profile_boundary_contact;
pub use profile_boundary_contact::{ProfileBoundaryContact, ProfileBoundaryReport};

#[path = "source_profile_shell_paired_contact.rs"]
mod profile_paired_contact;
pub use profile_paired_contact::{ProfilePairedContact, ProfilePairedReport};

#[path = "source_profile_shell_xy_plane_contact.rs"]
mod xy_plane_contact;
pub use xy_plane_contact::{TransitionXyPlaneContact, TransitionXyPlaneReport};

#[path = "source_profile_shell_display.rs"]
mod display;
pub use display::{ExpressionDisplayCell, ExpressionDisplayLimits, ExpressionDisplayReport};

#[path = "source_profile_shell_mesh.rs"]
mod display_mesh;
pub use display_mesh::{
    ExpressionMeshLimits, ExpressionMeshPending, ExpressionMeshReport, ExpressionMeshTile,
};

#[path = "source_profile_shell_edge_mesh.rs"]
mod edge_mesh;
pub use edge_mesh::{ShellBoundaryMeshReport, ShellBoundaryPartition};

#[path = "source_profile_shell_boundary_graph.rs"]
mod boundary_graph;
pub use boundary_graph::{
    BoundaryGraphPending, BoundaryGraphReport, BoundaryGraphSegment, BoundaryGraphVertex,
};

#[path = "source_profile_shell_boundary_loops.rs"]
mod boundary_loops;
pub use boundary_loops::{BoundaryFaceLoop, BoundaryLoopSegment};

#[path = "source_profile_shell_face_candidates.rs"]
mod face_candidates;
pub use face_candidates::{FaceCandidatePending, FaceCandidateReport, FaceTriangleCandidate};

#[path = "source_profile_shell_refine_triangles.rs"]
mod refine_triangles;
pub use refine_triangles::{
    RefinedTriangle, RegionalTransitionReport, TriangleNumericFailure, TriangleRefinementLimits,
    TriangleRefinementPending, TriangleRefinementReport,
};

#[path = "source_triangle_incidence.rs"]
mod triangle_incidence;
pub use triangle_incidence::TriangleIncidenceReport;
impl FaceCandidateReport {
    pub fn inspect_incidence(&self) -> TriangleIncidenceReport {
        triangle_incidence::inspect(
            &self
                .triangles
                .iter()
                .map(|t| t.vertices)
                .collect::<Vec<_>>(),
        )
    }
}

impl TriangleRefinementReport {
    /// Counts all leaves, including unqualified geometry. This audits topology
    /// preservation only, and does not admit pending leaves as material.
    pub fn inspect_leaf_incidence(&self) -> TriangleIncidenceReport {
        triangle_incidence::inspect(
            &self
                .accepted
                .iter()
                .map(|t| t.triangle.vertices)
                .chain(self.pending.iter().map(|t| t.triangle.vertices))
                .collect::<Vec<_>>(),
        )
    }
}

#[path = "source_profile_shell_bisect_edges.rs"]
mod bisect_edges;
pub use bisect_edges::{AdaptiveInteriorLimits, InteriorBisectionReport};

#[path="source_profile_shell/methods_enclose_support_flux.rs"]
mod methods_enclose_support_flux;
#[path="source_profile_shell/methods_assemble_shell.rs"]
mod methods_assemble_shell;
