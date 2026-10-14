//! Tessellation and interchange preserve topological face identity.
use super::*;
use std::collections::BTreeMap;


#[cfg(test)]
#[path = "../tests/brep_registry.rs"]
mod registry_tests;

mod tessellation;
mod nurbs;
mod polygons;
pub use tessellation::*;
pub use nurbs::*;
pub use polygons::*;
