//! healing report algorithm: auto_repair_impl.
use super::*;
pub fn auto_repair_impl(
    model: &Model,
    operations: &[AutoRepairOperation],
    prior: Option<&HealingReport>,
    tolerant_caps: Option<(f64, f64)>,
) -> HealingRun {
    let input_hash = model_hash(model);
    if let Some(prior) = prior
        && prior.status == HealingStatus::Complete
        && prior.output_hash == input_hash
        && prior.validate().is_ok()
        && prior.operations.len() == operations.len()
        && prior
            .operations
            .iter()
            .zip(operations)
            .all(|(record, operation)| record.operation == *operation)
    {
        return HealingRun {
            model: model.clone(),
            report: prior.clone(),
        };
    }

    let model_tolerance_mm = model
        .tolerance_context()
        .map(|context| context.spatial_bounds().absolute_mm)
        .unwrap_or(model.tolerance_mm);
    let displacement_budget_mm = tolerant_caps
        .map(|(_, cumulative)| cumulative)
        .unwrap_or(model_tolerance_mm * operations.len() as f64);
    let mut staged = model.clone();
    let mut records = Vec::with_capacity(operations.len());
    let mut displacement = Vec::new();
    let mut certificates = Vec::new();
    let mut sewn_edges = BTreeSet::new();
    let mut cumulative = 0.;

    for (sequence, operation) in operations.iter().enumerate() {
        let target = operation.target(&staged);
        let result = match operation {
            AutoRepairOperation::SnapVertex { vertex, to } => {
                if let Some((per_entity, cumulative_cap)) = tolerant_caps {
                    authorized_heal_endpoint_tolerant(
                        &staged,
                        *vertex,
                        *to,
                        per_entity,
                        cumulative_cap - cumulative,
                    )
                } else {
                    authorized_heal_endpoint(&staged, *vertex, *to)
                }
            }
            AutoRepairOperation::RefitEdge { edge, replacement } => {
                authorized_heal_refit(&staged, *edge, replacement.clone())
            }
        };
        match result {
            Ok(result) => {
                let mut entries = result.displacement.clone();
                for entry in &mut entries {
                    entry.operation = sequence;
                    entry.cumulative_mm += cumulative;
                }
                cumulative += result.cumulative_displacement_mm;
                let entity_ids = entries
                    .iter()
                    .flat_map(|entry| entry.entity_ids.iter().copied())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                if result.sew.complete {
                    sewn_edges.extend(
                        entity_ids
                            .iter()
                            .filter(|id| id.kind() == TopoKind::Edge)
                            .copied(),
                    );
                }
                certificates.push(HealingCertificateRefs::new(
                    sequence,
                    &result.sew,
                    &result.audit,
                ));
                displacement.extend(entries.into_iter().map(HealingDisplacementEntry::from));
                records.push(HealingOperationRecord {
                    sequence,
                    operation: operation.clone(),
                    target,
                    entity_ids,
                    rationale: operation.rationale().into(),
                    model_tolerance_mm,
                    entity_tolerance_mm: model_tolerance_mm,
                    cumulative_tolerance_mm: displacement_budget_mm,
                    displacement_mm: result.cumulative_displacement_mm,
                    cumulative_displacement_mm: cumulative,
                    status: HealingOperationStatus::Applied,
                });
                staged = result.model;
            }
            Err(error) => {
                let refusal = HealingRefusal {
                    operation: Some(sequence),
                    code: error.code.into(),
                    message: error.message,
                };
                records.push(HealingOperationRecord {
                    sequence,
                    operation: operation.clone(),
                    target,
                    entity_ids: Vec::new(),
                    rationale: operation.rationale().into(),
                    model_tolerance_mm,
                    entity_tolerance_mm: model_tolerance_mm,
                    cumulative_tolerance_mm: displacement_budget_mm,
                    displacement_mm: 0.,
                    cumulative_displacement_mm: cumulative,
                    status: HealingOperationStatus::Refused,
                });
                for (skipped_sequence, skipped) in operations.iter().enumerate().skip(sequence + 1)
                {
                    records.push(HealingOperationRecord {
                        sequence: skipped_sequence,
                        operation: skipped.clone(),
                        target: skipped.target(model),
                        entity_ids: Vec::new(),
                        rationale: skipped.rationale().into(),
                        model_tolerance_mm,
                        entity_tolerance_mm: model_tolerance_mm,
                        cumulative_tolerance_mm: displacement_budget_mm,
                        displacement_mm: 0.,
                        cumulative_displacement_mm: cumulative,
                        status: HealingOperationStatus::Skipped,
                    });
                }
                for record in &mut records[..sequence] {
                    record.status = HealingOperationStatus::RolledBack;
                }
                return HealingRun {
                    model: model.clone(),
                    report: HealingReport {
                        output_hash: input_hash.clone(),
                        input_hash,
                        status: HealingStatus::RolledBack,
                        operations: records,
                        gaps: gap_summary(&[]),
                        sewn_edges: Vec::new(),
                        conversions: Vec::new(),
                        displacement: Vec::new(),
                        certificates,
                        displacement_budget_mm,
                        cumulative_displacement_mm: 0.,
                        unresolved_defects: vec![refusal.clone()],
                        refusal_reasons: vec![refusal],
                        tolerance_unification: None,
        stages: Vec::new(),
                    },
                };
            }
        }
    }

    let report = HealingReport {
        input_hash,
        output_hash: model_hash(&staged),
        status: HealingStatus::Complete,
        operations: records,
        gaps: gap_summary(&displacement),
        sewn_edges: sewn_edges.into_iter().collect(),
        conversions: Vec::new(),
        displacement,
        certificates,
        displacement_budget_mm,
        cumulative_displacement_mm: cumulative,
        unresolved_defects: Vec::new(),
        refusal_reasons: Vec::new(),
        tolerance_unification: None,
        stages: Vec::new(),
    };
    HealingRun {
        model: staged,
        report,
    }
}
