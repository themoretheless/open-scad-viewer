//! Analytic plane/cone (frustum) intersection for a canonical planar patch
//! operand and a canonical conical frustum solid — the conic-section cell.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, orthogonal edge vectors); the cone
//! operand is the exact canonical frustum of `analytic::frustum` — four
//! rational ruled side patches linearly interpolated between the two radius
//! rings plus a bilinear cap per nonzero ring, with a true apex admitted
//! when exactly one radius is zero (the canonical constructor shares the
//! apex vertex across all four side patches) — recognized by
//! `recognize_cone` under rigid affine placement. An equal-radius frustum
//! is a cylinder and is refused (`UnsupportedSurface`, the plane/cylinder
//! cell owns it); anything else is an explicit `UnsupportedSurface` region
//! — this cell never falls back to numerical surface/surface subdivision.
//!
//! All angles are classified on outward binary64 bands widened by both
//! recognition deviations. With g = axis.normal, sigma = sin(alpha) and
//! alpha = atan(|slope|) the cone half-angle (slope = (r_top - r_bottom) /
//! height), the plane/axis angle beta satisfies sin(beta) = |g|:
//! |g| snapped to 1 (pure rounding): the section perpendicular to the
//!   axis is the exact circle with the radius LINEARLY INTERPOLATED between
//!   the rings at the section height, clipped by the patch rectangle
//!   (closed-form eccentric-angle clip). Provably outside the finite height
//!   resolves empty; a plane within the band of a nonzero-radius ring plane
//!   is a coincident cap/rim region (`CoincidentTrim`, never a guessed
//!   circle); a section at the apex level (radius <= band, apex ring
//!   included) stays `TangencyOrMultipleRoot`. Lifts are exact: iso-v lines
//!   on the side patches and ellipse arcs in the plane UV.
//! |g| snapped to 0 (pure rounding): the plane is parallel to the axis.
//!   A plane through the axis (apex on the plane within the band) yields
//!   the two exact straight ruling LINES through the apex (degree-1 NURBS,
//!   exact), clipped by the finite height and the patch rectangle
//!   (band-thin clips, boundary-coincident rulings and patch-corner touches
//!   stay unresolved), lifted as iso-u rulings on the side patches. An
//!   off-axis parallel plane yields the exact hyperbola below.
//! Oblique with ||g| - sigma| snapped to 0 (pure rounding — a plane built
//!   exactly parallel to a ruling): the exact rational parabola (polynomial
//!   quadratic Bezier arcs, all weights 1), clipped by the height rings,
//!   the nappe half-plane and the patch rectangle in one closed-form
//!   quadratic clip in the curve parameter. The parabola boundary is the
//!   degenerate threshold: an angle within the RECOGNITION-scale band of
//!   alpha but beyond the pure-rounding snap stays `TangencyOrMultipleRoot`
//!   — never a guessed conic.
//! Oblique with |g| > sigma: the exact ellipse (rational quadratic sweep,
//!   exact weights), clipped by the height rings, the nappe half-plane and
//!   the patch rectangle in one exact eccentric-angle clip (every
//!   constraint is linear in the plane UV, reusing `clip_ellipse`).
//! Oblique with |g| < sigma (and the off-axis parallel plane): the exact
//!   hyperbola. Each nappe arm is parameterized exactly as the rational
//!   quadratic x = x_c +- a (1+t^2)/(1-t^2), y = 2 b t/(1-t^2) and clipped
//!   by the height rings, the nappe half-plane and the patch rectangle —
//!   every constraint is quadratic in t, solved in closed form. The mirror
//!   arm of the double cone is clipped away by the nappe half-plane.
//! Any plane through the apex region (|d0| <= band, apex = the singular
//!   point of the side surface) stays `TangencyOrMultipleRoot` outside the
//!   exact through-axis ruling case; the apex is a multiple-root contact
//!   and is never guessed.
//!
//! Cone-side UV lifts: the circle (iso-v) and ruling (iso-u) cases are
//! exact. The ellipse/parabola/hyperbola side lifts are NOT shipped: on the
//! rational ruled side patches the conic unrolls through an azimuth/height
//! map with no exact LOW-DEGREE rational UV representation (an exact form
//! needs degree 4+ with seam splits), so `cone_uv` is null rather than a
//! numerical fit — the 3D conic and the plane-UV lift stay exact. This
//! mirrors the documented oblique plane/cylinder precedent.
//!
//! Scope note (shared with the other plane/* cells): only side-surface
//! contacts are reported; where the plane patch crosses a cap disk the cap
//! chord is not a component — coverage is NumericallyResolved, never
//! certified complete, and nothing here authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::cylinder_cylinder::arc_parameter;
pub(crate) use brep_core::intersections::{CanonicalPlane, recognize_plane};
use super::plane_sphere::{
    EllipseClip, Halfplane2, PlanePatchCurve, clip_ellipse, conic_arcs_2d, conic_sweep,
};
use super::sphere_cylinder::CylinderPatchCurve;
use brep_core::intersections::sphere_sphere::{RECOGNITION, ccw_intersect};
pub(crate) use brep_core::intersections::{CanonicalCone, recognize_cone};
use super::*;
use brep_core::Model;

mod plane_cone_component;
mod intersect_plane_cone;
mod conic_num_uv;
#[cfg(test)]
mod tests;
pub use plane_cone_component::*;
pub use intersect_plane_cone::*;
use conic_num_uv::*;
