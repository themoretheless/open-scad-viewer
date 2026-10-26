//! Analytic sphere/sphere intersection for canonical stereographic sphere solids.
//!
//! Both operands must be exact canonical spheres as built by `analytic::sphere`
//! (eight regular rational biquadratic stereographic patches over ordinary pole
//! vertices, quarter-disk UV trims), optionally carried through a rigid affine
//! placement. Anything else is an explicit `UnsupportedSurface` region — this
//! cell never falls back to numerical surface/surface subdivision. Center
//! classification uses outward binary64 interval arithmetic: separate and
//! contained-without-contact pairs resolve empty, concentric equal (and
//! concentric near-equal within the recognition band) pairs report a
//! `CoincidentTrim` region, and every tangency or within-error tangency band
//! reports `TangencyOrMultipleRoot` — tangent contacts are never reported as
//! point components, matching the numerical queries' tangency discipline.
//! A transverse pair yields the exact rational circle (four 90-degree arcs,
//! weights cos(pi/4)) plus, for each sphere, the circle's exact per-patch UV
//! lifts: planar sections of a stereographic patch are exact circles (or lines
//! through the UV origin when the plane contains the patch pole) clipped to
//! the patch quarter-disk. Nothing here authorizes a topology change.
mod frames;
pub mod pole_charts;
#[cfg(feature = "codec")]
mod serialization;
use super::*;
use crate::Model;
pub use pole_charts::{ImplicitSphereSection, WorldCircleClip, recognize_any};

/// Recognition tolerance relative to radius for the canonical structure
/// screen (vertex distances, frame orthonormality, control points).
pub const RECOGNITION: f64 = 1e-9;
/// Relative scale under which the UV circle degenerates to a UV line (the
/// section plane contains the patch pole, e.g. a great circle whose plane is
/// perpendicular to the sphere axis).
const UV_LINE: f64 = 1e-12;
const TAU: f64 = std::f64::consts::TAU;
const LINEAR: [f64; 3] = [0., 0.5, 1.];
const SQUARE: [f64; 3] = [0., 0., 1.];
pub const ARC_WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;

mod lift_clipped;
mod sphere_patch_circle;
#[cfg(test)]
mod tests;
pub use lift_clipped::*;
pub use sphere_patch_circle::*;
