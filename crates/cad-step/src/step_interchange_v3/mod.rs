//! Bounded, direct Part 21 topology interchange.
//!
//! Unlike the compatibility `/1` and `/2` routes this module never recognizes
//! constructors or reconstructs an AABB.  It translates the selected
//! MANIFOLD_SOLID_BREP/BREP_WITH_VOIDS graph directly to/from `Model`.

use brep_core::analytic_features::FeatureCertificate;
use brep_core::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::{CoedgeTrim, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const STEP_INTERCHANGE_V3_CAPABILITY: &str = "step-interchange/3";
pub const STEP_INTERCHANGE_V4_CAPABILITY: &str = "step-interchange/4";
pub const STEP_INTERCHANGE_V5_CAPABILITY: &str = "step-interchange/5";
pub const STEP_INTERCHANGE_V6_CAPABILITY: &str = "step-interchange/6";
pub const STEP_INTERCHANGE_V7_CAPABILITY: &str = "step-interchange/7";
pub const STEP_INTERCHANGE_V8_CAPABILITY: &str = "step-interchange/8";
pub const STEP_INTERCHANGE_V9_CAPABILITY: &str = "step-interchange/9";
pub const STEP_INTERCHANGE_V10_CAPABILITY: &str = "step-interchange/10";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_INSTANCES: usize = 65_536;
const MAX_PARSE_DEPTH: usize = 32;
const MAX_GRAPH_DEPTH: usize = 64;
const MAX_OCCURRENCES: usize = 256;
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;


#[cfg(test)]
#[path = "../tests/step_interchange_v3.rs"]
mod tests;

mod step_v3_report;
mod vector;
mod analytic_surface;
mod validate_exchange_structure;
mod compose_step_v7_occurrences;
mod compose_step_v8_occurrences;
mod import_step_v3;
mod quadratic_patch_knots;
mod certify_periodic_surface;
mod export_step_v3;
pub use step_v3_report::*;
use vector::*;
use analytic_surface::*;
use validate_exchange_structure::*;
pub use compose_step_v7_occurrences::*;
pub use compose_step_v8_occurrences::*;
pub use import_step_v3::*;
use quadratic_patch_knots::*;
pub(crate) use certify_periodic_surface::*;
pub use export_step_v3::*;
