//! Narrow G6 productization scaffold gated by the R1 ADR verdict (`narrow`),
//! expanded through R1.1 (elevated planar bilinear→bicubic) to Phase B:
//! non-rational single-span Bezier patches with deg_u,deg_v ∈ {1,2,3}, elevated
//! to uniform bicubic before the transverse gate (`nurbs-ss-bezier-le3/1`).
//!
//! Beyond Complete-empty: planar × planar may publish a transverse line as
//! Complete. Non-planar pairs may publish a Complete curve only when a Hausdorff
//! + parameter-correspondence certificate holds. Out-of-matrix pairs refuse.

//!
//! False Complete is a kill. Narrow Boolean imprint is Phase B (`bezier-le3`).

use crate::coverage_verifier::verify_complete_report;
use crate::intersections::{Coverage, Options, Report, UnresolvedReason};
use crate::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use crate::solid_audit::{LocallyValidatedModel, SolidAuditCertificate};
use crate::trim_sew::{
    ChartKind, ClassificationCertificate, RationalCurveDefinition, SewCertificate,
};
use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Model, TopoId, TopoKind, TopologyChange,
    TopologyLineageRecord,
};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

type GraphFrame<'a> = (usize, &'a Surface, [f64; 3], [f64; 3], [f64; 3], [f64; 3]);

mod g6_maturity;
mod certify_exact_planar_iso_intersection;
mod join_certified_multispan_fragments;
mod nurbs_boolean_imprint_certificate;
mod nurbs_boolean_graph_patch_v3;
mod author_general_operation_naming;
mod author_general_nurbs_boolean;
mod nurbs_boolean_graph_containment_v4;
#[cfg(test)]
mod tests;
pub use g6_maturity::*;
pub use certify_exact_planar_iso_intersection::*;
pub use join_certified_multispan_fragments::*;
pub use nurbs_boolean_imprint_certificate::*;
pub use nurbs_boolean_graph_patch_v3::*;
use author_general_operation_naming::*;
pub use author_general_nurbs_boolean::*;
pub use nurbs_boolean_graph_containment_v4::*;
