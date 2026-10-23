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

mod direct_builder;
mod exchange;
mod export;
mod identity;
mod import;
mod occurrences;
mod part21;
mod periodic;
mod placement;
mod read_curve;
mod read_surface;
mod writer;
#[cfg(test)]
mod tests;
pub(crate) use direct_builder::*;
pub(crate) use exchange::*;
pub use export::*;
pub(crate) use identity::*;
pub use import::*;
pub use occurrences::*;
pub(crate) use part21::*;
pub(crate) use periodic::*;
pub(crate) use placement::*;
pub(crate) use read_curve::*;
pub(crate) use read_surface::*;
pub(crate) use writer::*;

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

pub(crate) struct DirectBuildOptions {
    pub(crate) analytic_surfaces: bool,
    pub(crate) ap242_composition: bool,
    pub(crate) allow_degenerate: bool,
    pub(crate) whole_domain_proofs: bool,
    pub(crate) interior_point_selectors: bool,
    pub(crate) allow_open_shells: bool,
}

fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_V3_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct StepV3Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_entities: Vec<String>,
    pub metadata_loss: Vec<String>,
    pub instance_count: usize,
    pub reachable_count: usize,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub external_references: Vec<String>,
}

/// Lossless `/10` product envelope. The parsed B-rep is a derived editing
/// projection; `source` remains authoritative for occurrence and presentation
/// graphs that cannot be represented by `Model`.
#[derive(Clone, Debug)]
pub struct StepV10Document {
    pub source: String,
    pub graph_identity: String,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub operator_identities: Vec<String>,
    pub metadata_loss: Vec<String>,
}

/// Algebraic, whole-parameter-domain evidence carried by STEP `/8`.
///
/// `identity` names the exact rational tensor identity checked by the kernel;
/// it is deliberately not a sampling result.  A zero Jacobian lower bound is
/// permitted only for an explicitly named collapsed boundary.  In that case
/// `regular_open_domain` certifies the punctured parameter neighbourhood and
/// topology validation certifies that the whole boundary is one pole vertex.
#[derive(Clone, Debug)]
pub struct StepRegularityEvidence {
    pub carrier: &'static str,
    pub parameter_u: [f64; 2],
    pub parameter_v: [f64; 2],
    pub lifted_periods: [f64; 2],
    pub denominator_lower_bound: f64,
    pub jacobian_lower_bound: f64,
    pub collapsed_boundaries: Vec<&'static str>,
    pub regular_open_domain: bool,
    pub identity: &'static str,
}

#[derive(Clone, Debug)]
pub struct StepV8Certificate {
    pub capability: &'static str,
    pub complete: bool,
    pub regularity: Vec<StepRegularityEvidence>,
    pub sense_layers: [&'static str; 7],
    pub coupled_sense_cases: usize,
    pub notes: Vec<&'static str>,
}
