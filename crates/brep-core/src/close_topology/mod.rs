//! Bounded mixed-dimensional and non-manifold topology.
//!
//! The legacy `Model` remains a manifold-cell representation.  This module
//! closes the topology gap by assembling validated cells and sheets into an
//! explicit incidence complex.  Shared carriers are relationships between
//! immutable manifold parts; they are never smuggled through `Model::validate`
//! and can therefore never weaken `GloballyAuditedSolidSet`.

use crate::solid_audit::{GloballyAuditedSolidSet, LocallyValidatedModel, SolidAuditCertificate};
use crate::{Model, TopoId, TopoKind, TopologyLineageRecord};
use nurbs_core::{Error, Result};
use std::collections::BTreeSet;

pub const CLOSE_TOPOLOGY_CAPABILITY: &str = "close-topology/1";
pub const TOLERANT_COMPLEX_HEAL_CAPABILITY: &str = "tolerant-complex-heal/1";
pub const CLOSE_TOPOLOGY_STEP_CAPABILITY: &str = "close-topology-step/1";
pub const CLOSE_TOPOLOGY_IGES_CAPABILITY: &str = "close-topology-iges/1";
const MAX_PARTS: usize = 64;
const MAX_RELATIONS: usize = 4096;
const MAX_RADIAL_USES: usize = 16;


pub use interchange::{encode_complex, decode_complex, interchange_certificate, export_complex_step, import_complex_step, export_complex_iges, import_complex_iges};

mod interchange;

mod body_role;
mod complex_heal_certificate;
#[cfg(test)]
mod tests;
pub use body_role::*;
pub use complex_heal_certificate::*;
