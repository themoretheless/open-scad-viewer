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

#[test]
fn trimmed_surface_mass_properties_operation_reports_shared_area_and_centroid() {
    let mut request = surface_area_request(10_000, 1_000_000);
    request["toleranceMm2"] = json!(0.005);
    let area_only = run(&request);
    request["op"] = json!("brep_measure_exact_periodic_trim_surface_mass_properties");
    request["momentToleranceMm4"] = json!(1e-8);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    for key in ["surfaceCells", "trimCells", "boundsMm2"] {
        assert_eq!(report[key], area_only["value"][key]);
    }
    let centroid = report["centroidBoundsMm"].as_array().unwrap();
    assert_eq!(centroid.len(), 3);
    for (interval, expected) in centroid.iter().zip([0.1, 0.15, 1.]) {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    let tensor = &report["centralInertiaBoundsMm4"];
    assert_eq!(tensor[0][1], tensor[1][0]);
    assert_eq!(
        report["principalMomentBoundsMm4"].as_array().unwrap().len(),
        3
    );
}

#[test]
fn trimmed_surface_mass_properties_operation_withholds_unproved_center() {
    let mut request = surface_area_request(2, 1);
    request["op"] = json!("brep_measure_exact_periodic_trim_surface_mass_properties");
    request["momentToleranceMm4"] = json!(1e-8);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert!(report["boundsMm2"].as_array().is_some());
    assert!(report["centroidBoundsMm"].is_null());
    assert!(report["centralInertiaBoundsMm4"].is_null());
    assert!(report["principalMomentBoundsMm4"].is_null());
    assert_eq!(report["withinMomentTolerance"], false);
}

fn window_mass_request(window: [[f64; 2]; 2]) -> Value {
    let mut request = surface_area_request(10_000, 500_000);
    request["op"] = json!("brep_measure_exact_periodic_trim_window_mass_properties");
    request["uvWindow"] = json!(window);
    request["toleranceMm2"] = json!(0.001);
    request["momentToleranceMm4"] = json!(1e-8);
    request
}

#[test]
fn window_mass_properties_operation_reports_clipped_area_and_centroid() {
    let response = run(&window_mass_request([[3.75, 4.25], [0.375, 0.625]]));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let area = &report["boundsMm2"];
    assert!(area[0].as_f64().unwrap() <= 0.1875 && 0.1875 <= area[1].as_f64().unwrap());
    let centroid = report["centroidBoundsMm"].as_array().unwrap();
    for (interval, expected) in centroid.iter().zip([1. / 48., 1. / 12., 1.]) {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    assert_eq!(
        report["principalMomentBoundsMm4"].as_array().unwrap().len(),
        3
    );
}

#[test]
fn window_mass_properties_operation_distinguishes_empty_and_invalid_windows() {
    let response = run(&window_mass_request([[5., 6.], [0.25, 0.5]]));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["boundsMm2"], json!([0., 0.]));
    assert_eq!(report["surfaceCells"], 0);
    assert!(report["centroidBoundsMm"].is_null());
    assert!(report["principalMomentBoundsMm4"].is_null());
    assert_eq!(
        run(&window_mass_request([[4.25, 3.75], [0.375, 0.625]]))["ok"],
        false
    );
}

fn patch_set_mass_request(max_cells: usize) -> Value {
    let mut request = surface_area_request(max_cells, 1_000_000);
    request["op"] = json!("brep_measure_exact_periodic_trim_patch_set_mass_properties");
    request["contours"].as_array_mut().unwrap().push(json!([
        [0.1875, 0.375],
        [0.3125, 0.375],
        [0.3125, 0.625],
        [0.1875, 0.625],
        [0.1875, 0.375]
    ]));
    request
}

#[test]
fn patch_set_mass_operation_returns_shared_centroid_and_island_area() {
    let response = run(&patch_set_mass_request(20_000));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let components = report["componentAreas"].as_array().unwrap();
    assert_eq!(components.len(), 2);
    assert_eq!(components[1]["outerContour"], 2);
    for (interval, expected) in [
        (&report["boundsMm2"], 1.3125),
        (&components[1]["boundsMm2"], 0.0625),
        (&report["centroidBoundsMm"][0], 3. / 28.),
    ] {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    assert_eq!(
        report["centralInertiaBoundsMm4"].as_array().unwrap().len(),
        3
    );
}

#[test]
fn patch_set_mass_operation_does_not_publish_partial_components() {
    let response = run(&patch_set_mass_request(2));
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["status"], "workLimit");
    assert_eq!(report["minimumSurfaceCells"], 3);
    assert_eq!(report["componentAreas"], json!([]));
    assert!(report["boundsMm2"].is_null() && report["centroidBoundsMm"].is_null());
}

#[test]
fn patch_set_mass_operation_reports_principal_bounds_and_requested_tolerance() {
    let mut request = patch_set_mass_request(20_000);
    let legacy = run(&request);
    assert_eq!(legacy["ok"], true, "{legacy}");
    assert!(legacy["value"].get("principalMomentBoundsMm4").is_none());

    for (tolerance, met) in [(10., true), (1e-8, false)] {
        request["momentToleranceMm4"] = json!(tolerance);
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["withinMomentTolerance"], met);
        assert_eq!(report["boundsMm2"], legacy["value"]["boundsMm2"]);
        assert_eq!(
            report["principalMomentBoundsMm4"].as_array().unwrap().len(),
            3
        );
    }
}

