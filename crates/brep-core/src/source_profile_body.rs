//! Boundary-preserving exact weight-profile proposal on an immutable original Body.
//! Closed topology is inherited only after every retained contour is checked;
//! embedding, normals, volume and wall qualification must be rebuilt separately.
use crate::source_volume::Body;
use nurbs_core::{Error, Result, normal_alignment::WeightProfile, surface::Surface};
#[derive(Clone, Copy)]
pub struct FaceSpec {
    pub face: usize,
    pub anchor: [usize; 2],
}
pub struct Limits {
    pub controls: usize,
    pub fragments: usize,
    pub position_tolerance: f64,
}
struct Face<'a> {
    spec: FaceSpec,
    profile: WeightProfile<'a>,
}
pub struct Proposal<'a> {
    body: &'a Body,
    faces: Vec<Face<'a>>,
    axis_deviation_upper: [f64; 3],
    position_tolerance: f64,
}
/// Tangent-plane certificate attached to this exact proposal and an owned
/// original shared carrier. It does not admit volume or shell embedding.
pub struct Seam<'p, 'b> {
    proposal: &'p Proposal<'b>,
    faces: [usize; 2],
    edge: usize,
    threshold: f64,
    upper: f64,
}
impl Seam<'_, '_> {
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn upper_sine_squared(&self) -> f64 {
        self.upper
    }
}
pub struct SeamReport<'p, 'b> {
    pub seam: Option<Seam<'p, 'b>>,
    pub angular: Option<nurbs_core::normal_alignment::ProfilePathReport>,
    pub reason: &'static str,
}
/// Tangent-plane qualification for exactly one canonical carrier interval.
/// Does not qualify the rest of the original edge or admit a sewn Body.
pub struct RestrictedSeam<'p, 'b> {
    proposal: &'p Proposal<'b>,
    faces: [usize; 2],
    edge: usize,
    canonical: [f64; 2],
    threshold: f64,
    upper: f64,
}
impl RestrictedSeam<'_, '_> {
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn canonical_interval(&self) -> [f64; 2] {
        self.canonical
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn upper_sine_squared(&self) -> f64 {
        self.upper
    }
}
pub struct RestrictedSeamReport<'p, 'b> {
    pub seam: Option<RestrictedSeam<'p, 'b>>,
    pub angular: nurbs_core::normal_alignment::ProfilePathReport,
}
/// Restricted seam retaining both freshly admitted original carrier roots.
pub struct RootEndedSeam<'p, 'b> {
    restricted: RestrictedSeam<'p, 'b>,
    points: [crate::source_carrier_point::Point; 2],
    face: usize,
    axis: usize,
    parameters: [f64; 2],
    high: bool,
}

/// Original face contours subdivided at exact shared roots, preserving holes.
pub struct IncidentContour {
    face: usize,
    loops: Vec<Vec<crate::source_boundary_fragment::Fragment>>,
    middle: [usize; 2],
}
impl IncidentContour {
    pub fn face(&self) -> usize {
        self.face
    }
    pub fn loops(&self) -> &[Vec<crate::source_boundary_fragment::Fragment>] {
        &self.loops
    }
    pub fn middle(&self) -> [usize; 2] {
        self.middle
    }
}
pub struct IncidentContours {
    contours: [IncidentContour; 2],
}
impl IncidentContours {
    pub fn contours(&self) -> &[IncidentContour; 2] {
        &self.contours
    }
}
pub struct IncidentContourReport {
    pub proposal: Option<IncidentContours>,
    pub fragments: usize,
    pub reason: &'static str,
}
/// Exact full-Hermite side recipe bound to the retained root-ended B-rep seam.
/// Does not admit a closed transition face or certify the bridge interior.
pub struct TransitionSide<'s, 'p, 'b> {
    rooted: &'s RootEndedSeam<'p, 'b>,
}
impl TransitionSide<'_, '_, '_> {
    pub fn rooted(&self) -> &RootEndedSeam<'_, '_> {
        self.rooted
    }
    pub fn face(&self) -> usize {
        self.rooted.face
    }
    pub fn axis(&self) -> usize {
        self.rooted.axis
    }
    pub fn parameters(&self) -> [f64; 2] {
        self.rooted.parameters
    }
    pub fn high_free_end(&self) -> bool {
        self.rooted.high
    }
}
pub struct TransitionSideReport<'s, 'p, 'b> {
    pub side: Option<TransitionSide<'s, 'p, 'b>>,
    pub regularity: Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub reason: &'static str,
}
/// Four exact boundary expression views with root-owned corners. This is a
/// contour proposal; it does not admit a transition face or a closed Body.
pub struct TransitionBoundary<'s, 'p, 'b> {
    rooted: [&'s RootEndedSeam<'p, 'b>; 2],
}

