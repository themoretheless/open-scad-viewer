//! Indexed boundary topology over exact rational curve/surface definitions.
//! Validation certifies combinatorial incidence, not geometric solid validity.
// `unwrap`/`expect` in production code are counted per crate (`clippyUnwrap` pins, plan L6)
// and may only go down; prefer typed errors.
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
// Module families (ADR 0012, refactor plan M8); old paths below are re-exports.
pub mod body_analysis;
pub mod boolean;
pub mod contact;
pub mod features;
pub mod intersection;
pub mod minkowski;
pub mod model;
pub mod parts;
pub mod profile;
pub mod source_body;
pub mod sweep_offset;
use crate::profile::exact_affine_source;
use crate::profile::rational_identity;
pub(crate) use crate::profile::sketch_wire;
#[cfg(feature = "codec")]
mod serialization;
pub use brep_topology::{MAX_COEDGES, MAX_ENTITIES, MAX_FACES};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::features::aag;
pub use crate::features::aag_features;
pub use crate::features::aag_fillets;
pub use crate::body_analysis::analysis;
pub use crate::parts::analytic;
pub use crate::boolean::analytic_boolean;
pub use crate::features::analytic_features;
pub use nurbs_intersect::analytic_ss;
/// Section-authored geometric blades.
pub use crate::parts::blade;
pub use crate::model::boolean_robustness;
use crate::model::boolean_support;
use crate::boolean::box_sphere_boolean;
pub use crate::parts::circular_blend;
pub use crate::features::direct_edit;
pub use crate::features::edge_tangency;
#[doc(hidden)]
pub use crate::model::face_senses;
pub use crate::features::feature_family;
pub use crate::features::feature_graph;
pub use crate::features::feature_history;
pub use crate::features::feature_ops;
pub use crate::parts::gear;
pub use crate::features::history_snapshots;
use crate::boolean::imprint_assembly;
pub use crate::features::journal;
pub use crate::model::keypoints;
pub use crate::boolean::same_surface_uv_boolean;
pub use crate::boolean::retained_uv_boolean;
use crate::boolean::retained_uv_rim;
use crate::boolean::sphere_mate;
pub use crate::boolean::tolerant_boolean;
pub use crate::boolean::network_boolean;
pub use crate::parts::worm;
pub use gear::{GearGeometry, GearSpec, gear, gear_with_report};
use crate::model::body_edit;
pub use crate::contact::boundary_agreement;
pub use crate::contact::boundary_certificate;
pub use crate::contact::boundary_embedding;
pub use crate::contact::boundary_hull_contact;
pub use crate::boolean::close_topology;
pub use crate::contact::contact_bind;
pub use crate::contact::contact_end;
pub use crate::contact::contact_graft;
pub use crate::contact::contact_trim;
pub use crate::boolean::corner_close;
/// `nurbs-intersect` owns coverage verification; the lifted arrangement check lives in
/// `uv_coverage` because it needs kernel arrangements.
pub mod coverage_verifier {
    pub use crate::uv_coverage::verify_lifted_uv_arrangement_coverage;
    pub use nurbs_intersect::coverage_verifier::*;
}
use crate::boolean::coaxial_revolution_boolean;
use crate::boolean::revolution_section;
use crate::boolean::cone_sphere_boolean;
use crate::boolean::cylinder_sphere_boolean;
use crate::boolean::torus_sphere_boolean;
pub use crate::features::edge_partition;
pub use crate::contact::face_contact;
pub use crate::contact::face_contacts;
pub use crate::intersection::face_crossings;
pub use crate::body_analysis::face_curvature;
pub use crate::boolean::face_cut;
pub use crate::model::face_domain;
pub use crate::model::face_injectivity;
pub use crate::features::face_replacement;
pub use crate::model::face_window;
pub use crate::contact::fillet_contact;
pub use crate::contact::fillet_radius;
pub use crate::boolean::imprint_pipeline;
pub use crate::intersection::intersections;
pub use crate::intersection::nurbs_ss_g6;
pub use crate::intersection::nurbs_ss_general;
use crate::features::operation_tolerance;
pub use crate::boolean::operations;
pub use crate::sweep_offset::radius_transition_body;
pub use crate::features::region_replacement;
pub use crate::intersection::ss_face_membership;
pub use crate::intersection::ss_trimmed_face;
pub use crate::sweep_offset::sweep_boundary_owned;
#[cfg(test)]
mod sweep_boundary_owned_tests;
/// Authored trimmed patch networks with continuous admission audits.
pub use crate::source_body::trimmed_shell;
pub use crate::contact::watertight;
pub use operation_tolerance::{
    ModelToleranceHandoff, admit_operation_input, enforce_model_tolerance_handoff,
};
pub use crate::minkowski::affine_lattice;
pub use crate::profile::bezier_profile;
pub use crate::contact::boundary_contact_tube;
pub use crate::model::closed_solid;
#[cfg(feature = "codec")]
pub use crate::body_analysis::defects;
#[cfg(feature = "codec")]
pub use crate::features::healing;
#[cfg(feature = "codec")]
pub use crate::features::healing_report;
#[cfg(feature = "codec")]
pub use crate::features::import_policy;
pub use crate::parts::freeform_cuboid;
pub use crate::source_body::source_exchange_plan;
#[cfg(all(test, feature = "codec"))]
mod exchange_fixtures;
pub use freeform_cuboid::{freeform_cuboid_solid, freeform_cuboid_with_bump_face};
pub use crate::source_body::material_chord;
pub use crate::source_body::material_segment;
pub use crate::source_body::material_wall;
pub use crate::source_body::material_wall_coverage;
pub use crate::minkowski::minkowski3d;
pub use crate::model::mixed_root_model;
pub use crate::features::pcurve_repair;
pub use crate::profile::planar_trim;
pub use nurbs_intersect::predicate_evidence;
pub use crate::profile::prism;
pub use crate::profile::prism_frame;
pub use crate::profile::profile_certificate;
use crate::boolean::profile_imprint;
#[cfg(feature = "codec")]
pub use crate::intersection::profile_intersections;
pub use crate::profile::profile_region;
pub use crate::model::ray_parity;
pub use crate::model::root_model;
pub use crate::features::rounded_patch_network;
pub use crate::intersection::self_intersection;
pub use crate::contact::shared_boundary;
pub use crate::body_analysis::shell_distance;
pub use crate::body_analysis::shell_nesting;
pub use crate::body_analysis::shell_orientation;
pub use crate::body_analysis::shell_relation;
pub use crate::body_analysis::shell_thickness;
pub use crate::parts::sketch;
pub use crate::body_analysis::solid_audit;
pub use crate::body_analysis::solid_distance;
pub use crate::source_body::source_implicit_body;
use crate::boolean::sphere_boolean;
use crate::profile::stepped_prism;
pub use crate::sweep_offset::sweep_affine_boundary;
pub use crate::contact::sweep_cap_contacts;
pub use crate::sweep_offset::sweep_miter_layout;
pub use crate::sweep_offset::sweep_miter_owned;
pub use crate::sweep_offset::sweep_retained_caps;
pub use crate::sweep_offset::sweep_retained_charts;
pub use crate::sweep_offset::sweep_retained_decomposition;
pub use crate::sweep_offset::sweep_retained_walls;
pub use crate::contact::sweep_smoothness;
pub use crate::sweep_offset::sweep_station_reconstruction;
#[cfg(feature = "codec")]
pub use crate::features::tolerance_unify;
pub use crate::features::transactions;
pub use crate::model::transform;
use crate::model::transform_plane_charts;
pub use crate::boolean::trim_sew;
pub use crate::source_body::trimmed_edge_contact;
pub use crate::contact::trimmed_vertex_contact;
pub use crate::boolean::uv_arrangement;
pub use crate::boolean::uv_coverage;
use crate::boolean::uv_regions;
pub use crate::body_analysis::volume_validity;
pub use crate::source_body::wall_clearance;
pub use crate::source_body::wall_opposition;
pub use analytic::{
    AffinePose, AffineStationLimits, AffineStationWalls, FiberedBody, FiberedBodyAttempt,
    FiberedBodyLimits, FiberedBodyReport, FiberedSource, LoftCap, SmoothStationWalls,
    affine_station_walls, capped_loft_surfaces,
    capped_loft_with_caps, capped_loft_with_caps_checked,
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
use crate::model::model_report;
use crate::body_analysis::model_validation;
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

pub use crate::profile::planar_cap;
pub use crate::parts::rack;
pub use crate::parts::thread;

use crate::contact::control_hull_separation;
use crate::contact::face_contact_groups;

#[cfg(test)]
mod spatial_rmf_qualification;

pub use crate::sweep_offset::sweep_retained;

pub use crate::sweep_offset::miter_seams;

pub use crate::boolean::offset_end_boundary;

pub use crate::source_body::trimmed_face_recipe;

pub use crate::source_body::source_contact_point;

pub use crate::source_body::source_boundary_fragment;
pub use crate::source_body::source_carrier_point;

pub use crate::source_body::source_contour_proposal;

pub use crate::source_body::source_region_restore;

pub use crate::source_body::source_contour_winding;

pub use crate::source_body::source_world_wire;

use crate::source_body::source_line_chart_root;

pub use crate::source_body::source_shared_edge;

pub use crate::source_body::source_shared_edge_restore;

pub use crate::source_body::source_edge_restriction;

pub use crate::source_body::source_boundary_network;

pub use crate::source_body::source_boundary_display;

pub use crate::source_body::source_region_display;

pub use crate::source_body::source_collapsed_boundary;

pub use crate::source_body::source_shell_incidence;

pub use crate::source_body::source_shell_restore;

pub use crate::source_body::source_body_restore;

pub use crate::source_body::source_face_contacts;

pub use crate::source_body::source_allowed_contact;

pub use crate::source_body::source_plane_fiber;

pub use crate::source_body::source_pole_plane_image;

pub use crate::source_body::source_pole_paired_contact;
pub use crate::source_body::source_profile_boundary_identity;

pub use crate::source_body::source_paired_fiber_contact;

pub use crate::source_body::source_fiber_boundary;

pub use crate::source_body::source_fiber_contact;

pub use crate::source_body::source_affine_use;

pub use crate::source_body::source_mapped_edge;
pub use crate::source_body::source_projective_body_restore;
pub use crate::source_body::source_projective_edge;
pub use crate::source_body::source_projective_shell_restore;
pub use crate::source_body::source_projective_use;

pub use crate::source_body::trimmed_shell_recipe;

pub use crate::source_body::offset_edge_supports;

pub use crate::source_body::linear_canal;

pub use crate::source_body::linear_canal_body;

pub use crate::source_body::source_root_parameter;

pub use crate::source_body::source_halfplane_side;

pub use crate::source_body::source_interior_fiber;

pub use crate::source_body::source_interior_boundary;

pub use crate::source_body::source_interior_contact;

pub use crate::source_body::source_vertex_links;

pub use crate::source_body::source_vertex_contact;

pub use crate::source_body::source_hull_separation;

pub use crate::source_body::source_shell_geometry;

pub use crate::source_body::graph_body;
pub use crate::source_body::source_box;
pub use crate::source_body::source_volume;

pub use crate::source_body::source_body_model;

pub use crate::source_body::source_body_topology;

pub use crate::source_body::source_pole_tangency;
pub use crate::source_body::source_seam_tangency;

pub use crate::source_body::source_inverse_shear;

pub use crate::source_body::source_root_refinement;

pub use crate::source_body::source_exchange_endpoints;

pub use crate::source_body::source_exchange_trims;


pub use crate::source_body::source_support_shell;

pub use crate::source_body::source_pole_planar_contact;

pub use crate::source_body::source_ruled_projection_contact;

pub use crate::source_body::source_planar_material_hull;

pub use crate::source_body::source_corner_plane_image;

pub use crate::source_body::source_face_gap;

pub use crate::source_body::source_planar_flux;

pub use crate::source_body::source_material_segment;
pub use crate::source_body::source_profile_body;
pub use crate::source_body::source_retained_face_gap;
pub use crate::source_body::source_shared_chord_map;
pub use crate::source_body::source_wall_anchor;
pub use crate::source_body::source_wall_pair;

pub use crate::source_body::source_material_chord;

pub use crate::source_body::source_material_wall;
pub use crate::source_body::source_wall_search;

pub use crate::source_body::source_wall_scan;

pub use crate::source_body::source_wall_coverage;

pub use crate::source_body::source_wall_self_coverage;

pub use crate::source_body::wall_search;

pub use crate::sweep_offset::retained_annular_cap;

pub use crate::model::topology_reference;

use crate::parts::cuboid_primitive;
pub use cuboid_primitive::cuboid;

pub use crate::source_body::exact_graph_strip;
pub use crate::source_body::source_curved_boundary_contact;
pub use crate::source_body::source_paired_projection_contact;

pub use crate::source_body::rational_cap_translation;

pub use crate::source_body::rational_cap_affine;

pub use crate::boolean::algebraic_model;
pub use algebraic_model::{AlgebraicModel, ExactBooleanOptions, ExactModel};
