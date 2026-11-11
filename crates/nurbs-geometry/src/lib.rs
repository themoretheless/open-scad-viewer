//! Core NURBS types and point semantics: curves, surfaces, bounds, affine frames and their
//! identities. Split out of `nurbs-core` (refactor plan O3); `nurbs_core::core` and the
//! crate-root aliases re-export every module.
pub use math_core::{Error, Result};
#[doc(hidden)]
pub use nurbs_numerics::{INVALID_INPUT, NUMERIC_ERROR, RESOURCE_LIMIT, check, input, numeric, numeric_err, resource};
/// Numeric kernels, kept reachable under the paths these modules used inside `nurbs-core`.
#[doc(hidden)]
pub mod numerics {
    pub use nurbs_numerics::*;
}
#[doc(hidden)]
pub use nurbs_numerics::{bernstein_interpolation, bernstein_product, bernstein_tensor, compensated, distance_bounds, error_upper, exact_products, geometric_weights, guards, interval_vec3, outward_interval, periodic_chart, robust_solvers, sparse_linalg, sturm};

#[doc(hidden)]
pub mod sweep_support;
pub mod curve_point_identity;
pub mod curve_quadratic_separator;
pub mod curve_surface_plane;
pub mod retained_wall_coefficients;
pub mod surface_projection_jacobian;
pub mod conditioning;
pub mod affine;
pub mod bounds;
pub mod coordinate_frame;
pub mod geometry;
pub mod curve;
pub mod surface;
/// U/V exchange of a surface (formerly `editing::surface_edit::transpose`).
pub mod surface_transpose;
#[doc(hidden)]
pub mod vec3_ext;
/// Non-empty knot spans and natural parameter domains (formerly `certificates::audit`).
#[doc(hidden)]
pub mod span_domains;
#[doc(hidden)]
pub mod curve_identity;
#[doc(hidden)]
pub mod bounded_rational;
/// Formerly `numerics::simd_kernels`; moved above `core` by ADR 0012.
pub mod simd_kernels;
/// Formerly `numerics::exact_curve_segments`; moved above `core` by ADR 0012.
#[doc(hidden)]
pub mod exact_curve_segments;
