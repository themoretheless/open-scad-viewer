//! Tessellation and interchange preserve topological face identity.
use super::*;
use std::collections::BTreeMap;

mod tessellation;
mod certified_nurbs;
mod polygons;
#[cfg(test)]
mod registry_tests;
pub use tessellation::*;
pub use certified_nurbs::*;
pub use polygons::*;
