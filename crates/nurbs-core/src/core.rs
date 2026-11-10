//! Core types and point semantics: curves, surfaces, bounds, affine frames.
pub mod affine;
pub mod bounds;
pub mod canonical;
pub mod coordinate_frame;
pub mod geometry;
pub mod curve;
pub mod surface;
/// U/V exchange of a surface (formerly `editing::surface_edit::transpose`).
pub mod surface_transpose;
pub(crate) mod vec3_ext;
/// Non-empty knot spans and natural parameter domains (formerly `certificates::audit`).
pub(crate) mod span_domains;
pub(crate) mod curve_identity;

pub(crate) mod bounded_rational;
/// Formerly `numerics::simd_kernels`; moved above `core` by ADR 0012.
pub mod simd_kernels;
/// Formerly `numerics::exact_curve_segments`; moved above `core` by ADR 0012.
pub(crate) mod exact_curve_segments;