#[test]
fn patch_set_mass_operation_validates_tolerance_and_withholds_unproved_spectrum() {
    let mut request = patch_set_mass_request(2);
    for invalid in [json!(0.), json!(-1.), json!(null), json!("1")] {
        request["momentToleranceMm4"] = invalid;
        assert_eq!(run(&request)["ok"], false);
    }
    request["momentToleranceMm4"] = json!(10.);
    for status in ["workLimit", "buildRefused"] {
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], status);
        assert!(report["principalMomentBoundsMm4"].is_null());
        assert_eq!(report["withinMomentTolerance"], false);
        request["maxWork"] = json!(1);
    }
}

#[test]
fn patch_set_mass_operation_returns_offset_axis_moment_with_principal_moments() {
    let mut request = patch_set_mass_request(20_000);
    let legacy = run(&request);
    assert!(legacy["value"].get("axisMomentBoundsMm4").is_none());
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., -2.]});
    request["momentToleranceMm4"] = json!(10.);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["withinMomentTolerance"], true);
    assert_eq!(report["boundsMm2"], legacy["value"]["boundsMm2"]);
    let expected = 83. / 768. + 49. / 12288. + 1.3125 * (1. - 2. * 3. / 28.);
    let bounds = &report["axisMomentBoundsMm4"];
    assert!(bounds[0].as_f64().unwrap() <= expected && expected <= bounds[1].as_f64().unwrap());
}

#[test]
fn patch_set_mass_operation_rejects_malformed_axes_and_withholds_unproved_moments() {
    let mut request = patch_set_mass_request(2);
    for invalid in [
        json!(null),
        json!({"pointMm": [0., 0., 0.]}),
        json!({"pointMm": [0., 0.], "direction": [0., 0., 1.]}),
        json!({"pointMm": [0., 0., 0.], "direction": [0., 0., 0.]}),
    ] {
        request["inertiaAxis"] = invalid;
        assert_eq!(run(&request)["ok"], false);
    }
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    for status in ["workLimit", "buildRefused"] {
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], status);
        assert_eq!(report.get("axisMomentBoundsMm4"), Some(&Value::Null));
        request["maxWork"] = json!(1);
    }
}

