use geometry_bridge::{dispatch, execute};
use value_codec::{Value, json};

#[allow(dead_code)]
#[path = "../../nurbs-core/tests/common/periodic_ss_wire.rs"]
mod fixture;

fn request() -> Value {
    let a = fixture::section_at_height(1.);
    let b = fixture::section_at_height(1.5);
    json!({
        "op": "brep_qualify_periodic_band",
        "side": a.source().first(),
        "planes": [b.source().second(), a.source().second()],
        "toleranceMm": 1e-7,
    })
}

#[test]
fn periodic_band_qualifies_from_json_and_returns_original_sources() {
    let request = request();
    let response: Value = value_codec::from_str(&execute(&request.to_string())).unwrap();
    assert_eq!(response["ok"], true, "{response}");
    let value = &response["value"];
    assert_eq!(value["source"], request);
    assert_eq!(value["bodies"], 1);
    assert_eq!(value["faces"], 3);
    assert_eq!(value["edges"], 8);
    assert_eq!(value["boundaryEdges"], 0);
    let lo = value["volumeBoundsMm3"][0].as_f64().unwrap();
    let hi = value["volumeBoundsMm3"][1].as_f64().unwrap();
    let expected = std::f64::consts::PI / 2.;
    assert!(lo > 0. && lo <= expected && expected <= hi && hi - lo < 0.002);
}

#[test]
fn periodic_band_refuses_caller_certificates_and_duplicate_sections() {
    let mut input = request();
    input["qualified"] = json!(true);
    assert!(dispatch(input).is_err());
    let mut input = request();
    input["planes"][1] = input["planes"][0].clone();
    assert!(dispatch(input).is_err());
}

#[path = "../../brep-core/tests/common/full_periodic_cylinder.rs"]
mod cylinder;

#[test]
fn stock_intersection_routes_both_operands_through_the_native_dispatch() {
    let cylinder = cylinder::source();
    let stock = brep_core::cuboid([-2., -2., 0.5], [2., 2., 1.5]).unwrap();
    let request = json!({
        "op": "brep_intersect_periodic_stock_band",
        "side": cylinder,
        "stock": stock,
        "toleranceUv": 1e-8,
    });
    let response: Value = value_codec::from_str(&execute(&request.to_string())).unwrap();
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["source"], request);
    assert_eq!(report["bodies"], 1);
    assert_eq!(report["faces"], 3);
    assert_eq!(report["boundaryEdges"], 0);
    let [lo, hi] = [0, 1].map(|k| report["volumeBoundsMm3"][k].as_f64().unwrap());
    assert!(lo > 0. && lo <= std::f64::consts::PI && std::f64::consts::PI <= hi);
    let mut lateral = request;
    lateral["stock"] = json!(brep_core::cuboid([0., -2., 0.5], [2., 2., 1.5]).unwrap());
    assert!(dispatch(lateral).is_err());
}
