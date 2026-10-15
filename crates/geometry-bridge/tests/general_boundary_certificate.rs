use geometry_bridge::dispatch;
use value_codec::{Value, from_str, json};

fn fixture() -> Value {
    from_str(include_str!("fixtures/general-frame-cap-certificate-v1.json")).unwrap()
}

#[test]
fn original_frame_and_filled_caps_certify_through_the_public_native_protocol() {
    let data = fixture();
    let owned = dispatch(data["sweep"].clone()).unwrap();
    assert_eq!(owned["certified"], true, "{owned:?}");
    assert_eq!(owned["continuousBound"], true);
    assert_eq!(owned["solidGeometryCertified"], true);
    assert_eq!(owned["regularity"]["proven"], true);
    assert_eq!(owned["scope"], "original-sweep-law-and-authored-filled-caps");
    assert!(owned["boundaryErrorUpper"].as_f64().unwrap() < 0.5);
    assert_eq!(owned["filledCapErrorUpper"].as_array().unwrap().len(), 2);
    assert_eq!(owned["idealBodyEmbeddingCertified"], false);
    assert_eq!(owned["smoothnessCertified"], false);
    assert_eq!(owned["seamContinuity"], "C0");
    assert!(!owned["model"].is_null());

    let common = dispatch(data["common"].clone()).unwrap();
    assert_eq!(common["certified"], true, "{common:?}");
    assert_eq!(common["referenceVolume"]["proven"], true);
    assert_eq!(common["retainedVolume"]["proven"], true);
    assert_eq!(common["sourceMotionLawCertified"], false);
    assert_eq!(common["smoothnessCertified"], false);
}

#[test]
fn caller_flags_cannot_replace_any_native_certificate_stage() {
    let data = fixture();
    for (key, budget) in [
        ("sourceCells", 0),
        ("sourceProducts", 0),
        ("sourceExactWork", 0),
        ("regularityCells", 0),
    ] {
        let mut request = data["sweep"].clone();
        request["limits"][key] = json!(budget);
        request["certified"] = json!(true);
        request["continuousBound"] = json!(true);
        request["model"] = data["common"]["retained"].clone();
        let refused = dispatch(request).unwrap();
        assert_eq!(refused["certified"], false, "{key}: {refused:?}");
        assert!(refused["model"].is_null(), "{key}");
    }
    let mut request = data["sweep"].clone();
    request["limits"]["volume"]["maxPairs"] = json!(1);
    let refused = dispatch(request).unwrap();
    assert_eq!(refused["certified"], false);
    assert_eq!(refused["solidGeometryCertified"], false);
    assert!(refused["model"].is_null());

    for stage in ["referenceVolume", "retainedVolume"] {
        let mut request = data["common"].clone();
        request["limits"][stage]["maxPairs"] = json!(1);
        request["certified"] = json!(true);
        let refused = dispatch(request).unwrap();
        assert_eq!(refused["continuousBound"], true);
        assert_eq!(refused["certified"], false, "{stage}: {refused:?}");
        assert_eq!(refused[stage]["proven"], false);
    }
}

#[test]
fn malformed_source_is_rejected_before_a_zero_budget_refusal() {
    let data = fixture();
    let mut bad = data["sweep"].clone();
    bad["limits"]["sourceCells"] = json!(0);
    bad["sourceCaps"][0]["surface"]["weights"][0][0] = json!(0.);
    assert!(dispatch(bad).is_err());
    for field in ["frame_axis", "frame_normal"] {
        let mut bad = data["sweep"].clone();
        bad["source"][field] = Value::Null;
        assert!(dispatch(bad).is_err(), "{field}");
    }
    let mut incomplete = data["sweep"].clone();
    incomplete["sourceCaps"] = json!([]);
    assert!(dispatch(incomplete).is_err());
}

#[test]
fn certified_nonplanar_capped_body_survives_direct_step_topology_exchange() {
    let result = dispatch(fixture()["sweep"].clone()).unwrap();
    let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
    let (text, certificate, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(certificate.complete);
    let (restored, certificate, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(certificate.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(restored.bodies.len(), model.bodies.len());
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        let reverse = model.shells[0].faces.iter().find(|f| f.face == i).unwrap().reversed;
        let (u, v) = (&a.surface.knots_u, &a.surface.knots_v);
        let [ua, ub] = [u[a.surface.degree_u], u[a.surface.control_points.len()]];
        let [va, vb] = [v[a.surface.degree_v], v[a.surface.control_points[0].len()]];
        for (uf, vf) in [(0., 0.), (0.13, 0.27), (0.61, 0.83), (1., 1.)] {
            let (u, v) = (ua + (ub - ua) * uf, va + (vb - va) * vf);
            let p = a.surface.evaluate(if reverse { ua + ub - u } else { u }, v).unwrap().point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(p, q)| (p - q).abs() < 1e-10), "face {i}");
        }
    }
    if let Ok(path) = std::env::var("OSV_GENERAL_CERTIFICATE_STEP") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn ideal_source_embedding_is_native_owned_and_budgeted_separately_from_retained_solid() {
    let mut request=fixture()["sweep"].clone();
    request["idealBody"]=json!({"projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],"maxCells":100000,"maxExactWork":1000000});
    let full=dispatch(request.clone()).unwrap();
    assert_eq!(full["certified"],true,"{full:?}");assert_eq!(full["idealBodyEmbeddingCertified"],true);
    assert_eq!(full["idealBody"]["volumeRegularityCertified"],true);
    assert_eq!(full["idealBody"]["parameterTopologyPreserved"],true);
    assert_eq!(full["smoothnessCertified"],false);
    request["idealBody"]["maxCells"]=json!(full["idealBody"]["cells"].as_u64().unwrap()-1);
    request["idealBodyEmbeddingCertified"]=json!(true);
    let short=dispatch(request).unwrap();assert_eq!(short["certified"],false);
    assert_eq!(short["idealBodyEmbeddingCertified"],false);assert!(short["model"].is_null());
    assert_eq!(short["solidGeometryCertified"],true);
}
#[test]
fn complete_smoothness_protocol_cannot_omit_a_cap_or_promote_sharp_joins() {
    let owned=dispatch(fixture()["sweep"].clone()).unwrap();
    for order in [1,2] {
        let r=dispatch(json!({"op":"brep_complete_boundary_smoothness_audit","model":owned["model"],
            "order":order,"maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
            "capFaces":[],"seams":[],"fullBoundaryG2Certified":true})).unwrap();
        assert_eq!(r["fullBoundaryG1Certified"],false);assert_eq!(r["fullBoundaryG2Certified"],false);
        assert_eq!(r["idealSourceSmoothnessCertified"],false);
        assert!(!r["edgeIds"].as_array().unwrap().is_empty());
    }
}
