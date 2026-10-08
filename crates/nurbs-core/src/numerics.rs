//! Numeric kernels: compensated arithmetic, interval methods, robust solvers,
//! bounding structures and exact predicates.
pub(crate) mod distance_bounds;
pub(crate) mod interval_vec3;
pub(crate) mod exact_curve_segments;
pub(crate) mod exact_products;
pub(crate) mod periodic_chart;
pub(crate) mod curve_jets;
pub mod bezier_extraction;
pub mod compensated;
pub mod conditioning;
pub mod convex_distance;
pub mod dual;
pub mod interval_eval;
pub mod interval_newton;
pub mod normal_cone;
pub mod obb_tree;
pub mod robust_solvers;
pub mod simd_kernels;
pub mod sparse_linalg;
pub mod sturm;

/// Certified nonnegative error-bound arithmetic.
pub mod error_upper;
