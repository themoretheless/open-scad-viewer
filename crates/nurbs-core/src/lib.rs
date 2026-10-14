#![doc = include_str!("../README.md")]
//! Own binary64 rational B-spline kernel. No C/C++ or geometry dependency.
//! No polygon-core, WASM, browser or application dependency.
//! CAD B-rep over these curves/surfaces lives in `brep-core`.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod curve;
pub mod uv_curve_crossings;
pub mod coons;
pub mod continuity;
pub mod framed_sweep;
pub mod edit;
pub mod foundation;
pub mod trim_point;
pub mod curve_distance;
mod distance_bounds;
mod curve_jets;
pub mod curve_offset;
pub mod curve_offset_join;
pub mod curve_offset_wire;
mod curve_offset_diagnostics;
pub mod chord_intersection;
pub mod chord_arrangement;
pub mod chord_faces;
pub mod chord_embedding;
pub mod chord_winding;
pub mod chord_witness;
pub mod surface_distance;
pub mod surface_injectivity;
pub mod surface_quotient_injectivity;
pub mod surface_contact;
pub mod surface_contact_search;
pub mod radial_bounds;
pub mod ray_surface;
pub mod normal_alignment;
pub mod curve_surface_agreement;
pub mod curve_surface_plane;
pub mod curve_surface_affine;
mod curve_surface_composition;
mod periodic_chart;
pub mod trim_domain;
pub mod trim_simplicity;
pub mod trim_region_audit;
pub mod planar_area;
pub mod trimmed_surface_distance;
pub mod intersection;
pub mod ss_intersection;
pub mod surface;

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
fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
fn numeric(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, NUMERIC_ERROR, message)
}
#[cfg(feature = "transport")]
mod transport;
#[cfg(feature = "transport")]
pub use transport::{dispatch, execute};

#[cfg(all(test, feature = "transport"))]
#[path="tests.rs"]
mod tests;

pub mod chord_fill_selection;

pub mod natural_loft;
pub mod guided_loft;
pub mod loft_alignment;
pub mod loft_reparameterization;
pub mod loft_continuity;
pub mod gordon;
pub mod natural_spline;
pub mod closed_spline;
pub mod hermite;
pub mod grid_spline;
pub mod paths;
pub mod primitives;

pub mod retained_wall_coefficients;

pub mod sweeps;
mod sweep_support;
#[path="sweeps/audit/contour.rs"]
pub mod sweep_contour_audit;
#[path="sweeps/audit/pair.rs"]
pub mod sweep_pair_audit;
#[path="sweeps/audit/wall.rs"]
pub mod sweep_wall_audit;
pub mod curve_regularity;
pub mod surface_regularity;
pub mod surface_monotonicity;
pub mod surface_measure;
pub mod curve_decomposition_certificate;
pub mod polynomial;
pub use sweeps::progressive_miter;
pub use sweeps::progressive_sweep;
pub mod interval_eval;
#[path="sweeps/audit/cap_boundary.rs"]
pub mod sweep_cap_boundary;
pub mod curve_measure;

#[path="sweeps/audit/cap_wall.rs"]
pub mod sweep_cap_wall;

pub mod surface_linear_monotonicity;

pub mod section_projection;

pub mod affine;

pub mod section_circle_repair;

#[path="sweeps/audit/seam.rs"]
pub mod sweep_seam_audit;

pub mod retained_wall_domain;

pub mod sweep_seam_set;

pub mod sweep_section_correction;

pub mod surface_offset;

pub mod trimmed_offset_contact;

pub mod offset_source_boundary;

pub mod offset_contact_tangent;

pub mod offset_envelope;

pub mod offset_envelope_fit;

pub mod offset_contact_pcurve;

pub mod offset_contact_trims;

pub mod offset_face_loops;

pub mod offset_patch_boundary;

pub mod curve_partition_agreement;

pub mod boundary_partition;

pub mod contact_normal_agreement;

pub mod offset_contact_predictor;

pub mod offset_contact_path;

pub mod curve_axis_driver;

pub mod offset_path_trims;

pub mod offset_path_pcurves;

