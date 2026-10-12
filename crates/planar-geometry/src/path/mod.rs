//! First-class cubic Bézier paths for planar modeling.
//!
//! Algorithms adapted from the Curvex vector editor (MIT OR Apache-2.0),
//! reimplemented in binary64 to match the polygon-core CAD contract.
//! Paths flatten to polylines for planar boolean / offset / extrude.
use crate::{Result, check};
use math_core::{cross2, norm2, sub2};


use crate::limits::{FLATTEN_POINTS as MAX_FLATTEN_POINTS, PATH_SEGMENTS as MAX_SEGMENTS};

mod path_segment;
mod bezier_path;
mod average_axis;
#[cfg(test)]
mod tests;
pub use path_segment::*;
use bezier_path::*;
pub use average_axis::*;
