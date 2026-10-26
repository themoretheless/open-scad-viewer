//! Budgeted sewing of uniquely determined imported endpoint gaps.
//!
//! This is deliberately not a free-edge matcher. Candidates come only from
//! `GapEdges` evidence and the existing unique endpoint-snap policy. Every
//! authorized sequence is staged privately; a refusal publishes no partial
//! geometry or sewed-edge markers.

use crate::defects::{DefectBudget, DefectKind, DefectReport, classify_defects};
use crate::healing_report::{
    AutoRepairOperation, HealingRefusal, HealingRun, HealingStageReport, HealingStageSkip,
    HealingStatus, auto_repair_tolerant, model_hash,
};
use crate::Model;
use nurbs_core::{Error, Result};
use std::collections::BTreeSet;

pub const SEW_BUDGET_EXCEEDED: &str = "BREP_SEW_BUDGET_EXCEEDED";
pub const SEW_AUDIT_INCOMPLETE: &str = "BREP_SEW_AUDIT_INCOMPLETE";
pub const SEW_USER_CAP_EXCEEDED: &str = "BREP_SEW_USER_CAP_EXCEEDED";
pub const SEW_CUMULATIVE_CAP_EXCEEDED: &str = "BREP_SEW_CUMULATIVE_CAP_EXCEEDED";
pub const SEW_ALREADY_SEWED: &str = "BREP_SEW_ALREADY_SEWED";
pub const SEW_UNSUPPORTED: &str = "BREP_SEW_UNSUPPORTED";
pub const SEW_INCOMPLETE: &str = "BREP_SEW_INCOMPLETE";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SewingBudget {
    pub defects: DefectBudget,
    /// Maximum physical displacement of any one snapped vertex.
    pub user_cap_mm: f64,
    /// Maximum displacement multiplied by the authorized write-set sizes.
    pub cumulative_cap_mm: f64,
    pub max_candidates: usize,
}

fn fail(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn classify(model: &Model, budget: DefectBudget) -> Result<DefectReport> {
    let report = classify_defects(model, budget)?;
    if !report.complete {
        return Err(fail(
            SEW_AUDIT_INCOMPLETE,
            "Tolerant sewing requires a complete full-interval defect audit",
        ));
    }
    Ok(report)
}

fn gap_count(report: &DefectReport) -> usize {
    report
        .defects
        .iter()
        .filter(|defect| defect.kind == DefectKind::GapEdges)
        .count()
}

fn incident_edges(model: &Model, vertex: usize) -> Vec<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(edge, model)| model.vertices.contains(&vertex).then_some(edge))
        .collect()
}

