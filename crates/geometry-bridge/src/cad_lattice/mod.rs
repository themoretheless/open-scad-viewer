//! Native spatial lattice utilities: graph generation, component counting,
//! topology-preserving decimation, lightening cell construction, and FDM
//! printability / analytical strength screening with weak-spot markers.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet};

mod components;
mod graph;
mod print_fit;
mod markers;
mod print_report;
mod report_util;
mod strength;
mod strength_material;
mod strength_report;
mod weak_spots;
#[cfg(test)]
mod cell_tests;
#[cfg(test)]
mod print_tests;
#[cfg(test)]
mod strength_tests;
pub use components::*;
pub use graph::*;
pub use print_fit::*;
pub use markers::weak_spot_markers;
pub use print_report::{print_optimize, print_report};
pub use strength::strength;
pub use strength_material::{materials, structure_hint_op};
