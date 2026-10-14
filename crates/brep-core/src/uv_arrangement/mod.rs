//! Lifted-UV arrangements for frozen chart classes (G5b / F3 close).
//!
//! Supports multiple imprint curves per chart, monotone nesting, multiple holes,
//! periodic AnalyticCircle wrap strata, freeform chart touches, and missed-branch
//! mutation evidence. Periodic/pole rules refuse non-finite or wrap-ambiguous strata.

use crate::Model;
use nurbs_intersect::coverage_verifier::{UvCoverageCertificate, certify_lifted_uv_coverage};
use crate::trim_sew::{
    CellLabel, ChartEvent, ChartKind, ClassificationCertificate, SewCertificate,
    classify_chart_events, sew_closed_model_edges,
};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

mod uv_imprint_curve;
mod arrange_lifted_uv;
mod arrange_multispan_branch_graph_uv;
#[cfg(test)]
mod tests;
pub use uv_imprint_curve::*;
pub use arrange_lifted_uv::*;
pub use arrange_multispan_branch_graph_uv::*;
