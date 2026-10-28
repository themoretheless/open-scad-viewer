//! Generators and planar distortions for Bézier / polyline paths.
//!
//! Ported from Curvex path effects (MIT OR Apache-2.0), binary64 for polygon-core.
use crate::path::{BezierPath, PathSegment};
use crate::{Result, check};
use math_core::{add2, norm2, scale2, sub2};

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
const MAX_COPIES: usize = 256;
const MAX_SAMPLES: usize = 4096;

mod concentric_offset;
mod stipple_kind;
mod scatter_paths;
#[cfg(test)]
mod tests;
pub use concentric_offset::*;
pub use stipple_kind::*;
pub use scatter_paths::*;
