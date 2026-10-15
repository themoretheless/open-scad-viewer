//! Bounded regularized solid CSG using oriented polygon BSPs.
//! Implemented here in Rust; no external geometry engine. Binary64 predicates
//! use a common normalized frame. Results are stitched and topology-checked;
//! this is not an exact-arithmetic or globally certified geometry algorithm.
#[cfg(feature = "codec")]
#[path = "../boolean/serialization.rs"]
mod serialization;
use crate::{
    BuiltMesh, Construction, Error, MAX_TRIANGLES, Mesh, Result, cross, dot, norm, scale, sub,
};
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, BTreeSet};
mod validation;
mod material_audit;
pub use material_audit::{MaterialAudit, MaterialShell, audit_material};
type Point = math_core::V3;

#[cfg(test)]
mod tests;

mod operation;
mod boolean;
mod difference_many_refs;
pub use operation::*;
pub use boolean::*;
pub use difference_many_refs::*;
