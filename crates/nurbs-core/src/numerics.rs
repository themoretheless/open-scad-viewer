//! Numeric kernels live in the `nurbs-numerics` crate (refactor plan O3); this module keeps
//! the `nurbs_core::numerics::*` paths.
pub(crate) use nurbs_numerics::distance_bounds;
pub(crate) use nurbs_numerics::bernstein_product;
pub(crate) use nurbs_numerics::interval_vec3;
pub(crate) use nurbs_numerics::bernstein_tensor;
pub(crate) use nurbs_numerics::exact_products;
pub(crate) use nurbs_numerics::geometric_weights;
pub(crate) use nurbs_numerics::periodic_chart;
pub(crate) use nurbs_numerics::outward_interval;
pub use nurbs_numerics::compensated;
pub use nurbs_numerics::robust_solvers;
pub use nurbs_numerics::sparse_linalg;
pub use nurbs_numerics::sturm;
pub use nurbs_numerics::error_upper;
pub use nurbs_numerics::bernstein_interpolation;
pub use nurbs_numerics::guards;

#[cfg(test)]
#[path = "numerics/integration_tests.rs"]
mod integration_tests;
