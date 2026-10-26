//! Post-parse import routing with explicit defect and repair evidence.
//!
//! Existing import entry points retain their current admission behavior. These
//! additive facades apply a policy after parsing. `BestEffort` attempts the same
//! supported repairs as `RepairWithReport`, but returns the residual model and
//! defect report instead of failing because repair was incomplete or unsupported.

use crate::defects::{DefectBudget, DefectKind, DefectReport, DefectSeverity, classify_defects};
use crate::healing_report::{AutoRepairOperation, HealingReport, auto_repair};
use crate::{Model, TopoKind};
use std::collections::BTreeMap;

pub const IMPORT_DEFECTS: &str = "BREP_IMPORT_DEFECTS";
pub const IMPORT_REPAIR_INCOMPLETE: &str = "BREP_IMPORT_REPAIR_INCOMPLETE";
pub const IMPORT_AUDIT_INCOMPLETE: &str = "BREP_IMPORT_AUDIT_INCOMPLETE";
pub const IMPORT_BUDGET_EXCEEDED: &str = "BREP_IMPORT_BUDGET_EXCEEDED";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImportPolicy {
    Strict,
    #[default]
    RepairWithReport,
    BestEffort,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportOutcome {
    pub model: Model,
    /// Residual defects after the selected routing, including warnings and
    /// unsupported classes. `complete=false` means the audit budget was exhausted.
    pub defects: DefectReport,
    pub repair: Option<HealingReport>,
    pub policy: ImportPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportError {
    pub code: String,
    pub message: String,
    pub defects: Option<DefectReport>,
    pub repair: Option<HealingReport>,
}

impl ImportError {
    fn new(
        code: &str,
        message: impl Into<String>,
        defects: Option<DefectReport>,
        repair: Option<HealingReport>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            defects,
            repair,
        }
    }
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ImportError {}

impl From<nurbs_core::Error> for ImportError {
    fn from(error: nurbs_core::Error) -> Self {
        Self::new(error.code, error.message, None, None)
    }
}

pub type ImportResult<T> = std::result::Result<T, ImportError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportBudget {
    pub defects: DefectBudget,
    /// Maximum number of supported snap operations in one repair attempt.
    pub max_repairs: usize,
}

impl Default for ImportBudget {
    fn default() -> Self {
        Self {
            defects: DefectBudget::default(),
            max_repairs: 64,
        }
    }
}

fn has_errors(report: &DefectReport) -> bool {
    report
        .defects
        .iter()
        .any(|defect| defect.severity == DefectSeverity::Error)
}

fn incomplete(report: DefectReport, repair: Option<HealingReport>) -> ImportError {
    ImportError::new(
        IMPORT_AUDIT_INCOMPLETE,
        "Defect classification exhausted its full-interval work budget",
        Some(report),
        repair,
    )
}

/// Derive only endpoint snaps that are fully determined by gap evidence.
/// Conflicting or unresolved targets are left as residual defects. No refit,
/// flip, deletion, or intersection repair is inferred without a real recipe.
pub(crate) fn supported_snaps(
    model: &Model,
    report: &DefectReport,
) -> nurbs_core::Result<Vec<AutoRepairOperation>> {
    let mut targets = BTreeMap::<usize, Option<[f64; 3]>>::new();
    for defect in report.defects.iter().filter(|defect| {
        defect.kind == DefectKind::GapEdges && defect.severity == DefectSeverity::Error
    }) {
        for edge_id in defect
            .entities
            .iter()
            .filter(|id| id.kind() == TopoKind::Edge)
        {
            let Some(edge_index) = model.1.edges.iter().position(|id| id == edge_id) else {
                continue;
            };
            let edge = &model.edges[edge_index];
            for vertex_id in defect
                .entities
                .iter()
                .filter(|id| id.kind() == TopoKind::Vertex)
            {
                let Some(vertex) = model.1.vertices.iter().position(|id| id == vertex_id) else {
                    continue;
                };
                let Some(end) = edge.vertices.iter().position(|&index| index == vertex) else {
                    continue;
                };
                let point = crate::curve_point(&edge.curve, end as f64)?;
                if point.len() != 3 {
                    continue;
                }
                let target = [point[0], point[1], point[2]];
                match targets.get(&vertex) {
                    Some(Some(previous)) if previous != &target => {
                        targets.insert(vertex, None);
                    }
                    Some(None) => {}
                    _ => {
                        targets.insert(vertex, Some(target));
                    }
                }
            }
        }
    }
    Ok(targets
        .into_iter()
        .filter_map(|(vertex, target)| target.map(|to| AutoRepairOperation::SnapVertex { vertex, to }))
        .collect())
}

/// Route an already parsed model through classification and supported repair.
/// Strict refuses every classified defect. RepairWithReport additionally refuses
/// incomplete audits and residual Error defects. BestEffort returns both cases
/// with explicit residual evidence and any rolled-back repair report.
pub fn apply_import_policy(
    model: Model,
    policy: ImportPolicy,
    budget: ImportBudget,
) -> ImportResult<ImportOutcome> {
    let initial = classify_defects(&model, budget.defects)?;
    if policy == ImportPolicy::Strict {
        if !initial.defects.is_empty() {
            return Err(ImportError::new(
                IMPORT_DEFECTS,
                "Strict import refused classified defects",
                Some(initial),
                None,
            ));
        }
        if !initial.complete {
            return Err(incomplete(initial, None));
        }
        return Ok(ImportOutcome {
            model,
            defects: initial,
            repair: None,
            policy,
        });
    }
    if policy == ImportPolicy::RepairWithReport && !initial.complete {
        return Err(incomplete(initial, None));
    }

    let operations = supported_snaps(&model, &initial)?;
    if operations.len() > budget.max_repairs {
        return Err(ImportError::new(
            IMPORT_BUDGET_EXCEEDED,
            format!(
                "Import requires {} supported repairs, exceeding budget {}",
                operations.len(),
                budget.max_repairs
            ),
            Some(initial),
            None,
        ));
    }
    let (model, repair) = if operations.is_empty() {
        (model, None)
    } else {
        let run = auto_repair(&model, &operations, None);
        (run.model, Some(run.report))
    };
    let residual = classify_defects(&model, budget.defects)?;
    if policy == ImportPolicy::RepairWithReport {
        if has_errors(&residual) {
            return Err(ImportError::new(
                IMPORT_REPAIR_INCOMPLETE,
                "Repair completed with residual Error-severity defects",
                Some(residual),
                repair,
            ));
        }
        if !residual.complete {
            return Err(incomplete(residual, repair));
        }
    }
    Ok(ImportOutcome {
        model,
        defects: residual,
        repair,
        policy,
    })
}

/// Additive STEP facade; legacy `import_step_v*` signatures are unchanged.
pub fn import_step_with_policy(
    text: &str,
    policy: ImportPolicy,
    budget: ImportBudget,
) -> ImportResult<ImportOutcome> {
    let (model, _, _) = crate::step_interchange_v3::import_step_v3(text)?;
    apply_import_policy(model, policy, budget)
}

/// Additive IGES facade using the same post-parse routing as STEP.
pub fn import_iges_v2_with_policy(
    text: &str,
    policy: ImportPolicy,
    budget: ImportBudget,
) -> ImportResult<ImportOutcome> {
    let (model, _, _) = crate::iges_interchange_v2::import_iges_v2(text)?;
    apply_import_policy(model, policy, budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap_model() -> Model {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let vertex = model.edges[0].vertices[0];
        model.vertices[vertex].point[0] += model.tolerance_mm * 2.;
        model
    }

    #[test]
    fn clean_step_file_passes_every_policy() {
        let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let (text, _, _) = crate::export_step_v3(&model).unwrap();
        assert_eq!(ImportPolicy::default(), ImportPolicy::RepairWithReport);
        for policy in [
            ImportPolicy::Strict,
            ImportPolicy::RepairWithReport,
            ImportPolicy::BestEffort,
        ] {
            let outcome = import_step_with_policy(&text, policy, ImportBudget::default()).unwrap();
            assert_eq!(outcome.policy, policy);
            assert!(outcome.defects.complete);
            assert!(outcome.defects.defects.is_empty());
            assert!(outcome.repair.is_none());
            outcome.model.validate().unwrap();
        }
    }

    #[test]
    fn strict_refuses_any_defect_and_retains_defect_report() {
        let error = apply_import_policy(
            gap_model(),
            ImportPolicy::Strict,
            ImportBudget::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, IMPORT_DEFECTS);
        assert!(error.repair.is_none());
        assert!(
            error
                .defects
                .as_ref()
                .unwrap()
                .defects
                .iter()
                .any(|defect| defect.kind == DefectKind::GapEdges)
        );

        let mut warning = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        warning
            .1
            .local_tolerances_mm
            .insert(warning.1.vertices[0], warning.tolerance_mm * 2.);
        let error = apply_import_policy(
            warning,
            ImportPolicy::Strict,
            ImportBudget::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, IMPORT_DEFECTS);
        assert!(error.defects.unwrap().defects.iter().all(|defect| {
            defect.kind == DefectKind::ToleranceInconsistent
                && defect.severity == DefectSeverity::Warning
        }));
    }

    #[test]
    fn excessive_gap_rolls_back_and_best_effort_returns_residuals() {
        let model = gap_model();
        let error = apply_import_policy(
            model.clone(),
            ImportPolicy::RepairWithReport,
            ImportBudget::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, IMPORT_REPAIR_INCOMPLETE);
        assert!(has_errors(error.defects.as_ref().unwrap()));
        let repair = error.repair.as_ref().unwrap();
        assert_eq!(repair.status, crate::healing_report::HealingStatus::RolledBack);
        assert!(!repair.operations.is_empty());
        assert_eq!(repair.input_hash, crate::healing_report::model_hash(&model));
        assert_eq!(repair.output_hash, repair.input_hash);

        let outcome = apply_import_policy(
            model.clone(),
            ImportPolicy::BestEffort,
            ImportBudget::default(),
        )
        .unwrap();
        assert_eq!(outcome.model, model);
        assert!(has_errors(&outcome.defects));
        assert_eq!(
            outcome.repair.as_ref().unwrap().status,
            crate::healing_report::HealingStatus::RolledBack
        );
        assert!(outcome.model.validate().is_err());
    }

    #[test]
    fn unsupported_defect_refuses_repair_but_best_effort_returns_residuals() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        model.edges[0].degenerate = true;
        let error = apply_import_policy(
            model.clone(),
            ImportPolicy::RepairWithReport,
            ImportBudget::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, IMPORT_REPAIR_INCOMPLETE);
        assert!(error.repair.is_none());
        assert!(has_errors(error.defects.as_ref().unwrap()));

        let outcome = apply_import_policy(
            model.clone(),
            ImportPolicy::BestEffort,
            ImportBudget::default(),
        )
        .unwrap();
        assert_eq!(outcome.model, model);
        assert!(outcome.repair.is_none());
        assert!(has_errors(&outcome.defects));
        assert!(
            outcome
                .defects
                .defects
                .iter()
                .any(|defect| defect.kind == DefectKind::DegenerateEdge)
        );
    }

    #[test]
    fn repair_budget_is_typed_and_preserves_defect_evidence() {
        let error = apply_import_policy(
            gap_model(),
            ImportPolicy::BestEffort,
            ImportBudget {
                max_repairs: 0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code, IMPORT_BUDGET_EXCEEDED);
        assert!(has_errors(error.defects.as_ref().unwrap()));
    }
}