pub mod curve_surface_lift;

pub mod curve_point_identity;

pub mod surface_flux;

pub mod moving_radius;

pub mod moving_envelope;

pub mod surface_projection_jacobian;

pub mod curve_quadratic_separator;

pub mod surface_projected_jordan;

pub mod surface_self_chord;

pub mod surface_self_chord_coverage;

pub mod domains;

pub mod numerics;

pub mod core;

pub mod distance;

pub mod construct;

pub mod analysis;

pub mod editing;

pub mod join;

pub mod trim;

pub mod tessellation;

pub mod certificates;

pub mod offset;

pub mod retained_wall_domain_certificate;

pub use numerics::bezier_extraction;

pub use numerics::compensated;

pub use numerics::conditioning;

pub use numerics::convex_distance;

pub use numerics::dual;

pub use numerics::interval_newton;

pub use numerics::normal_cone;

pub use numerics::obb_tree;

pub use numerics::robust_solvers;

pub use numerics::simd_kernels;

pub use numerics::sparse_linalg;

pub use numerics::sturm;

pub use numerics::error_upper;

pub use core::bounds;

pub use core::canonical;

pub use core::coordinate_frame;

pub use crate::tessellation::shared_tessellation;

#[path="intersection/self_curve.rs"] pub mod curve_self_intersection;

#[path="intersection/self_surface.rs"] pub mod surface_self_intersection;

pub use crate::analysis::curve_pullback;

pub use crate::construct::surfaces::function_surface;

pub use crate::join::periodic_seam;

pub use crate::join::curve_join;

pub use crate::construct::surfaces::sections::sections;

pub use crate::construct::surfaces::sections::circle_sweep_repair;

pub use crate::construct::surfaces::sections::miter_section_correction;

pub use crate::construct::surfaces::sections::sweep_section_projection;

pub use crate::construct::surfaces::sections::phase as section_phase;

pub use crate::editing::weight_edit;

pub use crate::construct::curves::engineering_profiles;

pub use crate::construct::surfaces::boundary_fill;

pub use crate::analysis::curve_differential;

pub use crate::analysis::surface_differential;

pub use crate::analysis::curve_extrema;

pub use crate::analysis::curve_analysis;

pub use crate::editing::direct_edit;

pub use crate::editing::morph;

pub use crate::editing::fairing;

pub use crate::analysis::geodesic;

pub use crate::analysis::unrolling;

pub use crate::construct::curves::biarc;

pub use crate::construct::curves::curve_extension;

pub use crate::analysis::curve_deviation;

pub use crate::analysis::curve_plane;

pub use crate::tessellation::curve_tessellation;

pub use crate::tessellation::surface_tessellation;

pub use crate::distance::curve_surface_distance;

pub use crate::construct::curves::formula;

pub use crate::sweeps::profile_sweep;

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

pub use crate::construct::surfaces::hermite_patch;

pub use crate::construct::surfaces::patches;

pub use crate::sweeps::scaled_sweep;

pub use crate::construct::surfaces::triangular_patch;

pub use crate::sweeps::twist_sweep;

pub use crate::sweeps::two_guide_sweep;

pub use distance_bounds::DistanceStopReason;

pub use crate::analysis::frechet;

pub use crate::construct::curves::curve_reconstruction;

pub use crate::join::surface_join;

pub use crate::join::surface_g1;

pub use crate::join::surface_g2;

pub use crate::analysis::surface_shape_operator;

pub use crate::editing::surface_edit;









pub use crate::sweeps::helical_sweep;

pub use crate::construct::curves::rack;

pub use crate::construct::curves::thread;

pub use crate::analysis::surface_parameter_bounds;

pub use crate::construct::curves::curve_chain;

pub(crate) use numerics::exact_products;

#[path="distance/curve_distance.rs"] mod native_curve_distance;
#[path="distance/surface_distance.rs"] mod native_surface_distance;

#[path="analysis/curve_surface_composition.rs"] mod native_curve_surface_composition;
#[path="offset/curve_offset.rs"] mod native_curve_offset;

pub(crate) use numerics::exact_curve_segments;
