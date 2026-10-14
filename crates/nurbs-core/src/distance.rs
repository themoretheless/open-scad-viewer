//! Distance computations: curve-curve, surface-surface, curve-surface, and trimmed-surface distance.
pub use crate::curve_distance;
pub mod curve_surface_distance;
pub(crate) mod subdivision;
pub use crate::surface_distance;
pub use crate::trimmed_surface_distance;
