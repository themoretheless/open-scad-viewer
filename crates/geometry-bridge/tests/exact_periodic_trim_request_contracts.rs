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
