//! Analytic plane/sphere intersection for a canonical planar patch operand
//! and a canonical stereographic sphere solid.
//!
//! The plane operand is a standalone finite rectangular planar patch: a
//! one-face body-less model (a single open shell — bodies require closed
//! shells) whose single face is an exact bilinear affine surface over the
//! unit square with unit weights, exact unit-square boundary trims and four
//! corner vertices — the same surface construction a `cuboid` face uses,
//! admitted here as a rigidly placeable free patch. A
//! face embedded in a closed solid is not accepted: the operand must be the
//! patch alone. The sphere operand must be the exact canonical stereographic
//! sphere of `analytic::sphere` (recognized by `sphere_sphere::recognize`).
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Center
//! distance classification uses an outward binary64 band widened by both
//! recognition deviations: a plane clearly missing the sphere resolves
//! empty, a distance within the band of the radius reports
//! `TangencyOrMultipleRoot` (a tangent plane is never a point component),
//! and a clear crossing yields the exact rational circle of radius
//! sqrt(r^2 - d^2). The circle is clipped by the patch's finite rectangular
//! domain with exact trim parameters (circle/rectangle clipping in the
//! patch's UV coordinates, where the circle is an axis-aligned ellipse
//! because the patch is rectangular); a circle fully inside the domain
//! yields the full circle, boundary crossings yield clipped arcs, and a
//! tangency to a domain edge stays unresolved. Lifts: per-patch UV
//! circles/lines on the sphere via `sphere_sphere::lift` (full circle) or
//! `sphere_sphere::lift_clipped` (clipped arcs, with exact endpoint
//! inversion), and exact rational quadratic ellipse arcs in the plane UV.
//! Nothing here authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::sphere_sphere::{self, CanonicalSphere, RECOGNITION, SpherePatchCircle};
use super::*;
use brep_core::Model;

mod plane_patch_curve;
#[cfg(test)]
mod tests;
pub use plane_patch_curve::*;
