//! Analytic cone/torus (frustum/torus) intersection for canonical solids in
//! the coaxial configuration (the cone axis certified coincident with the
//! torus axis, either orientation — the anti-axial pair is oriented by the
//! sign of the axes' dot product, as in cone_cone).
//!
//! The cone operand must be the exact canonical conical frustum of
//! `analytic::frustum` (four rational ruled side patches linearly
//! interpolated between the two radius rings, a bilinear cap per nonzero
//! ring, a true apex admitted when exactly one radius is zero; an
//! equal-radius frustum is a cylinder and is refused by the recognizer) —
//! recognized by `plane_cone::recognize_cone`; the torus operand must be the
//! exact canonical ring-torus solid of `analytic::torus` (sixteen rational
//! biquadratic patches, strict ring tori only — the constructor refuses horn
//! and spindle tori), recognized by `plane_torus::recognize_torus`. Rigid
//! affine placements are admitted. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision. Coaxiality is certified, never forced:
//! pure-rounding axis tilts and center-line offsets snap, recognition-scale
//! tilts or offsets report `NearCoincidence`, and a clearly tilted or
//! off-axis pair is `UnsupportedSurface` (the general cone/torus pair is a
//! quartic, out of scope). Classification uses outward binary64 bands
//! widened by both recognition deviations.
//!
//! Both surfaces are surfaces of revolution about the shared axis, so the
//! section reduces to the meridian half-plane (rho >= 0, z): the cone side
//! is the slanted line (rho, z) = (r_b + m t, z_b + s t) in the cone's own
//! axial parameter t (s = +-1 the orientation sign, z_b the torus-axial
//! height of the cone's bottom ring center) and the torus meridian circle is
//! (rho - R)^2 + z^2 = r^2. Substituting the line gives one exact quadratic
//! (1+m^2) t^2 + 2 B t + C = 0 whose discriminant is r^2 (1+m^2) - D^2 with
//! D = (R - r_b) s + z_b m the signed line/circle-center cross — i.e. the
//! side line passes the meridian circle center at distance |D|/sqrt(1+m^2):
//! A line provably missing the circle (|D|/sqrt(1+m^2) > r beyond the
//!   band) yields no side contact, certified by the interval signs.
//! A tangent line (distance == r within the band, a double root) whose
//!   foot lands on the finite side segment stays `TangencyOrMultipleRoot`,
//!   never a guessed circle; a tangency whose foot is provably beyond the
//!   finite height belongs to the rim bands below.
//! Two distinct roots revolve into two exact circles (radius rho* =
//!   r_b + m t*, height z* = z_b + s t*). A root provably outside the
//!   cone's finite height range is honestly absent (clipped), so the
//!   resolved branch may yield one circle or two; a root within the band of
//!   a ring plane is the rim contact — unresolved, never guessed. A root
//!   whose radius collapses into the band would be the apex/pole degeneracy;
//!   it is structurally unreachable for a canonical strict ring torus (the
//!   meridian circle stays at rho >= R - r >= 1e-5, far beyond the band) and
//!   is kept as an honest guard, never guessed through.
//! Ring/rim bands: a rim circle (rho = r_ring, z = z_b + s t_ring) lying
//!   on the torus within the band — (r_ring - R)^2 + h^2 == r^2 — is a
//!   tangent boundary contact and stays unresolved independent of the side
//!   classification (this also owns the apex ring when its radius is zero).
//! Cap-plane contacts: a ring plane at torus-axial height h_c crosses the
//!   tube at the circles of radii R +- sqrt(r^2 - h_c^2) when |h_c| < r is
//!   provable (the plane/torus perpendicular branch, reduced here to the
//!   ring disk): a circle provably inside the ring disk is a cap circle
//!   component, a radius equal to the ring radius within the band is the rim
//!   tangency (unresolved), and a radius provably beyond the ring radius is
//!   absent. |h_c| == r within the band is the plane/tube tangency —
//!   unresolved. An inner radius R - sqrt(r^2 - h_c^2) collapsing into the
//!   band would degenerate to a pole point; it is structurally unreachable
//!   for a canonical strict ring torus (R - r >= 1e-5) and kept as an
//!   honest guard, never guessed through. A cone side can never coincide
//!   with a curved torus patch, so `CoincidentTrim` is deliberately unused
//!   by this cell.
//!
//! Resolved contacts are exact rational circles (four 90-degree arcs,
//! weights cos(pi/4)) with UV lifts on both surfaces: on the cone side the
//! iso-v line at the exact height fraction on all four side patches, on a
//! ring cap the exact UV circle of radius rho/(2 r_ring) about [1/2, 1/2],
//! and on the torus the iso-v parallels at the exact profile angle
//! phi = atan2(z, rho - R) mapped through the exact rational 90-degree-arc
//! parameter map t = s/(1+s), s = sqrt(2) tan(phi/2)/(1 - tan(phi/2)), one
//! degree-1 line per revolution quadrant patch of the profile row. Nothing
//! here authorizes a topology change.
use brep_core::intersections::{CanonicalCone, recognize_cone};
use brep_core::intersections::{CanonicalTorus, recognize_torus};
use super::plane_torus::{TorusPatchCurve, lift_parallel, torus_residual};
use super::sphere_cylinder::CylinderPatchCurve;
#[cfg(test)]
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use brep_core::intersections::sphere_sphere::{RECOGNITION, circle_arcs, circle_curve};
use super::*;
use brep_core::Model;

mod cone_torus_component;
#[cfg(test)]
mod tests;
pub use cone_torus_component::*;
