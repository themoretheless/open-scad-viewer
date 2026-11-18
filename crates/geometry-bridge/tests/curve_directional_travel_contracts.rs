use geometry_bridge::execute;
use value_codec::{Value, json};

fn request() -> Value {
    json!({
        "op": "nurbs_measure_curve_directional_travel",
        "curve": {
            "degree": 2,
            "knots": [0., 0., 0., 1., 1., 1.],
            "controlPoints": [[0., 0.], [4., 0.], [2., 0.]],
            "weights": [1., 1., 1.],
            "periodic": false,
        },
        "direction": [1., 0.],
        "toleranceMm": 1e-7,
        "maxCells": 4096,
    })
}

fn run(input: &Value) -> Value {
    value_codec::from_str(&execute(&input.to_string())).unwrap()
}

#[test]
fn directional_travel_request_returns_axial_bounds_and_optional_limit_status() {
    let mut input = request();
    let response = run(&input);
    assert_eq!(response["ok"], true, "{response}");
    let result = &response["value"];
    assert!(result["reverseStatus"].is_null());
    for (key, exact) in [
        ("totalBoundsMm", 10. / 3.),
        ("forwardBoundsMm", 8. / 3.),
        ("reverseBoundsMm", 2. / 3.),
        ("signedTravelBoundsMm", 2.),
    ] {
        let bounds = &result[key];
        assert!(bounds[0].as_f64().unwrap() <= exact && exact <= bounds[1].as_f64().unwrap());
    }
    input["reverseLimitMm"] = json!(0.6);
    assert_eq!(run(&input)["value"]["reverseStatus"], "exceeds");
    input["parameterInterval"] = json!([0., 0.5]);
    assert_eq!(run(&input)["value"]["reverseStatus"], "within");
}

#[test]
fn directional_travel_request_reports_coarse_bounds_and_refuses_invalid_inputs() {
    let mut input = request();
    input["maxCells"] = json!(1);
    input["reverseLimitMm"] = json!(0.7);
    let response = run(&input);
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(response["value"]["reverseStatus"], "undetermined");
    assert_eq!(response["value"]["stopReason"], "workLimit");
    input["parameterInterval"] = json!([0., 2.]);
    assert_eq!(run(&input)["ok"], false);
}

#[test]
fn directional_travel_batch_preserves_window_order_and_shared_work_limit() {
    let mut input = request();
    input["op"] = json!("nurbs_measure_curve_directional_travel_batch");
    input["parameterIntervals"] = json!([[0.875, 1.], [0., 0.5], [0., 1.]]);
    input["reverseLimitMm"] = json!(0.6);
    let response = run(&input);
    assert_eq!(response["ok"], true, "{response}");
    let value = &response["value"];
    let intervals = value["intervals"].as_array().unwrap();
    assert_eq!(intervals.len(), 3);
    for (report, (interval, status)) in intervals.iter().zip([
        ([0.875, 1.], "within"), ([0., 0.5], "within"), ([0., 1.], "exceeds"),
    ]) {
        assert_eq!(report["parameterInterval"], json!(interval));
        assert_eq!(report["reverseStatus"], status);
    }
    let used: u64 = intervals.iter().map(|report| report["cells"].as_u64().unwrap()).sum();
    assert_eq!(value["cells"].as_u64(), Some(used));
    assert!(used <= 4096);

    input["maxCells"] = json!(2);
    assert_eq!(run(&input)["ok"], false);
    input["maxCells"] = json!(3);
    let limited = run(&input);
    assert_eq!(limited["ok"], true, "{limited}");
    assert_eq!(limited["value"]["cells"], 3);
    assert_eq!(limited["value"]["intervals"][2]["stopReason"], "workLimit");
    assert_eq!(limited["value"]["intervals"][2]["reverseStatus"], "undetermined");
}
