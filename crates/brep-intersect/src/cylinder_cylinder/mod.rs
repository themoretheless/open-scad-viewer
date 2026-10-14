//! Analytic cylinder/cylinder intersection for canonical solids with
//! parallel axes (including the coaxial configuration).
//!
//! Both operands must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches with weights
//! [1, cos(pi/4), 1] and unit-square trims, two bilinear caps with inscribed
//! quarter-circle trims), optionally carried through a rigid affine
//! placement, as certified by the shared strict recognizer in
//! `sphere_cylinder`. Anything else is an explicit `UnsupportedSurface`
//! region — this cell never falls back to numerical surface/surface
//! subdivision. Axis parallelism and coaxiality are certified, never forced:
//! pure-rounding direction or center-line offsets snap, recognition-scale
//! offsets report `NearCoincidence`, and a clearly non-parallel pair is
//! `UnsupportedSurface` (the general cylinder/cylinder pair is a quartic;
//! even the equal-radius perpendicular case — a pair of ellipses — is out of
//! scope). Classification uses outward binary64 bands widened by the observed
//! recognition deviation. A transverse parallel pair reduces to a planar
//! circle/circle section in the common perpendicular plane and yields two
//! exact straight rulings (degree-1 lines) clipped by both finite heights;
//! the lifts are per-patch iso-u segments on the side patches of both
//! cylinders, duplicated across the quadrant seam when a ruling sits on it.
//! Every tangency — external (d == r1 + r2), internal (d == |r1 - r2|), a
//! band-thin height clip, the coaxial equal-radius coincident side band, and
//! stacked equal-radius cap-plane coincidences — stays an explicit unresolved
//! region; tangent contacts are never reported as point or guessed-line
//! components, matching the house tangency discipline. Coaxial cylinders with
//! provably different radii and no coincident cap plane resolve empty (the
//! sides never meet and no rim lies on the other side, which would need equal
//! radii); a coincident cap plane of nested coaxial cylinders shares the
//! smaller cap disk and is an honest `CoincidentTrim` region. Nothing here
//! authorizes a topology change.
#[cfg(feature = "codec")]
mod serialization;
use super::sphere_cylinder::{CanonicalCylinder, CylinderPatchCurve, recognize_cylinder};
use super::sphere_sphere::RECOGNITION;
use super::*;
use brep_core::Model;

mod cylinder_cylinder_component;
#[cfg(test)]
mod tests;
pub use cylinder_cylinder_component::*;
