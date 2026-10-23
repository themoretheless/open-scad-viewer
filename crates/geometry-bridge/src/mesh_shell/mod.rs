//! Sampled inward shell for closed triangle meshes. BVHs keep grid queries bounded.
use polygon_core::{BuiltMesh, Error, Mesh, Result, solid::proximity::closest_triangle};

use crate::Acceleration;
use math_core::{cross, dot, sub};

mod shell;
mod lattice;
#[cfg(test)]
mod tests;
pub use shell::*;
pub use lattice::*;