#[test]
fn patch_set_mass_operation_returns_tensor_about_fixed_reference_point() {
    let mut request = patch_set_mass_request(20_000);
    let legacy = run(&request);
    assert_eq!(legacy["ok"], true, "{legacy}");
    assert!(legacy["value"].get("referenceInertiaBoundsMm4").is_none());
    request["inertiaReferencePointMm"] = json!([1., 0., 0.]);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let tensor = &report["referenceInertiaBoundsMm4"];
    assert_eq!(
        report["centralInertiaBoundsMm4"],
        legacy["value"]["centralInertiaBoundsMm4"]
    );
    let expected_zz = 83. / 768. + 49. / 12288. + 1.3125 * (1. - 2. * 3. / 28.);
    let expected_xy = 1.3125 / 7.;
    for (interval, expected) in [
        (&tensor[2][2], expected_zz),
        (&tensor[0][1], expected_xy),
        (&report["axisMomentBoundsMm4"], expected_zz),
    ] {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    assert_eq!(tensor[0][1], tensor[1][0]);
}

#[test]
fn patch_set_mass_operation_validates_reference_point_and_withholds_unproved_tensor() {
    let mut request = patch_set_mass_request(2);
    for invalid in [json!(null), json!([0., 0.]), json!([0., 0., "0"])] {
        request["inertiaReferencePointMm"] = invalid;
        assert_eq!(run(&request)["ok"], false);
    }
    request["inertiaReferencePointMm"] = json!([0., 0., 0.]);
    for status in ["workLimit", "buildRefused"] {
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], status);
        assert_eq!(report.get("referenceInertiaBoundsMm4"), Some(&Value::Null));
        request["maxWork"] = json!(1);
    }
}

#[test]
fn patch_set_mass_operation_returns_requested_axis_radius_of_gyration() {
    let mut request = patch_set_mass_request(20_000);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    let legacy = run(&request);
    assert_eq!(legacy["ok"], true, "{legacy}");
    assert!(legacy["value"].get("axisRadiusOfGyrationBoundsMm").is_none());

    request["includeAxisRadiusOfGyration"] = json!(true);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let expected = ((83. / 768. + 49. / 12288.) / 1.3125 + 1. - 2. * 3. / 28_f64).sqrt();
    let bounds = &report["axisRadiusOfGyrationBoundsMm"];
    assert!(bounds[0].as_f64().unwrap() <= expected && expected <= bounds[1].as_f64().unwrap());
    assert_eq!(
        report["axisMomentBoundsMm4"],
        legacy["value"]["axisMomentBoundsMm4"]
    );
}

#[test]
fn patch_set_mass_operation_requires_axis_and_positive_area_for_gyration_radius() {
    let mut request = patch_set_mass_request(2);
    request["includeAxisRadiusOfGyration"] = json!(true);
    assert_eq!(run(&request)["ok"], false);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    request["includeAxisRadiusOfGyration"] = json!("true");
    assert_eq!(run(&request)["ok"], false);
    request["includeAxisRadiusOfGyration"] = json!(true);
    for status in ["workLimit", "buildRefused"] {
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let report = &response["value"];
        assert_eq!(report["status"], status);
        assert_eq!(
            report.get("axisRadiusOfGyrationBoundsMm"),
            Some(&Value::Null)
        );
        request["maxWork"] = json!(1);
    }
}

fn patch_set_density_request(max_cells: usize) -> Value {
    let mut request = patch_set_mass_request(max_cells);
    request["op"] = json!("brep_measure_exact_periodic_trim_patch_set_density");
    request["componentDensities"] = json!([
        {"outerContour": 2, "densityBoundsKgPerMm2": [6., 6.]},
        {"outerContour": 0, "densityBoundsKgPerMm2": [2., 2.]}
    ]);
    request
}

