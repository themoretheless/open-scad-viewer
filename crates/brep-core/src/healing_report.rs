//! Versioned healing artifact and atomic auto-repair audit trail.
//!
//! Every published operation goes through the existing authorized-heal
//! transaction path. A failure rolls the whole requested sequence back to the
//! input model; staged operations remain visible as `rolledBack`, never as
//! `applied`. JSON decoding rejects duplicate and unknown fields at every
//! report-owned object level, including refit curve definitions.

use crate::direct_edit::direct_edit_model_hash;
use crate::solid_audit::SolidAuditCertificate;
use crate::transactions::{
    HealDisplacementLedgerEntry, authorized_heal_endpoint, authorized_heal_endpoint_tolerant,
    authorized_heal_refit,
};
use crate::trim_sew::SewCertificate;
use crate::{Model, TopoId, TopoKind};
use nurbs_core::{Error, Result, curve::Curve};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use value_codec::{Deserialize, Serialize, Value, json};

pub const HEALING_REPORT_SCHEMA: &str = "brep-healing-report";
pub const HEALING_REPORT_VERSION: u32 = 3;
pub const HEALING_REPORT_CODEC: &str = "BREP_HEALING_REPORT_CODEC";
pub const HEALING_REPORT_INVALID: &str = "BREP_HEALING_REPORT_INVALID";
pub const HEALING_REPORT_CI_GATE: &str = "BREP_HEALING_CI_GATE";

fn report_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn decode<T: for<'a> Deserialize<'a>>(value: &Value, field: &str) -> value_codec::Result<T> {
    let field_value = value
        .as_object()
        .and_then(|object| object.get(field))
        .ok_or_else(|| value_codec::error(format!("Missing field {field}")))?;
    value_codec::from_value(field_value.clone())
        .map_err(|error| value_codec::error(format!("Invalid {field}: {error}")))
}

fn require_keys(value: &Value, expected: &[&str]) -> value_codec::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| value_codec::error("Expected object"))?;
    if object.len() != expected.len() || expected.iter().any(|field| !object.contains_key(*field)) {
        return Err(value_codec::error(format!(
            "Unknown or missing fields; expected exactly [{}]",
            expected.join(", ")
        )));
    }
    Ok(())
}

fn hash_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn model_hash(model: &Model) -> String {
    hash_hex(&direct_edit_model_hash(model))
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn certificate_ref(kind: &str, certificate: &impl std::fmt::Debug) -> String {
    format!(
        "{kind}:{}",
        hash_hex(&Sha256::digest(format!("{certificate:?}").as_bytes()))
    )
}

#[derive(Clone, Debug, PartialEq)]
pub enum AutoRepairOperation {
    SnapVertex { vertex: usize, to: [f64; 3] },
    RefitEdge { edge: usize, replacement: Curve },
}

impl AutoRepairOperation {
    pub(crate) fn target(&self, model: &Model) -> Option<TopoId> {
        match *self {
            Self::SnapVertex { vertex, .. } => model.1.vertices.get(vertex).copied(),
            Self::RefitEdge { edge, .. } => model.1.edges.get(edge).copied(),
        }
    }

    fn rationale(&self) -> &'static str {
        match self {
            Self::SnapVertex { .. } => {
                "Snap the vertex endpoint to the supplied coincident position within the authorized tolerance budget"
            }
            Self::RefitEdge { .. } => {
                "Refit the rational edge curve with one-to-one control correspondence within the authorized tolerance budget"
            }
        }
    }
}

impl Serialize for AutoRepairOperation {
    fn to_value(&self) -> Value {
        match self {
            Self::SnapVertex { vertex, to } => {
                json!({"kind": "endpointSnap", "vertex": vertex, "to": to})
            }
            Self::RefitEdge { edge, replacement } => {
                json!({"kind": "curveRefit", "edge": edge, "replacement": replacement})
            }
        }
    }
}

impl<'de> Deserialize<'de> for AutoRepairOperation {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let kind: String = decode(&value, "kind")?;
        match kind.as_str() {
            "endpointSnap" => {
                require_keys(&value, &["kind", "vertex", "to"])?;
                Ok(Self::SnapVertex {
                    vertex: decode(&value, "vertex")?,
                    to: decode(&value, "to")?,
                })
            }
            "curveRefit" => {
                require_keys(&value, &["kind", "edge", "replacement"])?;
                let replacement = value["replacement"].clone();
                require_keys(
                    &replacement,
                    &["degree", "knots", "controlPoints", "weights", "periodic"],
                )?;
                Ok(Self::RefitEdge {
                    edge: decode(&value, "edge")?,
                    replacement: value_codec::from_value(replacement)?,
                })
            }
            _ => Err(value_codec::error("Unknown auto-repair operation kind")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingStatus {
    Complete,
    RolledBack,
}

impl HealingStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::RolledBack => "rolledBack",
        }
    }

