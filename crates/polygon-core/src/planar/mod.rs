//! Planar shape kernel: Bézier paths, closed rings, Pathfinder, edit.
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
