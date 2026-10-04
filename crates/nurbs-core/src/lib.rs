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
/// Task-oriented API views; existing flat module paths remain supported.
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
pub use crate::core::affine;
pub use crate::tessellation::shared_tessellation;
pub use crate::intersection::self_curve as curve_self_intersection;
pub use crate::intersection::self_surface as surface_self_intersection;
pub use crate::analysis::curve_pullback;
pub use crate::construct::surfaces::function_surface;
pub use crate::join::periodic_seam;
pub use crate::join::curve_join;
pub use numerics::conditioning;
pub use crate::core::coordinate_frame;
pub use crate::construct::surfaces::sections::sections;
pub use crate::construct::surfaces::sections::section_projection;
pub use crate::construct::surfaces::sections::circle_repair as section_circle_repair;
pub use crate::construct::surfaces::sections::phase as section_phase;
pub use crate::editing::weight_edit;
pub use crate::construct::curves::engineering_profiles;
pub use crate::construct::surfaces::boundary_fill;
pub use crate::core::bounds;
pub use crate::construct::curves::closed_spline;
pub use crate::join::continuity;
pub use crate::construct::surfaces::coons;
pub use crate::core::curve;
pub use crate::analysis::curve_differential;
pub use crate::analysis::surface_differential;
pub use crate::analysis::curve_measure;
pub use crate::analysis::curve_extrema;
pub use crate::analysis::curve_analysis;
pub use crate::core::canonical;
pub use crate::editing::direct_edit;
pub use crate::editing::morph;
pub use crate::editing::fairing;
pub use crate::analysis::geodesic;
pub use crate::analysis::unrolling;
pub use crate::construct::curves::biarc;
pub use crate::construct::curves::curve_extension;
pub use numerics::interval_eval;
pub use crate::distance::curve_distance;
pub use crate::certificates::curve_decomposition_certificate;
use numerics::exact_curve_segments;
pub use crate::analysis::curve_deviation;
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
pub use crate::construct::surfaces::grid_spline;
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
pub use crate::construct::curves::hermite;
pub use crate::construct::surfaces::hermite_patch;
pub use crate::construct::surfaces::loft::natural_loft;
pub use crate::construct::surfaces::loft::guided_loft;
pub use crate::construct::surfaces::loft::continuity as loft_continuity;
pub use crate::construct::surfaces::loft::alignment as loft_alignment;
pub use crate::construct::curves::natural_spline;
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
use numerics::curve_jets;
pub use crate::offset::curve_offset;
pub use crate::analysis::frechet;
pub use crate::construct::curves::curve_reconstruction;
pub(crate) use crate::offset::diagnostics as curve_offset_diagnostics;
pub use crate::offset::curve_offset_join;
pub use crate::offset::curve_offset_wire;
pub use crate::analysis::curve_surface_agreement;
pub(crate) use crate::analysis::curve_surface_composition;
pub mod intersection;
use numerics::periodic_chart;
pub use crate::analysis::radial_bounds;
pub use crate::analysis::ray_surface;
pub use crate::intersection::ss_intersection;
pub use crate::core::surface;
pub use crate::analysis::surface_measure;
pub use crate::analysis::curve_regularity;
pub use crate::sweeps::audit::pair as sweep_pair_audit;
pub use crate::analysis::surface_injectivity;
pub use crate::analysis::surface_regularity;
pub use crate::join::surface_join;
pub use crate::join::surface_g1;
pub use crate::join::surface_g2;
pub use crate::analysis::surface_shape_operator;
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
pub use crate::sweeps::audit::seam as sweep_seam_audit;
pub use crate::trim::trim_domain;
pub use crate::trim::trim_region_audit;
pub use crate::trim::trim_simplicity;
pub use crate::distance::trimmed_surface_distance;

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

pub use crate::sweeps::progressive_miter;

pub use crate::sweeps::helical_sweep;

pub use crate::construct::curves::rack;

pub use crate::construct::curves::thread;

use numerics::exact_products;

pub use crate::analysis::surface_parameter_bounds;

pub use crate::construct::curves::curve_chain;
pub use numerics::compensated;
pub use numerics::dual;
pub use numerics::robust_solvers;
pub use numerics::interval_newton;
pub use numerics::sturm;
pub use numerics::convex_distance;
pub use numerics::obb_tree;
pub use numerics::normal_cone;
pub use numerics::bezier_extraction;