#[test]
fn patch_set_density_assigns_mass_by_outer_contour_in_any_input_order() {
    let mut request = patch_set_density_request(20_000);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let mass = 2.875;
    let center_x = 0.34375 / mass;
    let center_y = 0.375 / mass;
    let inertia_z = 2. * 83. / 768. + 6. * 49. / 12288.
        - mass * (center_x * center_x + center_y * center_y);
    for (interval, expected) in [
        (&report["boundsMm2"], 1.3125),
        (&report["massBoundsKg"], mass),
        (&report["centerOfMassBoundsMm"][0], center_x),
        (&report["centralInertiaBoundsKgMm2"][2][2], inertia_z),
    ] {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    request["componentDensities"].as_array_mut().unwrap().reverse();
    assert_eq!(run(&request), response);
}

#[test]
fn patch_set_density_rejects_incomplete_and_non_outer_density_assignments() {
    let valid = patch_set_density_request(2);
    let response = run(&valid);
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(response["value"]["status"], "workLimit");
    assert_eq!(response["value"].get("massBoundsKg"), Some(&Value::Null));
    for (contour, density) in [(0, [6., 6.]), (1, [6., 6.]), (2, [0., 6.])] {
        let mut request = valid.clone();
        request["componentDensities"].as_array_mut().unwrap()[0] =
            json!({"outerContour": contour, "densityBoundsKgPerMm2": density});
        assert_eq!(run(&request)["ok"], false);
    }
    let mut missing = valid;
    missing["componentDensities"].as_array_mut().unwrap().pop();
    assert_eq!(run(&missing)["ok"], false);
}

#[test]
fn patch_set_density_reports_each_component_mass_by_contour() {
    let response = run(&patch_set_density_request(20_000));
    assert_eq!(response["ok"], true, "{response}");
    let components = response["value"]["componentMasses"].as_array().unwrap();
    assert_eq!(components.len(), 2);
    for (component, (contour, expected)) in components.iter().zip([(0, 2.5), (2, 0.375)]) {
        assert_eq!(component["outerContour"], contour);
        let interval = &component["boundsKg"];
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    let incomplete = run(&patch_set_density_request(2));
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(incomplete["value"]["componentMasses"], json!([]));
    assert_eq!(incomplete["value"]["massBoundsKg"], Value::Null);
}

#[test]
fn patch_set_density_reports_fraction_intervals_by_outer_contour() {
    let mut request = patch_set_density_request(20_000);
    request["componentDensities"][0]["densityBoundsKgPerMm2"] = json!([6., 8.]);
    request["componentDensities"][1]["densityBoundsKgPerMm2"] = json!([2., 3.]);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let components = response["value"]["componentMasses"].as_array().unwrap();
    assert_eq!(components.len(), 2);
    let references = [
        (0, [2.5 / 3., 3.75 / 4.125]),
        (2, [0.375 / 4.125, 0.5 / 3.]),
    ];
    for (component, (contour, [minimum, maximum])) in components.iter().zip(references) {
        assert_eq!(component["outerContour"], contour);
        let fraction = &component["massFractionBounds"];
        let lo = fraction[0].as_f64().unwrap();
        let hi = fraction[1].as_f64().unwrap();
        assert!(0. <= lo && lo <= minimum && maximum <= hi && hi <= 1.);
    }
    request["maxSurfaceCells"] = json!(2);
    let incomplete = run(&request);
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(incomplete["value"]["componentMasses"], json!([]));
}

#[test]
fn patch_set_density_returns_signed_center_distance_to_requested_plane() {
    let mut request = patch_set_density_request(20_000);
    let legacy = run(&request);
    assert_eq!(legacy["ok"], true, "{legacy}");
    assert!(legacy["value"].get("centerOfMassPlaneDistanceBoundsMm").is_none());

    request["centerOfMassPlane"] =
        json!({"pointMm": [1., 0., 0.], "normal": [2., -2., 0.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let interval = &response["value"]["centerOfMassPlaneDistanceBoundsMm"];
    let expected = ((0.34375 - 0.375) / 2.875 - 1.) / 2_f64.sqrt();
    assert!(interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap());
    assert!(interval[1].as_f64().unwrap() < 0.);
    request["maxSurfaceCells"] = json!(2);
    let incomplete = run(&request);
    assert_eq!(
        incomplete["value"].get("centerOfMassPlaneDistanceBoundsMm"),
        Some(&Value::Null)
    );
    request["centerOfMassPlane"]["normal"] = json!([0., 0., 0.]);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_returns_requested_moment_about_a_fixed_axis() {
    let mut request = patch_set_density_request(20_000);
    let prior = run(&request);
    assert_eq!(prior["ok"], true, "{prior}");
    assert!(prior["value"].get("axisMomentBoundsKgMm2").is_none());
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., -2.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let expected = 2. * 83. / 768. + 6. * 49. / 12288. - 2. * 0.34375 + 2.875;
    let interval = &response["value"]["axisMomentBoundsKgMm2"];
    assert!(interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap());
    assert_eq!(response["value"]["massBoundsKg"], prior["value"]["massBoundsKg"]);
    request["maxSurfaceCells"] = json!(2);
    let incomplete = run(&request);
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(incomplete["value"].get("axisMomentBoundsKgMm2"), Some(&Value::Null));
    request["inertiaAxis"]["direction"] = json!([0., 0., 0.]);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_returns_requested_tensor_about_fixed_reference_point() {
    let mut request = patch_set_density_request(20_000);
    let prior = run(&request);
    assert_eq!(prior["ok"], true, "{prior}");
    assert!(prior["value"].get("referenceInertiaBoundsKgMm2").is_none());
    request["inertiaReferencePointMm"] = json!([1., 0., 0.]);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let tensor = &report["referenceInertiaBoundsKgMm2"];
    let expected_zz = 2. * 83. / 768. + 6. * 49. / 12288. - 2. * 0.34375 + 2.875;
    for (interval, expected) in [
        (&tensor[2][2], expected_zz),
        (&tensor[0][1], 0.375),
        (&report["axisMomentBoundsKgMm2"], expected_zz),
    ] {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    assert_eq!(tensor[0][1], tensor[1][0]);

    request["maxSurfaceCells"] = json!(2);
    let incomplete = run(&request);
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(
        incomplete["value"].get("referenceInertiaBoundsKgMm2"),
        Some(&Value::Null)
    );
    request["inertiaReferencePointMm"] = json!([1., 0.]);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_encloses_weighted_principal_moments() {
    let mut request = patch_set_density_request(20_000);
    assert!(run(&request)["value"].get("principalMomentBoundsKgMm2").is_none());
    request["momentToleranceKgMm2"] = json!(10.);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    assert_eq!(report["withinMomentTolerance"], true);
    let bounds = report["principalMomentBoundsKgMm2"].as_array().unwrap();
    assert_eq!(bounds.len(), 3);
    let mass = 2.875;
    let cx = 0.34375 / mass;
    let cy = 0.375 / mass;
    let vx = 2. * (1. / 16. - 13. / 768.) + 6. * 49. / 12288. - mass * cx * cx;
    let vy = 2. / 16. - mass * cy * cy;
    let vz = 2. * (1.5 * 1.5_f64.powi(2) / 12. - 0.25 / 12.) + 6. * 0.0625 * 0.25 / 12.;
    let average = vz + (vx + vy) / 2.;
    let split = ((vx - vy) / 2.).hypot(mass * cx * cy);
    let mut exact = [vx + vy, average - split, average + split];
    exact.sort_by(f64::total_cmp);
    for (interval, value) in bounds.iter().zip(exact) {
        assert!(interval[0].as_f64().unwrap() <= value && value <= interval[1].as_f64().unwrap());
    }

    request["maxSurfaceCells"] = json!(2);
    let limited = run(&request);
    assert_eq!(limited["value"].get("principalMomentBoundsKgMm2"), Some(&Value::Null));
    assert_eq!(limited["value"]["withinMomentTolerance"], false);
    request["momentToleranceKgMm2"] = json!(0.);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_returns_radius_of_gyration_from_weighted_mass_and_moment() {
    let mut request = patch_set_density_request(20_000);
    request["includeAxisRadiusOfGyration"] = json!(true);
    assert_eq!(run(&request)["ok"], false);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let report = &response["value"];
    let mass: f64 = 2.875;
    let moment = 2. * 83. / 768. + 6. * 49. / 12288. - 2. * 0.34375 + mass;
    let radius = (moment / mass).sqrt();
    let interval = &report["axisRadiusOfGyrationBoundsMm"];
    assert!(interval[0].as_f64().unwrap() <= radius && radius <= interval[1].as_f64().unwrap());
    request["maxSurfaceCells"] = json!(2);
    let incomplete = run(&request);
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(
        incomplete["value"].get("axisRadiusOfGyrationBoundsMm"),
        Some(&Value::Null)
    );
}

#[test]
fn patch_set_density_encloses_first_mass_moment_about_requested_plane() {
    let mut request = patch_set_density_request(20_000);
    assert!(
        run(&request)["value"]
            .get("firstMassMomentAboutPlaneBoundsKgMm")
            .is_none()
    );
    request["centerOfMassPlane"] = json!({"pointMm": [1., 0., 0.], "normal": [2., -2., 0.]});
    let expected = (0.34375 - 0.375 - 2.875) / 2_f64.sqrt();
    for normal in [[2., -2., 0.], [-2., 2., 0.]] {
        request["centerOfMassPlane"]["normal"] = json!(normal);
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let bounds = &response["value"]["firstMassMomentAboutPlaneBoundsKgMm"];
        let reference = if normal[0] > 0. { expected } else { -expected };
        assert!(
            bounds[0].as_f64().unwrap() <= reference && reference <= bounds[1].as_f64().unwrap()
        );
    }
    request["maxSurfaceCells"] = json!(2);
    let limited = run(&request);
    assert_eq!(limited["ok"], true, "{limited}");
    assert_eq!(
        limited["value"].get("firstMassMomentAboutPlaneBoundsKgMm"),
        Some(&Value::Null)
    );
}

fn separated_density_request() -> Value {
    let mut request = patch_set_density_request(20_000);
    request["contours"] = json!([
        [[0.125, 0.125], [0.1875, 0.125], [0.1875, 0.1875], [0.125, 0.1875], [0.125, 0.125]],
        [[0.125, 0.8125], [0.1875, 0.8125], [0.1875, 0.875], [0.125, 0.875], [0.125, 0.8125]]
    ]);
    request["componentDensities"][0]["outerContour"] = json!(1);
    request["toleranceMm2"] = json!(1e-5);
    request
}

#[test]
fn patch_set_density_certifies_axis_of_separated_components() {
    let mut request = separated_density_request();
    request["includePrincipalAxes"] = json!(true);
    assert_eq!(run(&request)["ok"], false);
    request["momentToleranceKgMm2"] = json!(1e-8);
    request["toleranceMm2"] = json!(1e-5);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let axes = response["value"]["principalAxes"].as_array().unwrap_or_else(|| panic!("{response}"));
    assert_eq!(axes.len(), 3);
    let least = &axes[0];
    assert!(least["sinAngleUpper"].as_f64().unwrap() < 0.5, "{response}");
    assert!(least["direction"][2].as_f64().unwrap().abs() > 0.9);
    assert!(axes[1].is_null() && axes[2].is_null());

    request["maxSurfaceCells"] = json!(1);
    let incomplete = run(&request);
    assert_eq!(incomplete["ok"], true, "{incomplete}");
    assert_eq!(incomplete["value"].get("principalAxes"), Some(&Value::Null));
}

#[test]
fn patch_set_density_encloses_point_distance_to_certified_axis() {
    let mut request = separated_density_request();
    request["principalAxisProbePointMm"] = json!([1., 0., 1.]);
    request["momentToleranceKgMm2"] = json!(1e-8);
    assert_eq!(run(&request)["ok"], false);
    request["includePrincipalAxes"] = json!(true);
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let axes = &response["value"]["principalAxes"];
    let distance = &axes[0]["distanceBoundsMm"];
    let exact = 0.84375;
    let lo = distance[0].as_f64().unwrap();
    let hi = distance[1].as_f64().unwrap();
    assert!(lo <= exact && exact <= hi && hi - lo < 0.2, "{response}");
    assert!(axes[1].is_null() && axes[2].is_null());
    request["principalAxisProbePointMm"] = json!([1., 0.]);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_encloses_direction_angle_to_principal_axis() {
    let mut request = separated_density_request();
    request["momentToleranceKgMm2"] = json!(1e-8);
    request["principalAxisProbeDirection"] = json!([1., 0., 1.]);
    assert_eq!(run(&request)["ok"], false);
    request["includePrincipalAxes"] = json!(true);
    let forward = run(&request);
    assert_eq!(forward["ok"], true, "{forward}");
    let axes = &forward["value"]["principalAxes"];
    let angle = &axes[0]["angleSineBounds"];
    let (lo, hi) = (angle[0].as_f64().unwrap(), angle[1].as_f64().unwrap());
    assert!(0. <= lo && lo <= 0.5_f64.sqrt() && 0.5_f64.sqrt() <= hi && hi <= 1.);
    assert!(axes[1].is_null() && axes[2].is_null());
    request["principalAxisProbeDirection"] = json!([-2., 0., -2.]);
    assert_eq!(run(&request), forward);
    request["principalAxisProbeDirection"] = json!([0., 0., 0.]);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_classifies_eccentricity_of_requested_balance_axis() {
    let mut request = patch_set_density_request(20_000);
    request["axisBalanceToleranceMm"] = json!(0.5);
    assert_eq!(run(&request)["ok"], false);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., 1.]});
    let expected = (1. - 0.34375_f64 / 2.875).hypot(0.375 / 2.875);
    for (tolerance, status) in [(0.5, "outside"), (2., "within")] {
        request["axisBalanceToleranceMm"] = json!(tolerance);
        let response = run(&request);
        assert_eq!(response["ok"], true, "{response}");
        let balance = &response["value"]["axisBalance"];
        let bounds = &balance["eccentricityBoundsMm"];
        assert!(
            bounds[0].as_f64().unwrap() <= expected && expected <= bounds[1].as_f64().unwrap()
        );
        assert_eq!(balance["status"], status);
    }
    request["maxSurfaceCells"] = json!(2);
    let limited = run(&request);
    assert_eq!(limited["ok"], true, "{limited}");
    assert_eq!(limited["value"].get("axisBalance"), Some(&Value::Null));
    request["axisBalanceToleranceMm"] = json!(-1.);
    assert_eq!(run(&request)["ok"], false);
}

#[test]
fn patch_set_density_returns_static_unbalance_about_requested_axis() {
    let mut request = patch_set_density_request(20_000);
    request["includeStaticUnbalance"] = json!(true);
    assert_eq!(run(&request)["ok"], false);
    request["inertiaAxis"] = json!({"pointMm": [1., 0., 5.], "direction": [0., 0., -2.]});
    let response = run(&request);
    assert_eq!(response["ok"], true, "{response}");
    let unbalance = &response["value"]["staticUnbalance"];
    let exact = [0.34375_f64 - 2.875, 0.375, 0.];
    let vector = unbalance["vectorBoundsKgMm"].as_array().unwrap();
    assert_eq!(vector.len(), 3);
    for (interval, expected) in vector.iter().zip(exact) {
        assert!(
            interval[0].as_f64().unwrap() <= expected && expected <= interval[1].as_f64().unwrap()
        );
    }
    let magnitude = &unbalance["magnitudeBoundsKgMm"];
    let expected = exact[0].hypot(exact[1]);
    assert!(magnitude[0].as_f64().unwrap() <= expected && expected <= magnitude[1].as_f64().unwrap());
    request["maxSurfaceCells"] = json!(2);
    assert_eq!(run(&request)["value"].get("staticUnbalance"), Some(&Value::Null));
}
