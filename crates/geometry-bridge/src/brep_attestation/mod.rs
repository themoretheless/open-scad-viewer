//! Native admission of the complete SemanticProgramEnvelopeV1 object and
//! native source attestation.
//!
//! Port of the envelope-object subset of validateSemanticProgramV1
//! (src/services/semanticProgramValidator.ts l.2137: exact envelope keys and
//! the semantic-program-envelope schema literal) and of
//! semanticSourceDescriptor / attestSemanticProgramSource (l.2334/2348),
//! including a faithful port of parseGeometrySourceRoutingHeader
//! (src/core/geometryRouting.ts l.88).
//!
//! Envelope consistency semantics: the separately transported graph-begin
//! fields remain the execution inputs; when a component is transported both
//! ways the two copies must be deep-equal ("duplicates must match exactly"),
//! and components carried only inside the envelope are adopted into
//! admission. The envelope never replaces execution inputs.
use super::{Result, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::semantic_contract_error as invalid;

mod admit_envelope_object;
mod attest_source;
#[cfg(test)]
mod tests;
pub use admit_envelope_object::*;
pub use attest_source::*;
