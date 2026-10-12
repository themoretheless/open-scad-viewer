//! Analytic sphere/cylinder intersection for canonical solids in the axial
//! configuration (sphere center certified on the cylinder axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the cylinder
//! operand must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches, two bilinear
//! caps with inscribed-circle trims), either optionally carried through a
//! rigid affine placement. Anything else is an explicit `UnsupportedSurface`
//! region — this cell never falls back to numerical surface/surface
//! subdivision. Axiality is certified, never forced: a perpendicular offset
//! at pure rounding scale snaps to the axis, an offset inside the
//! recognition-scale band reports a `NearCoincidence` region, and a clearly
//! off-axis pair is `UnsupportedSurface` (the general sphere/cylinder pair is
//! a quartic, out of scope). Classification uses outward binary64 bands
//! widened by the observed recognition deviation. Resolved contacts are exact
//! rational circles (four 90-degree arcs, weights cos(pi/4)) with UV lifts on
//! both surfaces: iso-v lines across all four cylinder side patches, an exact
//! UV circle on a cap face, and per-patch UV circles/lines on the sphere.
//! Every tangency — cap-plane touch, rim (cap circle radius == cylinder
//! radius), the r == R coincident band — stays an explicit unresolved region;
//! tangent contacts are never reported as point or guessed-circle components,
//! matching the house tangency discipline. Nothing here authorizes a topology
//! change.
use brep_core::intersections::sphere_sphere::{
    self, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_arcs, circle_curve, lift,
};
use brep_core::intersections::{CanonicalCylinder, recognize_cylinder};
use super::*;
use brep_core::Model;

mod cylinder_patch_curve;
#[cfg(test)]
mod tests;
pub use cylinder_patch_curve::*;
