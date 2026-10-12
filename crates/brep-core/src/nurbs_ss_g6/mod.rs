//! NURBS solid Booleans over the G6 surface–surface engine in `nurbs-intersect`:
//! narrow bicubic imprint (`nurbs-boolean-bezier-le3/*`), graph-patch cells and
//! the general multispan Boolean (`nurbs-boolean/1`). Every certificate here is
//! Model-bound; the surface-only certificates live in `nurbs_intersect::nurbs_ss`.

use crate::solid_audit::{LocallyValidatedModel, SolidAuditCertificate};
use crate::trim_sew::{ChartKind, ClassificationCertificate, SewCertificate};
use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Model, TopoId, TopoKind, TopologyChange,
    TopologyLineageRecord,
};
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use nurbs_intersect::nurbs_ss::*;
use nurbs_intersect::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use nurbs_intersect::Options;
use std::collections::BTreeSet;

type GraphFrame<'a> = (usize, &'a Surface, [f64; 3], [f64; 3], [f64; 3], [f64; 3]);

pub const NURBS_BOOLEAN_CAPABILITY_V1: &str = "nurbs-boolean-bezier-le3/1";
pub const NURBS_BOOLEAN_CAPABILITY: &str = "nurbs-boolean-bezier-le3/2";
pub const NURBS_BOOLEAN_CAPABILITY_V3: &str = "nurbs-boolean-bezier-le3/3";
pub const NURBS_BOOLEAN_CAPABILITY_V4: &str = "nurbs-boolean-bezier-le3/4";
pub const NURBS_BOOLEAN_CAPABILITY_V5: &str = "nurbs-boolean-bezier-le3/5";
pub const NURBS_BOOLEAN_CAPABILITY_V7: &str = "nurbs-boolean-bezier-le3/7";
pub const NURBS_BOOLEAN_CAPABILITY_V8: &str = "nurbs-boolean-bezier-le3/8";
/// Product Boolean successor over nurbs-ss/1 for the admitted multispan×affine cell.
pub const NURBS_BOOLEAN_SS_CAPABILITY: &str = "nurbs-boolean/1";
/// Authority for nurbs-boolean/1 general authorship. Distinct from finite `/8`.
pub const GENERAL_NURBS_BOOLEAN_AUTHORITY: &str = "author-general-nurbs-boolean";
pub const NURBS_BOOLEAN_V1_MATURITY: G6Maturity = G6Maturity::Unavailable;

mod nurbs_boolean_imprint_certificate;
mod nurbs_boolean_graph_patch_v3;
mod author_general_operation_naming;
mod author_general_nurbs_boolean;
mod nurbs_boolean_graph_containment_v4;
#[cfg(test)]
mod tests;
pub use nurbs_boolean_imprint_certificate::*;
pub use nurbs_boolean_graph_patch_v3::*;
use author_general_operation_naming::*;
pub use author_general_nurbs_boolean::*;
pub use nurbs_boolean_graph_containment_v4::*;
