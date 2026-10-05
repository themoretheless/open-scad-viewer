use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn send(op: &str, mut request: Value) -> Value {
    request["op"] = json!(op);
    dispatch(request).unwrap()
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
fn legacy_scalar_adapters_preserve_explicit_native_refusal() {
    let request = raw();
    let path = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[0,0,0],[0,0,10]],"weights":[1,1],"periodic":false});
    let scale = json!({"degree":1,"knots":[0,0,1,1],"controlPoints":[[1,0,0],[1,0,0]],"weights":[1,1],"periodic":false});
    for operation in ["surface_scaled_sweep", "surface_profile_sweep"] {
        let authored = json!({"operation":operation,"profile":request["profiles"][0],"path":path,"scale":law(1.),"origin":[0,0,0],"normal":[1,0,0],"sections":5,"maxDeviation":0.01});
        // These legacy operations are not implemented by the existing kernel.
        // Transport must preserve rejection, without inventing a geometry fallback.
        let mut migrated = authored.clone();
        migrated["op"] = json!("brep_sweep_constructor");
        let actual = dispatch(migrated).unwrap_err();
        let mut original = authored;
        original["op"] = json!(operation);
        original["scale"] = scale.clone();
        original["max_deviation"] = json!(0.01);
        let baseline = dispatch(original).unwrap_err();
        assert_eq!(actual.code, baseline.code);
        assert_eq!(actual.message, baseline.message);
    }
}
