use geometry_bridge::dispatch;
use value_codec::json;
#[path = "support/rational_closed_material.rs"]
mod support;

#[test]
fn original_rational_frame_filled_caps_reach_native_json_and_fresh_model_audit() {
    for (nonplanar, affine) in [(false, false), (true, false), (true, true)] {
        let mut input = support::request(nonplanar, affine);
        input["model"] = json!({"caller":"ignored invalid construct model"});
        input["certified"] = json!(true);
        input["fullBoundaryG2Certified"] = json!(true);
        let built = dispatch(input.clone()).unwrap();
        support::full(&built, "construct");
        input["op"] = json!("brep_rational_closed_material_audit");
        input["model"] = built["model"].clone();
        input["limits"]["constructionRows"] = json!(0);
        let audited = dispatch(input.clone()).unwrap();
        support::full(&audited, "audit");
        assert_eq!(audited["sourceWork"]["constructionRows"], 0);
        assert_eq!(input["model"], audited["model"]);
        if affine {
            assert_eq!(audited["proofCoordinates"]["columnFactor"]["scale"], 0.125);
        }
    }
}
#[test]
fn forged_success_and_short_native_owner_stages_never_return_a_model() {
    let base = support::request(true, false);
    for (owner, key) in [
        ("", "sourceCells"),
        ("", "sourceExactWork"),
        ("", "constructionRows"),
        ("", "maxBindingWork"),
        ("", "maxOrientationCells"),
        ("", "maxCapCells"),
        ("volume", "maxCells"),
        ("volume", "maxExactWork"),
        ("smoothness", "maxExactWork"),
        ("smoothness", "maxAgreementWork"),
        ("smoothness", "maxRegularityCells"),
    ] {
        let mut input = base.clone();
        input["certified"] = json!(true);
        input["fullBoundaryG2Certified"] = json!(true);
        input["solidGeometryCertified"] = json!(true);
        if owner.is_empty() {
            input["limits"][key] = json!(0);
        } else {
            input["limits"][owner][key] = json!(0);
        }
        let r = dispatch(input).unwrap();
        assert_eq!(r["certified"], false, "short {owner}/{key}");
        assert_eq!(r["fullBoundaryG2Certified"], false);
        assert!(r["model"].is_null());
    }
    let built = dispatch(base.clone()).unwrap();
    support::full(&built, "before mutation");
    let mut input = base;
    input["op"] = json!("brep_rational_closed_material_audit");
    input["model"] = built["model"].clone();
    let old = input["model"]["faces"][0]["surface"]["weights"][1][1]
        .as_f64()
        .unwrap();
    input["model"]["faces"][0]["surface"]["weights"][1][1] = json!(old.next_up());
    input["certified"] = json!(true);
    let r = dispatch(input).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
}
#[test]
fn explicit_c0_keeps_global_geometry_without_g1_g2_claims() {
    let mut input = support::request(true, false);
    input["source"]["profiles"][0][2]["controlPoints"][1][1] = json!(2_f64.next_up());
    input["material"]["trims"][0][2]["controlPoints"][1][1] = json!(2_f64.next_up());
    let strict = dispatch(input.clone()).unwrap();
    assert_eq!(strict["certified"], false);
    assert!(strict["model"].is_null());
    input["limits"]["smoothness"]["order"] = json!(0);
    let c0 = dispatch(input).unwrap();
    assert_eq!(c0["certified"], true);
    assert_eq!(c0["seamContinuity"], "C0");
    assert_eq!(c0["topologyPreserved"], true);
    assert_eq!(c0["fullBoundaryG1Certified"], false);
    assert_eq!(c0["fullBoundaryG2Certified"], false);
    assert_eq!(c0["c0"]["allFacesRegular"], true);
    assert_eq!(c0["c0"]["edgeCorrespondenceCertified"], true);
}

#[test]
fn native_json_distinguishes_g1_curvature_jump_and_validates_source_before_empty_budget() {
    let mut input = support::request(true, false);
    input["source"]["profiles"][0][2]["controlPoints"][2][1] = json!(2_f64.next_up());
    input["material"]["trims"][0][2]["controlPoints"][2][1] = json!(2_f64.next_up());
    assert_eq!(dispatch(input.clone()).unwrap()["certified"], false);
    input["limits"]["smoothness"]["order"] = json!(1);
    let g1 = dispatch(input).unwrap();
    assert_eq!(g1["certified"], true);
    assert_eq!(g1["seamContinuity"], "G1");
    assert_eq!(g1["fullBoundaryG1Certified"], true);
    assert_eq!(g1["idealSourceBoundaryG1Certified"], true);
    assert_eq!(g1["fullBoundaryG2Certified"], false);
    assert_eq!(g1["idealSourceBoundaryG2Certified"], false);
    let mut invalid = support::request(true, false);
    invalid["source"]["motion"][3]["weights"][0] = json!(0.);
    invalid["limits"]["maxCapCells"] = json!(0);
    assert!(dispatch(invalid).is_err());
}
