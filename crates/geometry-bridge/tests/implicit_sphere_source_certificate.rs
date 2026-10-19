use geometry_bridge::dispatch;
use value_codec::json;
#[path = "support/implicit_sphere.rs"]
mod support;
#[test]
fn original_pole_g2_reaches_rust_json_construct_and_unchanged_model_audit() {
    for (radius, center) in [
        (2., [0.; 3]),
        (3_f64.next_up(), [0.; 3]),
        (3., [0.; 3]),
        (2., [8., -4., 6.]),
    ] {
        let mut input = support::request(radius, center);
        input["model"] = json!({"fake":"ignored construct model"});
        input["certified"] = json!(true);
        let built = dispatch(input.clone()).unwrap();
        support::full(&built, "construct");
        assert_eq!(built["source"]["radius"], radius);
        assert_eq!(built["source"]["center"], json!(center));
        if radius == 2. {
            assert_eq!(built["poleEdges"].as_array().unwrap().len(), 2);
            assert_eq!(built["regularity"]["allParameterChartsRegular"], false);
        }
        input["op"] = json!("brep_implicit_sphere_source_audit");
        input["model"] = built["model"].clone();
        let audited = dispatch(input.clone()).unwrap();
        support::full(&audited, "audit");
        assert_eq!(audited["model"], input["model"]);
    }
}
#[test]
fn wrong_original_radius_inexact_translation_and_short_proofs_never_produce_solid() {
    let input = support::request(2., [0.; 3]);
    let built = dispatch(input.clone()).unwrap();
    support::full(&built, "before mutation");
    let mut wrong = input.clone();
    wrong["op"] = json!("brep_implicit_sphere_source_audit");
    wrong["model"] = built["model"].clone();
    wrong["source"]["radius"] = json!(2_f64.next_up());
    wrong["certified"] = json!(true);
    wrong["fullBoundaryG2Certified"] = json!(true);
    let r = dispatch(wrong).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    assert_eq!(r["idealSourceBoundaryG2Certified"], false);
    let r = dispatch(support::request(3_f64.next_up(), [8., -4., 6.])).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    assert_eq!(r["reason"], "original-sphere-exact-translation-unproved");
    for key in ["implicitExactWork", "implicitCells", "regularityCells"] {
        let mut short = input.clone();
        short["limits"][key] = json!(0);
        short["certified"] = json!(true);
        let r = dispatch(short).unwrap();
        assert_eq!(r["certified"], false, "{key}");
        assert!(r["model"].is_null());
    }
    let mut invalid = input;
    invalid["source"]["radius"] = json!(-1.);
    invalid["limits"]["implicitExactWork"] = json!(0);
    assert!(dispatch(invalid).is_err());
}
