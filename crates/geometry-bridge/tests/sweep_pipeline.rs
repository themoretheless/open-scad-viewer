use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn send(op: &str, mut request: Value) -> Value {
    request["op"] = json!(op);
    dispatch(request).unwrap_or_else(|error| panic!("{op}: {error}"))
}
fn law(value: f64) -> Value {
    json!({"degree":1,"knots":[0,0,1,1],"values":[value,value],"weights":[1,1]})
}
fn raw() -> Value {
    json!({"profiles":[{"degree":1,"knots":[0,0,1,1],"controlPoints":[[0.1,0,0],[0.2,0,0]],"weights":[1,1],"periodic":false}],"points":[[0,0,0],[0,0,10]],"scale":law(1.),"twist":law(0.),"options":{"normal":[1,0,0],"maxSteps":4,"maxDeviation":0.001}})
}
#[test]
fn native_stream_retains_source_and_does_not_admit_exhausted_previews() {
    let mut request = raw();
    request["kind"] = json!("raw-miter");
    let stream = send("brep_sweep_stream_start", request.clone());
    request["profiles"][0]["controlPoints"][0][0] = json!(999.);
    let first = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(first["done"], false);
    let final_result = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(final_result["done"], true);
    let expected = send("brep_sweep_constructor", {
        let mut r = raw();
        r["operation"] = json!("curve_progressive_miter");
        r
    });
    assert_eq!(final_result["value"]["sections"], expected["sections"]);
    assert_eq!(final_result["value"]["report"], expected["report"]);
    assert!(
        dispatch({
            let mut r = stream;
            r["op"] = json!("brep_sweep_stream_next");
            r
        })
        .is_err()
    );
    let mut short = raw();
    short["kind"] = json!("raw-miter");
    short["twist"]["values"] = json!([0, 90]);
    short["options"]["maxSteps"] = json!(1);
    short["options"]["maxDeviation"] = json!(1e-15);
    let stream = send("brep_sweep_stream_start", short);
    let preview = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(preview["value"]["report"]["accepted"], false);
    let result = send("brep_sweep_stream_next", stream);
    assert_eq!(result["done"], true);
    assert!(result["value"]["sections"].is_null());
}
#[test]
fn native_stream_release_cancels_owned_refinement() {
    let mut r = raw();
    r["kind"] = json!("raw-miter");
    let stream = send("brep_sweep_stream_start", r);
    send("brep_sweep_stream_release", stream.clone());
    assert!(
        dispatch({
            let mut r = stream;
            r["op"] = json!("brep_sweep_stream_next");
            r
        })
        .is_err()
    );
}
#[test]
fn native_constructor_converts_degree_laws_before_geometry() {
    let mut authored = raw();
    authored["twist"]["values"] = json!([0, 30]);
    authored["operation"] = json!("curve_progressive_miter_level");
    authored["preview_steps"] = json!(2);
    let actual = send("brep_sweep_constructor", authored.clone());
    let mut native = authored;
    native["op"] = json!("curve_progressive_miter_level");
    native["scale"] = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[1,0,0],[1,0,0]],"weights":[1,1],"periodic":false});
    native["twist"] = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[30.*std::f64::consts::PI/180.,0,0]],"weights":[1,1],"periodic":false});
    native["normal"] = json!([1, 0, 0]);
    native["closed"] = json!(false);
    native["miter_limit"] = json!(4);
    native["initial_steps"] = json!(1);
    native["max_steps"] = json!(4);
    native["max_deviation"] = json!(0.001);
    assert_eq!(actual, dispatch(native).unwrap());
}
fn source() -> Value {
    let circle = send(
        "curve_circle",
        json!({"center":[0,0,0],"normal":[0,0,1],"radius":0.5}),
    );
    let mut r = raw();
    r["loops"] = json!([[circle]]);
    r["options"]["maxSteps"] = json!(1);
    r["options"]["maxDeviation"] = json!(0.01);
    r["options"]["circleCorrection"] =
        json!({"quantum":2f64.powi(-40),"tolerance":1e-9,"maxWork":100000});
    send("brep_progressive_miter_body", r)
}
#[test]
fn native_ownership_rejects_copies_mutation_and_exhausted_placement() {
    let source = source();
    assert_eq!(source["boundaryCertificate"]["continuousBound"], true);
    let request = json!({"source":source,"owner":source["_nativeOwner"],"matrix":[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]],"options":{"quantum":2f64.powi(-40),"maxWork":100000,"maxDeviation":1e-9}});
    let placed = send("brep_transform_certified_miter", request.clone());
    assert_eq!(placed["boundaryCertificate"]["withinBudget"], true);
    for mode in 0..3 {
        let mut invalid = request.clone();
        invalid["op"] = json!("brep_transform_certified_miter");
        match mode {
            0 => invalid["owner"] = Value::Null,
            1 => {
                invalid["source"]["model"]["faces"][0]["surface"]["controlPoints"][0][0][0] =
                    json!(99)
            }
            _ => invalid["options"]["maxWork"] = json!(1),
        };
        assert!(dispatch(invalid).is_err());
    }
    send(
        "brep_sweep_release_owner",
        json!({"owner":placed["_nativeOwner"]}),
    );
    send(
        "brep_sweep_release_owner",
        json!({"owner":source["_nativeOwner"]}),
    );
}
#[test]
fn native_profile_body_stream_matches_owned_station_topology() {
    let circle = send(
        "curve_circle",
        json!({"center":[0,0,0],"normal":[0,0,1],"radius":0.5}),
    );
    let request = json!({"kind":"profile","loops":[[circle]],"path":{"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,1],"periodic":false},"scale":law(1.),"twist":law(0.),"options":{"normal":[1,0,0],"initialSections":5,"maxSections":5,"maxDeviation":0.001}});
    let body = send("brep_progressive_profile_body", request.clone());
    assert_eq!(body["model"]["faces"].as_array().unwrap().len(), 18);
    let stream = send("brep_sweep_stream_start", request);
    let preview = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(preview["value"]["report"]["accepted"], true);
    let result = send("brep_sweep_stream_next", stream);
    assert_eq!(result["value"]["model"], body["model"]);
}

#[test]
fn accepted_miter_preview_remains_owned_until_final_certificates() {
    let circle = send(
        "curve_circle",
        json!({"center":[0,0,0],"normal":[0,0,1],"radius":0.5}),
    );
    let mut request = raw();
    request["kind"] = json!("miter");
    request["loops"] = json!([[circle]]);
    request["options"]["maxSteps"] = json!(1);
    request["options"]["maxDeviation"] = json!(0.01);
    request["options"]["circleCorrection"] =
        json!({"quantum":2f64.powi(-40),"tolerance":1e-9,"maxWork":100000});
    let sync = send("brep_progressive_miter_body", request.clone());
    let stream = send("brep_sweep_stream_start", request.clone());
    let mut preview = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(preview["value"]["report"]["accepted"], true);
    preview["value"]["patches"][0]["controlPoints"][0][0][0] = json!(999.);
    request["twist"]["values"] = json!([0, 90]);
    let result = send("brep_sweep_stream_next", stream);
    assert_eq!(result["done"], true);
    for key in ["model", "approximation", "boundaryCertificate", "volume"] {
        assert_eq!(result["value"][key], sync[key], "{key}");
    }
    for owner in [&sync["_nativeOwner"], &result["value"]["_nativeOwner"]] {
        send("brep_sweep_release_owner", json!({"owner":owner}));
    }
}

#[test]
fn scalar_adapters_match_native_construction() {
    let request = raw();
    let path = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,1],"periodic":false});
    let scale = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[1,0,0],[1,0,0]],"weights":[1,1],"periodic":false});
    for operation in ["surface_scaled_sweep", "surface_profile_sweep"] {
        let authored = json!({"operation":operation,"profile":request["profiles"][0],"path":path,"scale":law(1.),"origin":[0,0,0],"normal":[1,0,0],"sections":5,"maxDeviation":0.01});
        let mut migrated = authored.clone();
        migrated["op"] = json!("brep_sweep_constructor");
        let actual = dispatch(migrated).unwrap();
        let mut original = authored;
        original["op"] = json!(operation);
        original["scale"] = scale.clone();
        original["max_deviation"] = json!(0.01);
        let baseline = dispatch(original).unwrap();
        assert_eq!(actual, baseline);
    }
}

