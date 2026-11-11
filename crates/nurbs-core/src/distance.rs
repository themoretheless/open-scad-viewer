//! Distance computations: curve-curve, surface-surface, curve-surface, and trimmed-surface distance.
pub(crate) mod curve_jets;
pub mod polar_surface_differential;
pub mod rational_pole_differential;
pub mod curve_axis_driver;
pub mod synchronized_boundary_normals;
pub mod uv_curve_crossings;
pub mod curve_deviation;
pub mod curve_distance;
pub mod curve_surface_distance;
pub(crate) mod subdivision;
pub mod surface_distance;
/// Interval enclosures of curve and surface evaluation (formerly `numerics::interval_eval`).
pub mod interval_eval;
/// Interval Newton root isolation (formerly `numerics::interval_newton`).
pub mod interval_newton;
/// Radial and axis-separation bounds of curves (formerly `analysis::radial_bounds`).
pub mod radial_bounds;
/// Formerly `numerics::convex_distance`; moved above `core` by ADR 0012.
pub mod convex_distance;
/// Formerly `numerics::obb_tree`; moved above `core` by ADR 0012.
pub mod obb_tree;
