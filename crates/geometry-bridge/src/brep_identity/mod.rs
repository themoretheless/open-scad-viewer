//! Native identity-digest admission for semantic operations and occurrences.
//!
//! Byte-exact Rust port of the host identity derivers in
//! src/core/semanticProgram.ts (identityDigest + stableJson) and of the
//! identity subsets of validateOperations / validateOccurrences in
//! src/services/semanticProgramValidator.ts, including the staticParent
//! runtime ancestor-chain walk, moduleActivationAnchor (l.600) and
//! isChildrenExpansionContinuation (l.669). Out of scope (host-side):
//! provenance/tessellationIntents/diagnostics (brep_provenance.rs) and the
//! occurrence-production replay (brep_production.rs).
use super::{Result, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use value_codec::Number;


pub(crate) use canonical_json::{js_number, stable_json};

mod canonical_json;

#[cfg(test)]
mod tests;

mod identity_digest;
mod occurrence_index;
mod validate_occurrence_identity;
pub use identity_digest::*;
use occurrence_index::*;
pub use validate_occurrence_identity::*;
