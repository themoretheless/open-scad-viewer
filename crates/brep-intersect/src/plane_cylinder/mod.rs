//! Analytic plane/cylinder intersection for a canonical planar patch operand
//! and a canonical cylinder solid.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, orthogonal edge vectors); the
//! cylinder operand is the exact canonical six-face cylinder of
//! `analytic::cylinder` (recognized by `sphere_cylinder::recognize_cylinder`).
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. The axis/plane
//! angle is certified, never forced: pure-rounding tilts snap to
//! perpendicular or parallel, recognition-scale tilts report
//! `NearCoincidence`, and every clearly oblique configuration is handled
//! exactly. Classification uses outward binary64 bands widened by both
//! recognition deviations.
//!
//! Plane perpendicular to the axis: the section is a circle of the
//!   cylinder radius at the section height. Provably beyond both cap planes
//!   resolves empty; a plane within the band of a cap plane is a coincident
//!   cap/rim region (`CoincidentTrim`) — never a guessed circle; a clear
//!   section yields the exact circle, clipped by the patch's finite
//!   rectangular domain with exact trim parameters (domain-edge tangencies
//!   stay unresolved). Lifts: exact ellipse arcs in the plane UV and iso-v
//!   circle arcs on the side patches (full circle: all four patches; clipped
//!   arcs: per-patch sub-u ranges with the exact rational-arc angle
//!   parameter of `cylinder_cylinder::arc_parameter`).
//! Plane parallel to the axis: distance d from the axis. d > R resolves
//!   empty, d == R within the band stays `TangencyOrMultipleRoot` (a tangent
//!   plane is never a guessed line), d < R yields two exact straight lines
//!   (degree-1 NURBS) clipped by the finite height and by the patch
//!   rectangle (band-thin clips and patch-corner touches stay unresolved),
//!   lifted as iso-u rulings on the side patches (seam rulings duplicated on
//!   both adjacent patches) and as degree-1 segments in the plane UV.
//! Oblique plane: the section is the exact ellipse with semi-minor R and
//!   semi-major R / |axis.n| — a rational quadratic NURBS with exact weights
//!   — clipped by the finite height and the patch rectangle in one exact
//!   eccentric-angle clip (all six constraints are linear in the plane UV);
//!   cap-plane tangencies and boundary tangencies stay unresolved. The lift
//!   into the plane UV is the exact rotated ellipse. The lift onto the
//!   cylinder side is NOT shipped in this slice: on the rational side
//!   patches the ellipse unrolls to a cosine curve with no low-degree exact
//!   rational UV representation, so `cylinder_uv` is null rather than a
//!   numerical fit — the 3D ellipse and the plane lift stay exact.
//!
//! Scope note (shared with the cylinder/cylinder cell): only side-surface
//! contacts are reported; where the plane patch would cross a cap disk
//! (parallel/oblique cases) the cap chord is not a component — coverage is
//! NumericallyResolved, never certified complete, and nothing here
//! authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::cylinder_cylinder::{arc_parameter, lift_line};
pub(crate) use brep_core::intersections::{CanonicalPlane, recognize_plane};
use super::plane_sphere::{
    EllipseClip, Halfplane2, PlanePatchCurve, clip_ellipse, conic_arcs_2d, conic_sweep,
};
pub(crate) use brep_core::intersections::{CanonicalCylinder, recognize_cylinder};
use super::sphere_cylinder::CylinderPatchCurve;
use brep_core::intersections::sphere_sphere::{RECOGNITION, ccw_intersect};
use super::*;
use brep_core::Model;

mod plane_cylinder_component;
#[cfg(test)]
mod tests;
pub use plane_cylinder_component::*;
