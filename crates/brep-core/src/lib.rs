//! Indexed boundary topology over exact rational curve/surface definitions.
//! Validation certifies combinatorial incidence, not geometric solid validity.
mod exact_affine_source;
mod rational_identity;
#[cfg(feature = "codec")]
mod serialization;
pub use brep_topology::{MAX_COEDGES, MAX_ENTITIES, MAX_FACES};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub mod aag;
pub mod aag_features;
pub mod aag_fillets;
pub mod analysis;
pub mod analytic;
pub mod analytic_boolean;
pub mod analytic_features;
pub mod analytic_ss;
/// Section-authored geometric blades.
pub mod blade;
pub mod boolean_robustness;
mod boolean_support;
mod box_sphere_boolean;
pub mod circular_blend;
pub mod direct_edit;
pub mod edge_tangency;
#[doc(hidden)]
pub mod face_senses;
pub mod feature_family;
pub mod feature_graph;
pub mod feature_history;
pub mod feature_ops;
pub mod gear;
pub mod history_snapshots;
mod imprint_assembly;
pub mod journal;
pub mod keypoints;
pub mod same_surface_uv_boolean;
pub mod retained_uv_boolean;
mod retained_uv_rim;
mod sphere_mate;
pub mod tolerant_boolean;
pub mod worm;
pub use gear::{GearGeometry, GearSpec, gear, gear_with_report};
mod body_edit;
pub mod boundary_agreement;
pub mod boundary_certificate;
pub mod boundary_embedding;
pub mod boundary_hull_contact;
pub mod close_topology;
pub mod contact_bind;
pub mod contact_end;
pub mod contact_graft;
pub mod contact_trim;
pub mod corner_close;
pub mod coverage_verifier;
mod cylinder_sphere_boolean;
pub mod edge_partition;
pub mod face_contact;
pub mod face_contacts;
pub mod face_crossings;
pub mod face_curvature;
pub mod face_cut;
pub mod face_domain;
pub mod face_injectivity;
pub mod face_replacement;
pub mod face_window;
pub mod fillet_contact;
pub mod fillet_radius;
pub mod imprint_pipeline;
pub mod intersections;
pub mod nurbs_ss_g6;
pub mod nurbs_ss_general;
mod operation_tolerance;
pub mod operations;
pub mod radius_transition_body;
pub mod region_replacement;
pub mod ss_face_membership;
pub mod ss_trimmed_face;
pub mod sweep_boundary_owned;
#[cfg(test)]
mod sweep_boundary_owned_tests;
/// Authored trimmed patch networks with continuous admission audits.
pub mod trimmed_shell;
pub mod watertight;
pub use operation_tolerance::{
    ModelToleranceHandoff, admit_operation_input, enforce_model_tolerance_handoff,
};
pub mod affine_lattice;
pub mod bezier_profile;
pub mod boundary_contact_tube;
pub mod closed_solid;
#[cfg(feature = "codec")]
pub mod defects;
#[cfg(feature = "codec")]
pub mod healing;
#[cfg(feature = "codec")]
pub mod healing_report;
#[cfg(feature = "codec")]
pub mod import_policy;
pub mod freeform_cuboid;
pub mod source_exchange_plan;
#[cfg(all(test, feature = "codec"))]
mod exchange_fixtures;
pub use freeform_cuboid::{freeform_cuboid_solid, freeform_cuboid_with_bump_face};
pub mod material_chord;
pub mod material_segment;
pub mod material_wall;
pub mod material_wall_coverage;
pub mod minkowski3d;
pub mod mixed_root_model;
pub mod pcurve_repair;
pub mod planar_trim;
pub mod predicate_evidence;
pub mod prism;
pub mod prism_frame;
pub mod profile_certificate;
mod profile_imprint;
#[cfg(feature = "codec")]
pub mod profile_intersections;
pub mod profile_region;
pub mod ray_parity;
pub mod root_model;
pub mod rounded_patch_network;
pub mod self_intersection;
pub mod shared_boundary;
pub mod shell_distance;
pub mod shell_nesting;
pub mod shell_orientation;
pub mod shell_relation;
pub mod shell_thickness;
pub mod sketch;
pub mod solid_audit;
pub mod solid_distance;
pub mod source_implicit_body;
mod sphere_boolean;
mod stepped_prism;
pub mod sweep_affine_boundary;
pub mod sweep_cap_contacts;
pub mod sweep_miter_layout;
pub mod sweep_miter_owned;
pub mod sweep_retained_caps;
pub mod sweep_retained_charts;
pub mod sweep_retained_decomposition;
pub mod sweep_retained_walls;
pub mod sweep_smoothness;
pub mod sweep_station_reconstruction;
#[cfg(feature = "codec")]
pub mod tolerance_unify;
pub mod transactions;
pub mod transform;
mod transform_plane_charts;
pub mod trim_sew;
pub mod trimmed_edge_contact;
pub mod trimmed_vertex_contact;
pub mod uv_arrangement;
pub mod uv_coverage;
mod uv_regions;
pub mod volume_validity;
pub mod wall_clearance;
pub mod wall_opposition;
pub use analytic::{
    AffinePose, AffineStationLimits, AffineStationWalls, FiberedBody, FiberedBodyAttempt,
    FiberedBodyLimits, FiberedBodyReport, LoftCap, SmoothStationWalls, affine_station_walls,
    capped_loft_surfaces, capped_loft_with_caps, capped_loft_with_caps_checked,
    construct_fibered_body, cylinder, frustum, natural_section_loft, periodic_section_loft,
    progressive_affine_profile_body, progressive_authored_profile_body,
    progressive_guided_profile_body, progressive_profile_body, rational_section_loft, revolve,
    revolve_angle, revolve_region, revolve_region_angle, revolve_wire, revolve_wire_angle,
    ruled_loft, section_loft_surfaces, smooth_station_walls, sphere, torus, tube,
    verify_fibered_body,
};
pub use analytic_boolean::{BooleanCertificate, analytic_boolean, analytic_boolean_audited};
pub use analytic_features::{
    AUDITED_MULTI_EDGE_FILLET_CAPABILITY, AuditedFeatureResult, EXACT_ANALYTIC_SHELL_CAPABILITY,
    EXACT_ANNULAR_FILLET_CAPABILITY, EXACT_BENT_RMF_SWEEP_CAPABILITY,
    EXACT_CONVEX_CHAMFER_CAPABILITY, EXACT_CONVEX_PRISM_FILLET_CAPABILITY,
    EXACT_MULTI_SECTION_LOFT_CAPABILITY, EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY,
    EXACT_SIMPLE_PRISM_FILLET_CAPABILITY, EXACT_VALENCE3_CORNER_BLEND_CAPABILITY,
    EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY, FeatureCertificate, analytic_chamfer, analytic_fillet,
    analytic_fillet_chain, analytic_shell, analytic_solid_loft, audited_bent_rmf_sweep,
    audited_multi_edge_fillet, audited_multi_section_loft, audited_parallel_frame_sweep,
    exact_analytic_shell, exact_annular_fillet, exact_convex_chamfer, exact_convex_prism_fillet,
    exact_layered_prism_fillet, exact_simple_prism_fillet, exact_valence3_corner_blend,
    exact_variable_radius_fillet, export_iges, frame_law_ruled_sweep, import_iges,
};
pub use close_topology::{
    AuditedTopologyComplex, BodyRole, CLOSE_TOPOLOGY_CAPABILITY, CLOSE_TOPOLOGY_IGES_CAPABILITY,
    CLOSE_TOPOLOGY_STEP_CAPABILITY, ComplexHealCertificate, ComplexHealPlan,
    ComplexInterchangeCertificate, ComplexPart, CorrespondenceKind, EdgeRadialRing, EdgeUseRef,
    ExactParameterPartition, FaceRef, LocalCorrespondence, MixedDimensionalBrep, SharedFace,
    TOLERANT_COMPLEX_HEAL_CAPABILITY, TopologyComplexCertificate, VertexFan, VertexUseRef,
    certify_complex_heal, complex_relation_id, };
