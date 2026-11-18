//! Exact support geometry for a convex plane/cylinder rim blend.
//! Authors support patches, contact rails and open B-rep sheets, not sewn solids.
use crate::{Result, invalid};
use nurbs_core::{curve::Curve, surface::Surface};

mod circular_blend_span;
mod plane_cylinder_rim;
#[cfg(test)]
mod tests;
pub use circular_blend_span::*;
pub use plane_cylinder_rim::*;
