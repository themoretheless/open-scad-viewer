//! Repaired geometry is published only after fresh continuous solid admission.
//! Failed admission keeps the caller's original model as the transaction output.
use super::*;
use crate::healing_report::{self, AutoRepairOperation, HealingReport, HealingRun, HealingStatus};
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
    flipped_shells: Vec<usize>,
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
    /// Transfer the independently admitted body to the next modeling operation.
    pub fn into_solid(self) -> Option<QualifiedClosedSolid> {
        self.solid
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
    pub fn flipped_shells(&self) -> &[usize] {
        &self.flipped_shells
    }
}
pub fn heal_and_qualify(
    model: &Model,
    operations: &[AutoRepairOperation],
    policy: AdmissionPolicy,
) -> GeometricHealing {
    let run = healing_report::auto_repair(model, operations, None);
    admit_repair(model, run, policy, None)
}

/// Apply the supplied geometric repairs, rebuild material shell roles and
/// orientations, then independently admit the result as a closed solid.
/// A failed stage preserves the original model as the transaction output.
pub fn heal_and_rebuild_shells(
    model: &Model,
    operations: &[AutoRepairOperation],
    policy: ShellRepairPolicy,
) -> GeometricHealing {
    let run = healing_report::auto_repair_before_shell_rebuild(model, operations);
    admit_repair(model, run, policy.readmit, Some(policy))
}

/// Discover and sew uniquely matched endpoint gaps within the specified
/// displacement bounds, then rebuild and independently qualify the shells.
pub fn sew_gaps_and_rebuild_shells(
    model: &Model,
    sewing: crate::healing::SewingBudget,
    policy: ShellRepairPolicy,
) -> Result<GeometricHealing> {
    let run = crate::healing::tolerant_sew::sew_before_shell_repair(model, sewing)?;
    Ok(admit_repair(model, run, policy.readmit, Some(policy)))
}

fn admit_repair(
    model: &Model,
    run: HealingRun,
    admission: AdmissionPolicy,
    shell_policy: Option<ShellRepairPolicy>,
) -> GeometricHealing {
    let mut outcome = GeometricHealing {
        original: model.clone(),
        solid: None,
        attempt: run.report,
        geometry: None,
        flipped_shells: Vec::new(),
        refusal: None,
        status: GeometricHealingStatus::RepairRefused,
    };
    if outcome.attempt.status != HealingStatus::Complete {
        return outcome;
    }
    outcome.status = GeometricHealingStatus::GeometryUnproved;
    if let Some(policy) = shell_policy {
        let repaired = rebuild_even_odd_shells(&run.model, policy);
        outcome.geometry = repaired.attempts().last().cloned();
        outcome.flipped_shells = repaired.flipped_shells().to_vec();
        outcome.refusal = repaired.refusal().cloned();
        outcome.solid = repaired.solid().cloned();
        if outcome.solid.is_some() {
            outcome.status = GeometricHealingStatus::Accepted;
        }
        return outcome;
    }
    match qualify(run.model, admission) {
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
