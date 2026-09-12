//! Planar shape kernel: Bézier paths, closed rings, Pathfinder, edit.
//! Independent of triangle-mesh, NURBS and SDF kernels.
//!
//! [`rings`] is the boolean/offset engine on closed polylines.
//! [`pathfinder`] is the z-order scenario layer (divide / crop / trim / merge)
//! built on `rings::planar`.
//!
//! Coordinates are millimeters; algorithms use binary64. Builds on stable Rust
//! (no `#![feature]`).
pub mod corners;
pub mod curve;
pub mod attribute_mesh;
pub mod curve_boolean;
pub mod edit;
pub mod effects;
pub mod path;
pub mod path_offset;
pub mod pathfinder;
pub mod render;
pub mod rings;
pub mod scissors;
pub mod stroke;
pub mod tessellation;

pub use math_core::{Error, Result};

/// Shared print/strength layer budget. Not a CAD handle limit.
pub const MAX_LAYERS: usize = 2_048;

/// One horizontal slice: closed rings in millimeters. Not a CAD handle.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerSection {
    pub z_mm: f64,
    pub contours: rings::Rings,
}
pub(crate) const INVALID_INPUT: &str = "PLANAR_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