/// Two complementary original source-chart contours at one owned section.
/// Selection of the retained region and profile-face admission remain separate.
pub struct TransitionSection<'a, 's, 'p, 'b> {
    boundary: &'a TransitionBoundary<'s, 'p, 'b>,
    high: bool,
    contours: CutContourProposal,
}
/// Owned exact section first-jet identity with complete regularity coverage.
/// Does not admit the whole shell, radius or wall thickness.
pub struct TransitionSectionJoin<'j, 'a, 's, 'p, 'b> {
    section: &'j TransitionSection<'a, 's, 'p, 'b>,
}
impl TransitionSectionJoin<'_, '_, '_, '_, '_> {
    pub fn section(&self) -> &TransitionSection<'_, '_, '_, '_> {
        self.section
    }
}
pub struct TransitionSectionJoinReport<'j, 'a, 's, 'p, 'b> {
    pub join: Option<TransitionSectionJoin<'j, 'a, 's, 'p, 'b>>,
    pub regularity: Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub reason: &'static str,
}

/// Both complementary arcs classified, with the exterior side selected.
pub struct ExteriorSection<'r, 'a, 's, 'p, 'b> {
    section: &'r TransitionSection<'a, 's, 'p, 'b>,
    kept: usize,
}

/// Embedded owned retained-profile region, with explicit collapsed-boundary
/// quotient when applicable. No surrounding shell or volume admission.
pub struct RetainedProfileFace<'r, 'e, 'a, 's, 'p, 'b> {
    exterior: &'r ExteriorSection<'e, 'a, 's, 'p, 'b>,
    factored_boundary: Option<usize>,
}
impl RetainedProfileFace<'_, '_, '_, '_, '_, '_> {
    pub fn exterior(&self) -> &ExteriorSection<'_, '_, '_, '_, '_> {
        self.exterior
    }
    pub fn factored_boundary(&self) -> Option<usize> {
        self.factored_boundary
    }
}
pub struct RetainedProfileFaceReport<'r, 'e, 'a, 's, 'p, 'b> {
    pub face: Option<RetainedProfileFace<'r, 'e, 'a, 's, 'p, 'b>>,
    pub quotient: nurbs_core::surface_quotient_injectivity::Report,
    pub ordinary: Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub pole: Option<nurbs_core::normal_alignment::ProfilePoleCover>,
    pub reason: &'static str,
}
pub struct ExteriorSectionReport<'r, 'a, 's, 'p, 'b> {
    pub region: Option<ExteriorSection<'r, 'a, 's, 'p, 'b>>,
    pub controls: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
