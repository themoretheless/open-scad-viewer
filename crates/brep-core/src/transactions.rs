//! Native transactions retaining complete geometry and topology identity tables.
//! Admission uses the existing kernel validator, not a certified solid proof.
use crate::ChangeSet;
use crate::Model;
use crate::solid_audit::{LocallyValidatedModel, SolidAuditCertificate};
use crate::trim_sew::{
    AuthorizedHealPlan, HealOperation, SewCertificate, apply_authorized_heal,
    apply_authorized_heal_checked, prove_model_edge_correspondence,
};
use brep_topology::RevisionState;
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::{Error, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct ModelSnapshot {
    model: Arc<Model>,
}
impl ModelSnapshot {
    pub fn new(model: Model) -> Result<Self> {
        model.validate()?;
        Ok(Self {
            model: Arc::new(model),
        })
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
}
impl RevisionState for ModelSnapshot {
    type Model = Model;
    fn same_admission_policy(&self, _other: &Self) -> bool {
        true
    }
    fn model(&self) -> &Model {
        self.model()
    }
    fn try_edit(&self, edit: impl FnOnce(&mut Model) -> Result<()>) -> Result<Self> {
        let mut model = (*self.model).clone();
        edit(&mut model)?;
        Self::new(model)
    }
}
pub type ModelStore = brep_topology::RevisionStore<ModelSnapshot>;
pub type ModelCheckout = brep_topology::RevisionCheckout<ModelSnapshot>;
pub type ModelTransaction = brep_topology::RevisionTransaction<ModelSnapshot>;

/// Cooperative hard-cancel for an authorized heal before publication.
#[derive(Clone, Debug, Default)]
pub struct HealCancellation {
    cancelled: Arc<AtomicBool>,
}
impl HealCancellation {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Isolated heal transaction. The source snapshot is never mutated; failures,
/// rollback, and cancellation discard the staged model.
#[derive(Debug)]
pub struct AuthorizedHealTransaction {
    original: ModelSnapshot,
    staged: Option<Model>,
    staged_plan: Option<AuthorizedHealPlan>,
    displacement: Vec<HealDisplacementLedgerEntry>,
    cancellation: HealCancellation,
}
impl AuthorizedHealTransaction {
    pub fn begin(original: ModelSnapshot, cancellation: HealCancellation) -> Self {
        Self {
            original,
            staged: None,
            staged_plan: None,
            displacement: Vec::new(),
            cancellation,
        }
    }
    pub fn apply(&mut self, plan: &AuthorizedHealPlan) -> Result<()> {
        self.staged = None;
        self.staged_plan = None;
        self.displacement.clear();
        if self.cancellation.is_cancelled() {
            return Err(Error::new(
                "BREP_HEAL_CANCELLED",
                "Authorized heal was cancelled",
            ));
        }
        let mut cumulative = 0.;
        let displacement = plan
            .operations()
            .iter()
            .enumerate()
            .map(|(operation, recipe)| {
                if self.cancellation.is_cancelled() {
                    return Err(Error::new(
                        "BREP_HEAL_CANCELLED",
                        format!("Authorized heal was cancelled before operation {operation}"),
                    ));
                }
                let actual = recipe.displacement(self.original.model())?;
                cumulative += actual * recipe.write_set(self.original.model()).len() as f64;
                Ok(HealDisplacementLedgerEntry {
                    operation,
                    actual_mm: actual,
                    cumulative_mm: cumulative,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let staged = apply_authorized_heal_checked(self.original.model(), plan, |operation| {
            if self.cancellation.is_cancelled() {
                Err(Error::new(
                    "BREP_HEAL_CANCELLED",
                    format!("Authorized heal was cancelled at operation {operation}"),
                ))
            } else {
                Ok(())
            }
        })?;
        if self.cancellation.is_cancelled() {
            return Err(Error::new(
                "BREP_HEAL_CANCELLED",
                "Authorized heal was cancelled",
            ));
        }
        self.staged = Some(staged);
        self.staged_plan = Some(plan.clone());
        self.displacement = displacement;
        Ok(())
    }
    pub fn rollback(&mut self) {
        self.staged = None;
        self.staged_plan = None;
        self.displacement.clear();
    }
    pub fn staged(&self) -> Option<&Model> {
        self.staged.as_ref()
    }
    pub fn commit(mut self) -> Result<AuthorizedHealResult> {
        if self.cancellation.is_cancelled() {
            return Err(Error::new(
                "BREP_HEAL_CANCELLED",
                "Authorized heal was cancelled",
            ));
        }
        let model = self
            .staged
            .take()
            .ok_or_else(|| Error::new("BREP_HEAL_NOT_APPLIED", "No authorized heal is staged"))?;
        let plan = self.staged_plan.take().ok_or_else(|| {
            Error::new("BREP_HEAL_NOT_APPLIED", "No authorized heal plan is staged")
        })?;
        let audited = LocallyValidatedModel::new(model)?.audit()?;
        let sew = audited.certificate().sew.clone();
        let audit = audited.certificate().clone();
        let model = audited.into_model();
        if !model.persistent_naming_complete() {
            return Err(Error::new(
                "BREP_HEAL_NAMING_INCOMPLETE",
                "Authorized heal produced incomplete persistent naming",
            ));
        }
        let reapplied = apply_authorized_heal(&model, &plan)?;
        use value_codec::Serialize;
        if model.to_value() != reapplied.to_value() {
            return Err(Error::new(
                "BREP_HEAL_NOT_IDEMPOTENT",
                "Byte/idempotent reapplication changed the healed model",
            ));
        }
        let context = model.tolerance_context()?.spec_identity();
        let change_set = model.1.change_set.clone();
        change_set
            .validate()
            .map_err(|error| Error::new(error.code, error.message))?;
        let cumulative_displacement_mm = self
            .displacement
            .last()
            .map(|entry| entry.cumulative_mm)
            .unwrap_or(0.);
        Ok(AuthorizedHealResult {
            model,
            capability: "authorized-heal-gap-le1/2",
            status: "Complete",
            context,
            displacement: self.displacement,
            cumulative_displacement_mm,
            sew,
            audit,
            change_set,
            naming_complete: true,
        })
    }
}

#[derive(Clone, Debug)]
pub struct HealDisplacementLedgerEntry {
    pub operation: usize,
    pub actual_mm: f64,
    pub cumulative_mm: f64,
}

#[derive(Clone, Debug)]
pub struct AuthorizedHealResult {
    pub model: Model,
    pub capability: &'static str,
    pub status: &'static str,
    pub context: ToleranceSpecIdentity,
    pub displacement: Vec<HealDisplacementLedgerEntry>,
    pub cumulative_displacement_mm: f64,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
}
impl value_codec::Serialize for AuthorizedHealResult {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "model":self.model,
            "certificate":{
                "capability":self.capability,
                "status":self.status,
                "context":{
                    "version":self.context.version,
                    "canonical":self.context.canonical
                },
                "displacementLedger":self.displacement.iter().map(|entry| value_codec::json!({
                    "operation":entry.operation,
                    "actualMm":entry.actual_mm,
                    "cumulativeMm":entry.cumulative_mm
                })).collect::<Vec<_>>(),
                "cumulativeDisplacementMm":self.cumulative_displacement_mm,
                "sew":{
                    "matched":self.sew.matched,
                    "complete":self.sew.complete,
                    "displacementBudgetOk":self.sew.displacement_budget_ok
                },
                "audit":{
                    "ok":self.audit.ok,
                    "bodyCount":self.audit.body_count,
                    "shellCount":self.audit.shell_count,
                    "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked
                },
                "changeSet":self.change_set,
                "namingComplete":self.naming_complete
            }
        })
    }
}

pub fn authorized_heal_endpoint(
    model: &Model,
    vertex: usize,
    to: [f64; 3],
) -> Result<AuthorizedHealResult> {
    let context = model.tolerance_context()?;
    let expected_id =
        *model.1.vertices.get(vertex).ok_or_else(|| {
            Error::new("BREP_HEAL_RECIPE_INVALID", "Vertex index is out of range")
        })?;
    let proof = model
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.vertices.contains(&vertex))
        .find_map(|(edge, _)| prove_model_edge_correspondence(model, &context, edge).ok())
        .ok_or_else(|| {
            Error::new(
                "BREP_HEAL_PROOF_REQUIRED",
                "Vertex has no native opposite manifold boundary proof",
            )
        })?;
    let operation = HealOperation::EndpointSnap {
        vertex,
        expected_id,
        to,
        correspondence: proof,
    }
    .bind_native_proof();
    execute_native_plan(model, &context, vec![operation])
}

pub fn authorized_heal_refit(
    model: &Model,
    edge: usize,
    replacement: nurbs_core::curve::Curve,
) -> Result<AuthorizedHealResult> {
    let context = model.tolerance_context()?;
    let expected_id = *model
        .1
        .edges
        .get(edge)
        .ok_or_else(|| Error::new("BREP_HEAL_RECIPE_INVALID", "Edge index is out of range"))?;
    let proof = prove_model_edge_correspondence(model, &context, edge)?;
    let operation = HealOperation::CurveRefit {
        edge,
        expected_id,
        replacement,
        correspondence: proof,
    }
    .bind_native_proof();
    execute_native_plan(model, &context, vec![operation])
}

fn execute_native_plan(
    model: &Model,
    context: &cad_predicates::ToleranceContext,
    operations: Vec<HealOperation>,
) -> Result<AuthorizedHealResult> {
    let cell = context.spatial_bounds().absolute_mm;
    let plan = AuthorizedHealPlan::new(model, context, operations, cell, cell)?;
    let snapshot = ModelSnapshot::new(model.clone())?;
    let mut transaction = AuthorizedHealTransaction::begin(snapshot, HealCancellation::default());
    transaction.apply(&plan)?;
    transaction.commit()
}

/// Maximum encoded snapshot size; decoding checks this before JSON allocation.
pub const MAX_MODEL_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
impl ModelSnapshot {
    /// Versioned full-model snapshot. Store identities, checkouts and history
    /// are intentionally session-local and are not serialized here.
    pub fn to_json(&self) -> Result<String> {
        use value_codec::Serialize;
        let value = value_codec::json!({"format":"brep-model-snapshot","schemaVersion":1,"model":self.model().to_value()});
        let encoded = value_codec::to_string(&value)
            .map_err(|e| nurbs_core::Error::new("BREP_SNAPSHOT_INVALID", e.to_string()))?;
        if encoded.len() > MAX_MODEL_SNAPSHOT_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_SNAPSHOT_RESOURCE_LIMIT",
                "Snapshot exceeds 8 MiB",
            ));
        }
        Ok(encoded)
    }
    /// Decode a strict snapshot envelope and re-run kernel validation. This is
    /// not the legacy import route: identity tables must be explicitly present.
    pub fn from_json(encoded: &str) -> Result<Self> {
        use value_codec::Deserialize;
        let invalid = |message: &str| nurbs_core::Error::new("BREP_SNAPSHOT_INVALID", message);
        if encoded.len() > MAX_MODEL_SNAPSHOT_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_SNAPSHOT_RESOURCE_LIMIT",
                "Snapshot exceeds 8 MiB",
            ));
        }
        let value: value_codec::Value =
            value_codec::from_str_strict(encoded).map_err(|e| invalid(&e.to_string()))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid("Expected snapshot object"))?;
        if object.len() != 3
            || object.get("format") != Some(&value_codec::json!("brep-model-snapshot"))
            || object.get("schemaVersion") != Some(&value_codec::json!(1))
        {
            return Err(invalid("Unknown snapshot format, version or fields"));
        }
        let model = object
            .get("model")
            .ok_or_else(|| invalid("Missing model"))?;
        if !model
            .as_object()
            .is_some_and(|m| m.contains_key("topologyIds"))
        {
            return Err(invalid("Snapshot requires topology identity tables"));
        }
        let model = Model::from_value(model.clone()).map_err(|e| invalid(&e.to_string()))?;
        Self::new(model)
    }
}

