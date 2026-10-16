use geometry_bridge::dispatch;
use value_codec::json;
#[path = "support/fibered_body.rs"]
mod support;
#[test]
fn original_volume_and_both_physical_g2_caps_reach_json_and_unchanged_audit() {
    let mut input = support::request();
    input["certified"] = json!(false);
    input["model"] = json!({"ignored":"caller construct model"});
    let built = dispatch(input.clone()).unwrap();
    support::full(&built);
    input["op"] = json!("brep_fibered_source_audit");
    input["model"] = built["model"].clone();
    let audited = dispatch(input.clone()).unwrap();
    support::full(&audited);
    assert_eq!(audited["model"], input["model"]);
}
#[test]
fn wrong_original_source_and_exhausted_stages_never_return_model_or_accept_caller_flags() {
    let input = support::request();
    let built = dispatch(input.clone()).unwrap();
    support::full(&built);
    let mut wrong = input.clone();
    wrong["op"] = json!("brep_fibered_source_audit");
    wrong["model"] = built["model"].clone();
    wrong["model"]["faces"][0]["surface"]["controlPoints"][1][1][0] = json!(1.25_f64.next_up());
    wrong["certified"] = json!(true);
    wrong["fullBoundaryG2Certified"] = json!(true);
    let r = dispatch(wrong).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    for key in [
        "sourceTraceWork",
        "bindingRows",
        "agreementWork",
        "regularityCells",
        "normalCells",
        "smoothnessWork",
    ] {
        let mut short = input.clone();
        short["limits"][key] = json!(0);
        short["certified"] = json!(true);
        short["fullBoundaryG2Certified"] = json!(true);
        let r = dispatch(short).unwrap();
        for flag in [
            "certified",
            "topologyPreserved",
            "absenceOfSelfIntersections",
            "fullBoundaryG1Certified",
            "fullBoundaryG2Certified",
        ] {
            assert_eq!(r[flag], false, "{key}/{flag}");
        }
        assert!(r["model"].is_null(), "{key}");
    }
    let mut invalid = input;
    invalid["limits"]["bindingRows"] = json!(0);
    invalid["source"]["motion"][0]["weights"][5] = json!(-1.);
    assert!(dispatch(invalid).is_err());
}

#[test]
fn original_c1_motion_knot_and_inner_wall_cusp_keep_their_real_continuity() {
    let mut g1 = support::three_piece_request();
    g1["source"]["motion"][1]["controls"][2][0][0] = json!(1.625);
    let r = dispatch(g1.clone()).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    assert_eq!(r["reason"], "original-multipiece-motion-jets-unproved");
    g1["limits"]["order"] = json!(1);
    let r = dispatch(g1.clone()).unwrap();
    assert_eq!(r["certified"], true);
    assert_eq!(r["fullBoundaryG1Certified"], true);
    assert_eq!(r["fullBoundaryG2Certified"], false);
    assert_eq!(r["seamContinuity"], "G1");
    g1["op"] = json!("brep_fibered_source_audit");
    g1["model"] = r["model"].clone();
    let a = dispatch(g1).unwrap();
    assert_eq!(a["certified"], true);
    assert_eq!(a["model"], r["model"]);
    let r = dispatch(support::hole_request(0)).unwrap();
    assert_eq!(r["certified"], true);
    assert_eq!(r["topologyPreserved"], true);
    assert_eq!(r["seamContinuity"], "C0");
    assert_eq!(r["fullBoundaryG1Certified"], false);
    for order in [1, 2] {
        let mut v = support::hole_request(order);
        v["fullBoundaryG2Certified"] = json!(true);
        let r = dispatch(v).unwrap();
        assert_eq!(r["certified"], false);
        assert!(r["model"].is_null());
        assert_eq!(
            r["reason"],
            "complete-cap-wall-oriented-regularity-unproved"
        );
    }
}
