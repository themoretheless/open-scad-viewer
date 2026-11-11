//! Numeric kernels of the NURBS stack: compensated arithmetic, interval methods, robust
//! solvers, bounding structures and exact predicates. Split out of `nurbs-core` (refactor
//! plan O3); `nurbs_core::numerics` re-exports every module.
// `unwrap`/`expect` in production code are counted per crate (`clippyUnwrap` pins, plan L6)
// and may only go down; prefer typed errors.
#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
pub use math_core::{Error, Result};

#[doc(hidden)]
pub const INVALID_INPUT: &str = "NURBS_INVALID_INPUT";
#[doc(hidden)]
pub const NUMERIC_ERROR: &str = "NURBS_NUMERIC_ERROR";
#[doc(hidden)]
pub const RESOURCE_LIMIT: &str = "NURBS_RESOURCE_LIMIT";
#[doc(hidden)]
pub fn input(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[doc(hidden)]
pub fn numeric_err(message: impl Into<String>) -> Error {
    Error::new(NUMERIC_ERROR, message)
}
#[doc(hidden)]
pub fn resource(message: impl Into<String>) -> Error {
    Error::new(RESOURCE_LIMIT, message)
}
#[doc(hidden)]
pub fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
#[doc(hidden)]
pub fn numeric(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, NUMERIC_ERROR, message)
}

#[doc(hidden)]
pub mod distance_bounds;
#[doc(hidden)]
pub mod bernstein_product;
#[doc(hidden)]
pub mod interval_vec3;
#[doc(hidden)]
pub mod bernstein_tensor;
#[doc(hidden)]
pub mod exact_products;
#[doc(hidden)]
pub mod geometric_weights;
#[doc(hidden)]
pub mod periodic_chart;
#[doc(hidden)]
pub mod outward_interval;
pub mod compensated;
pub mod robust_solvers;
pub mod sparse_linalg;
pub mod sturm;
/// Certified nonnegative error-bound arithmetic.
pub mod error_upper;
/// Formerly the flat `crate::bernstein_interpolation` module; `lib.rs` keeps that path as a re-export.
pub mod bernstein_interpolation;
/// Finite-value checks, budgets and canonical hashing (formerly `foundation::guards`).
pub mod guards;
