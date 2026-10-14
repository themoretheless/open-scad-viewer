//! Editor-precision helpers: snap, align/distribute, cut, measure.
//! Pure geometry — no UI / document layer.
use crate::path::BezierPath;
use crate::{Result, check};

mod h_align;
mod compute_distance_marks;
#[cfg(test)]
mod tests;
pub use h_align::*;
pub use compute_distance_marks::*;