pub struct TransitionSectionReport<'a, 's, 'p, 'b> {
    pub section: Option<TransitionSection<'a, 's, 'p, 'b>>,
    pub fragments: usize,
    pub reason: &'static str,
}
/// Admitted embedded transition expression with its root-owned contour.
/// This does not establish closure or embedding of the surrounding Body.
pub struct TransitionFace<'a, 's, 'p, 'b> {
    boundary: &'a TransitionBoundary<'s, 'p, 'b>,
    projection: [[f64; 3]; 2],
}
/// Complete radius-law coverage attached to the admitted exact face recipe.
/// Normal alignment, shell contacts and wall thickness remain separate gates.
pub struct TransitionRadius<'f, 'a, 's, 'p, 'b> {
    face: &'f TransitionFace<'a, 's, 'p, 'b>,
    centers: nurbs_core::curve::Curve,
    radius: nurbs_core::curve::Curve,
    law_domains: [[f64; 2]; 2],
    error_upper: f64,
}
impl TransitionRadius<'_, '_, '_, '_, '_> {
    pub fn face(&self) -> &TransitionFace<'_, '_, '_, '_> {
        self.face
    }
    pub fn centers(&self) -> &nurbs_core::curve::Curve {
        &self.centers
    }
    pub fn radius(&self) -> &nurbs_core::curve::Curve {
        &self.radius
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn law_domains(&self) -> [[f64; 2]; 2] {
        self.law_domains
    }
}
pub struct TransitionRadiusReport<'f, 'a, 's, 'p, 'b> {
    pub certificate: Option<TransitionRadius<'f, 'a, 's, 'p, 'b>>,
    pub coverage: Option<nurbs_core::profile_expression_radius::Report>,
    pub reason: &'static str,
}
pub struct TransitionCoefficientRadiusReport<'f, 'a, 's, 'p, 'b> {
    pub certificate: Option<TransitionRadius<'f, 'a, 's, 'p, 'b>>,
    pub bound: Option<nurbs_core::profile_expression_radius::CoefficientRadiusBound>,
    pub reason: &'static str,
}
/// Whole-boundary first-jet agreement with the original profile support.
/// Neighboring body incidence and material admission are separate gates.
pub struct TransitionSupportTangency<'f, 'a, 's, 'p, 'b> {
    face: &'f TransitionFace<'a, 's, 'p, 'b>,
}
impl TransitionSupportTangency<'_, '_, '_, '_, '_> {
    pub fn face(&self) -> &TransitionFace<'_, '_, '_, '_> {
        self.face
    }
}
pub struct TransitionSupportTangencyReport<'f, 'a, 's, 'p, 'b> {
    pub certificate: Option<TransitionSupportTangency<'f, 'a, 's, 'p, 'b>>,
    /// Order: section low/high, then rail low/high; absent entries were not visited.
    pub boundaries: Vec<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}

pub struct TransitionRail {
    edge: crate::source_shared_edge::SharedEdge,
    neighbor_contour: IncidentContour,
    neighbor: usize,
    transition_reversed: bool,
}
impl TransitionRail {
    pub fn edge(&self) -> &crate::source_shared_edge::SharedEdge {
        &self.edge
    }
    pub fn neighbor_contour(&self) -> &IncidentContour {
        &self.neighbor_contour
    }
    pub fn neighbor_side(&self) -> usize {
        self.neighbor
    }
    pub fn transition_reversed(&self) -> bool {
        self.transition_reversed
    }
}
/// Two sewn rail incidences; entry and terminal edges remain open.
pub struct TransitionRailIncidence<'f, 'a, 's, 'p, 'b> {
    face: &'f TransitionFace<'a, 's, 'p, 'b>,
    rails: [TransitionRail; 2],
    reverse_face: bool,
}
impl<'f, 'a, 's, 'p, 'b> TransitionRailIncidence<'f, 'a, 's, 'p, 'b> {
    pub fn qualify_sections<'i>(
        &'i self,
        sections: [&ExteriorSection<'_, '_, 's, 'p, 'b>; 2],
    ) -> Result<TransitionPatchReport<'i, 'f, 'a, 's, 'p, 'b>> {
        use crate::source_boundary_fragment::{Endpoint, Role};
        let mut out = TransitionPatchReport {
            patch: None,
            reason: "profile-transition-section-incidence-owner-unproven",
        };
        if sections[0].section.high == sections[1].section.high
            || sections
                .iter()
                .any(|r| !std::ptr::eq(r.section.boundary, self.face.boundary))
        {
            return Ok(out);
        }
        let sections = if sections[0].section.high {
            [sections[1], sections[0]]
        } else {
            sections
        };
        let mut kept = Vec::new();
        for (index, region) in sections.iter().enumerate() {
            let wire = region.kept_loop();
            let contact = wire.last().unwrap();
            let axis = self.face.boundary.rooted[0].axis;
            let descending = contact.curve().control_points[1][1 - axis]
                < contact.curve().control_points[0][1 - axis];
            let kept_descending = descending ^ contact.reversed();
            let transition_descending = (index == 0) ^ self.reverse_face;
            if kept_descending == transition_descending {
                out.reason = "profile-transition-section-incidence-orientation-conflict";
                return Ok(out);
            }
            for end in 0..2 {
                let rail = if kept_descending { 1 - end } else { end };
                let expected = self.face.boundary.rooted[rail].points[index].root();
                if !matches!(&contact.endpoints()[end],Endpoint::Crossing{point,role:Role::Contact} if point.same_definition(expected))
                {
                    out.reason = "profile-transition-section-incidence-root-unproven";
                    return Ok(out);
                }
            }
            kept.push(wire.to_vec());
        }
        out.patch = Some(TransitionPatchIncidence {
            rails: self,
            retained: [kept.remove(0), kept.remove(0)],
        });
        out.reason = "profile-transition-four-edge-incidence-qualified";
        Ok(out)
    }
    pub fn face(&self) -> &TransitionFace<'_, '_, '_, '_> {
        self.face
    }
    pub fn rails(&self) -> &[TransitionRail; 2] {
        &self.rails
    }
    pub fn reverse_face(&self) -> bool {
        self.reverse_face
    }
}
/// All four patch edges have opposite neighboring uses. Surrounding shell
/// admission, retained-profile immersion and global embedding remain separate.
pub struct TransitionPatchIncidence<'i, 'f, 'a, 's, 'p, 'b> {
    rails: &'i TransitionRailIncidence<'f, 'a, 's, 'p, 'b>,
    retained: [Vec<crate::source_boundary_fragment::Fragment>; 2],
}