#[test]
fn authored_profile_constructor_publishes_native_premises_without_promoting_scope() {
    let mut request = raw();
    request["operation"] = json!("surface_progressive_sweep_profiles");
    request["path"] = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,1],"periodic":false});
    request["options"]["orientation"] = json!("authored");
    request["options"]["frameAxis"] =
        json!({"degree":1,"knots":[0,0,1,1],"values":[[1,0,0],[1,0,0]],"weights":[1,1]});
    request["options"]["frameNormal"] =
        json!({"degree":1,"knots":[0,0,1,1],"values":[[0,1,0],[0,1,0]],"weights":[1,1]});
    request["options"]["initialSections"] = json!(2);
    request["options"]["maxSections"] = json!(2);
    let result = send("brep_sweep_constructor", request.clone());
    assert_eq!(result["report"]["accepted"], true);
    assert_eq!(
        result["report"]["sourceFrameSmoothness"]["sourceFrameSmoothnessCertified"],
        true
    );
    assert_eq!(
        result["report"]["sourceFrameSmoothness"]["continuousBound"],
        false
    );
    assert_eq!(
        result["report"]["authoredFrameRegularity"]["regularityCertified"],
        true
    );
    assert_eq!(
        result["report"]["authoredFrameRegularity"]["continuousBound"],
        false
    );
    for level in result["levels"].as_array().unwrap() {
        assert_eq!(
            level["authoredFrameRegularity"],
            result["report"]["authoredFrameRegularity"]
        );
    }
    let mut stream_request = request.clone();
    stream_request["kind"] = json!("raw-profile");
    let stream = send("brep_sweep_stream_start", stream_request);
    let mut first = send("brep_sweep_stream_next", stream.clone());
    assert_eq!(first["done"], false);
    assert_eq!(
        first["value"]["report"]["authoredFrameRegularity"]["regularityCertified"],
        true
    );
    first["value"]["report"]["authoredFrameRegularity"]["regularityCertified"] = json!(false);
    let final_result = send("brep_sweep_stream_next", stream);
    assert_eq!(final_result["done"], true);
    assert_eq!(
        final_result["value"]["report"]["authoredFrameRegularity"]["regularityCertified"],
        true
    );
    request["options"]["frameRegularityMaxCells"] = json!(0);
    let exhausted = send("brep_sweep_constructor", request);
    assert_eq!(
        exhausted["report"]["authoredFrameRegularity"]["regularityCertified"],
        false
    );
    assert_eq!(
        exhausted["report"]["accepted"],
        result["report"]["accepted"]
    );
}

