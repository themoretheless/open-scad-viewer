//! Enclosures of the ideal polyline minimum-rotation transport construction.
//! Includes normalization, corner bisectors and the closing transport. The
//! distributed correction and rotated stations are separate
//! obligations; this report is not a whole-sweep certificate.
use super::scalar_certificate::Status;
use crate::sweep_support::interval_vec3::{cross, dot_tight as dot, scale as mul, sub};
use crate::{Result, check, distance_bounds::Interval as I};
type V = [I; 3];
pub type VectorEnclosure = [[f64; 2]; 3];

mod report;
mod certify;
#[cfg(test)]
mod tests;
pub use report::*;
pub use certify::*;
