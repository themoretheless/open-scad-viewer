//! Original-source certificate must be reapplied to the imported model itself.
use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[path = "support/fibered_source_reference.rs"]
mod reference;
#[path = "support/fibered_body.rs"]
mod support;
fn roundtrip(mut input: Value, name: &str) {
    let source = input.clone();
    let reference = reference::reference(&source);
    let built = dispatch(input.clone()).unwrap();
    support::full(&built);
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":built["model"]})).unwrap();
    assert_eq!(exported["certificate"]["complete"], true, "{exported}");
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(imported["certificate"]["complete"], true, "{imported}");
    input["op"] = json!("brep_fibered_source_audit");
    input["model"] = imported["model"].clone();
    let audited = dispatch(input).unwrap();
    if let Ok(dir) = std::env::var("OSV_FIBERED_STEP_DIR") {
        let dir = format!("{dir}/{name}");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, v) in [
            ("built", &built),
            ("imported", &imported),
            ("audited", &audited),
        ] {
            std::fs::write(
                format!("{dir}/{name}.json"),
                value_codec::to_string_pretty(v).unwrap(),
            )
            .unwrap();
        }
        std::fs::write(
            format!("{dir}/source.json"),
            value_codec::to_string_pretty(&source).unwrap(),
        )
        .unwrap();
        std::fs::write(
            format!("{dir}/reference.json"),
            value_codec::to_string_pretty(&reference).unwrap(),
        )
        .unwrap();
        std::fs::write(
            format!("{dir}/body.step"),
            exported["text"].as_str().unwrap(),
        )
        .unwrap();
    }
    support::full(&audited);
    assert_eq!(audited["model"], imported["model"]);
}

#[test]
fn exact_original_fibers_and_physical_caps_survive_step_reaudit() {
    roundtrip(support::request(), "basic");
}
#[test]
fn nonbinary_rational_oblique_frames_in_both_directions_survive_step_reaudit() {
    roundtrip(support::oblique_request(false), "oblique-ascending");
    roundtrip(support::oblique_request(true), "oblique-descending");
}
#[test]
fn all_three_original_motion_pieces_survive_step_reaudit() {
    roundtrip(support::three_piece_request(), "three-pieces");
}
#[test]
fn imported_source_binding_rejects_one_ulp_changes_and_short_gauge_budget() {
    let mut input = support::request();
    let built = dispatch(input.clone()).unwrap();
    support::full(&built);
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":built["model"]})).unwrap();
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    input["op"] = json!("brep_fibered_source_audit");
    input["model"] = imported["model"].clone();
    let audited = dispatch(input.clone()).unwrap();
    support::full(&audited);
    let mut short = input.clone();
    short["limits"]["bindingRows"] = json!(audited["work"]["bindingRows"].as_u64().unwrap() - 1);
    let r = dispatch(short).unwrap();
    assert_eq!(r["certified"], false);
    assert!(r["model"].is_null());
    for (entity, index, field) in [("faces", 4, "surface"), ("edges", 0, "curve")] {
        let mut wrong = input.clone();
        let g = &mut wrong["model"][entity][index][field];
        if entity == "faces" {
            g["controlPoints"][0][0][0] = json!(2_f64.next_up());
        } else {
            g["weights"][1] = json!(1_f64.next_up());
        }
        wrong["certified"] = json!(true);
        wrong["fullBoundaryG2Certified"] = json!(true);
        let before = wrong["model"].clone();
        let r = dispatch(wrong.clone()).unwrap();
        assert_eq!(r["certified"], false);
        assert_eq!(r["fullBoundaryG2Certified"], false);
        assert!(r["model"].is_null());
        assert_eq!(wrong["model"], before);
    }
}
