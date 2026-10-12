//! Display-independent mass properties. Green's theorem integrates each authored
//! UV trim; the divergence theorem integrates the oriented closed boundary.
//! Convergence evidence is numerical, never a geometric solid certificate.
use super::*;
use nurbs_intersect::predicate_evidence::{ComposedEvidence, PredicateEvidence, compose_predicate_evidence};
use crate::solid_audit::{SolidAuditCertificate, audit_solid};
use cad_predicates::ToleranceSpecIdentity;

pub const CERTIFIED_MASS_PROPERTIES_CAPABILITY: &str = "certified-mass-properties/2";

mod certified_cylinder_radius;
mod certified_mass_properties;
mod mass_properties;
#[cfg(test)]
mod tests;
pub use certified_cylinder_radius::*;
pub use certified_mass_properties::*;
pub use mass_properties::*;
