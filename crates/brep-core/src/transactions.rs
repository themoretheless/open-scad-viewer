//! Native transactions retaining complete geometry and topology identity tables.
//! Admission uses the existing kernel validator, not a certified solid proof.
#[cfg(feature = "codec")]
mod serialization;
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
#[cfg(feature = "codec")]
pub use serialization::{
    MAX_MODEL_HISTORY_BYTES, MAX_MODEL_SNAPSHOT_BYTES, history_from_json, history_to_json,
};
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
        if model != reapplied {
            return Err(Error::new(
                "BREP_HEAL_NOT_IDEMPOTENT",
                "Reapplication changed healed geometry or topology identity",
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
