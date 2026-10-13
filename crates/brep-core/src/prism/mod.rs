//! Exact extrusion of retained planar trim curves and conservative recognition
//! of XY prisms. Display meshes never participate in either operation.
use super::*;

pub const MAX_PROFILE_LOOPS: usize = 64;
pub const MAX_PROFILE_CURVES: usize = 254;
pub const MAX_PROFILE_CONTROLS: usize = 8192;
const TOLERANCE: f64 = 1e-7;

mod profile_prism;
mod recognize;
#[cfg(test)]
mod imported_cap_tests;
pub use profile_prism::*;
pub use recognize::*;
