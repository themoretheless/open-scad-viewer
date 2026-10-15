//! Analytic sphere/cone (frustum) intersection for canonical solids in the
//! axial configuration (sphere center certified on the cone axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the cone
//! operand must be the exact canonical conical frustum of `analytic::frustum`
//! (four rational ruled side patches linearly interpolated between the two
//! radius rings, a bilinear cap per nonzero ring, a true apex admitted when
//! exactly one radius is zero; an equal-radius frustum is a cylinder and is
//! refused by the recognizer) — recognized by `plane_cone::recognize_cone`
//! under rigid affine placement. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision. Axiality is certified, never forced: a
//! perpendicular offset at pure rounding scale snaps to the axis, an offset
//! inside the recognition-scale band reports a `NearCoincidence` region, and
//! a clearly off-axis pair is `UnsupportedSurface` (the general sphere/cone
//! pair is a quartic, out of scope). Classification uses outward binary64
//! bands widened by both recognition deviations.
//!
//! In the axial coordinate t measured from the bottom ring center (sphere
//! center at t = s0), the cone side carries the linear radius profile
//! rho(t) = r_bottom + slope*t, so side contacts solve
//! (t - s0)^2 + rho(t)^2 = r^2 — one quadratic in the offset q = t - s0:
//! (1+m^2) q^2 + 2 rho_c m q + (rho_c^2 - r^2) = 0 with m = slope and
//! rho_c = rho(s0). Its discriminant is 4 (r^2 (1+m^2) - rho_c^2), i.e. the
//! side line passes the sphere center at distance |rho_c|/sqrt(1+m^2); a
//! sphere within the band of that distance is tangent to the side and stays
//! `TangencyOrMultipleRoot` (double root), never a guessed circle. Two
//! distinct roots clipped to the finite height (0, h) give the exact circles
//! of radius rho(t); a root within the band of a ring plane is the rim
//! contact and stays unresolved, and a root whose radius collapses into the
//! band is the apex contact (the sphere passes through the singular apex
//! point) and stays unresolved. Cap contacts mirror the sphere/cylinder
//! cell: a sphere crossing a ring cap plane inside the disk yields the exact
//! circle in that cap plane, a cap-plane touch or a cap circle whose radius
//! meets the ring radius (rim equality) stays `TangencyOrMultipleRoot`.
//! Provable misses (sphere axially beyond the solid, side line unreachable,
//! cap circles provably outside the disks) resolve empty, certified by the
//! interval signs. There is no sphere/cone surface coincidence, so
//! `CoincidentTrim` does not arise; every degeneracy reported here is an
//! explicit tangency region. Resolved contacts are exact rational circles
//! (four 90-degree arcs, weights cos(pi/4)) with UV lifts on both surfaces:
//! iso-v lines across all four cone side patches, an exact UV circle on a
//! cap face, and per-patch UV circles/lines on the sphere. Nothing here
//! authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::plane_cone::{CanonicalCone, recognize_cone};
use super::sphere_cylinder::CylinderPatchCurve;
#[cfg(test)]
use brep_core::intersections::sphere_sphere::ARC_WEIGHT;
use brep_core::intersections::sphere_sphere::{
    self, CanonicalSphere, RECOGNITION, SpherePatchCircle, circle_arcs, circle_curve, lift,
};
use super::*;
use brep_core::Model;

mod sphere_cone_component;
#[cfg(test)]
mod tests;
pub use sphere_cone_component::*;