/// Rail order, then entry/terminal exterior order. Shell closure is separate.
pub struct OuterEdgeAssembly<'o, 'i, 'f, 'a, 's, 'p, 'b> {
    patch: &'o TransitionPatchIncidence<'i, 'f, 'a, 's, 'p, 'b>,
    edges: [crate::source_shared_edge::SharedEdge; 4],
    retained_factored: [Option<usize>; 2],
}
impl OuterEdgeAssembly<'_, '_, '_, '_, '_, '_, '_> {
    pub fn patch(&self) -> &TransitionPatchIncidence<'_, '_, '_, '_, '_, '_> {
        self.patch
    }
    pub fn edges(&self) -> &[crate::source_shared_edge::SharedEdge; 4] {
        &self.edges
    }
    pub fn retained_factored(&self) -> [Option<usize>; 2] {
        self.retained_factored
    }
}
pub struct OuterEdgeReport<'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub assembly: Option<OuterEdgeAssembly<'o, 'i, 'f, 'a, 's, 'p, 'b>>,
    pub work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
pub struct TransitionPatchReport<'i, 'f, 'a, 's, 'p, 'b> {
    pub patch: Option<TransitionPatchIncidence<'i, 'f, 'a, 's, 'p, 'b>>,
    pub reason: &'static str,
}
pub struct TransitionRailReport<'f, 'a, 's, 'p, 'b> {
    pub incidence: Option<TransitionRailIncidence<'f, 'a, 's, 'p, 'b>>,
    pub work: u64,
    pub reason: &'static str,
}
pub struct TransitionFaceReport<'a, 's, 'p, 'b> {
    pub face: Option<TransitionFace<'a, 's, 'p, 'b>>,
    pub interior: Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub reason: &'static str,
}
pub fn propose_transition_boundary<'s, 'p, 'b>(
    sides: [&TransitionSide<'s, 'p, 'b>; 2],
) -> Result<TransitionBoundary<'s, 'p, 'b>> {
    let a = sides[0].rooted;
    let b = sides[1].rooted;
    if !std::ptr::eq(a.restricted.proposal, b.restricted.proposal)
        || a.face != b.face
        || a.axis != b.axis
        || a.parameters != b.parameters
        || a.high == b.high
    {
        return Err(Error::new(
            "BREP_PROFILE_TRANSITION_BOUNDARY",
            "Require opposite sides of one owned transition recipe",
        ));
    }
    Ok(TransitionBoundary {
        rooted: if a.high { [b, a] } else { [a, b] },
    })
}
pub struct RootEndedSeamReport<'p, 'b> {
    pub seam: Option<RootEndedSeam<'p, 'b>>,
    pub carrier_spans: usize,
    pub angular: Option<nurbs_core::normal_alignment::ProfilePathReport>,
    pub reason: &'static str,
}
/// An owned new section end with an exact source-parameter recipe. Interval
/// coordinates enclose that recipe; they are not rounded replacement vertices.
pub struct CutEndpoint<'p, 'b> {
    proposal: &'p Proposal<'b>,
    face: usize,
    neighbor: usize,
    edge: usize,
    axis: usize,
    parameter: f64,
    high: bool,
    canonical: [f64; 2],
    neighbor_uv: [[f64; 2]; 2],
    world: [[f64; 2]; 3],
}
/// Two unchanged selected-face fragments sharing one certified crossing root.
/// Neighbor ranges are transported through the same private canonical carrier;
/// they remain expression views until neighbor contour admission is rebuilt.
pub struct CutPartition<'c, 'p, 'b> {
    endpoint: &'c CutEndpoint<'p, 'b>,
    point: crate::source_contact_point::SourcePoint,
    selected: [crate::source_boundary_fragment::Fragment; 2],
    neighbor_ranges: [[[f64; 2]; 2]; 2],
}
/// Freshly admitted original shared-edge halves, retaining both root definitions.
/// This is not a closed replacement contour or a new corrected Body.
pub struct NeighborPartition {
    fragments: [crate::source_boundary_fragment::Fragment; 2],
    edges: [crate::source_shared_edge::SharedEdge; 2],
}
impl NeighborPartition {
    pub fn fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.fragments
    }
    pub fn edges(&self) -> &[crate::source_shared_edge::SharedEdge; 2] {
        &self.edges
    }
}
pub struct NeighborPartitionReport {
    pub carrier_spans: usize,
    pub partition: Option<NeighborPartition>,
    pub mapping_cells: usize,
    pub exact_work: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}

