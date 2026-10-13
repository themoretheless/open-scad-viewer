//! Tolerant reader for G-code produced by other slicers and firmware flavors.
//! It recovers movements, layers and totals for preview; it does not validate
//! a file for printing; volume uses the supplied, declared or default filament diameter.

use crate::foreign_extrusion::Extrusion;
use crate::foreign_feedrate::Feedrate;
use crate::foreign_words::{
    self,
    Command::{G, InvalidTool, M, T},
};
use crate::{
    GcodeBounds, GcodeMove, GcodePreview, MAX_LAYERS, MAX_LINE_BYTES, MAX_MOVES, MAX_OUTPUT_BYTES,
    Result, invalid, number, valid_coordinate,
};
use std::borrow::Cow;

/// Default filament diameter when a foreign file declares none.
pub const ASSUMED_FILAMENT_DIAMETER_MM: f64 = 1.75;
const MAX_ARC_SEGMENTS: usize = 64;
const ARC_SEGMENT_MM: f64 = 1.0;

mod generator;
mod parse_any;
#[cfg(test)]
mod tests;
pub use generator::*;
pub use parse_any::*;
