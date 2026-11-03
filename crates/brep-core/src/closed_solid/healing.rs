//! Repaired geometry is published only after fresh continuous solid admission.
//! Failed admission keeps the caller's original model as the transaction output.
use super::*;
use crate::healing_report::{self, AutoRepairOperation, HealingReport, HealingStatus};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometricHealingStatus {
    Accepted,
    RepairRefused,
    GeometryUnproved,
}
pub struct GeometricHealing {
    original: Model,
    solid: Option<QualifiedClosedSolid>,
    attempt: HealingReport,
    geometry: Option<Arc<volume_validity::Report>>,
    refusal: Option<Error>,
    status: GeometricHealingStatus,
}
impl GeometricHealing {
    pub fn status(&self) -> GeometricHealingStatus {
        self.status
    }
    pub fn model(&self) -> &Model {
        self.solid
            .as_ref()
            .map(|s| s.model())
            .unwrap_or(&self.original)
    }
    pub fn solid(&self) -> Option<&QualifiedClosedSolid> {
        self.solid.as_ref()
    }
    /// This is the attempted repair's audit, not authorization to publish it.
    /// `status()` and `solid()` include the independent geometric outcome.
    pub fn repair_attempt(&self) -> &HealingReport {
        &self.attempt
    }
    pub fn geometry(&self) -> Option<&volume_validity::Report> {
        self.geometry.as_deref()
    }
    pub fn refusal(&self) -> Option<&Error> {
        self.refusal.as_ref()
    }
}
pub fn heal_and_qualify(
    model: &Model,
    operations: &[AutoRepairOperation],
    policy: AdmissionPolicy,
) -> GeometricHealing {
    let run = healing_report::auto_repair(model, operations, None);
    let mut outcome = GeometricHealing {
        original: model.clone(),
        solid: None,
        attempt: run.report,
        geometry: None,
        refusal: None,
        status: GeometricHealingStatus::RepairRefused,
    };
    if outcome.attempt.status != HealingStatus::Complete {
        return outcome;
    }
    outcome.status = GeometricHealingStatus::GeometryUnproved;
    match qualify(run.model, policy) {
        Ok(admitted) => {
            outcome.geometry = Some(admitted.geometry.clone());
            outcome.solid = admitted.solid;
            if outcome.solid.is_some() {
                outcome.status = GeometricHealingStatus::Accepted;
            }
        }
        Err(error) => outcome.refusal = Some(error),
    }
    outcome
}
