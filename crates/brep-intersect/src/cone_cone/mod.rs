//! Analytic cone/cone (frustum) intersection for canonical solids in the
//! coaxial configuration (axes parallel and coincident, either orientation).
//!
//! Both operands must be the exact canonical conical frustum of
//! `analytic::frustum` (four rational ruled side patches linearly
//! interpolated between the two radius rings, a bilinear cap per nonzero
//! ring, a true apex admitted when exactly one radius is zero; an
//! equal-radius frustum is a cylinder and is refused by the recognizer) —
//! recognized by `plane_cone::recognize_cone` under rigid affine placement.
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Coaxiality is
//! certified, never forced: a pure-rounding axis tilt or perpendicular
//! offset snaps to the shared axis, a recognition-scale tilt or offset
//! reports a `NearCoincidence` region, and a clearly non-parallel or
//! off-axis pair is `UnsupportedSurface` (the general cone/cone pair is a
//! quartic, out of scope). Classification uses outward binary64 bands
//! widened by both recognition deviations.
//!
//! In the shared axial coordinate s measured from the first cone's bottom
//! ring center along its axis (the second cone oriented by the sign of the
//! axes' dot product, so apex-to-apex and base-to-base anti-axial pairs are
//! still coaxial), each side carries the linear radius profile rho_i(s)
//! between its rings. Side contacts solve the single linear equation
//! rho_1(s) == rho_2(s): with different tapers there is at most one height
//! s*, reported as the exact circle of radius rho(s*) when s* is certified
//! strictly inside both height ranges (a root clipped by either finite
//! height is honestly absent); a root within the band of any of the four
//! ring planes is a rim contact, and a root whose radius collapses into the
//! band is the apex-meeting contact — both stay `TangencyOrMultipleRoot`,
//! never a guessed circle. With equal tapers inside the band the profiles
//! are parallel: coincident over the overlapping height range reports
//! `CoincidentTrim` (never a surface component), clearly distinct profiles
//! resolve empty, and equality at a single ring plane only is the rim
//! tangency, unresolved. Any two ring planes (one per cone) coinciding
//! within the band carry a rim/rim, rim-on-cap or apex-on-cap boundary
//! contact and stay `TangencyOrMultipleRoot` independent of the side
//! classification. Resolved contacts are exact rational circles (four
//! 90-degree arcs, weights cos(pi/4)) with iso-v lifts on all four side
//! patches of both cones. Nothing here authorizes a topology change.
use brep_core::intersections::{CanonicalCone, recognize_cone};
use super::sphere_cylinder::CylinderPatchCurve;
use brep_core::intersections::sphere_sphere::{RECOGNITION, circle_curve};
use super::*;
use brep_core::Model;

mod cone_cone_component;
#[cfg(test)]
mod tests;
pub use cone_cone_component::*;
