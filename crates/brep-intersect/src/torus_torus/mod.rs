//! Analytic torus/torus intersection for canonical solids in the coaxial
//! configuration (the two axes certified parallel and coincident; the two
//! center planes may differ by an axial offset h).
//!
//! Both operands must be the exact canonical ring-torus solid of
//! `analytic::torus` (sixteen rational biquadratic patches, strict ring tori
//! only — the constructor refuses horn and spindle tori), recognized by
//! `plane_torus::recognize_torus`. Rigid affine placements are admitted.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: pure-rounding axis tilts and center-line offsets
//! snap, recognition-scale tilts or offsets report `NearCoincidence`, and a
//! clearly tilted or off-axis pair is `UnsupportedSurface` (the general
//! torus/torus pair is a quartic, out of scope). Classification uses outward
//! binary64 bands widened by both recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section reduces to the meridian half-plane (rho >= 0, z measured along
//! the first torus's axis from its center plane): circle
//! (rho - R_1)^2 + z^2 = r_1^2 against circle
//! (rho - R_2)^2 + (z - h)^2 = r_2^2 with h the axial offset of the second
//! center plane. Circle/circle in the meridian plane (the same closed form
//! as sphere_torus: a = (d^2 + r_1^2 - r_2^2)/(2d), l^2 = r_1^2 - a^2 with d
//! the meridian center distance between (R_1, 0) and (R_2, h)) gives zero,
//! one (double) or two roots; each root (rho*, z*) revolves into the exact
//! 3D circle of radius rho* at height z*:
//! two distinct roots with rho* provably beyond the band revolve into two
//!   exact circles; a one-circle resolved configuration is structurally
//!   unreachable for a strict ring-torus pair (every meridian root keeps
//!   rho* >= R - r >= 1e-5, far beyond the band) and is documented honestly
//!   rather than tested;
//! a double root (the meridian circles tangent, d == r_1 + r_2 externally
//!   or d == |r_1 - r_2| internally) stays `TangencyOrMultipleRoot` — the
//!   tangent contact revolves into a circle but is never a guessed circle;
//! a root whose radius rho* collapses into the band degenerates to a pole
//!   point on the axis — unresolved, never guessed (unreachable for a
//!   canonical strict ring torus, kept as an honest guard);
//! equal meridian circles (R_1 == R_2, h == 0, r_1 == r_2 within the
//!   bands) are coincident surfaces and report `CoincidentTrim`, never a
//!   guessed curve;
//! no roots (provably separate d > r_1 + r_2, or provably contained
//!   d + min(r_1, r_2) < max(r_1, r_2), beyond the band) resolves empty by
//!   the interval signs.
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on BOTH tori: iso-v parallels at the
//! exact profile angle phi = atan2(z*, rho* - R) mapped through the exact
//! rational 90-degree-arc parameter map t = s/(1+s),
//! s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one degree-1 line per revolution
//! quadrant patch of the profile row, on both 4x4 tilings (the second
//! torus's profile angle is measured in its own frame — its axis may be
//! anti-parallel to the first). Nothing here authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
#[cfg(test)]
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use brep_core::intersections::sphere_sphere::{RECOGNITION, circle_curve};
use super::*;
use brep_core::Model;

mod torus_torus_component;
#[cfg(test)]
mod tests;
pub use torus_torus_component::*;
