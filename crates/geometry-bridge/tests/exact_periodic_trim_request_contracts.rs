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

#[test]
fn contour_audit_returns_ordered_exact_split_parameters() {
    let mut request = request();
    request["op"] = json!("brep_audit_exact_periodic_trim_contour");
    request["contour"] = json!(0);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert!(report["refusalStage"].is_null());
    let joins = report["joins"].as_array().unwrap();
    assert_eq!(joins.len(), 6);
    assert_eq!(
        joins[1],
        json!({
            "segment": 0, "parameter": {"numerator": "1", "denominator": "2"}, "kind": "corner",
        })
    );
    assert_eq!(joins[4]["segment"], 2);

    request["contour"] = json!(2);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn contour_audit_withholds_partial_joins_with_only_build_budget() {
    let mut request = request();
    let model = run(&request);
    let build_work = model["value"]["workUsed"].as_u64().unwrap();
    request["op"] = json!("brep_audit_exact_periodic_trim_contour");
    request["contour"] = json!(0);
    request["maxWork"] = json!(build_work + 1);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    assert!(response["value"]["joins"].is_null());
    assert_eq!(response["value"]["refusalStage"], "audit");
    assert!(response["value"]["workUsed"].as_u64().unwrap() <= build_work + 1);
}

fn range_length_request() -> Value {
    let mut request = request();
    request["op"] = json!("brep_measure_exact_periodic_trim_range");
    request["contour"] = json!(0);
    request["segment"] = json!(0);
    request["maxCells"] = json!(100);
    request["range"] = json!([
        {"numerator": "4", "denominator": "5"},
        {"numerator": "1", "denominator": "3"},
    ]);
    request
}

#[test]
fn range_length_operation_encloses_exact_rational_endpoints() {
    let response = run(&range_length_request());
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert!(report["refusalStage"].is_null());
    assert_eq!(report["minimumCells"], 4);
    let length = &report["length"];
    let bounds = &length["boundsMm"];
    let expected = 7. / 15.;
    assert!(bounds[0].as_f64().unwrap() <= expected && expected <= bounds[1].as_f64().unwrap());
    assert_eq!(length["withinTolerance"], true);
}

#[test]
fn range_length_operation_reports_budget_and_rejects_zero_denominator() {
    let mut request = range_length_request();
    request["maxCells"] = json!(3);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    assert!(response["value"]["length"].is_null());
    assert_eq!(response["value"]["refusalStage"], "measure");
    request["range"][0]["denominator"] = json!("0");
    assert_eq!(run(&request)["ok"], false);
}

fn contour_station_request(distance: f64, cells: usize) -> Value {
    let mut request = request();
    request["op"] = json!("brep_locate_exact_periodic_trim_contour_station");
    request["contour"] = json!(0);
    request["distanceMm"] = json!(distance);
    request["parameterTolerance"] = json!(1e-6);
    request["maxCells"] = json!(cells);
    request
}

#[test]
fn contour_station_operation_contains_seam_and_original_vertex() {
    for (distance, parameter) in [(0.5, 0.5), (1., 1.)] {
        let response = run(&contour_station_request(distance, 1024));
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], "tolerance");
        let lo = report["parameterBounds"][0].as_f64().unwrap();
        let hi = report["parameterBounds"][1].as_f64().unwrap();
        assert!(lo <= parameter && parameter <= hi && hi - lo <= 1e-6);
    }
}

#[test]
fn contour_station_operation_distinguishes_limits_from_outside() {
    for (distance, cells, status, bounds) in [
        (0.5, 6, "workLimit", json!([0., 1.])),
        (6., 6, "outside", Value::Null),
    ] {
        let response = run(&contour_station_request(distance, cells));
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["value"]["status"], status);
        assert_eq!(response["value"]["parameterBounds"], bounds);
    }
    let mut request = contour_station_request(0.5, 6);
    request["maxWork"] = json!(1);
    let response = run(&request);
    assert_eq!(response["value"]["status"], "buildRefused");
    assert!(response["value"]["parameterBounds"].is_null());
}

