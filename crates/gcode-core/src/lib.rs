//! G-code encoding and independent FDM parse. Not a print process and not
//! printer I/O. No mesh/NURBS/WASM dependency.
//!
//! Coordinates are millimeters unless `G20` / inch units are set. Internal
//! feedrate is mm/s and is written as `F` in mm/min.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

mod block;
mod command;
mod emit;
mod interpret;
mod lex;
mod plan;
mod preview;
mod profile;

pub use block::{Block, Program, Word};
pub use command::Command;
pub use emit::{emit, emit_to};
pub use interpret::{parse, parse_fdm, parse_fdm_with, parse_program};
pub use lex::{lex_line, lex_program};
pub use preview::{GcodeMove, GcodePreview, ThermalState};
pub use plan::{deposited_volume_mm3, path_length_mm, PlannedLayer, PlannedPath};
pub use profile::{ExtrusionMode, MachineProfile, Units};

pub const DIALECT: &str = "open-scad-viewer/print-preview 1";
pub const MAX_LAYERS: usize = 2_048;
pub const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_BLOCKS: usize = 500_000;
pub const MAX_MOVES: usize = 500_000;
pub const MAX_LINE_BYTES: usize = 4_096;
pub const MAX_TEMPLATE_BYTES: usize = 64 * 1024;
pub const MAX_ARC_SEGMENTS: usize = 4_096;

pub use math_core::{Error, Result};

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}
