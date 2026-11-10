#![doc = include_str!("../README.md")]
//! Own binary64 rational B-spline kernel. No C/C++ or geometry dependency.
//! No polygon-core, WASM, browser or application dependency.
//! CAD B-rep over these curves/surfaces lives in `brep-core`.

/// Task-oriented API views; existing flat module paths remain supported.
pub mod domains;
pub mod numerics;
pub use crate::editing::constrained_surface;
pub use crate::editing::polyhedral_spline;
pub mod core;
pub mod distance;
pub mod construct;
pub mod analysis;
pub mod editing;
pub mod join;
pub mod trim;
pub mod tessellation;
pub mod certificates;
pub use crate::core::affine;
pub use crate::sweeps::affine_volume;
pub use crate::sweeps::fibered_volume;
pub use crate::analysis::implicit_quadric;
pub use crate::tessellation::shared_tessellation;
pub use crate::intersection::self_curve as curve_self_intersection;
pub use crate::intersection::self_surface as surface_self_intersection;
pub use crate::analysis::curve_pullback;
pub use crate::construct::surfaces::function_surface;
pub use crate::join::periodic_seam;
pub use crate::join::curve_join;
pub use crate::core::conditioning;
pub use crate::core::coordinate_frame;
pub use crate::foundation::sections;
pub use crate::construct::surfaces::sections::section_projection;
pub use crate::construct::surfaces::sections::circle_sweep_repair;
pub use crate::construct::surfaces::sections::miter_section_correction;
pub use crate::construct::surfaces::sections::sweep_section_projection;
pub use crate::construct::surfaces::sections::circle_repair as section_circle_repair;
pub use crate::join::section_phase;
pub use crate::editing::weight_edit;
pub use crate::editing::linear_simplification;
pub use crate::construct::curves::engineering_profiles;
pub use crate::construct::surfaces::boundary_fill;
pub use crate::core::bounds;
pub use crate::foundation::closed_spline;
pub use crate::join::continuity;
pub use crate::construct::surfaces::coons;
pub use crate::core::curve;
pub use crate::core::geometry;
pub use crate::analysis::point_inversion;
pub use crate::analysis::curve_differential;
pub use crate::analysis::surface_differential;
pub use crate::analysis::curve_measure;
pub use crate::analysis::curve_extrema;
pub use crate::analysis::curve_analysis;
pub use crate::foundation::canonical;
pub use crate::editing::direct_edit;
pub use crate::editing::morph;
pub use crate::editing::fairing;
pub use crate::editing::fairing::{curvature_comb_samples, curve_torsion_at};
pub use crate::analysis::geodesic;
pub use crate::analysis::unrolling;
pub use crate::construct::curves::biarc;
pub use crate::editing::curve_extension;
pub use distance::interval_eval;
pub use crate::distance::curve_distance;
pub use crate::certificates::curve_decomposition_certificate;
use crate::core::exact_curve_segments;
pub use crate::distance::curve_deviation;
pub use crate::analysis::curve_plane;
pub use crate::tessellation::curve_tessellation;
pub use crate::offset::surface_offset;
pub use crate::tessellation::surface_tessellation;
pub use crate::distance::curve_surface_distance;
use numerics::distance_bounds;
pub use crate::editing::edit;
pub use crate::construct::curves::formula;
pub mod foundation;
pub mod sweeps;
pub use crate::sweeps::framed_sweep;
pub use crate::sweeps::profile_sweep;
pub use crate::sweeps::progressive_sweep;
pub use crate::construct::surfaces::gordon;
pub use crate::foundation::grid_spline;
pub use crate::construct::curves::helix;
pub use crate::construct::curves::involute;
pub use crate::construct::curves::logarithmic_spiral;
pub use crate::construct::curves::lissajous;
pub use crate::construct::curves::trochoid;
pub use crate::construct::curves::circular_rolling;
pub use crate::construct::curves::archimedean_spiral;
pub use crate::construct::curves::catenary;
pub use crate::construct::surfaces::catenoid;
pub use crate::construct::surfaces::helicoid;
pub use crate::construct::curves::toroidal_spiral;
pub use crate::construct::curves::spherical_spiral;
pub use crate::construct::curves::clothoid;
pub use crate::construct::surfaces::screw_surface;
pub use crate::sweeps::pipe;
pub use crate::sweeps::ribbon;
pub use crate::construct::surfaces::transitions::circle as circle_transition;
pub use crate::construct::surfaces::transitions::ellipse as ellipse_transition;
pub use crate::construct::surfaces::transitions::circle_rectangle as circle_rectangle_transition;
pub use crate::construct::surfaces::extrusion_patches;
pub use crate::foundation::hermite;
pub use crate::construct::surfaces::hermite_patch;
pub use crate::foundation::natural_loft;
#[cfg(feature = "codec")]
pub use crate::construct::surfaces::loft::guided_loft;
pub use crate::construct::surfaces::loft::continuity as loft_continuity;
#[cfg(feature = "codec")]
pub use crate::construct::surfaces::loft::alignment as loft_alignment;
pub use crate::foundation::natural_spline;
pub use crate::construct::surfaces::patches;
pub use crate::construct::curves::paths;
pub use crate::construct::surfaces::polynomial;
pub use crate::construct::curves::primitives;
pub use crate::sweeps::scaled_sweep;
pub use crate::construct::surfaces::triangular_patch;
pub use crate::trim::trim_point;
pub use crate::sweeps::twist_sweep;
pub use crate::sweeps::two_guide_sweep;
pub use distance_bounds::DistanceStopReason;
pub mod offset;
pub use crate::offset::chord::{
    arrangement as chord_arrangement, embedding as chord_embedding, faces as chord_faces,
    fill_selection as chord_fill_selection, intersection as chord_intersection,
    winding as chord_winding, witness as chord_witness,
};
use distance::curve_jets;
pub use crate::offset::curve_offset;
pub use crate::analysis::class_a::{gaussian_and_mean_curvature_grid, zebra_reflection_stripes};
pub use crate::analysis::frechet;
pub use crate::analysis::frechet::{discrete_frechet_distance_3d, hausdorff_curve_discrete};
pub use crate::core::curve::Curve as NurbsCurve;
pub use crate::core::surface::{Surface as NurbsSurface, ruled_surface};
pub use crate::construct::curves::curve_reconstruction;
pub(crate) use crate::offset::diagnostics as curve_offset_diagnostics;
pub use crate::offset::curve_offset_join;
pub use crate::offset::curve_offset_wire;
pub use crate::analysis::curve_surface_agreement;
pub use crate::analysis::surface_collar;
pub(crate) use crate::analysis::curve_surface_composition;
pub mod intersection;
use numerics::periodic_chart;
pub use crate::distance::radial_bounds;
pub use crate::analysis::skin_radius;
pub use crate::analysis::ray_surface;
pub use crate::intersection::ss_intersection;
pub use crate::core::surface;
pub use crate::analysis::surface_measure;
pub use crate::analysis::curve_regularity;
pub use crate::sweeps::audit::pair as sweep_pair_audit;
pub use crate::analysis::surface_injectivity;
pub use crate::analysis::surface_regularity;
pub use crate::analysis::surface_correspondence;
pub use crate::join::surface_join;
pub use crate::join::surface_g1;
pub use crate::join::surface_g2;
pub use crate::analysis::surface_shape_operator;
pub use crate::analysis::surface_normal_trace;
pub use crate::analysis::curve_secant_trace;
pub use crate::editing::surface_edit;
pub use crate::intersection::surface_contact;
pub use crate::intersection::surface_contact_search;
pub use crate::distance::surface_distance;
pub use crate::analysis::surface_monotonicity;
pub use crate::analysis::surface_linear_monotonicity;
pub use crate::sweeps::audit::wall as sweep_wall_audit;
pub use crate::sweeps::audit::contour as sweep_contour_audit;
pub use crate::sweeps::audit::cap_boundary as sweep_cap_boundary;
pub use crate::sweeps::audit::cap_wall as sweep_cap_wall;
pub use crate::core::retained_wall_coefficients;
pub use crate::analysis::retained_wall_domain_certificate;
pub use crate::sweeps::audit::seam as sweep_seam_audit;
pub use crate::trim::trim_domain;
pub use crate::trim::trim_region_audit;
pub use crate::trim::trim_simplicity;
pub use crate::trim::trimmed_surface_distance;

