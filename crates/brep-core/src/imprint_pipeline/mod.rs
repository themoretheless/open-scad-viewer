//! Shared imprint / empty-algebra pipeline for analytic curved Boolean (A2).
//!
//! Topology authorship for admitted curved pairs must go through this module —
//! never through `prismatic_boolean` or mesh/Manifold. Sphere imprint retains
//! pair-specific UV scanning in `sphere_boolean`; all families converge on the
//! same closed-shell assembly and audit. Parallel cylinder walls are authored
//! as exact rational-arc profile splits after Complete SS generators.

use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;


#[cfg(test)]
#[path = "../tests/imprint_pipeline.rs"]
mod tests;

mod spatial_relation;
mod loft_curve_loops;
mod valence3_cuboid_max_corner;
mod cylinder_envelope;
pub use spatial_relation::*;
pub(crate) use loft_curve_loops::*;
pub(crate) use valence3_cuboid_max_corner::*;
pub use cylinder_envelope::*;