pub const MAX_MODEL_HISTORY_BYTES: usize = 32 * 1024 * 1024;
/// Encode current state and both history stacks. Pending transactions and store
/// identity are not portable; loading always creates a fresh revision-zero store.
pub fn history_to_json(store: &ModelStore) -> Result<String> {
    let invalid = |m: &str| nurbs_core::Error::new("BREP_HISTORY_INVALID", m);
    let mut bytes = 0usize;
    let mut encode = |snapshot: &ModelSnapshot| -> Result<value_codec::Value> {
        let encoded = snapshot.to_json()?;
        bytes = bytes.saturating_add(encoded.len());
        if bytes > MAX_MODEL_HISTORY_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_HISTORY_RESOURCE_LIMIT",
                "History exceeds 32 MiB",
            ));
        }
        value_codec::from_str(&encoded).map_err(|e| invalid(&e.to_string()))
    };
    let current = encode(store.snapshot())?;
    let (undo, redo) = store.history_snapshots();
    let undo = undo.iter().map(&mut encode).collect::<Result<Vec<_>>>()?;
    let redo = redo.iter().map(&mut encode).collect::<Result<Vec<_>>>()?;
    let envelope = value_codec::json!({"format":"brep-model-history","schemaVersion":1,"current":current,"undo":undo,"redo":redo});
    let encoded = value_codec::to_string(&envelope).map_err(|e| invalid(&e.to_string()))?;
    if encoded.len() > MAX_MODEL_HISTORY_BYTES {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 MiB",
        ));
    }
    Ok(encoded)
}
pub fn history_from_json(encoded: &str) -> Result<ModelStore> {
    let invalid = |m: &str| nurbs_core::Error::new("BREP_HISTORY_INVALID", m);
    if encoded.len() > MAX_MODEL_HISTORY_BYTES {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 MiB",
        ));
    }
    let value: value_codec::Value =
        value_codec::from_str_strict(encoded).map_err(|e| invalid(&e.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected history object"))?;
    if object.len() != 5
        || object.get("format") != Some(&value_codec::json!("brep-model-history"))
        || object.get("schemaVersion") != Some(&value_codec::json!(1))
    {
        return Err(invalid("Unknown history format, version or fields"));
    }
    let undo = object
        .get("undo")
        .and_then(value_codec::Value::as_array)
        .ok_or_else(|| invalid("Missing undo stack"))?;
    let redo = object
        .get("redo")
        .and_then(value_codec::Value::as_array)
        .ok_or_else(|| invalid("Missing redo stack"))?;
    if undo.len().saturating_add(redo.len()) > brep_topology::MAX_TOPOLOGY_HISTORY {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 snapshots",
        ));
    }
    let decode = |value: &value_codec::Value| -> Result<ModelSnapshot> {
        ModelSnapshot::from_json(
            &value_codec::to_string(value).map_err(|e| invalid(&e.to_string()))?,
        )
    };
    let current = decode(
        object
            .get("current")
            .ok_or_else(|| invalid("Missing current snapshot"))?,
    )?;
    let undo = undo.iter().map(decode).collect::<Result<Vec<_>>>()?;
    let redo = redo.iter().map(decode).collect::<Result<Vec<_>>>()?;
    ModelStore::from_history(current, undo, redo)
}