pub use math_core::{Error, Result};
pub(crate) const INVALID_INPUT: &str = "NURBS_INVALID_INPUT";
pub(crate) const NUMERIC_ERROR: &str = "NURBS_NUMERIC_ERROR";
pub(crate) const RESOURCE_LIMIT: &str = "NURBS_RESOURCE_LIMIT";
pub(crate) fn input(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
pub(crate) fn numeric_err(message: impl Into<String>) -> Error {
    Error::new(NUMERIC_ERROR, message)
}
pub(crate) fn resource(message: impl Into<String>) -> Error {
    Error::new(RESOURCE_LIMIT, message)
}

pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
pub(crate) fn numeric(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, NUMERIC_ERROR, message)
}
#[cfg(feature = "transport")]
mod transport;
#[cfg(feature = "transport")]
pub use transport::{dispatch, execute};

#[cfg(all(test, feature = "transport"))]
mod tests;

pub use crate::sweeps::progressive_miter;

pub use crate::sweeps::helical_sweep;

pub use crate::construct::curves::rack;

pub use crate::construct::curves::thread;
pub use crate::construct::curves::worm;

use numerics::exact_products;

pub use crate::analysis::surface_parameter_bounds;

pub use crate::construct::curves::curve_chain;
pub use numerics::compensated;
pub use foundation::dual;
pub use numerics::robust_solvers;
pub use distance::interval_newton;
pub use numerics::sturm;
pub use distance::convex_distance;
pub use distance::obb_tree;
pub use analysis::normal_cone;
pub use foundation::bezier_extraction;

