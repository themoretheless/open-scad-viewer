//! Conservative, deterministic certificates used by product NURBS consumers.
//!
//! Every geometric enclosure is derived from positive-weight convex-hull
//! properties. `next_down`/`next_up` make binary64 arithmetic outward rounded.
//! Ambiguous regularity and projection cases remain explicitly unresolved.
use crate::{
    Result, check,
    curve::Curve,
    numeric, resource,
    surface::{Axis, Surface},
};
use cad_predicates::ToleranceContext;
use math_core::{next_down, next_up};
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
use serialization::tolerance_evidence;
#[cfg(feature = "codec")]
pub use serialization::{project_curve, project_surface};

pub mod approximate_edits;
#[cfg(feature = "codec")]
pub use approximate_edits::serialization::{
    rebuild_curve, rebuild_surface, reduce_curve_degree, reduce_surface_axis, remove_curve_knot,
};
pub use approximate_edits::{
    SurfaceSimplification, rebuild_curve_report, rebuild_surface_report,
    reduce_curve_degree_report, reduce_surface_axis_report, remove_curve_knot_report,
};
pub mod periodic_edits;
use periodic_edits::seam_certificate;
#[cfg(feature = "codec")]
pub use periodic_edits::serialization::{
    edit_periodic_curve, edit_periodic_surface, split_periodic_curve,
};
pub use periodic_edits::{
    PeriodicEditOperation, edit_periodic_curve_report, edit_periodic_surface_report,
    split_periodic_curve_report,
};
pub mod approximation;
#[cfg(feature = "codec")]
pub use approximation::serialization::{approximate_curve, interpolate_polyline};
pub use approximation::{approximate_curve_report, interpolate_polyline_report};
pub mod exact_edits;
#[cfg(feature = "codec")]
pub use exact_edits::serialization::{certify_exact_edit, reparameterize_curve};
pub use exact_edits::{ExactEditOperation, certify_exact_edit_report, reparameterize_curve_report};
pub mod fitting;
pub mod min_zone_fit;
pub mod interpolation;
pub mod bracketed;
pub mod oslo;
pub mod knot_removal;
#[cfg(feature = "codec")]
pub use fitting::serialization::{
    fit_curve_cloud_certified, fit_curve_points, fit_surface_cloud_certified,
    interpolate_surface_grid,
};
pub use fitting::{
    fit_curve_cloud_certified_report, fit_curve_points_report, fit_surface_cloud_certified_report,
    interpolate_surface_grid_report,
};
pub mod parameter_mapping;
#[cfg(feature = "codec")]
pub use parameter_mapping::serialization::{
    certify_reparameterization, evaluate_reparameterized_curve, materialize_reparameterized_curve,
};
pub use parameter_mapping::{
    MapPiece, ParameterMapping, certify_reparameterization_report,
    evaluate_reparameterized_curve_report, materialize_reparameterized_curve_report,
};
pub mod certificates;
#[cfg(test)]
use certificates::{
    NormalBoxClass, SurfaceRegularityClass, certify_surface_cell, classify_normal_box,
};
#[cfg(feature = "codec")]
pub use certificates::{certify_curve, certify_surface};
pub use certificates::{certify_curve_report, certify_surface_report};
mod surface_projection;
pub use surface_projection::{
    BoundaryProjection, SurfaceCandidateClassification, SurfaceProjection,
    SurfaceProjectionCandidate, SurfaceProjectionProof, project_surface_report,
};

mod common;
pub(crate) use common::*;
mod curve_projection;
pub use curve_projection::{
    CandidateClassification, CurveProjection, ProjectionCandidate, ProjectionStatus,
    project_curve_report,
};
#[cfg(test)]
#[path = "foundation/tests/surface_reduction_validation_tests.rs"]
mod surface_reduction_validation_tests;

#[cfg(test)]
#[path = "foundation/tests/deviation_span_tests.rs"]
mod deviation_span_tests;

#[cfg(test)]
#[path = "foundation/tests/rebuild_tests.rs"]
mod rebuild_tests;
#[cfg(test)]
#[path = "foundation/tests/surface_rebuild_tests.rs"]
mod surface_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/periodic_rebuild_tests.rs"]
mod periodic_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/periodic_surface_rebuild_tests.rs"]
mod periodic_surface_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/normal_regularity_regressions.rs"]
mod normal_regularity_regressions;
