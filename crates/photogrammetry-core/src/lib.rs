//! Deterministic photo reconstruction. Algorithms use Rust std only; host owns codecs and UI.
pub mod bundle;
pub mod calibration;
pub mod camera;
pub mod dense;
pub mod diagnostics;
pub mod evaluation;
pub mod features;
mod matching;
pub(crate) use math_core as math;
#[cfg(feature = "gpu")]
pub mod gpu;
mod model;
mod pipeline;
mod seeding;
pub use model::{Image, Point, Reconstruction};
pub use pipeline::{
    reconstruct, reconstruct_detailed, ReconstructionOptions, ReconstructionOutcome,
};
pub use seeding::SeedOptions;
/// Compute backend for the heavy stages. `Cpu` is the deterministic reference
/// and the default; `Gpu` is opt-in and requires the `gpu` crate feature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Acceleration {
    #[default]
    Cpu,
    Gpu,
}
pub type Result<T> = std::result::Result<T, String>;
#[cfg(test)]
mod integration_tests;
