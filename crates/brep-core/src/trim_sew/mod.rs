//! Lifted-UV arrangements, material cell classification, and exact sewing.
//!
//! Extends planar_trim to frozen chart classes (plane / poly / analytic circle).
//! Sewing is exact-match only: gap/duplicate/orientation mismatches refuse and
//! roll back atomically. No mesh weld, tolerance growth, or auto-heal.

use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result, curve::Curve, surface::Axis, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

use crate::{ChangeKind, ChangeProvenance, Model, TopoId, TopoKind, TopologyChange};
pub use nurbs_intersect::RationalCurveDefinition;
use nurbs_intersect::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};

#[cfg(test)]
#[path = "../tests/trim_sew.rs"]
mod tests;

mod authorized_heal_plan;
mod chart_kind;
mod classify_imprint_circle_events;
mod prove_curve_pcurve_correspondence;
pub use authorized_heal_plan::*;
pub use chart_kind::*;
pub use classify_imprint_circle_events::*;
pub use prove_curve_pcurve_correspondence::*;
