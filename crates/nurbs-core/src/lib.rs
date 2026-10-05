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
