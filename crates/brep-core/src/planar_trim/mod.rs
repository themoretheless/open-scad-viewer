//! Finite analytic planar arrangements over retained rational NURBS spans.
//!
//! Lines and circular quadratic arcs are admitted. Intersections, winding and
//! areas use their analytic carriers; output boundaries are trims/reversals of
//! the input curves, never chords or fitted replacements. Ill-conditioned or
//! tolerance-ambiguous events fail explicitly. This is numerical binary64
//! analytic geometry, not an independent exact-arithmetic certificate.
use nurbs_core::{Error, Result, curve::Curve};
use std::f64::consts::{PI, TAU};

const MAX_SPANS: usize = 256;
const MAX_FRAGMENTS: usize = 2048;
type Point = [f64; 2];

mod point_location;
mod validate;
mod locate_point;
#[cfg(test)]
mod tests;
pub use point_location::*;
pub use validate::*;
pub use locate_point::*;