/// Two source-owned closed contour candidates separated by one original UV
/// section. Closure alone does not qualify region containment or a corrected face.
pub struct CutContourProposal {
    loops: [Vec<crate::source_boundary_fragment::Fragment>; 2],
}
impl CutContourProposal {
    pub fn loops(&self) -> &[Vec<crate::source_boundary_fragment::Fragment>; 2] {
        &self.loops
    }
}
pub struct CutContourReport {
    pub proposal: Option<CutContourProposal>,
    pub fragments: usize,
    pub reason: &'static str,
}

pub struct CutPartitionReport<'c, 'p, 'b> {
    pub partition: Option<CutPartition<'c, 'p, 'b>>,
    pub mapping_cells: usize,
    pub reason: &'static str,
}

pub struct CutEndpointReport<'p, 'b> {
    pub endpoint: Option<CutEndpoint<'p, 'b>>,
    pub spans: usize,
    pub reason: &'static str,
}

pub struct Report<'a> {
    pub proposal: Option<Proposal<'a>>,
    pub controls: usize,
    pub fragments: usize,
    pub reason: &'static str,
}
fn same(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && a.knots_u == b.knots_u
        && a.knots_v == b.knots_v
        && a.control_points == b.control_points
        && a.weights == b.weights
}
pub struct TransitionBoundaryReplayLimits {
    /// Each side has this independent, explicit work allocation.
    pub seam: RootSeamReplayLimits,
    pub controls: usize,
    pub side_cells: usize,
    pub side_spans: usize,
}
pub struct TransitionBoundaryReplayReport<T> {
    pub result: Option<T>,
    pub reason: &'static str,
}
/// Rebuild owners within one call, then inspect without exporting borrowed proof.
/// The callback may run fresh face/shell gates while all exact owners are alive.

/// Replayed incident contours and their fresh common 3D carrier.
/// This is a boundary assembly, not closed-shell or volume admission.
pub struct RootedContourReplay<'p, 'b> {
    pub seam: RootEndedSeam<'p, 'b>,
    pub contours: IncidentContours,
    pub edge: crate::source_shared_edge::SharedEdge,
}
pub struct RootedContourReplayReport<'p, 'b> {
    pub assembly: Option<RootedContourReplay<'p, 'b>>,
    pub fragments: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}

#[derive(Clone, Copy)]
pub struct RootSeamReplayLimits {
    pub cut_spans: usize,
    pub mapping_cells: usize,
    pub max_sine_squared: f64,
    pub angular_cells: usize,
    pub seam_spans: usize,
}


#[path = "source_profile_shell.rs"]
mod assembled_shell;
pub use assembled_shell::{
    AuditedContactPair, ContactAuditLimits, ExpressionShell, OriginalAdditionalContact,
    OriginalAdditionalProof, OriginalAdditionalReport, OriginalContactReport, OriginalFiberContact,
    OriginalFiberProof, OriginalFiberReport, OriginalPlanarContact, PairContactProof,
    ShellAssemblyReport, ShellContactAudit, ShellFace, ShellFaceKind, ShellPair, ShellPairKind,
    ShellSupportPair, ShellSupportReport, ShellUse, SupportPairReport, SupportProduct,
    TransitionNeighborTangency, TransitionNeighborTangencyReport, TransitionPlaneContact,
    TransitionPlaneReport,
};
pub use assembled_shell::{
    ExpressionAlignedChordMinimum, ExpressionWallCell, ExpressionWallPendingReason,
    ExpressionWallQualification, ExpressionWallReport,
};

