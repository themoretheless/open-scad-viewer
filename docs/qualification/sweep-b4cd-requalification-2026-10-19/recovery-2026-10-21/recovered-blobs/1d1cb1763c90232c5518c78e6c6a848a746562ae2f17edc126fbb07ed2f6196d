//! Deterministic photo reconstruction. Algorithms use Rust std only; host owns codecs and UI.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod bundle;
pub mod calibration;
pub mod camera;
pub mod dense;
pub mod diagnostics;
pub mod evaluation;
pub mod features;
pub mod host_matching;
mod matching;
pub(crate) use math_core as math;
#[cfg(feature = "gpu")]
pub mod gpu;
mod model;
mod pipeline;
mod seeding;
/// Compute backend for the heavy stages. `Cpu` is the deterministic reference
/// and the default; `Gpu` is opt-in and requires the `gpu` crate feature.
pub use math_core::{Acceleration, Error, Result};
pub use model::{Image, Point, Reconstruction};
pub use pipeline::{
    ReconstructionOptions, ReconstructionOutcome, reconstruct, reconstruct_detailed,
    reconstruct_detailed_with_matches,
};
pub use seeding::SeedOptions;
pub const INVALID_INPUT: &str = "PHOTOGRAMMETRY_INVALID_INPUT";
pub fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[cfg(test)]
mod integration_tests;
