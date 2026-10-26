//! Immutable original support faces for one authored B-rep edge.
//! Full face audits precede preparation; no fillet or topology edit is admitted.
use crate::{
    Error, Model, Result, TopoId,
    trimmed_face_recipe::{self, Boundary, Limits, QualifiedFace},
};
use cad_predicates::ToleranceContext;
use nurbs_core::curve::Curve;

mod propose_world_contacts;
mod support;
#[cfg(test)]
mod tests;
pub use propose_world_contacts::*;
pub use support::*;