    fn parse(value: &Value) -> value_codec::Result<Self> {
        match value.as_str() {
            Some("complete") => Ok(Self::Complete),
            Some("rolledBack") => Ok(Self::RolledBack),
            _ => Err(value_codec::error("Unknown healing status")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingOperationStatus {
    Applied,
    RolledBack,
    Refused,
    Skipped,
}

impl HealingOperationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::RolledBack => "rolledBack",
            Self::Refused => "refused",
            Self::Skipped => "skipped",
        }
    }

    fn parse(value: &Value) -> value_codec::Result<Self> {
        match value.as_str() {
            Some("applied") => Ok(Self::Applied),
            Some("rolledBack") => Ok(Self::RolledBack),
            Some("refused") => Ok(Self::Refused),
            Some("skipped") => Ok(Self::Skipped),
            _ => Err(value_codec::error("Unknown healing operation status")),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingRefusal {
    pub operation: Option<usize>,
    pub code: String,
    pub message: String,
}

impl Serialize for HealingRefusal {
    fn to_value(&self) -> Value {
        json!({"operation": self.operation, "code": self.code, "message": self.message})
    }
}

impl<'de> Deserialize<'de> for HealingRefusal {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["operation", "code", "message"])?;
        Ok(Self {
            operation: decode(&value, "operation")?,
            code: decode(&value, "code")?,
            message: decode(&value, "message")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingConversion {
    pub entity_id: TopoId,
    pub from: String,
    pub to: String,
    pub rationale: String,
}

impl Serialize for HealingConversion {
    fn to_value(&self) -> Value {
        json!({
            "entityId": self.entity_id,
            "from": self.from,
            "to": self.to,
            "rationale": self.rationale
        })
    }
}

impl<'de> Deserialize<'de> for HealingConversion {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["entityId", "from", "to", "rationale"])?;
        Ok(Self {
            entity_id: decode(&value, "entityId")?,
            from: decode(&value, "from")?,
            to: decode(&value, "to")?,
            rationale: decode(&value, "rationale")?,
        })
    }
}

/// Target-selection policy for edge/vertex tolerance unification.
///
/// `RaiseToMax` implements the import contract `max(model, local)` and may
/// raise the model tolerance only up to the explicit budget cap. `ClampToModel`
/// keeps the model scalar fixed and normalizes every local override to it.
/// `Custom` selects an explicit common target. Lowering tolerances is allowed;
/// every selected target above the cap is refused before model mutation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnifyPolicy {
    RaiseToMax,
    ClampToModel,
    Custom(f64),
}

impl UnifyPolicy {
    fn kind(self) -> &'static str {
        match self {
            Self::RaiseToMax => "raiseToMax",
            Self::ClampToModel => "clampToModel",
            Self::Custom(_) => "custom",
        }
    }
}

impl Serialize for UnifyPolicy {
    fn to_value(&self) -> Value {
        match *self {
            Self::Custom(target_mm) => json!({"kind": self.kind(), "targetMm": target_mm}),
            _ => json!({"kind": self.kind()}),
        }
    }
}

impl<'de> Deserialize<'de> for UnifyPolicy {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let kind: String = decode(&value, "kind")?;
        match kind.as_str() {
            "raiseToMax" => {
                require_keys(&value, &["kind"])?;
                Ok(Self::RaiseToMax)
            }
            "clampToModel" => {
                require_keys(&value, &["kind"])?;
                Ok(Self::ClampToModel)
            }
            "custom" => {
                require_keys(&value, &["kind", "targetMm"])?;
                Ok(Self::Custom(decode(&value, "targetMm")?))
            }
            _ => Err(value_codec::error("Unknown tolerance unification policy")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToleranceChange {
    pub entity_id: TopoId,
    pub before_mm: f64,
    pub after_mm: f64,
}

impl Serialize for ToleranceChange {
    fn to_value(&self) -> Value {
        json!({
            "entityId": self.entity_id,
            "beforeMm": self.before_mm,
            "afterMm": self.after_mm
        })
    }
}

impl<'de> Deserialize<'de> for ToleranceChange {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["entityId", "beforeMm", "afterMm"])?;
        Ok(Self {
            entity_id: decode(&value, "entityId")?,
            before_mm: decode(&value, "beforeMm")?,
            after_mm: decode(&value, "afterMm")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToleranceUnificationReport {
    pub policy: UnifyPolicy,
    pub target_mm: f64,
    pub cap_mm: f64,
    pub changes: Vec<ToleranceChange>,
}

impl Serialize for ToleranceUnificationReport {
    fn to_value(&self) -> Value {
        json!({
            "policy": self.policy,
            "targetMm": self.target_mm,
            "capMm": self.cap_mm,
            "changes": self.changes
        })
    }
}

impl<'de> Deserialize<'de> for ToleranceUnificationReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["policy", "targetMm", "capMm", "changes"])?;
        Ok(Self {
            policy: decode(&value, "policy")?,
            target_mm: decode(&value, "targetMm")?,
            cap_mm: decode(&value, "capMm")?,
            changes: decode(&value, "changes")?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HealingGapSummary {
    pub before_mm: f64,
    pub after_mm: f64,
}

impl Serialize for HealingGapSummary {
    fn to_value(&self) -> Value {
        json!({"beforeMm": self.before_mm, "afterMm": self.after_mm})
    }
}

impl<'de> Deserialize<'de> for HealingGapSummary {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["beforeMm", "afterMm"])?;
        Ok(Self {
            before_mm: decode(&value, "beforeMm")?,
            after_mm: decode(&value, "afterMm")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingDisplacementEntry {
    pub operation: usize,
    pub entity_ids: Vec<TopoId>,
    pub actual_mm: f64,
    pub before_mm: f64,
    pub after_mm: f64,
    pub cumulative_mm: f64,
}

impl From<HealDisplacementLedgerEntry> for HealingDisplacementEntry {
    fn from(entry: HealDisplacementLedgerEntry) -> Self {
        Self {
            operation: entry.operation,
            entity_ids: entry.entity_ids,
            actual_mm: entry.actual_mm,
            before_mm: entry.before_mm,
            after_mm: entry.after_mm,
            cumulative_mm: entry.cumulative_mm,
        }
    }
}

impl Serialize for HealingDisplacementEntry {
    fn to_value(&self) -> Value {
        json!({
            "operation": self.operation,
            "entityIds": self.entity_ids,
            "actualMm": self.actual_mm,
            "beforeMm": self.before_mm,
            "afterMm": self.after_mm,
            "cumulativeMm": self.cumulative_mm
        })
    }
}

impl<'de> Deserialize<'de> for HealingDisplacementEntry {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(
            &value,
            &[
                "operation",
                "entityIds",
                "actualMm",
                "beforeMm",
                "afterMm",
                "cumulativeMm",
            ],
        )?;
        Ok(Self {
            operation: decode(&value, "operation")?,
            entity_ids: decode(&value, "entityIds")?,
            actual_mm: decode(&value, "actualMm")?,
            before_mm: decode(&value, "beforeMm")?,
            after_mm: decode(&value, "afterMm")?,
            cumulative_mm: decode(&value, "cumulativeMm")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingCertificateRefs {
    pub operation: usize,
    pub sew: String,
    pub audit: String,
    pub sew_matched: usize,
    pub sew_complete: bool,
    pub sew_displacement_budget_ok: bool,
    pub audit_ok: bool,
}

impl HealingCertificateRefs {
    fn new(operation: usize, sew: &SewCertificate, audit: &SolidAuditCertificate) -> Self {
        Self {
            operation,
            sew: certificate_ref("sew", sew),
            audit: certificate_ref("audit", audit),
            sew_matched: sew.matched,
            sew_complete: sew.complete,
            sew_displacement_budget_ok: sew.displacement_budget_ok,
            audit_ok: audit.ok,
        }
    }
}

impl Serialize for HealingCertificateRefs {
    fn to_value(&self) -> Value {
        json!({
            "operation": self.operation,
            "sew": self.sew,
            "audit": self.audit,
            "sewMatched": self.sew_matched,
            "sewComplete": self.sew_complete,
            "sewDisplacementBudgetOk": self.sew_displacement_budget_ok,
            "auditOk": self.audit_ok
        })
    }
}

impl<'de> Deserialize<'de> for HealingCertificateRefs {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(
            &value,
            &[
                "operation",
                "sew",
                "audit",
                "sewMatched",
                "sewComplete",
                "sewDisplacementBudgetOk",
                "auditOk",
            ],
        )?;
        Ok(Self {
            operation: decode(&value, "operation")?,
            sew: decode(&value, "sew")?,
            audit: decode(&value, "audit")?,
            sew_matched: decode(&value, "sewMatched")?,
            sew_complete: decode(&value, "sewComplete")?,
            sew_displacement_budget_ok: decode(&value, "sewDisplacementBudgetOk")?,
            audit_ok: decode(&value, "auditOk")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingOperationRecord {
    pub sequence: usize,
    pub operation: AutoRepairOperation,
    pub target: Option<TopoId>,
    pub entity_ids: Vec<TopoId>,
    pub rationale: String,
    pub model_tolerance_mm: f64,
    pub entity_tolerance_mm: f64,
    pub cumulative_tolerance_mm: f64,
    pub displacement_mm: f64,
    pub cumulative_displacement_mm: f64,
    pub status: HealingOperationStatus,
}

impl Serialize for HealingOperationRecord {
    fn to_value(&self) -> Value {
        json!({
            "sequence": self.sequence,
            "operation": self.operation,
            "target": self.target,
            "entityIds": self.entity_ids,
            "rationale": self.rationale,
            "modelToleranceMm": self.model_tolerance_mm,
            "entityToleranceMm": self.entity_tolerance_mm,
            "cumulativeToleranceMm": self.cumulative_tolerance_mm,
            "displacementMm": self.displacement_mm,
            "cumulativeDisplacementMm": self.cumulative_displacement_mm,
            "status": self.status.as_str()
        })
    }
}

impl<'de> Deserialize<'de> for HealingOperationRecord {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(
            &value,
            &[
                "sequence",
                "operation",
                "target",
                "entityIds",
                "rationale",
                "modelToleranceMm",
                "entityToleranceMm",
                "cumulativeToleranceMm",
                "displacementMm",
                "cumulativeDisplacementMm",
                "status",
            ],
        )?;
        Ok(Self {
            sequence: decode(&value, "sequence")?,
            operation: decode(&value, "operation")?,
            target: decode(&value, "target")?,
            entity_ids: decode(&value, "entityIds")?,
            rationale: decode(&value, "rationale")?,
            model_tolerance_mm: decode(&value, "modelToleranceMm")?,
            entity_tolerance_mm: decode(&value, "entityToleranceMm")?,
            cumulative_tolerance_mm: decode(&value, "cumulativeToleranceMm")?,
            displacement_mm: decode(&value, "displacementMm")?,
            cumulative_displacement_mm: decode(&value, "cumulativeDisplacementMm")?,
            status: HealingOperationStatus::parse(&value["status"])?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingStageSkip {
    pub entity: TopoId,
    pub code: String,
    pub message: String,
    pub measured_mm: Option<f64>,
}

impl Serialize for HealingStageSkip {
    fn to_value(&self) -> Value {
        json!({"entity": self.entity, "code": self.code, "message": self.message, "measuredMm": self.measured_mm})
    }
}

impl<'de> Deserialize<'de> for HealingStageSkip {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["entity", "code", "message", "measuredMm"])?;
        Ok(Self {
            entity: decode(&value, "entity")?,
            code: decode(&value, "code")?,
            message: decode(&value, "message")?,
            measured_mm: decode(&value, "measuredMm")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingStageReport {
    pub stage: String,
    pub fixed: usize,
    pub skipped: usize,
    pub remaining: usize,
    pub skipped_details: Vec<HealingStageSkip>,
}

impl Serialize for HealingStageReport {
    fn to_value(&self) -> Value {
        json!({"stage": self.stage, "fixed": self.fixed, "skipped": self.skipped, "remaining": self.remaining, "skippedDetails": self.skipped_details})
    }
}

impl<'de> Deserialize<'de> for HealingStageReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        require_keys(&value, &["stage", "fixed", "skipped", "remaining", "skippedDetails"])?;
        Ok(Self {
            stage: decode(&value, "stage")?,
            fixed: decode(&value, "fixed")?,
            skipped: decode(&value, "skipped")?,
            remaining: decode(&value, "remaining")?,
            skipped_details: decode(&value, "skippedDetails")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingReport {
    pub input_hash: String,
    pub output_hash: String,
    pub status: HealingStatus,
    pub operations: Vec<HealingOperationRecord>,
    pub gaps: HealingGapSummary,
    pub sewn_edges: Vec<TopoId>,
    pub conversions: Vec<HealingConversion>,
    pub displacement: Vec<HealingDisplacementEntry>,
    pub certificates: Vec<HealingCertificateRefs>,
    pub displacement_budget_mm: f64,
    pub cumulative_displacement_mm: f64,
    pub unresolved_defects: Vec<HealingRefusal>,
    pub refusal_reasons: Vec<HealingRefusal>,
    /// Present when this artifact records a tolerance-unification pass.
    pub tolerance_unification: Option<ToleranceUnificationReport>,
    /// Per-stage fix/skip/residual metrics for compound healing pipelines.
    pub stages: Vec<HealingStageReport>,
}



impl Serialize for HealingReport {
    fn to_value(&self) -> Value {
        json!({
            "schema": HEALING_REPORT_SCHEMA,
            "version": HEALING_REPORT_VERSION,
            "inputHash": self.input_hash,
            "outputHash": self.output_hash,
            "status": self.status.as_str(),
            "operations": self.operations,
            "gaps": self.gaps,
            "sewnEdges": self.sewn_edges,
            "conversions": self.conversions,
            "displacement": self.displacement,
            "certificates": self.certificates,
            "displacementBudgetMm": self.displacement_budget_mm,
            "cumulativeDisplacementMm": self.cumulative_displacement_mm,
            "unresolvedDefects": self.unresolved_defects,
            "refusalReasons": self.refusal_reasons,
            "toleranceUnification": self.tolerance_unification,
            "stages": self.stages
        })
    }
}

impl<'de> Deserialize<'de> for HealingReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let schema: String = decode(&value, "schema")?;
        let version: u32 = decode(&value, "version")?;
        if schema != HEALING_REPORT_SCHEMA || !(1..=HEALING_REPORT_VERSION).contains(&version) {
            return Err(value_codec::error(format!(
                "Unsupported healing report schema/version: {schema}/{version}"
            )));
        }
        let mut fields = vec![
            "schema",
            "version",
            "inputHash",
            "outputHash",
            "status",
            "operations",
            "gaps",
            "sewnEdges",
            "conversions",
            "displacement",
            "certificates",
            "displacementBudgetMm",
            "cumulativeDisplacementMm",
            "unresolvedDefects",
            "refusalReasons",
        ];
        if version >= 2 {
            fields.push("toleranceUnification");
        }
        if version >= 3 {
            fields.push("stages");
        }
        require_keys(&value, &fields)?;
        Ok(Self {
            input_hash: decode(&value, "inputHash")?,
            output_hash: decode(&value, "outputHash")?,
            status: HealingStatus::parse(&value["status"])?,
            operations: decode(&value, "operations")?,
            gaps: decode(&value, "gaps")?,
            sewn_edges: decode(&value, "sewnEdges")?,
            conversions: decode(&value, "conversions")?,
            displacement: decode(&value, "displacement")?,
            certificates: decode(&value, "certificates")?,
            displacement_budget_mm: decode(&value, "displacementBudgetMm")?,
            cumulative_displacement_mm: decode(&value, "cumulativeDisplacementMm")?,
            unresolved_defects: decode(&value, "unresolvedDefects")?,
            refusal_reasons: decode(&value, "refusalReasons")?,
            tolerance_unification: if version == 1 {
                None
            } else {
                decode(&value, "toleranceUnification")?
            },
            stages: if version < 3 {
                Vec::new()
            } else {
                decode(&value, "stages")?
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealingRun {
    pub model: Model,
    pub report: HealingReport,
}

fn gap_summary(displacement: &[HealingDisplacementEntry]) -> HealingGapSummary {
    HealingGapSummary {
        before_mm: displacement
            .iter()
            .map(|entry| entry.before_mm)
            .fold(0., f64::max),
        after_mm: displacement
            .iter()
            .map(|entry| entry.after_mm)
            .fold(0., f64::max),
    }
}

/// Execute real authorized snap/refit operations in caller order.
///
/// The optional prior report makes replay on its output a no-op only when its
/// hashes, successful status, validation and complete operation list all match.
pub fn auto_repair(
    model: &Model,
    operations: &[AutoRepairOperation],
    prior: Option<&HealingReport>,
) -> HealingRun {
    auto_repair_impl(model, operations, prior, None)
}

pub(crate) fn auto_repair_tolerant(
    model: &Model,
    operations: &[AutoRepairOperation],
    per_entity_cap_mm: f64,
    cumulative_cap_mm: f64,
) -> HealingRun {
    auto_repair_impl(
        model,
        operations,
        None,
        Some((per_entity_cap_mm, cumulative_cap_mm)),
    )
}



#[cfg(test)]
#[path = "healing_report/tests/mod.rs"]
mod tests;

#[path="healing_report/methods_validate.rs"]
mod methods_validate;
#[path="healing_report/algorithm_auto_repair_impl.rs"]
mod algorithm_auto_repair_impl;
use algorithm_auto_repair_impl::auto_repair_impl;
