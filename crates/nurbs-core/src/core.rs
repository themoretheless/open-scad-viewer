//! Core types and point semantics: curves, surfaces, bounds, affine frames.
pub use crate::affine;
pub mod bounds;
pub mod canonical;
pub mod coordinate_frame;
pub use crate::curve;
pub use crate::surface;
pub(crate) mod vec3_ext;
