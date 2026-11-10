//! Numeric kernels: compensated arithmetic, interval methods, robust solvers,
//! bounding structures and exact predicates.
pub(crate) mod distance_bounds;
pub(crate) mod bernstein_product;
pub(crate) mod interval_vec3;
pub(crate) mod bernstein_tensor;
pub(crate) mod exact_products;
pub(crate) mod geometric_weights;
pub(crate) mod periodic_chart;
pub(crate) mod outward_interval;
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
