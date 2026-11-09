use brep_core::defects::DefectBudget;
use brep_core::healing::tolerant_sew::{SewingBudget, sew_tolerant, sew_tolerant_certified};
use brep_core::{Model, cuboid};

fn sewing_budget(model: &Model) -> SewingBudget {
    SewingBudget {
        defects: DefectBudget::default(),
        user_cap_mm: model.tolerance_mm * 3.,
        cumulative_cap_mm: model.tolerance_mm * 100.,
        max_candidates: 64,
    }
}

fn gap_model() -> Model {
    let mut model = cuboid([0.; 3], [2.; 3]).unwrap();
    let vertex = model.edges[0].vertices[0];
    model.vertices[vertex].point[0] += model.tolerance_mm * 2.;
    model
}

#[test]
fn faceted_fillet_publishes_measured_audited_handoff() {
    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let (result, handoff) = brep_core::operations::fillet_edges_certified(&model, &[0], 1., 8).unwrap();
    assert!(handoff.measured_output_mm > 0.);
    assert!(result.tolerance_mm >= model.tolerance_mm);
    assert!(result.tolerance_mm >= handoff.measured_output_mm);
    assert!(handoff.audit.ok);
}

#[test]
fn analytic_fillet_enforces_the_same_handoff() {
    let model = cuboid([0.; 3], [10.; 3]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|edge| {
            let [a, b] = edge.vertices.map(|v| model.vertices[v].point);
            a[0] == b[0] && a[1] == b[1] && a[2] != b[2]
        })
        .unwrap();
    let (result, _) = brep_core::analytic_features::analytic_fillet(&model, edge, 1.).unwrap();
    assert!(result.tolerance_mm >= model.tolerance_mm);
    result.validate().unwrap();
}

#[test]
fn sewing_publishes_measured_handoff_and_typed_input_refusal() {
    let model = gap_model();
    let certified = sew_tolerant_certified(&model, sewing_budget(&model)).unwrap();
    assert!(certified.tolerance_handoff.measured_output_mm > 0.);
    assert!(certified.run.model.tolerance_mm >= model.tolerance_mm);
    assert!(certified.run.model.tolerance_mm >= certified.tolerance_handoff.measured_output_mm);
    assert!(certified.tolerance_handoff.audit.ok);

    let mut invalid = model.clone();
    invalid.tolerance_mm = 0.;
    let error = sew_tolerant(&invalid, sewing_budget(&model)).unwrap_err();
    assert_eq!(error.code, "BREP_TOLERANCE_CONTRACT_REFUSED");
}
