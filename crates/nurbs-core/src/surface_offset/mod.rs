//! Signed normal offsets of authored rational surfaces, for rolling-ball search.
//! Rectangle enclosures cover every admitted source parameter; numerical jets
//! only propose continuation steps. Neither API certifies an embedded offset,
//! contact with trimmed faces, G1/G2 continuity, or a fillet topology change.
use crate::{Result, check, distance_bounds::Interval as I, numeric, surface::Surface};

mod bounds;
mod intersection_candidates;
#[cfg(test)]
mod tests;
pub use bounds::*;
pub use intersection_candidates::*;
