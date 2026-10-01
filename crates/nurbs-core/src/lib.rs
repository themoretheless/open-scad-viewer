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
pub mod surface_contact;
pub mod surface_contact_search;
pub mod radial_bounds;
pub mod ray_surface;
pub mod curve_surface_agreement;
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