#[test]
fn miter_correction_uses_authored_axis_and_retains_budget_refusal() {
    let curve = |end: bool| {
        let z = if end {
            10. + 0.25 * 1.25_f64.sqrt()
        } else {
            0.25
        };
        let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.], [0., 1.]];
        json!({"degree":2,"knots":[0,1,2,3,4,5,6,7,8],
            "controlPoints":xy.map(|[x,y]| [x,y,z-if end {0.5*y} else {0.}]),
            "weights":[1,1,1,1,1,1],"periodic":true})
    };
    let request = json!({"sections":[[curve(false)],[curve(true)]],"points":[[0,0,0],[0,0,10]],
        "options":{"frameAxis":{"degree":1,"knots":[2,2,5,5],"controlPoints":[[0,0,1],[0,0.5,1]],"weights":[1,1],"periodic":false},
            "capCorrection":{"quantum":2_f64.powi(-40),"tolerance":1e-9,"maxWork":1000000,"authoredFrame":true}}});
    let result = send("brep_miter_correct_sections", request.clone());
    let mut authored_law = request.clone();
    authored_law["options"]["frameAxis"] = json!({"degree":1,"knots":[2,2,5,5],
        "values":[[0,0,1],[0,0.5,1]],"weights":[1,1]});
    assert_eq!(
        send("brep_miter_correct_sections", authored_law.clone()),
        result
    );
    authored_law["options"]["capCorrection"] = Value::Null;
    assert_eq!(
        send("brep_miter_correct_sections", authored_law),
        Value::Null
    );
    assert_eq!(result["exactPlanarSections"], json!([0, 1]));
    assert!(result["wallDisplacementUpper"].as_f64().unwrap() < 1e-9);
    let mut exhausted = request.clone();
    exhausted["op"] = json!("brep_miter_correct_sections");
    exhausted["options"]["capCorrection"]["maxWork"] = json!(0);
    assert!(
        dispatch(exhausted)
            .unwrap_err()
            .to_string()
            .contains("authored cap correction unproved: work-limit")
    );
    let mut path_axis = request;
    path_axis["op"] = json!("brep_miter_correct_sections");
    path_axis["options"]["capCorrection"]["authoredFrame"] = json!(false);
    assert!(dispatch(path_axis).is_err());
}

