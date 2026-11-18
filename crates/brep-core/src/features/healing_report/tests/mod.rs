use super::*;

fn fixture() -> (Model, Vec<AutoRepairOperation>) {
    let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let context = model.tolerance_context().unwrap();
    let cell = context.spatial_bounds().absolute_mm;
    let vertex = model.edges[0].vertices[0];
    let writes = 1 + model
        .edges
        .iter()
        .filter(|edge| edge.vertices.contains(&vertex))
        .count();
    let mut to = model.vertices[vertex].point;
    to[0] += cell / writes as f64 * 0.25;
    let edge = model
        .edges
        .iter()
        .position(|edge| !edge.vertices.contains(&vertex))
        .unwrap();
    let mut replacement = model.edges[edge].curve.clone();
    replacement.control_points[1][2] += cell * 0.25;
    (
        model,
        vec![
            AutoRepairOperation::SnapVertex { vertex, to },
            AutoRepairOperation::RefitEdge { edge, replacement },
        ],
    )
}

#[test]
fn auto_repair_records_real_snap_refit_and_replays_identically() {
    let (model, operations) = fixture();
    let run = auto_repair(&model, &operations, None);
    assert_eq!(run.report.status, HealingStatus::Complete);
    assert_ne!(run.model, model);
    assert_eq!(run.report.output_hash, model_hash(&run.model));
    assert_eq!(run.report.operations.len(), 2);
    assert_eq!(run.report.certificates.len(), 2);
    assert_eq!(run.report.displacement.len(), 2);
    assert!(run.report.gaps.before_mm > 0.);
    assert_eq!(run.report.gaps.after_mm, 0.);
    assert!(!run.report.sewn_edges.is_empty());
    assert!(run.report.conversions.is_empty());
    assert!(run.report.unresolved_defects.is_empty());
    assert!(run.report.cumulative_displacement_mm <= run.report.displacement_budget_mm);
    for (sequence, entry) in run.report.displacement.iter().enumerate() {
        assert_eq!(entry.operation, sequence);
        assert!(!entry.entity_ids.is_empty());
        assert!(entry.before_mm > 0.);
        assert_eq!(entry.after_mm, 0.);
    }
    for certificate in &run.report.certificates {
        assert!(certificate.sew.starts_with("sew:"));
        assert!(certificate.audit.starts_with("audit:"));
        assert!(certificate.sew_complete && certificate.audit_ok);
    }
    run.report.validate_ci_gate(0).unwrap();

    let independent = auto_repair(&model, &operations, None);
    assert_eq!(independent.model, run.model);
    assert_eq!(
        independent.report.to_json().unwrap(),
        run.report.to_json().unwrap()
    );
    assert_eq!(
        run.report.to_json().unwrap(),
        run.report.to_json().unwrap(),
        "serialization must be deterministic"
    );

    let replay = auto_repair(&run.model, &operations, Some(&run.report));
    assert_eq!(replay, run, "replay must not duplicate or alter the trail");
}

#[test]
fn failed_sequence_rolls_back_applied_status_and_displacement() {
    let (model, operations) = fixture();
    let requested = vec![
        operations[0].clone(),
        AutoRepairOperation::SnapVertex {
            vertex: usize::MAX,
            to: [0.; 3],
        },
        operations[1].clone(),
    ];
    let run = auto_repair(&model, &requested, None);
    assert_eq!(run.model, model);
    assert_eq!(run.report.status, HealingStatus::RolledBack);
    assert_eq!(run.report.input_hash, run.report.output_hash);
    assert_eq!(run.report.input_hash, model_hash(&model));
    assert!(run.report.displacement.is_empty());
    assert!(run.report.sewn_edges.is_empty());
    assert_eq!(run.report.cumulative_displacement_mm, 0.);
    assert_eq!(
        run.report.gaps,
        HealingGapSummary {
            before_mm: 0.,
            after_mm: 0.
        }
    );
    assert_eq!(run.report.certificates.len(), 1);
    assert_eq!(
        run.report
            .operations
            .iter()
            .map(|operation| operation.status)
            .collect::<Vec<_>>(),
        vec![
            HealingOperationStatus::RolledBack,
            HealingOperationStatus::Refused,
            HealingOperationStatus::Skipped,
        ]
    );
    assert_eq!(run.report.unresolved_defects.len(), 1);
    assert_eq!(run.report.refusal_reasons, run.report.unresolved_defects);
    assert_eq!(run.report.refusal_reasons[0].operation, Some(1));
    assert_eq!(
        run.report.refusal_reasons[0].code,
        "BREP_HEAL_RECIPE_INVALID"
    );
    assert_eq!(
        run.report.validate_ci_gate(0).unwrap_err().code,
        HEALING_REPORT_CI_GATE
    );
    run.report.validate_ci_gate(1).unwrap();
    let json = run.report.to_json().unwrap();
    assert_eq!(HealingReport::from_json(&json).unwrap(), run.report);
}

#[test]
fn versioned_json_round_trips_and_rejects_unknown_fields() {
    let (model, operations) = fixture();
    let report = auto_repair(&model, &operations, None).report;
    let json = report.to_json().unwrap();
    assert!(json.contains("\"schema\":\"brep-healing-report\""));
    assert!(json.contains(&format!("\"version\":{HEALING_REPORT_VERSION}")));
    let decoded = HealingReport::from_json(&json).unwrap();
    assert_eq!(decoded, report);
    assert_eq!(decoded.to_json().unwrap(), json);

    let mut legacy: Value = value_codec::from_str_strict(&json).unwrap();
    legacy["version"] = json!(1);
    legacy.as_object_mut().unwrap().remove("toleranceUnification");
    legacy.as_object_mut().unwrap().remove("stages");
    let migrated = HealingReport::from_json(&value_codec::to_string(&legacy).unwrap()).unwrap();
    assert_eq!(migrated.tolerance_unification, None);
    assert_eq!(migrated.to_json().unwrap(), json);

    let mut root: Value = value_codec::from_str_strict(&json).unwrap();
    root.as_object_mut()
        .unwrap()
        .insert("unknown".into(), Value::Bool(true));
    assert!(HealingReport::from_json(&value_codec::to_string(&root).unwrap()).is_err());

    let mut nested: Value = value_codec::from_str_strict(&json).unwrap();
    nested["operations"][0]
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), Value::Bool(true));
    assert!(HealingReport::from_json(&value_codec::to_string(&nested).unwrap()).is_err());

    let mut curve: Value = value_codec::from_str_strict(&json).unwrap();
    curve["operations"][1]["operation"]["replacement"]
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), Value::Bool(true));
    assert!(HealingReport::from_json(&value_codec::to_string(&curve).unwrap()).is_err());

    let mut version: Value = value_codec::from_str_strict(&json).unwrap();
    version["version"] = json!(HEALING_REPORT_VERSION + 1);
    assert!(HealingReport::from_json(&value_codec::to_string(&version).unwrap()).is_err());
}