pub use crate::distance::uv_curve_crossings;

pub use crate::join::surface_quotient_injectivity;

/// Public facade: the implementation lives in `analysis::normal_alignment` (ADR 0012); the
/// profile and boundary types it used to re-export stay reachable here.
pub mod normal_alignment {
    pub use crate::analysis::normal_alignment::*;
    pub use crate::surface_boundary_secant::{OriginalBoundarySecant,boundary_secant_bounds,OriginalBoundaryDirection,boundary_secant_direction_bounds};
    pub use crate::surface_weight_profile::{WeightProfile,ProfileDirection,ProfileBoundary,ProfilePathReport,ProfilePoleLimits,ProfilePoleNormal,ProfilePoleCover,ProfileSection,ProfileEndTransition,ProfileTransitionRail,TransitionRegularityReport,qualify_weight_profile};
    pub use crate::profile_section_bridge::{ProfileSectionBridge,ProfileBridgeRail,RailConstrainedBridge,ProfileBridgeChordCoverage};
}

pub use crate::core::curve_surface_plane;

pub use crate::analysis::curve_surface_affine;

pub use crate::trim::planar_area;

#[cfg(feature = "codec")]
pub use crate::construct::loft_reparameterization;

use crate::core::sweep_support;

pub use crate::analysis::retained_wall_domain;

pub use sweeps::seam_set as sweep_seam_set;

pub use sweeps::section_correction as sweep_section_correction;

pub use offset::trimmed_contact as trimmed_offset_contact;

pub use offset::source_boundary as offset_source_boundary;

pub use offset::contact_tangent as offset_contact_tangent;

pub use offset::envelope as offset_envelope;

pub use offset::envelope_fit as offset_envelope_fit;

pub use offset::contact_pcurve as offset_contact_pcurve;

pub use offset::contact_trims as offset_contact_trims;

pub use offset::face_loops as offset_face_loops;

pub use offset::patch_boundary as offset_patch_boundary;

pub use crate::analysis::curve_partition_agreement;

pub use crate::analysis::boundary_partition;

pub use crate::analysis::contact_normal_agreement;

pub use offset::contact_predictor as offset_contact_predictor;

pub use offset::contact_path as offset_contact_path;

pub use crate::distance::curve_axis_driver;

pub use offset::path_trims as offset_path_trims;

pub use offset::path_pcurves as offset_path_pcurves;

pub use crate::analysis::curve_surface_lift;

pub use crate::core::curve_point_identity;

pub use crate::distance::surface_flux;

pub use crate::analysis::moving_radius;

pub use crate::analysis::moving_envelope;

pub use crate::core::surface_projection_jacobian;
pub use crate::analysis::profile_projection_jacobian;
pub use crate::trim::profile_projected_jordan;

pub use crate::core::curve_quadratic_separator;

pub use crate::trim::surface_projected_jordan;

pub use crate::join::surface_self_chord;

pub use crate::join::surface_self_chord_coverage;

pub use crate::join::factored_surface_normals;

pub use crate::distance::polar_surface_differential;

pub use crate::join::profile_expression_radius;

pub use crate::join::profile_expression_bernstein;

pub use numerics::bernstein_interpolation as bernstein_interpolation;

pub use crate::distance::rational_pole_differential;

pub use crate::join::profile_section_bridge;

pub use crate::analysis::surface_boundary_secant;

pub use crate::analysis::surface_weight_profile;
pub use crate::join::surface_blend;
pub use crate::join::radius_transition;
pub use crate::join::corner_blend;

pub use crate::construct::surfaces::bellows;

pub use crate::construct::surfaces::sphere_envelope;

pub use crate::intersection::parallel_sections;

pub use crate::offset::machining as planar_toolpath;

#[path = "join/surface_g2_pcurves.rs"]
pub mod surface_g2_pcurves;
#[path = "join/surface_g1_pcurves.rs"]
pub mod surface_g1_pcurves;

pub use crate::analysis::surface_coordinate_flow;

pub use crate::construct::surfaces::worm_wheel;

pub use crate::analysis::surface_occlusion;
pub use crate::construct::surfaces::wave_surface;
pub use crate::trim::periodic_trim;
pub use crate::construct::surfaces::superellipsoid;
pub use crate::construct::surfaces::optical_shapes;
pub use crate::construct::surfaces::wing_loft;
pub use crate::construct::surfaces::profile_spring;
pub use crate::construct::curves::radial_cam;
pub use crate::distance::synchronized_boundary_normals;
/// Whole original-span strict coordinate clocks for rational curves.
pub use crate::analysis::curve_coordinate_clock;

pub use crate::construct::surfaces::original_graph_transition;
