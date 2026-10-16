use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn fixture() -> Value {
    value_codec::from_str(include_str!("fixtures/smooth-closed-material-v1.json")).unwrap()
}
fn assert_full(r: &Value) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "continuousBound",
        "withinBudget",
        "topologyPreserved",
        "shellRolesCertified",
        "smoothnessCertified",
        "fullBoundaryG1Certified",
        "fullBoundaryG2Certified",
    ] {
        assert_eq!(r[key], true, "{key}: {r:?}");
    }
    assert_eq!(r["seamContinuity"], "G2");
    assert_eq!(r["physicalCapCount"], 0);
    assert!(r["filledCapErrorUpper"].is_null());
    assert_eq!(r["idealSourceSmoothnessCertified"], false);
    assert_eq!(r["original"]["certified"], true);
    assert_eq!(r["original"]["material"]["certified"], true);
    for p in [&r["original"]["volume"], &r["retainedVolume"]] {
        for key in [
            "embeddingCertified",
            "volumeRegularityCertified",
            "parameterTopologyPreserved",
        ] {
            assert_eq!(p[key], true, "{key}: {p:?}");
        }
        assert!(p["cells"].as_u64().unwrap() > 0);
    }
    assert_eq!(r["retainedBinding"]["certified"], true);
    assert_eq!(r["retainedBinding"]["inspectedFaces"], 128);
    assert_eq!(r["smoothness"]["allFacesRegular"], true);
    assert_eq!(r["smoothness"]["edgeCorrespondenceCertified"], true);
    assert_eq!(r["smoothness"]["vertexLinksComplete"], true);
    assert_eq!(r["smoothness"]["edgeIds"].as_array().unwrap().len(), 256);
    let seams = r["smoothness"]["seams"].as_array().unwrap();
    assert_eq!(seams.len(), 256);
    for s in seams {
        assert_eq!(s["certified"], true);
        assert_eq!(s["exactIdentity"], true);
        assert_eq!(s["regularityCertified"], true);
    }
    assert!(r["boundaryErrorUpper"].as_f64().unwrap() <= 0.05);
    let model: brep_core::Model = value_codec::from_value(r["model"].clone()).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
}
#[test]
fn original_and_actual_closed_g2_solid_are_owned_by_the_native_json_protocol() {
    let request = fixture();
    let before = request.clone();
    let r = dispatch(request.clone()).unwrap();
    assert_full(&r);
    assert_eq!(request, before);
}
#[test]
fn success_fields_and_caller_models_cannot_bypass_native_stage_refusals() {
    let base = fixture();
    let full = dispatch(base.clone()).unwrap();
    assert_full(&full);
    let mut cases = Vec::new();
    for key in [
        "maxWork",
        "maxBindingWork",
        "maxExactWork",
        "maxAgreementWork",
        "maxRegularityCells",
        "maxOrientationCells",
    ] {
        let mut v = base.clone();
        v["smoothing"][key] = json!(0);
        cases.push(v);
    }
    let mut v = base.clone();
    v["smoothing"]["atlas"]["maxCells"] = json!(0);
    cases.push(v);
    let mut v = base.clone();
    v["idealBody"]["maxCells"] = json!(0);
    cases.push(v);
    for mut v in cases {
        v["model"] = full["model"].clone();
        v["certified"] = json!(true);
        v["solidGeometryCertified"] = json!(true);
        v["fullBoundaryG2Certified"] = json!(true);
        v["topologyPreserved"] = json!(true);
        let r = dispatch(v).unwrap();
        assert_eq!(r["certified"], false);
        assert_eq!(r["solidGeometryCertified"], false);
        assert_eq!(r["fullBoundaryG2Certified"], false);
        assert!(r["model"].is_null());
        assert!(!r["reason"].is_null());
    }
    let mut v = base.clone();
    v["material"]["surface"]["weights"][0][0] = json!(0.);
    v["smoothing"]["maxWork"] = json!(0);
    assert!(dispatch(v).is_err());
}
#[test]
fn supplied_model_audit_recomputes_source_binding_and_global_volume() {
    let base = fixture();
    let full = dispatch(base.clone()).unwrap();
    assert_full(&full);
    let mut audit = base.clone();
    audit["op"] = json!("brep_sweep_smooth_closed_material_audit");
    audit["model"] = full["model"].clone();
    let r = dispatch(audit.clone()).unwrap();
    assert_full(&r);
    let mut changed: brep_core::Model = value_codec::from_value(full["model"].clone()).unwrap();
    changed.faces[0].surface.control_points[1][2][0] += 1e-10;
    changed.validate().unwrap();
    let mut v = audit.clone();
    v["model"] = value_codec::to_value(&changed).unwrap();
    v["certified"] = json!(true);
    v["fullBoundaryG2Certified"] = json!(true);
    let r = dispatch(v).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    assert_eq!(r["reason"], "affine-retained-binding-unproved");
    let mut v = audit;
    v["source"]["scale"]["controlPoints"][0][0] = json!(1. + 1e-12);
    let r = dispatch(v).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    assert_eq!(r["original"]["certified"], false);
}
#[test]
fn native_step_roundtrip_recomputes_g2_global_volume_topology_and_orientation() {
    let request = fixture();
    let r = dispatch(request.clone()).unwrap();
    assert_full(&r);
    let model: brep_core::Model = value_codec::from_value(r["model"].clone()).unwrap();
    let (text, exported, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(exported.complete);
    let (restored, imported, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(imported.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(
        (
            restored.faces.len(),
            restored.edges.len(),
            restored.vertices.len(),
            restored.shells.len(),
            restored.bodies.len()
        ),
        (
            model.faces.len(),
            model.edges.len(),
            model.vertices.len(),
            model.shells.len(),
            model.bodies.len()
        )
    );
    let mut audit = request;
    audit["op"] = json!("brep_sweep_smooth_closed_material_audit");
    audit["model"] = value_codec::to_value(&restored).unwrap();
    let r = dispatch(audit).unwrap();
    assert_full(&r);
}