fn append_skip_reasons(report: &mut crate::healing_report::HealingReport) {
    let reasons = report
        .stages
        .iter()
        .flat_map(|stage| {
            stage
                .skipped_details
                .iter()
                .map(|skip| HealingRefusal {
                    operation: None,
                    code: skip.code.clone(),
                    message: format!("{}: {}", stage.stage, skip.message),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    report.unresolved_defects.extend(reasons.clone());
    report.refusal_reasons.extend(reasons);
}

/// Sew only endpoint gaps with a unique existing curve-endpoint target.
///
/// `user_cap_mm` limits each vertex displacement. `cumulative_cap_mm` uses the
/// same write-set weighting as the authorized transaction. Cap refusals and
/// post-repair residual gaps never expose a partially sewn model.
pub fn sew_tolerant(model: &Model, budget: SewingBudget) -> Result<HealingRun> {
    if !budget.user_cap_mm.is_finite()
        || budget.user_cap_mm <= 0.
        || !budget.cumulative_cap_mm.is_finite()
        || budget.cumulative_cap_mm <= 0.
        || budget.max_candidates == 0
        || budget.max_candidates > 4096
    {
        return Err(fail(
            SEW_BUDGET_EXCEEDED,
            "Sewing caps must be finite and positive; candidates must be in 1..4096",
        ));
    }

    let initial = classify(model, budget.defects)?;
    let candidates = crate::import_policy::supported_snaps(model, &initial)?;
    if candidates.len() > budget.max_candidates {
        return Err(fail(
            SEW_BUDGET_EXCEEDED,
            format!(
                "Tolerant sewing candidates {} exceed budget {}",
                candidates.len(),
                budget.max_candidates
            ),
        ));
    }

    let mut operations = Vec::with_capacity(candidates.len());
    let mut measured = Vec::with_capacity(candidates.len());
    let mut skipped_details = Vec::new();
    let mut required_cumulative = 0.;
    for operation in candidates {
        let AutoRepairOperation::SnapVertex { vertex, to } = operation else {
            return Err(fail(
                SEW_UNSUPPORTED,
                "Tolerant sewing accepts only endpoint snap candidates",
            ));
        };
        let incident = incident_edges(model, vertex);
        if incident
            .iter()
            .any(|&edge| model.1.sewed_edges.contains(&model.1.edges[edge]))
        {
            return Err(fail(
                SEW_ALREADY_SEWED,
                "A requested snap would rewrite an already sewed edge",
            ));
        }
        let actual = crate::distance(&model.vertices[vertex].point, &to);
        required_cumulative += actual * (incident.len() + 1) as f64;
        let entity = model.1.vertices[vertex];
        if actual > budget.user_cap_mm {
            skipped_details.push(HealingStageSkip {
                entity,
                code: SEW_USER_CAP_EXCEEDED.into(),
                message: format!(
                    "Measured endpoint displacement {actual} exceeds user cap {} mm",
                    budget.user_cap_mm
                ),
                measured_mm: Some(actual),
            });
        } else if required_cumulative > budget.cumulative_cap_mm {
            skipped_details.push(HealingStageSkip {
                entity,
                code: SEW_CUMULATIVE_CAP_EXCEEDED.into(),
                message: format!(
                    "Write-set-weighted displacement {required_cumulative} exceeds cumulative cap {} mm",
                    budget.cumulative_cap_mm
                ),
                measured_mm: Some(actual),
            });
        }
        measured.push(actual);
        operations.push(AutoRepairOperation::SnapVertex { vertex, to });
    }

    let mut run = auto_repair_tolerant(
        model,
        &operations,
        budget.user_cap_mm,
        budget.cumulative_cap_mm,
    );

    if run.report.status == HealingStatus::Complete && !operations.is_empty() {
        let mut sewn = BTreeSet::new();
        for operation in &operations {
            let AutoRepairOperation::SnapVertex { vertex, .. } = *operation else {
                return Err(fail(SEW_UNSUPPORTED, "Unexpected tolerant sewing operation"));
            };
            for edge in incident_edges(&run.model, vertex) {
                sewn.insert(run.model.1.edges[edge]);
            }
        }
        run.model.1.sewed_edges.extend(sewn.iter().copied());
        run.report.sewn_edges = sewn.into_iter().collect();
        run.report.output_hash = model_hash(&run.model);
        run.model.validate().map_err(|error| {
            fail(
                SEW_INCOMPLETE,
                format!("Sewed model failed post-validation: {}", error.message),
            )
        })?;
    }

    let residual = classify(&run.model, budget.defects)?;
    let remaining = gap_count(&residual);
    if run.report.status == HealingStatus::Complete && !operations.is_empty() && remaining > 0 {
        return Err(fail(
            SEW_INCOMPLETE,
            "Authorized snaps left residual endpoint gaps; staged sewing was discarded",
        ));
    }

    if run.report.status == HealingStatus::RolledBack {
        let refusal = run.report.refusal_reasons.first().cloned();
        let skipped_entities = skipped_details
            .iter()
            .map(|skip| skip.entity)
            .collect::<BTreeSet<_>>();
        for (operation, actual) in operations.iter().zip(&measured) {
            let Some(entity) = operation.target(&run.model) else {
                continue;
            };
            if skipped_entities.contains(&entity) {
                continue;
            }
            skipped_details.push(HealingStageSkip {
                entity,
                code: refusal
                    .as_ref()
                    .map(|reason| reason.code.clone())
                    .unwrap_or_else(|| SEW_INCOMPLETE.into()),
                message: "Authorized tolerant sewing rolled back without publication".into(),
                measured_mm: Some(*actual),
            });
        }
    } else if operations.is_empty() {
        for defect in initial
            .defects
            .iter()
            .filter(|defect| defect.kind == DefectKind::GapEdges)
        {
            if let Some(entity) = defect.entities.first() {
                skipped_details.push(HealingStageSkip {
                    entity: *entity,
                    code: SEW_UNSUPPORTED.into(),
                    message: "Gap has no unique authorized endpoint snap".into(),
                    measured_mm: defect.measured,
                });
            }
        }
    }

    run.report.stages.push(HealingStageReport {
        stage: "tolerantSewing".into(),
        fixed: usize::from(run.report.status == HealingStatus::Complete) * operations.len(),
        skipped: skipped_details.len(),
        remaining,
        skipped_details,
    });
    append_skip_reasons(&mut run.report);
    run.report.validate()?;
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::healing_report::{HEALING_REPORT_VERSION, HealingOperationStatus};

    fn gap_model(multiplier: f64) -> Model {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let vertex = model.edges[0].vertices[0];
        model.vertices[vertex].point[0] += model.tolerance_mm * multiplier;
        model
    }

    fn budget(model: &Model, user: f64, cumulative: f64) -> SewingBudget {
        SewingBudget {
            defects: DefectBudget::default(),
            user_cap_mm: model.tolerance_mm * user,
            cumulative_cap_mm: model.tolerance_mm * cumulative,
            max_candidates: 64,
        }
    }

    fn stage(run: &HealingRun) -> &HealingStageReport {
        run.report
            .stages
            .iter()
            .find(|stage| stage.stage == "tolerantSewing")
            .unwrap()
    }

    #[test]
    fn tolerant_success_records_displacement_ids_and_versioned_stage() {
        let model = gap_model(2.);
        let vertex = model.edges[0].vertices[0];
        let writes = incident_edges(&model, vertex).len() + 1;
        let run = sew_tolerant(&model, budget(&model, 3., 3. * writes as f64)).unwrap();
        assert_eq!(run.report.status, HealingStatus::Complete, "{:#?}", run.report);
        assert_eq!(stage(&run).fixed, 1);
        assert_eq!(stage(&run).remaining, 0);
        assert!(run.report.cumulative_displacement_mm > 0.);
        assert!(run.report.cumulative_displacement_mm <= run.report.displacement_budget_mm);
        assert_eq!(run.report.displacement[0].actual_mm, model.tolerance_mm * 2.);
        run.model.validate().unwrap();
        let expected = incident_edges(&run.model, vertex)
            .into_iter()
            .map(|edge| run.model.1.edges[edge])
            .collect::<BTreeSet<_>>();
        assert_eq!(run.model.1.sewed_edges, expected);
        assert_eq!(run.report.sewn_edges, expected.into_iter().collect::<Vec<_>>());
        assert_eq!(HEALING_REPORT_VERSION, 3);
        let json = run.report.to_json().unwrap();
        assert!(json.contains("\"stage\":\"tolerantSewing\""));
        assert_eq!(crate::healing_report::HealingReport::from_json(&json).unwrap(), run.report);
    }

    #[test]
    fn tolerant_replay_and_model_serialization_preserve_sewed_identities() {
        let model = gap_model(2.);
        let run = sew_tolerant(&model, budget(&model, 3., 100.)).unwrap();
        assert_eq!(run.report.status, HealingStatus::Complete);
        assert!(!run.model.1.sewed_edges.is_empty());
        let replay = sew_tolerant(&run.model, budget(&model, 3., 100.)).unwrap();
        assert_eq!(stage(&replay).fixed, 0);
        assert_eq!(replay.model, run.model);
        assert_eq!(model_hash(&replay.model), model_hash(&run.model));
        assert_eq!(replay.report.input_hash, replay.report.output_hash);

        let encoded = value_codec::to_string(&value_codec::to_value(&run.model).unwrap()).unwrap();
        assert!(encoded.contains("\"sewedEdges\""));
        let value = value_codec::from_str_strict(&encoded).unwrap();
        let decoded: Model = value_codec::from_value(value).unwrap();
        assert_eq!(decoded, run.model);
        assert_eq!(decoded.1.sewed_edges, run.model.1.sewed_edges);
    }

    #[test]
    fn user_cap_skip_preserves_measured_gap_and_input_model() {
        let model = gap_model(2.);
        let run = sew_tolerant(&model, budget(&model, 1., 100.)).unwrap();
        assert_eq!(run.model, model);
        assert_eq!(run.report.status, HealingStatus::RolledBack);
        assert_eq!(run.report.cumulative_displacement_mm, 0.);
        assert!(run.report.displacement.is_empty());
        assert_eq!(stage(&run).skipped, 1);
        assert_eq!(stage(&run).skipped_details[0].code, SEW_USER_CAP_EXCEEDED);
        assert_eq!(stage(&run).skipped_details[0].measured_mm, Some(model.tolerance_mm * 2.));
        assert!(run.model.1.sewed_edges.is_empty());
        run.report.validate().unwrap();
    }

    #[test]
    fn cumulative_cap_skip_is_typed_and_atomic() {
        let model = gap_model(2.);
        let run = sew_tolerant(&model, budget(&model, 3., 1.)).unwrap();
        assert_eq!(run.model, model);
        assert_eq!(run.report.status, HealingStatus::RolledBack);
        assert_eq!(run.report.input_hash, run.report.output_hash);
        assert_eq!(run.report.cumulative_displacement_mm, 0.);
        assert_eq!(stage(&run).fixed, 0);
        assert_eq!(stage(&run).skipped, 1);
        assert_eq!(stage(&run).skipped_details[0].code, SEW_CUMULATIVE_CAP_EXCEEDED);
        assert!(run.model.1.sewed_edges.is_empty());
        run.report.validate().unwrap();
    }

    #[test]
    fn mixed_accepted_and_over_cap_gaps_roll_back_without_partial_markers() {
        let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let accepted = 0;
        let rejected = (1..model.vertices.len())
            .find(|&candidate| {
                model.edges.iter().all(|edge| {
                    !(edge.vertices.contains(&accepted) && edge.vertices.contains(&candidate))
                })
            })
            .unwrap();
        model.vertices[accepted].point[0] += model.tolerance_mm * 2.;
        model.vertices[rejected].point[0] += model.tolerance_mm * 8.;
        let input_hash = model_hash(&model);
        let run = sew_tolerant(&model, budget(&model, 3., 1000.)).unwrap();
        assert_eq!(run.model, model);
        assert_eq!(run.report.status, HealingStatus::RolledBack);
        assert_eq!(run.report.input_hash, input_hash);
        assert_eq!(run.report.output_hash, input_hash);
        assert!(run.report.displacement.is_empty());
        assert!(run.model.1.sewed_edges.is_empty());
        assert!(run.report.sewn_edges.is_empty());
        assert_eq!(stage(&run).fixed, 0);
        assert!(stage(&run).remaining >= 2);
        assert!(
            stage(&run).skipped_details.iter().any(|skip| {
                skip.code == SEW_USER_CAP_EXCEEDED
                    && skip.measured_mm.is_some_and(|measured| {
                        (measured - model.tolerance_mm * 8.).abs() <= model.tolerance_mm * 1e-9
                    })
            }),
            "{:#?}",
            stage(&run)
        );
        assert!(run.report.operations.iter().any(|operation| {
            matches!(operation.status, HealingOperationStatus::Refused | HealingOperationStatus::RolledBack)
        }));
        run.report.validate().unwrap();
    }
}