#[test]
fn contour_station_bounds_operation_encloses_world_point_and_keeps_parameter_contract() {
    let mut request = contour_station_request(0.5, 1024);
    let original = run(&request);
    assert!(original["value"].get("pointBoundsMm").is_none());
    request["op"] = json!("brep_enclose_exact_periodic_trim_contour_station");
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(
        report["parameterBounds"],
        original["value"]["parameterBounds"]
    );
    assert_eq!(report["status"], "tolerance");
    let axes = report["pointBoundsMm"].as_array().unwrap();
    assert_eq!(axes.len(), 3);
    for (axis, expected) in axes.iter().zip([0., 0., 0.25]) {
        let lo = axis[0].as_f64().unwrap();
        let hi = axis[1].as_f64().unwrap();
        assert!(lo <= expected && expected <= hi && hi - lo < 1e-5);
    }
}

#[test]
fn contour_station_bounds_operation_encloses_coarse_brackets_only_when_proved() {
    let mut request = contour_station_request(0.5, 6);
    request["op"] = json!("brep_enclose_exact_periodic_trim_contour_station");
    let response = run(&request);
    assert_eq!(response["value"]["status"], "workLimit");
    assert!(response["value"]["pointBoundsMm"][0][1].as_f64().unwrap() >= 0.5);

    request["distanceMm"] = json!(6.);
    let outside = run(&request);
    assert_eq!(outside["value"]["status"], "outside");
    assert!(outside["value"]["pointBoundsMm"].is_null());

    request["maxWork"] = json!(1);
    let refused = run(&request);
    assert_eq!(refused["value"]["status"], "buildRefused");
    assert!(refused["value"]["pointBoundsMm"].is_null());
}

fn contour_division_request(cells: usize) -> Value {
    let mut request = contour_station_request(0., cells);
    request["op"] = json!("brep_divide_exact_periodic_trim_contour_by_length");
    request["divisions"] = json!(5);
    request
}

#[test]
fn contour_division_operation_returns_ordered_world_boxes_and_closure() {
    let response = run(&contour_division_request(2048));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["tableCells"], 6);
    assert!(report["cells"].as_u64().unwrap() <= 2048);
    let stations = report["stations"].as_array().unwrap();
    assert_eq!(stations.len(), 6);
    assert_eq!(stations[0]["parameterBounds"], json!([0., 0.]));
    assert_eq!(stations[5]["parameterBounds"], json!([4., 4.]));
    assert!(
        stations
            .iter()
            .all(|station| station["status"] == "tolerance")
    );
    for station in [&stations[0], &stations[5]] {
        let y = &station["pointBoundsMm"][1];
        assert!(y[0].as_f64().unwrap() <= 0.5 && y[1].as_f64().unwrap() >= 0.5);
    }
}

#[test]
fn contour_division_operation_keeps_proved_boxes_under_cell_limit() {
    let response = run(&contour_division_request(6));
    assert_eq!(response["ok"], true, "{response}");
    let stations = response["value"]["stations"].as_array().unwrap();
    assert_eq!(stations[1]["status"], "workLimit");
    assert!(stations[1]["pointBoundsMm"].as_array().is_some());
    let mut request = contour_division_request(6);
    request["maxWork"] = json!(1);
    let response = run(&request);
    assert!(response["value"]["stations"].is_null());
    assert_eq!(response["value"]["refusalStage"], "build");
}

fn patch_area_request(cells: usize, tolerance: f64) -> Value {
    let mut request = request();
    request["op"] = json!("brep_bound_exact_periodic_trim_patch_area");
    request["outerContour"] = json!(0);
    request["toleranceMm2"] = json!(tolerance);
    request["maxCells"] = json!(cells);
    request
}

