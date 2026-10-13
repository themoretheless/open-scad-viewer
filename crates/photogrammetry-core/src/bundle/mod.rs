//! Joint pose/point refinement with fixed intrinsics and no external numerical runtime.
//!
//! Huber-weighted LM eliminates independent 3x3 point blocks before solving the bounded
//! camera system. The anchor pose is fixed; a projection-preserving similarity fixes
//! the initial anchor/scale-camera baseline after each candidate, removing scale drift.
//! Inputs are committed only after successful completion, including the final callback.
use crate::Result;
use crate::camera::Camera;
use crate::math::{M3, V3, add, cross, det, dot, mm, mv, norm, rotation, scale, sub, tr, unit};
use std::ops::Range;

mod observation;
mod optimize;
#[cfg(test)]
mod tests;
pub use observation::*;
pub use optimize::*;
