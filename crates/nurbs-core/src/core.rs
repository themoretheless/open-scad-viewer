//! Core types live in the `nurbs-geometry` crate (refactor plan O3); this module keeps the
//! `nurbs_core::core::*` paths.
pub use nurbs_geometry::sweep_support;
pub use nurbs_geometry::curve_point_identity;
pub use nurbs_geometry::curve_quadratic_separator;
pub use nurbs_geometry::curve_surface_plane;
pub use nurbs_geometry::retained_wall_coefficients;
pub use nurbs_geometry::surface_projection_jacobian;
pub use nurbs_geometry::conditioning;
pub use nurbs_geometry::affine;
pub use nurbs_geometry::bounds;
pub use nurbs_geometry::coordinate_frame;
pub use nurbs_geometry::geometry;
pub use nurbs_geometry::curve;
pub use nurbs_geometry::derivatives;
pub use nurbs_geometry::surface;
pub use nurbs_geometry::surface_transpose;
pub use nurbs_geometry::vec3_ext;
pub use nurbs_geometry::span_domains;
pub use nurbs_geometry::curve_identity;
pub use nurbs_geometry::bounded_rational;
pub use nurbs_geometry::simd_kernels;
pub use nurbs_geometry::exact_curve_segments;

#[cfg(test)]
#[path = "core/integration_tests.rs"]
mod integration_tests;