#[test]
fn cartesian_tangent_loft_reaches_native_transport() {
    let a = send(
        "curve_bezier",
        json!({"points":[[0,0,0],[20,0,0]],"weights":[1,2]}),
    );
    let b = send(
        "curve_bezier",
        json!({"points":[[0,0,40],[20,0,40]],"weights":[3,1]}),
    );
    let guide = send(
        "curve_bezier",
        json!({"points":[[0,0,0],[0,0,40]],"weights":[1,2]}),
    );
    let built = send(
        "surface_guided_loft_cartesian",
        json!({"curves":[a,b],"parameters":[2,7],"guides":[guide],
        "guide_parameters":[0],"start_tangents":[[0,0,16],[0,0,24]],"end_tangents":[[0,0,4],[0,0,8]],
        "errorBudget":1e-6,"maxCells":50000,"maxMapEvaluations":200000}),
    );
    assert_eq!(
        built["certificate"]["tangents"][0]["targetUnits"],
        "authored-dP/dt"
    );
}

#[test]
fn progressive_miter_shares_owned_automatic_cap_correction() {
    let fixture: Value = value_codec::from_str(include_str!(
        "../src/fixtures/periodic-automatic-cap-shear-request.json"
    ))
    .unwrap();
    let source = &fixture["source"];
    let to_law = |curve: &Value, scalar: bool| {
        let values = if scalar {
            json!(
                curve["controlPoints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p[0].clone())
                    .collect::<Vec<_>>()
            )
        } else {
            curve["controlPoints"].clone()
        };
        json!({"degree":curve["degree"],"knots":curve["knots"],"weights":curve["weights"],"values":values})
    };
    let request = json!({"kind":"progressive-miter", "loops":source["loops"], "points":source["points"],
        "scale":to_law(&source["scale"],true), "twist":to_law(&source["twist"],true),
        "options":{"normal":source["normal"],"closed":source["closed"],"miterLimit":source["miterLimit"],
            "initialSteps":source["initialSteps"],"maxSteps":source["maxSteps"],"maxDeviation":source["maxDeviation"],
            "axisScale":to_law(&source["axisScale"],false),"centerLaw":to_law(&source["centerLaw"],false),
            "frameAxis":to_law(&source["frameAxis"],false),"frameNormal":to_law(&source["frameNormal"],false),
            "orientationGuide":source["orientationGuide"]}});
    let mut exhausted = request.clone();
    exhausted["options"]["retainedWallMaxInjectivityCells"] = json!(0);
    exhausted["op"] = json!("brep_progressive_miter_body");
    assert!(
        dispatch(exhausted)
            .unwrap_err()
            .message
            .contains("regularity unproved")
    );
    let body = send("brep_progressive_miter_body", request);
    assert_eq!(
        body["sectionCorrection"]["reason"],
        "automatic-bounded-cap-planarity"
    );
    assert_eq!(body["boundaryCertificate"]["continuousBound"], true);
    assert_eq!(body["boundaryCertificate"]["withinBudget"], true);
}

#[test]
fn anisotropic_hollow_profile_reaches_native_solid_admission() {
    let outer = send("curve_circle", json!({"center":[0,0,0],"normal":[0,0,1],"radius":3}));
    let hole = send("curve_circle", json!({"center":[0,0,0],"normal":[0,0,-1],"radius":1}));
    let request = json!({"loops":[[outer],[hole]],
        "path":{"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,1],"periodic":false},
        "scale":{"degree":1,"knots":[0,0,1,1],"values":[1,2],"weights":[1,1]},
        "twist":{"degree":1,"knots":[0,0,1,1],"values":[0,5],"weights":[1,1]},
        "options":{"normal":[1,0,0],"initialSections":3,"maxSections":257,"maxDeviation":0.001,
            "axisScale":{"degree":1,"knots":[0,0,1,1],"values":[[1,1,1],[2,1,1]],"weights":[1,1]},
            "centerLaw":{"degree":1,"knots":[0,0,1,1],"values":[[0,0,0],[0.5,0,0]],"weights":[1,1]}}});
    let body = send("brep_progressive_profile_body", request);
    if let Ok(path) = std::env::var("OSV_SWEEP_DIAGNOSTIC_MODEL") {
        std::fs::write(path, body["model"].to_string()).unwrap();
    }
    let mut summary = body["volume"].clone();
    summary.as_object_mut().unwrap().remove("disjointGroups");
    assert_eq!(body["volume"]["solidGeometryCertified"],true,"native constructor volume: {}",summary);
    let model = body["model"].clone();
    let document = json!({"nodes":[{"id":"body","op":"brep_progressive_sweep"}]});
    let report = send("brep_sweep_solid_admission", json!({"documentJson":document.to_string(),"nodeId":"body","model":model}));
    assert_eq!(report["solidGeometryCertified"],true);
    assert_eq!(report["boundaryEmbeddingCertified"],true);
    let mut damaged = model;
    damaged["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(999.);
    assert!(dispatch(json!({"op":"brep_sweep_solid_admission","documentJson":document.to_string(),"nodeId":"body","model":damaged})).is_err());
}

#[test]
fn source_law_payload_is_native_and_keeps_audit_options_separate() {
    let mut request = raw();
    request["kind"] = json!("source");
    request["path"] = json!({"degree":1,"knots":[2,2,7,7],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,2],"periodic":false});
    request["twist"]["values"] = json!([0,90]);
    request["options"]["errorMaxCells"] = json!(0);
    request["options"]["rmfTransportSteps"] = json!(0);
    let before = request.clone();
    let payload = send("brep_sweep_law_payload",request.clone());
    assert_eq!(payload["path"],before["path"]);
    assert_eq!(payload["twist"]["controlPoints"][1][0],std::f64::consts::FRAC_PI_2);
    assert_eq!(payload["twist"]["weights"],before["twist"]["weights"]);
    assert_eq!(payload["scale"]["weights"],before["scale"]["weights"]);
    assert!(payload.get("rmf_transport_steps").is_none());
    assert!(payload.get("error_max_cells").is_none());
    assert_eq!(before,request);
    request["options"]["orientation"] = json!("authored");
    assert!(dispatch({request["op"] = json!("brep_sweep_law_payload");request}).is_err());
}
