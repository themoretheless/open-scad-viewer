//! Analytic plane/torus intersection for a canonical planar patch operand
//! and a canonical ring-torus solid, in the axial configurations only.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, plane operand first). The torus
//! operand must be the exact canonical ring torus of `analytic::torus`:
//! sixteen rational biquadratic patches tiling four profile quadrants times
//! four revolution quadrants, with exact tensor-product weights, exact
//! unit-square boundary trims and sixteen ring/quadrant vertices, optionally
//! carried through a rigid affine placement (`recognize_torus` below). The
//! canonical constructor admits only strict ring tori (R - r >= 1e-5): horn
//! (r == R) and spindle (r > R) tori are refused at construction, so the
//! recognizer never certifies one and this cell never sees a
//! self-intersecting torus. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision.
//!
//! Only axial plane placements are analytic here:
//! Plane perpendicular to the torus axis at axial height h from the center
//!   plane: |h| > r (clear beyond the outward band) resolves empty; |h| == r
//!   within the band is a tangency and stays `TangencyOrMultipleRoot`, never
//!   a guessed point or circle; |h| < r yields the two exact circles of
//!   radii R +- sqrt(r^2 - h^2) centered on the axis in the plane (h == 0 is
//!   the equator pair R +- r of the same branch). The circles are clipped by
//!   the patch's finite rectangular domain with exact eccentric-angle trims
//!   (`plane_sphere::clip_ellipse`); a tangency to a domain edge stays
//!   unresolved. An inner radius R - sqrt(r^2 - h^2) collapsing into the
//!   band would be an inner-tangency/horn configuration and stays
//!   unresolved (unreachable for a canonical strict ring torus, where
//!   R - r >= 1e-5; kept as an honest guard, never guessed through).
//! Plane containing the axis (normal perpendicular to the axis, center in
//!   the plane at pure-rounding scale): the two exact meridian circles of
//!   radius r centered at +-R along the in-plane line where the plane meets
//!   the center plane, clipped the same way. A parallel-to-axis plane at a
//!   certified nonzero distance cuts a Cassini oval, not a circle pair:
//!   clearly offset planes are `UnsupportedSurface`, and an offset inside
//!   the recognition-scale band reports `NearCoincidence` — never snapped.
//! Oblique planes are a quartic section with Villarceau degeneracies:
//!   clearly oblique planes are `UnsupportedSurface`, tilts inside the
//!   recognition-scale band off either axial configuration report
//!   `NearCoincidence`.
//! A plane can never coincide with a curved torus patch, so `CoincidentTrim`
//! is deliberately unused by this cell.
//!
//! Resolved contacts are exact rational circles (full: four 90-degree arcs,
//! weights cos(pi/4), knots 0..=4; clipped: exact swept arcs) with UV lifts
//! on both operands: on the plane, exact rational quadratic ellipse arcs per
//! the `recognize_plane` metric; on the torus the perpendicular-plane
//! circles are iso-v parallels (constant exact v over the profile quadrant,
//! u clipped per revolution quadrant) and the through-axis circles are
//! iso-u meridians (constant exact u over the revolution quadrant, v
//! clipped per profile quadrant) — both are degree-1 UV line segments on
//! the exact patch grid, one per covered patch. Classification uses outward
//! binary64 bands widened by both recognition deviations; the angular snap
//! is 64 binary64 epsilons plus the recognition-error angular allowances,
//! and recognition-scale tilts report `NearCoincidence`, never forced.
//! Nothing here authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::plane_sphere::{
    EllipseClip, PlanePatchCurve, clip_ellipse, conic_arcs_2d, conic_sweep,
};
use brep_core::intersections::sphere_sphere::{RECOGNITION, ccw_intersect, circle_curve};
pub(crate) use brep_core::intersections::{CanonicalTorus, recognize_torus};
use super::*;
use brep_core::Model;


type TorusLift<'a> = dyn Fn(&CanonicalTorus, &[(f64, f64)]) -> Vec<TorusPatchCurve> + 'a;

mod torus_patch_curve;
#[cfg(test)]
mod tests;
pub use torus_patch_curve::*;
