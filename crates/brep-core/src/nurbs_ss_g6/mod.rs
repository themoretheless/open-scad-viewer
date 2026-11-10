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

use crate::intersections::Options;
#[cfg(test)]
use crate::intersections::Coverage;
use crate::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use crate::solid_audit::{LocallyValidatedModel, SolidAuditCertificate};
use crate::trim_sew::{
    ChartKind, ClassificationCertificate, SewCertificate,
};
use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Model, TopoId, TopoKind, TopologyChange,
    TopologyLineageRecord,
};
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeSet;

type GraphFrame<'a> = (usize, &'a Surface, [f64; 3], [f64; 3], [f64; 3], [f64; 3]);

mod author_general_nurbs_boolean;
mod author_general_operation_naming;
mod g6_maturity;
mod nurbs_boolean_graph_containment_v4;
mod nurbs_boolean_graph_patch_v3;
mod nurbs_boolean_imprint_certificate;
#[cfg(test)]
mod tests;
/// Surface/surface certificates are owned by `nurbs-intersect` (ADR 0011).
pub use nurbs_intersect::nurbs_ss::{
    BranchCompletenessCertificate, BranchComponent, BranchGraph, BranchGraphAuthority, BranchOrientation, CertifiedBranchFragment, ExactIsoAuthority, ExactIsoAxis, ExactIsoIntersectionCertificate, FragmentAuthority, G6Component, G6Maturity, G6_CAPABILITY, G6_MATURITY, IsoEndpointLocation, IsoEndpointOwnership, RationalBezierDecomposition, RationalBezierPatchSpan, TensorSpanId, TransverseSpanEvidence, active_breaks, affine_graph_projection, affine_plane_uv, affine_uv_trace, as_uniform_bicubic, bbox, certify_exact_planar_iso_intersection, certify_exact_rational_planar_iso_intersection, certify_multispan_ss, complete_curve, complete_empty, complete_line, decompose_rational_bezier_spans, fragment_authority, is_bezier_le3_positive, is_uniform_bicubic_positive, join_certified_multispan_fragments, joined_endpoint_key, narrow_transverse_bezier_le3, narrow_transverse_bicubic, planar_support, positive_multispan_graph, rational_weight_bounds, rational_weights_constant_on_fixed_axis, refuse, uv_location,
};
pub use author_general_nurbs_boolean::*;
use author_general_operation_naming::*;
pub use g6_maturity::*;
pub use nurbs_boolean_graph_containment_v4::*;
pub use nurbs_boolean_graph_patch_v3::*;
pub use nurbs_boolean_imprint_certificate::*;
