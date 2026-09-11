//! Planar shape kernel: Bézier paths, closed rings, Pathfinder, edit.
//! Independent of triangle-mesh, NURBS and SDF kernels.
//!
//! [`rings`] is the boolean/offset engine on closed polylines.
//! [`pathfinder`] is the z-order scenario layer (divide / crop / trim / merge)
//! built on `rings::planar`.
pub mod corners;
pub mod edit;
pub mod effects;
pub mod path;
pub mod pathfinder;
pub mod rings;
pub mod stroke;

pub use math_core::{Error, Result};
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new("PLANAR_INVALID_INPUT", message)
}
pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(error(message))
    }
}
