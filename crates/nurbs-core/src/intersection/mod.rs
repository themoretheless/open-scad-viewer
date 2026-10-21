//! Certified bounded general NURBS curve/curve and curve/surface intersection.
//!
//! Coverage uses outward-rounded Bernstein/interval hull exclusion, half-open
//! knot ownership, Krawczyk uniqueness on terminal boxes, and derivative-order
//! contact classification. Unresolved appears only at resource or conditioning
//! boundaries. Reports carry ToleranceContext evidence and CoedgeTrim maps.
use crate::{Result, check, curve::Curve, surface::Surface};
use cad_predicates::ToleranceContext;
pub(crate) use math_core::{cross as cross3, dot as dot3};
#[cfg(feature = "codec")]
pub use serialization::resource_boundary_probe;
#[cfg(feature = "codec")]
pub(crate) use serialization::tolerance_evidence;
#[path = "../intersection/plane.rs"]
mod plane;
use plane::{invert_plane_uv, plane_of};
#[path = "../intersection/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{intersect_curve_curve, intersect_curve_curve_bounded, intersect_curve_surface};
#[path = "../intersection/curve_surface.rs"]
mod curve_surface;
pub use curve_surface::*;
#[path = "../intersection/curve_curve.rs"]
mod curve_curve;
pub use curve_curve::*;
#[path = "../intersection/bezier_clip.rs"]
pub(crate) mod bezier_clip;
/// Curve self-intersection audit (formerly `crate::curve_self_intersection`).
pub mod self_curve;
/// Surface self-intersection audit (formerly `crate::surface_self_intersection`).
pub mod self_surface;
/// Certified surface/surface intersection (formerly `crate::ss_intersection`).
pub mod ss_intersection;
/// Contact certificates for surface/surface sections (formerly `crate::surface_contact`).
pub mod surface_contact;
/// Conservative contact search over two surface domains (formerly `crate::surface_contact_search`).
pub mod surface_contact_search;
/// Adaptive octree isosurface extraction from SDFs: dual contouring with QEF,
/// marching cubes, Taubin smoothing and small-component cleanup (items 941, 967).
pub mod isosurface;
pub mod planar_scalar;
pub mod planar_coincidence;

pub(crate) const MAX_BOXES: usize = 8192;
const MAX_DEGREE: usize = 25;
const MAX_CONTROLS: usize = 256;
pub(crate) const MAX_SPANS: usize = 4096;
const TRANSVERSE_SINE: f64 = 1e-8;
pub(crate) use math_core::{next_down, next_up};

const VERSION: &str = "nurbs-foundation/5";

mod context;
mod intersect_curve_surface_report;
#[cfg(test)]
mod original_curve_exclusion_tests;
pub(crate) use context::*;
pub use intersect_curve_surface_report::*;

#[cfg(test)]
mod bounded_curve_tests;





/// Original-surface parallel plane sections with explicit unresolved charts.
pub mod parallel_sections;
mod planar_curve_curve;
pub use planar_curve_curve::{CertifiedCurveCrossing, CertifiedCurveIntersection, intersect_curve_curve_certified_report};
mod certified_curve_plane;
pub mod certified_curve_surface;
pub use certified_curve_plane::{CertifiedPlaneCrossing, CertifiedCurvePlaneIntersection, intersect_curve_plane_certified_report};
