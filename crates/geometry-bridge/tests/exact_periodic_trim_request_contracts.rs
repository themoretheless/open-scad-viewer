use geometry_bridge::execute;
use nurbs_core::{
    curve::Curve,
    surface::{Surface, extrude},
};
use value_codec::{Value, json};

fn support() -> Surface {
    extrude(
        &Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        },
        [0., 0., 2.],
    )
    .unwrap()
}

fn request() -> Value {
    json!({
        "op": "brep_build_exact_periodic_trim_sheet",
        "surface": support(),
        "contours": [
            [[3.5, 0.125], [0.5, 0.125], [0.5, 0.875], [3.5, 0.875], [3.5, 0.125]],
            [[0.125, 0.25], [0.375, 0.25], [0.375, 0.75], [0.125, 0.75], [0.125, 0.25]]
        ],
        "toleranceMm": 1e-8,
        "maxWork": 1_000_000
    })
}

fn run(request: &Value) -> Value {
    value_codec::from_str(&execute(&request.to_string())).unwrap()
}

#[test]
fn json_round_trip_preserves_open_sheet_and_hole() {
    let response = run(&request());
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert!(report["refusalStage"].is_null());
    assert!(report["workUsed"].as_u64().unwrap() > 0);
    let model: brep_core::Model = value_codec::from_str(&report["model"].to_string()).unwrap();
    model.validate().unwrap();
    assert_eq!(model.faces.len(), 1);
    assert_eq!(model.faces[0].holes, vec![1]);
    assert!(!model.shells[0].closed);
    assert!(model.bodies.is_empty());
}

#[test]
fn refusal_and_invalid_budget_do_not_publish_geometry() {
    let mut request = request();
    request["maxWork"] = json!(1);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    assert!(response["value"]["model"].is_null());
    assert_eq!(response["value"]["refusalStage"], "patch");

    request["maxWork"] = json!(1_000_001);
    let response = run(&request);
    assert_eq!(response["ok"], false, "{response}");
}

#[test]
fn binding_operation_retains_model_and_returns_edge_id_and_segment_intervals() {
    let mut request = request();
    let original = run(&request);
    assert!(original["value"].get("edgeBindings").is_none());
    request["op"] = json!("brep_build_exact_periodic_trim_sheet_with_bindings");
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["model"], original["value"]["model"]);
    let bindings = report["edgeBindings"].as_array().unwrap();
    let ids = report["model"]["topologyIds"]["edges"].as_array().unwrap();
    assert_eq!(bindings.len(), ids.len());
    for (binding, id) in bindings.iter().zip(ids) {
        assert_eq!(&binding["edgeId"], id);
    }
    assert_eq!(bindings[0]["interval"], json!([0., 0.5]));
    assert_eq!(bindings[1]["interval"], json!([0.5, 1.]));
    assert_eq!(bindings[1]["segment"], 0);
    assert!(bindings.iter().any(|binding| binding["contour"] == 1));
}

#[test]
fn exhausted_binding_request_withholds_model_and_bindings() {
    let mut request = request();
    request["op"] = json!("brep_build_exact_periodic_trim_sheet_with_bindings");
    request["maxWork"] = json!(1);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    assert!(response["value"]["model"].is_null());
    assert_eq!(response["value"]["edgeBindings"], json!([]));
    assert_eq!(response["value"]["refusalStage"], "patch");
}

#[test]
fn input_binding_operation_serializes_exact_directed_rationals() {
    let mut request = request();
    let original = run(&request);
    request["op"] = json!("brep_build_exact_periodic_trim_sheet_with_input_bindings");
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["model"], original["value"]["model"]);
    let bindings = report["edgeBindings"].as_array().unwrap();
    let ids = report["model"]["topologyIds"]["edges"].as_array().unwrap();
    assert_eq!(bindings.len(), ids.len());
    for (binding, id) in bindings.iter().zip(ids) {
        assert_eq!(&binding["edgeId"], id);
    }
    let rational =
        |numerator, denominator| json!({"numerator": numerator, "denominator": denominator});
    assert_eq!(
        bindings[0]["parameters"],
        json!([rational("0", "1"), rational("1", "2")])
    );
    let hole = bindings
        .iter()
        .find(|binding| binding["contour"] == 1)
        .unwrap();
    assert_eq!(hole["segment"], 3);
    assert_eq!(
        hole["parameters"],
        json!([rational("1", "1"), rational("0", "1")])
    );
}

#[test]
fn input_binding_operation_withholds_sources_when_work_is_exhausted() {
    let mut request = request();
    request["op"] = json!("brep_build_exact_periodic_trim_sheet_with_input_bindings");
    request["maxWork"] = json!(1);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    assert!(response["value"]["model"].is_null());
    assert_eq!(response["value"]["edgeBindings"], json!([]));
    assert_eq!(response["value"]["refusalStage"], "patch");
}
