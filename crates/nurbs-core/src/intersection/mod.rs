//! Certified bounded general NURBS curve/curve and curve/surface intersection.
//!
//! Coverage uses outward-rounded Bernstein/interval hull exclusion, half-open
//! knot ownership, Krawczyk uniqueness on terminal boxes, and derivative-order
//! contact classification. Unresolved appears only at resource or conditioning
//! boundaries. Reports carry ToleranceContext evidence and CoedgeTrim maps.
use crate::{Result, check, curve::Curve, resource, surface::Surface};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

pub(crate) const MAX_BOXES: usize = 8192;
const MAX_DEGREE: usize = 25;
const MAX_CONTROLS: usize = 256;
pub(crate) const MAX_SPANS: usize = 4096;
const TRANSVERSE_SINE: f64 = 1e-8;
const VERSION: &str = "nurbs-foundation/5";

mod next_down;
mod intersect_curve_curve;
mod split_grid_v;
mod intersect_curve_surface;
pub(crate) use next_down::*;
pub use intersect_curve_curve::*;
pub(crate) use split_grid_v::*;
pub use intersect_curve_surface::*;

mod typed;
pub use typed::{
    CurveCurveIntersection, CurveCurveReport, CurveSurfaceComponent, CurveSurfaceIntersection,
    intersect_curve_curve_bounded_report, intersect_curve_curve_report,
    intersect_curve_surface_report,
};
mod contact_types;
pub use contact_types::{ContactClass,UnresolvedReason};
