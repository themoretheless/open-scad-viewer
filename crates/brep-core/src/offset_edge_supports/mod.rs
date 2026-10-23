//! Immutable original support faces for one authored B-rep edge.
//! Full face audits precede preparation; no fillet or topology edit is admitted.
use crate::{
    trimmed_face_recipe::{self, Boundary, Limits, QualifiedFace},
    Error, Model, Result, TopoId,
};
use cad_predicates::ToleranceContext;
use nurbs_core::curve::Curve;

mod support;
mod propose_world_contacts;
#[cfg(test)]
mod tests;
pub use support::*;
pub use propose_world_contacts::*;
