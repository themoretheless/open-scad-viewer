//! healing report operations: validate.
use super::*;
impl HealingReport {
pub fn validate(&self) -> Result<()> {
        let invalid = |message: &str| report_error(HEALING_REPORT_INVALID, message);
        if !valid_hash(&self.input_hash) || !valid_hash(&self.output_hash) {
            return Err(invalid(
                "Model hashes must be 64 lowercase hexadecimal digits",
            ));
        }
        let finite_nonnegative = |value: f64| value.is_finite() && value >= 0.;
        if !finite_nonnegative(self.displacement_budget_mm)
            || !finite_nonnegative(self.cumulative_displacement_mm)
            || self.cumulative_displacement_mm > self.displacement_budget_mm
            || !finite_nonnegative(self.gaps.before_mm)
            || !finite_nonnegative(self.gaps.after_mm)
        {
            return Err(invalid("Invalid gap or displacement budget"));
        }
        for (sequence, operation) in self.operations.iter().enumerate() {
            if operation.sequence != sequence
                || !finite_nonnegative(operation.model_tolerance_mm)
                || !finite_nonnegative(operation.entity_tolerance_mm)
                || !finite_nonnegative(operation.cumulative_tolerance_mm)
                || !finite_nonnegative(operation.displacement_mm)
                || !finite_nonnegative(operation.cumulative_displacement_mm)
                || operation.cumulative_displacement_mm > operation.cumulative_tolerance_mm
                || operation.rationale.is_empty()
            {
                return Err(invalid(
                    "Invalid operation sequence, rationale or displacement",
                ));
            }
        }
        let mut previous = 0.;
        for entry in &self.displacement {
            if entry.operation >= self.operations.len()
                || !finite_nonnegative(entry.actual_mm)
                || !finite_nonnegative(entry.before_mm)
                || !finite_nonnegative(entry.after_mm)
                || !finite_nonnegative(entry.cumulative_mm)
                || entry.cumulative_mm < previous
            {
                return Err(invalid("Invalid displacement ledger"));
            }
            previous = entry.cumulative_mm;
        }
        if self
            .displacement
            .last()
            .map(|entry| entry.cumulative_mm)
            .unwrap_or(0.)
            != self.cumulative_displacement_mm
        {
            return Err(invalid(
                "Displacement ledger total disagrees with the report",
            ));
        }
        if self
            .certificates
            .iter()
            .any(|certificate| certificate.operation >= self.operations.len())
        {
            return Err(invalid("Certificate refers to an unknown operation"));
        }
        match self.status {
            HealingStatus::Complete => {
                if self
                    .operations
                    .iter()
                    .any(|operation| operation.status != HealingOperationStatus::Applied)
                {
                    return Err(invalid("Complete report contains a non-applied operation"));
                }
            }
            HealingStatus::RolledBack => {
                if self.output_hash != self.input_hash
                    || !self.displacement.is_empty()
                    || self.cumulative_displacement_mm != 0.
                {
                    return Err(invalid(
                        "Rollback retained output displacement or model hash",
                    ));
                }
                let refused = self
                    .operations
                    .iter()
                    .position(|operation| operation.status == HealingOperationStatus::Refused)
                    .ok_or_else(|| invalid("Rollback has no refused operation"))?;
                if self
                    .operations
                    .iter()
                    .enumerate()
                    .any(|(sequence, operation)| {
                        operation.status
                            != if sequence < refused {
                                HealingOperationStatus::RolledBack
                            } else if sequence == refused {
                                HealingOperationStatus::Refused
                            } else {
                                HealingOperationStatus::Skipped
                            }
                    })
                {
                    return Err(invalid("Rollback operation statuses are inconsistent"));
                }
            }
        }
        for stage in &self.stages {
            if stage.stage.is_empty()
                || stage.skipped != stage.skipped_details.len()
                || stage.skipped_details.iter().any(|skip| {
                    skip.code.is_empty()
                        || skip.message.is_empty()
                        || skip.measured_mm.is_some_and(|value| {
                            !value.is_finite() || value < 0.
                        })
                })
            {
                return Err(invalid("Invalid healing stage metrics or skip evidence"));
            }
        }
        if let Some(unification) = &self.tolerance_unification {
            if !unification.target_mm.is_finite()
                || unification.target_mm <= 0.
                || !unification.cap_mm.is_finite()
                || unification.cap_mm <= 0.
                || unification.target_mm > unification.cap_mm
                || matches!(unification.policy, UnifyPolicy::Custom(target) if target.to_bits() != unification.target_mm.to_bits())
            {
                return Err(invalid("Invalid tolerance unification target or cap"));
            }
            let mut entities = BTreeSet::new();
            for change in &unification.changes {
                if !matches!(change.entity_id.kind(), TopoKind::Vertex | TopoKind::Edge)
                    || !entities.insert(change.entity_id)
                    || !change.before_mm.is_finite()
                    || change.before_mm <= 0.
                    || change.after_mm.to_bits() != unification.target_mm.to_bits()
                    || change.before_mm.to_bits() == change.after_mm.to_bits()
                {
                    return Err(invalid("Invalid tolerance unification change"));
                }
            }
        }
        Ok(())
    }
/// Fail CI when unresolved defects exceed the caller's explicit threshold.
    pub fn validate_ci_gate(&self, max_unresolved: usize) -> Result<()> {
        self.validate()?;
        if self.unresolved_defects.len() > max_unresolved {
            return Err(report_error(
                HEALING_REPORT_CI_GATE,
                format!(
                    "Unresolved healing defects {} exceed threshold {max_unresolved}",
                    self.unresolved_defects.len()
                ),
            ));
        }
        Ok(())
    }
pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        value_codec::to_string(&self.to_value()).map_err(|error| {
            report_error(
                HEALING_REPORT_CODEC,
                format!("Cannot encode healing report: {error}"),
            )
        })
    }
pub fn from_json(text: &str) -> Result<Self> {
        let value: Value = value_codec::from_str_strict(text).map_err(|error| {
            report_error(
                HEALING_REPORT_CODEC,
                format!("Cannot decode healing report JSON: {error}"),
            )
        })?;
        let report = Self::from_value(value).map_err(|error| {
            report_error(
                HEALING_REPORT_CODEC,
                format!("Invalid healing report: {error}"),
            )
        })?;
        report.validate()?;
        Ok(report)
    }
}
