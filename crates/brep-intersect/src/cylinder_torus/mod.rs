//! Analytic cylinder/torus intersection for canonical solids in the coaxial
//! configuration (the cylinder axis certified coincident with the torus
//! axis).
//!
//! The cylinder operand must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches, two bilinear
//! caps with inscribed-circle trims), recognized by
//! `sphere_cylinder::recognize_cylinder`; the torus operand must be the exact
//! canonical ring-torus solid of `analytic::torus` (sixteen rational
//! biquadratic patches tiling four profile quadrants times four revolution
//! quadrants with exact tensor-product weights, strict ring tori only —
//! the constructor refuses horn and spindle tori), recognized by
//! `plane_torus::recognize_torus`. Rigid affine placements are admitted.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: pure-rounding axis tilts and center-line offsets
//! snap, recognition-scale tilts or offsets report `NearCoincidence`, and a
//! clearly tilted or off-axis pair is `UnsupportedSurface` (the general
//! cylinder/torus pair is a quartic, out of scope). Classification uses
//! outward binary64 bands widened by both recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section reduces to the meridian half-plane (rho >= 0, z): the cylinder
//! side is the vertical line rho = R_c and the torus side is the circle
//! (rho - R)^2 + z^2 = r^2.
//! The line provably missing the circle (|R_c - R| > r beyond the band)
//! yields no side contact; the tangent line (|R_c - R| == r within the
//! band, contact at z == 0 — the one configuration where the two square
//! roots would coincide) stays `TangencyOrMultipleRoot`, never a guessed
//! circle; otherwise z = +-sqrt(r^2 - (R_c - R)^2) are two distinct exact
//! circles of radius R_c. A resolved branch therefore always yields two
//! distinct circles — a single-circle meridian resolution is structurally
//! unreachable (it would require the tangent double root), and this is
//! documented rather than tested. The circles are clipped by the cylinder's
//! finite height: a circle provably beyond a cap plane is absent, and a
//! circle landing on a cap plane within the band is the rim tangency —
//! unresolved, never guessed.
//! Cap-plane contacts: the cap plane at cylinder axial position q
//! crosses the torus at torus-axial height h_c; when |h_c| < r is provable
//! the plane cuts the two exact circles of radii R +- sqrt(r^2 - h_c^2)
//! (the plane/torus perpendicular branch, reduced here to the cap disk):
//! a circle provably inside the cap disk (radius < R_c) is a cap circle
//! component, a radius equal to R_c within the band is the rim tangency
//! (unresolved), and a radius provably beyond R_c is absent. |h_c| == r
//! within the band is the plane/tube tangency — unresolved. An inner
//! radius R - sqrt(r^2 - h_c^2) collapsing into the band would degenerate
//! to a pole point; it is unreachable for a canonical strict ring torus
//! (R - r >= 1e-5) and kept as an honest guard, never guessed through.
//! A cylinder side can never coincide with a curved torus patch, so
//! `CoincidentTrim` is deliberately unused by this cell.
//!
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on both surfaces: on the cylinder side
//! the iso-v line at the exact height fraction on all four side patches, on
//! a cap disk the exact UV circle of radius rho/(2R_c) about [1/2, 1/2], and
//! on the torus the iso-v parallels at the exact profile angle
//! phi = atan2(z, rho - R) mapped through the exact rational 90-degree-arc
//! parameter map t = s/(1+s), s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one
//! degree-1 line per revolution quadrant patch of the profile row. Nothing
//! here authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::plane_torus::{
    CanonicalTorus, TorusPatchCurve, lift_parallel, recognize_torus, torus_residual,
};
use super::sphere_cylinder::{CanonicalCylinder, CylinderPatchCurve, recognize_cylinder};
#[cfg(test)]
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use brep_core::intersections::sphere_sphere::{RECOGNITION, circle_arcs, circle_curve};
use super::*;
use brep_core::Model;

mod cylinder_torus_component;
#[cfg(test)]
mod tests;
pub use cylinder_torus_component::*;