#[path = "source_profile_projected_region.rs"]
mod projected_region;
pub use assembled_shell::{TransitionRuledContact, TransitionRuledReport};
pub use projected_region::{
    OriginalProjectedRegion, OriginalProjectedRegionReport, ProfileProjectedRegion,
    ProfileProjectedRegionReport,
};

pub use assembled_shell::{TransitionSectionContact, TransitionSectionContactReport};

pub use assembled_shell::{OriginalHullContact, OriginalHullReport};

pub use assembled_shell::{OriginalPoleContact, OriginalPoleProof, OriginalPoleReport};

pub use assembled_shell::{ProfileHullContact, ProfileHullReport};

pub use assembled_shell::{OriginalBoundaryContact, OriginalBoundaryProof, OriginalBoundaryReport};

pub use assembled_shell::{TransitionXyHullContact, TransitionXyHullReport};

pub use assembled_shell::{TransitionXyBoundsContact, TransitionXyBoundsReport};

pub use assembled_shell::{ProfileVertexContact, ProfileVertexReport};

pub use assembled_shell::{
    EmbeddedExpressionGeometry, ExpressionVolumeLimits, ExpressionVolumeReport, MaterialFluxReport,
};
pub use assembled_shell::{ExpressionFaceClearance, ExpressionGapReport};
pub use assembled_shell::{ExpressionSegmentCandidate, ExpressionSegmentReport};
pub use assembled_shell::{ExpressionVolumeQualification, OrientedExpressionVolume};
pub use assembled_shell::{TransitionCoordinateContact, TransitionCoordinateReport};

#[cfg(feature = "codec")]
#[path = "source_profile_shell_replay.rs"]
mod shell_replay;
#[cfg(feature = "codec")]
pub use shell_replay::{ShellReplayLimits, ShellReplayReport, with_restored_expression_shell};

#[cfg(feature = "codec")]
#[path = "source_profile_recipe_identity.rs"]
mod recipe_identity;

pub use assembled_shell::{
    ExpressionDisplayCell, ExpressionDisplayLimits, ExpressionDisplayReport,
};

pub use assembled_shell::{
    ExpressionMeshLimits, ExpressionMeshPending, ExpressionMeshReport, ExpressionMeshTile,
};

pub use assembled_shell::{
    BoundaryFaceLoop, BoundaryGraphPending, BoundaryGraphReport, BoundaryGraphSegment,
    BoundaryGraphVertex, BoundaryLoopSegment, ShellBoundaryMeshReport, ShellBoundaryPartition,
};

pub use assembled_shell::{
    RefinedTriangle, RegionalTransitionReport, TriangleNumericFailure, TriangleRefinementLimits,
    TriangleRefinementPending, TriangleRefinementReport,
};

pub use assembled_shell::{AdaptiveInteriorLimits, InteriorBisectionReport};

#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{
    restore, restore_cut_endpoint, restore_incident_contours, restore_root_ended_seam,
    restore_section_bridge, with_restored_transition_boundary,
};

#[path="source_profile_body/methods_restricted.rs"]
mod methods_restricted;
#[path="source_profile_body/methods_definition.rs"]
mod methods_definition;
#[path="source_profile_body/methods_qualify_join.rs"]
mod methods_qualify_join;
#[path="source_profile_body/methods_qualify_face.rs"]
mod methods_qualify_face;
#[path="source_profile_body/methods_qualify_support_tangency.rs"]
mod methods_qualify_support_tangency;
#[path="source_profile_body/methods_face.rs"]
mod methods_face;
#[path="source_profile_body/methods_qualify_transported_neighbor.rs"]
mod methods_qualify_transported_neighbor;
#[path="source_profile_body/methods_split_source_use.rs"]
mod methods_split_source_use;
#[path="source_profile_body/methods_cut_endpoint.rs"]
mod methods_cut_endpoint;
#[path="source_profile_body/methods_inspect_curved_edge_interval_policy.rs"]
mod methods_inspect_curved_edge_interval_policy;
#[path="source_profile_body/algorithm_propose_cut_contours.rs"]
mod algorithm_propose_cut_contours;
pub use algorithm_propose_cut_contours::propose_cut_contours;
#[path="source_profile_body/algorithm_prepare.rs"]
mod algorithm_prepare;
pub use algorithm_prepare::prepare;