pub use nurbs_ss_g6::{
    BranchCompletenessCertificate, BranchComponent, BranchGraph, BranchOrientation,
    CertifiedBranchFragment, ContainedGraphBooleanCertificate, CurvedGraphBooleanCertificate,
    G6_CAPABILITY, G6_MATURITY, G6Component, G6Maturity, GENERAL_NURBS_BOOLEAN_AUTHORITY,
    GeneralNurbsBooleanCertificate, GeneralNurbsBooleanNaming, NURBS_BOOLEAN_CAPABILITY,
    NURBS_BOOLEAN_CAPABILITY_V1, NURBS_BOOLEAN_CAPABILITY_V3, NURBS_BOOLEAN_CAPABILITY_V4,
    NURBS_BOOLEAN_CAPABILITY_V5, NURBS_BOOLEAN_CAPABILITY_V7, NURBS_BOOLEAN_SS_CAPABILITY,
    NURBS_BOOLEAN_V1_MATURITY, NurbsBooleanImprintCertificate, RationalBezierDecomposition,
    RationalBezierPatchSpan, TensorSpanId, TransverseSpanEvidence, author_general_nurbs_boolean,
    canonical_bezier_graph_solid, canonical_multispan_graph_solid, canonical_rational_graph_solid,
    certify_multispan_ss, decompose_rational_bezier_spans, join_certified_multispan_fragments,
    narrow_transverse_bezier_le3, narrow_transverse_bicubic, nurbs_boolean_graph_containment_v4,
    nurbs_boolean_graph_patch_unequal_v4, nurbs_boolean_graph_patch_v3,
    nurbs_boolean_imprint_solids, nurbs_boolean_rational_graph_patch_v5,
    nurbs_boolean_transverse_bicubic,
};
pub use nurbs_ss_general::{
    GENERAL_SS_CAPABILITY, GeneralBranchGraph, GeneralBranchKind, GeneralPcurve, GeneralSsBranch,
    TraceCorrespondence, branch_graph_from_surface_intersection, rational_traces_from_branch_graph,
    verify_general_ss_branch_graph,
};
#[cfg(feature = "codec")]
pub use nurbs_ss_general::{branch_graph_from_ss_report, rational_traces_from_ss_report};
pub use operations::{
    boolean, chamfer, chamfer_edges, extrude_polygon, extrude_polygon_with_holes, faceted_cylinder,
    faceted_loft, faceted_revolve, faceted_sphere, faceted_sweep, fillet, fillet_edges,
};

