use geometry_bridge::dispatch;
use value_codec::{Value, json};

fn request(op: &str) -> Value {
    let mesh = polygon_core::solid::primitives::cube([10., 10., 10.], false).unwrap();
    json!({"op": op, "mesh": mesh, "zMin": 0., "zMax": 1., "layerHeightMm": 0.2})
}

#[test]
fn export_is_verified_and_preview_is_independently_readable() {
    let result = dispatch(request("mesh_gcode")).unwrap();
    assert_eq!(result["layerCount"], 5);
    assert_eq!(result["preview"]["layers"], 5);
    let preview = dispatch(json!({"op": "gcode_preview", "gcode": result["gcode"]})).unwrap();
    assert_eq!(preview, result["preview"]);
    assert!(preview["extrusionMm"].as_f64().unwrap() > 0.);
    assert!(preview["depositedVolumeMm3"].as_f64().unwrap() > 0.);
    assert!(preview["estimatedTimeS"].as_f64().unwrap() > 0.);
    assert_eq!(preview["bounds"]["max"][2], 0.8);
    let moves = preview["moves"].as_array().unwrap();
    assert!(moves.iter().any(|m| m["extruded"] == json!(true)));
    assert!(moves.iter().any(|m| m["layerIndex"] == 4));
}

#[test]
fn invalid_explicit_options_never_fall_back_to_defaults() {
    for op in ["mesh_toolpaths", "mesh_gcode"] {
        for key in [
            "layerHeightMm",
            "lineWidthMm",
            "wallCount",
            "infillSpacingMm",
            "feedrateMmS",
            "travelFeedrateMmS",
            "filamentDiameterMm",
        ] {
            for value in [Value::Null, json!("0.2"), json!(false), json!(-1), json!(0)] {
                let mut req = request(op);
                req[key] = value;
                assert_eq!(
                    dispatch(req).unwrap_err().code,
                    "GEOMETRY_INVALID_INPUT",
                    "{op}/{key}"
                );
            }
        }
        for value in [json!(1.5), json!(9), json!(4294967297u64)] {
            let mut req = request(op);
            req["wallCount"] = value;
            assert_eq!(dispatch(req).unwrap_err().code, "GEOMETRY_INVALID_INPUT");
        }
    }
}

#[test]
fn no_downloadable_job_is_returned_for_an_empty_range() {
    let mut req = request("mesh_gcode");
    req["zMin"] = json!(20.);
    req["zMax"] = json!(21.);
    assert_eq!(dispatch(req).unwrap_err().code, "GCODE_EMPTY_PLAN");
}

#[test]
fn preview_rejects_unsupported_modes_and_bad_inputs() {
    let result = dispatch(request("mesh_gcode")).unwrap();
    for command in ["G91", "M83", "G2 X1 Y1 I1", "G1 XNaN", "G1 ☃1"] {
        let text = format!("{}\n{command}\n", result["gcode"].as_str().unwrap());
        assert!(
            dispatch(json!({"op": "gcode_preview", "gcode": text})).is_err(),
            "{command}"
        );
    }
    assert!(dispatch(json!({"op": "gcode_preview", "gcode": 123})).is_err());
}
