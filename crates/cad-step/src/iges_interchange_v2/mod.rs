//! Strict finite IGES 5.3 direct B-rep interchange.
//!
//! The legacy `iges-interchange/1` walking slice remains frozen.  This
//! successor parses physical 80-column records and maps the admitted IGES
//! topology graph directly; it never recognizes constructors, derives an
//! AABB, or crosses through a mesh.

use brep_core::analytic_features::FeatureCertificate;
use brep_core::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopoId, TopoKind, TopologyIds};
use brep_topology::Vertex;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub const IGES_INTERCHANGE_V2_CAPABILITY: &str = "iges-interchange/2";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 131_072;
const MAX_ENTITIES: usize = 65_536;
const MAX_VALUES: usize = 1_000_000;
const MAX_TOPOLOGY: usize = 32_768;

type TopologyMapping = Vec<(TopoKind, usize, usize)>;

mod iges_v2_report;
mod import_iges_v2;
mod export_iges_v2;
#[cfg(test)]
mod tests;
pub use iges_v2_report::*;
pub use import_iges_v2::*;
pub use export_iges_v2::*;