pub use brep_topology::{
    Body, ChangeKind, ChangeProvenance, ChangeSet, FaceUse, Shell, TopoId, TopoKind,
    TopologyChange, Vertex,
};
pub type Edge = brep_topology::Edge<Curve>;
pub type Coedge = brep_topology::Coedge<Curve>;
pub type Loop = brep_topology::Loop<Curve>;
pub type Face = brep_topology::Face<Surface>;
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TopologyIds {
    pub vertices: Vec<TopoId>,
    pub edges: Vec<TopoId>,
    pub loops: Vec<TopoId>,
    pub faces: Vec<TopoId>,
    pub shells: Vec<TopoId>,
    pub bodies: Vec<TopoId>,
    pub lineage: Vec<TopologyLineageRecord>,
    pub change_set: ChangeSet,
    /// Optional edge/vertex tolerance overrides; absence inherits `tolerance_mm`.
    pub local_tolerances_mm: BTreeMap<TopoId, f64>,
    /// Persistent identities marked by successful tolerant sewing.
    pub sewed_edges: BTreeSet<TopoId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyLineageRecord {
    pub operation: String,
    pub entity_kind: String,
    pub parents: Vec<TopoId>,
    pub children: Vec<TopoId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model(
    pub brep_topology::Model<Curve, Surface, Curve>,
    pub TopologyIds,
);

impl std::ops::Deref for Model {
    type Target = brep_topology::Model<Curve, Surface, Curve>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Model {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
mod model_identity;
mod model_report;
mod model_validation;
pub use model_report::Report;
pub(crate) use model_validation::validate_pole_boundary;

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: "BREP_INVALID_TOPOLOGY",
        message: message.into(),
    }
}
pub(crate) fn require(ok: bool, message: &str) -> Result<()> {
    ok.ok_or_else(|| invalid(message))
}
pub(crate) fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
pub(crate) fn curve_point(c: &Curve, t: f64) -> Result<Vec<f64>> {
    let d = c.domain();
    // Literal endpoints avoid an affine roundoff stepping outside the domain.
    let u = if t == 0. {
        d[0]
    } else if t == 1. {
        d[1]
    } else {
        d[0] + t * (d[1] - d[0])
    };
    Ok(c.evaluate(u)?.point)
}
impl Model {
    /// Canonical regularized empty solid. It has no placeholder shell or body.
    pub fn empty(tolerance_mm: f64) -> Result<Self> {
        let model = Self(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm,
            },
            TopologyIds::default(),
        );
        model.validate()?;
        Ok(model)
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
            && self.edges.is_empty()
            && self.loops.is_empty()
            && self.faces.is_empty()
            && self.shells.is_empty()
            && self.bodies.is_empty()
    }
}
fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    }
}