#[test]
fn patch_area_operation_subtracts_holes_and_retains_coarse_bounds() {
    for (cells, tolerance, status) in [(100, 1e-8, "tolerance"), (10, 1e-30, "bounded")] {
        let response = run(&patch_area_request(cells, tolerance));
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], status);
        assert_eq!(report["withinTolerance"], status == "tolerance");
        assert_eq!(report["minimumCells"], 10);
        for (axis, expected) in report["boundsMm2"]
            .as_array()
            .unwrap()
            .iter()
            .zip([-0.75, -0.5, 0.])
        {
            assert!(axis[0].as_f64().unwrap() <= expected && expected <= axis[1].as_f64().unwrap());
        }
    }
}

#[test]
fn patch_area_operation_reports_missing_cells_and_non_outer_contour() {
    let mut request = patch_area_request(9, 1e-8);
    let limited = run(&request);
    assert_eq!(limited["value"]["status"], "workLimit");
    assert!(limited["value"]["boundsMm2"].is_null());
    request["outerContour"] = json!(1);
    assert_eq!(run(&request)["value"]["status"], "unavailable");
    request["maxWork"] = json!(1);
    assert_eq!(run(&request)["value"]["status"], "buildRefused");
}

#[test]
fn directional_area_operation_admits_both_normal_orientations() {
    let mut request = patch_area_request(100, 1e-8);
    request["op"] = json!("brep_bound_exact_periodic_trim_directional_area");
    for sign in [-1., 1.] {
        request["normal"] = json!([sign, sign, 0.]);
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        let expected = -sign * 1.25 / 2.0_f64.sqrt();
        assert_eq!(report["status"], "tolerance");
        assert_eq!(report["minimumCells"], 10);
        assert!(report["boundsMm2"][0].as_f64().unwrap() <= expected);
        assert!(report["boundsMm2"][1].as_f64().unwrap() >= expected);
    }
}

#[test]
fn directional_area_operation_checks_normal_and_preserves_cell_shortage() {
    let mut request = patch_area_request(9, 1e-8);
    request["op"] = json!("brep_bound_exact_periodic_trim_directional_area");
    request["normal"] = json!([0., 0., 0.]);
    assert_eq!(run(&request)["ok"], false);
    request["normal"] = json!([1., 1., 0.]);
    let response = run(&request);
    assert_eq!(response["value"]["status"], "workLimit");
    assert!(response["value"]["boundsMm2"].is_null());
}

fn surface_area_request(max_surface: usize, max_trim: usize) -> Value {
    let mut request = request();
    request["op"] = json!("brep_measure_exact_periodic_trim_surface_area");
    request["outerContour"] = json!(0);
    request["toleranceMm2"] = json!(0.05);
    request["toleranceUv"] = json!(1e-10);
    request["maxSurfaceCells"] = json!(max_surface);
    request["maxTrimCells"] = json!(max_trim);
    request
}

#[test]
fn surface_area_operation_measures_retained_true_area_with_a_hole() {
    let response = run(&surface_area_request(10_000, 1_000_000));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["status"], "tolerance");
    let lo = report["boundsMm2"][0].as_f64().unwrap();
    let hi = report["boundsMm2"][1].as_f64().unwrap();
    assert!(lo <= 1.25 && 1.25 <= hi);
    assert!(report["surfaceCells"].as_u64().unwrap() <= 10_000);
    assert!(report["trimCells"].as_u64().unwrap() <= 1_000_000);
}

#[test]
fn surface_area_operation_distinguishes_incomplete_coverage_and_coarse_bounds() {
    let missing = run(&surface_area_request(1, 1));
    assert_eq!(missing["value"]["status"], "workLimit");
    assert!(missing["value"]["boundsMm2"].is_null());
    assert_eq!(missing["value"]["minimumSurfaceCells"], 2);
    let coarse = run(&surface_area_request(2, 1));
    assert_eq!(coarse["value"]["trimCells"], 1);
    assert_eq!(coarse["value"]["status"], "workLimit");
    assert!(coarse["value"]["boundsMm2"][1].as_f64().unwrap() >= 1.25);
    let mut request = surface_area_request(2, 1);
    request["outerContour"] = json!(1);
    assert_eq!(run(&request)["value"]["status"], "unavailable");
}
