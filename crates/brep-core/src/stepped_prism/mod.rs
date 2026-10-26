//! Bounded regularized Boolean over analytic XY cells and exact Z slabs.
//! Retained source curves author every cell. Complete opposite cell faces are
//! cancelled; topology is sewn only across those proven shared faces.
use super::*;

type Profile = Vec<Vec<Curve>>;
pub(crate) const MAX_LAYERS: usize = 8;
const MAX_CELLS: usize = 32;
const MAX_PROFILES: usize = 16;

mod assemble;
mod layer;
pub(crate) use assemble::boolean;
use layer::*;
pub(crate) use layer::{Layer, recognize};