#[cfg(test)]
mod tests;

pub mod planar_cap;
pub mod rack;
pub mod thread;

mod control_hull_separation;
mod face_contact_groups;

#[cfg(test)]
mod spatial_rmf_qualification;

pub mod sweep_retained;

pub mod miter_seams;

pub mod offset_end_boundary;

pub mod trimmed_face_recipe;

pub mod source_contact_point;

pub mod source_boundary_fragment;
pub mod source_carrier_point;

pub mod source_contour_proposal;

pub mod source_region_restore;

pub mod source_contour_winding;

pub mod source_world_wire;

mod source_line_chart_root;

pub mod source_shared_edge;

pub mod source_shared_edge_restore;

pub mod source_edge_restriction;

pub mod source_boundary_network;

pub mod source_boundary_display;

pub mod source_region_display;

pub mod source_collapsed_boundary;

pub mod source_shell_incidence;

pub mod source_shell_restore;

pub mod source_body_restore;

pub mod source_face_contacts;

pub mod source_allowed_contact;

pub mod source_plane_fiber;

pub mod source_pole_plane_image;

pub mod source_pole_paired_contact;
pub mod source_profile_boundary_identity;

pub mod source_paired_fiber_contact;

pub mod source_fiber_boundary;

pub mod source_fiber_contact;

pub mod source_affine_use;

pub mod source_mapped_edge;
pub mod source_projective_body_restore;
pub mod source_projective_edge;
pub mod source_projective_shell_restore;
pub mod source_projective_use;

pub mod trimmed_shell_recipe;

pub mod offset_edge_supports;

pub mod linear_canal;

pub mod linear_canal_body;

pub mod source_root_parameter;

pub mod source_halfplane_side;

pub mod source_interior_fiber;

pub mod source_interior_boundary;

pub mod source_interior_contact;

pub mod source_vertex_links;

pub mod source_vertex_contact;

pub mod source_hull_separation;

pub mod source_shell_geometry;

pub mod graph_body;
pub mod source_box;
pub mod source_volume;

pub mod source_body_model;

pub mod source_body_topology;

pub mod source_pole_tangency;
pub mod source_seam_tangency;

pub mod source_inverse_shear;

pub mod source_root_refinement;

pub mod source_exchange_endpoints;

pub mod source_exchange_trims;


pub mod source_support_shell;

pub mod source_pole_planar_contact;

pub mod source_ruled_projection_contact;

pub mod source_planar_material_hull;

pub mod source_corner_plane_image;

pub mod source_face_gap;

pub mod source_planar_flux;

pub mod source_material_segment;
pub mod source_profile_body;
pub mod source_retained_face_gap;
pub mod source_shared_chord_map;
pub mod source_wall_anchor;
pub mod source_wall_pair;

pub mod source_material_chord;

pub mod source_material_wall;
pub mod source_wall_search;

pub mod source_wall_scan;

pub mod source_wall_coverage;

pub mod source_wall_self_coverage;

pub mod wall_search;

pub mod retained_annular_cap;

pub mod topology_reference;

#[path = "cuboid_primitive.rs"]
mod cuboid_primitive;
pub use cuboid_primitive::cuboid;

pub mod exact_graph_strip;
pub mod source_curved_boundary_contact;
pub mod source_paired_projection_contact;

pub mod rational_cap_translation;

pub mod rational_cap_affine;

pub mod algebraic_model;
pub use algebraic_model::{AlgebraicModel, ExactBooleanOptions, ExactModel};
