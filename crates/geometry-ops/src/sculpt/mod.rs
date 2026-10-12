//! Representation-independent sculpt brushes. The caller supplies vertex
//! positions together with per-vertex unit normals and one-ring adjacency; the
//! engine returns displaced positions and never touches topology. Kernels own
//! normal/adjacency construction and post-edit validity checks.
use crate::{Point, Result, codec, fail, finite};
use math_core::{add, dot, norm, scale, sub, unit};
use std::collections::BTreeSet;
use value_codec::{Deserialize, Map, Serialize, Value, error};

mod falloff;
#[cfg(test)]
mod tests;
pub use falloff::*;
