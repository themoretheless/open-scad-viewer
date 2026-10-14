//! Typed curve/curve and curve/surface intersection reports.
//!
//! Compiled beside the legacy `Value` queries of the parent module, which keep
//! their own implementations and wire format; only typed names are exported.
use crate::{Result, check, curve::Curve, surface::Surface};
use cad_predicates::ToleranceContext;
pub(crate) use math_core::{cross as cross3, dot as dot3};
#[cfg(feature = "codec")]
#[path = "../serialization.rs"]
mod serialization;
#[cfg(feature = "codec")]
#[allow(unused_imports)]
pub(crate) use serialization::tolerance_evidence;
#[path = "../plane.rs"]
mod plane;
use plane::{invert_plane_uv, plane_of};
#[path = "../curve_surface.rs"]
mod curve_surface;
pub use curve_surface::*;
#[path = "../curve_curve.rs"]
mod curve_curve;
pub use curve_curve::*;
#[path = "../bezier_clip.rs"]
pub(crate) mod bezier_clip;

pub(crate) const MAX_BOXES: usize = 8192;
const MAX_DEGREE: usize = 25;
const MAX_CONTROLS: usize = 256;
pub(crate) const MAX_SPANS: usize = 4096;
const TRANSVERSE_SINE: f64 = 1e-8;
#[cfg(feature = "codec")]
const VERSION: &str = "nurbs-foundation/5";
pub(crate) use math_core::{next_down, next_up};

mod context;
mod intersect_curve_surface_report;
pub(crate) use context::*;
pub use intersect_curve_surface_report::*;
