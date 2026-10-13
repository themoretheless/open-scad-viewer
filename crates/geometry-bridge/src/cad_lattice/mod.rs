//! Native spatial lattice utilities: graph generation, component counting,
//! topology-preserving decimation, and lightening cell construction.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet};

mod components;
mod graph;
mod print_fit;
#[cfg(test)]
mod cell_tests;
#[cfg(test)]
mod print_tests;
pub use components::*;
pub use graph::*;
pub use print_fit::*;
