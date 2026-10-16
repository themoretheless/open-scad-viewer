//! Native STEP re-import is independently audited against the original source.
//! This does not claim independent external STEP-engine qualification.
use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[path = "support/rational_cap_flux.rs"]
mod flux;
#[path = "support/rational_closed_material.rs"]
mod support;

fn roundtrip(mut input: Value, name: &str) {
    let reference = flux::reference(&input);
    let built = dispatch(input.clone()).unwrap();
    support::full(&built, "construct");
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":built["model"]})).unwrap();
    assert_eq!(
        exported["certificate"]["complete"], true,
        "{name}: {exported:?}"
    );
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(
        imported["certificate"]["complete"], true,
        "{name}: {imported:?}"
    );
    for key in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
        assert_eq!(
            built["model"][key].as_array().unwrap().len(),
            imported["model"][key].as_array().unwrap().len(),
            "{name}/{key}"
        );
    }
    let model: brep_core::Model = value_codec::from_value(imported["model"].clone()).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    input["op"] = json!("brep_rational_closed_material_audit");
    input["model"] = imported["model"].clone();
    input["limits"]["constructionRows"] = json!(0);
    let audited = dispatch(input.clone()).unwrap();
    support::full(&audited, "import-audit");
    assert_eq!(audited["sourceWork"]["constructionRows"], 0);
    assert_eq!(audited["model"], imported["model"]);
    if let Ok(dir) = std::env::var("OSV_RATIONAL_CAP_STEP_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(format!("{name}.reference.json")),
            value_codec::to_string_pretty(&reference).unwrap(),
        )
        .unwrap();
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(format!("{name}.step")),
            exported["text"].as_str().unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{name}.audit.json")),
            value_codec::to_string_pretty(&audited).unwrap(),
        )
        .unwrap();
        input["model"] = Value::Null;
        input["op"] = json!("brep_rational_closed_material_construct");
        std::fs::write(
            dir.join(format!("{name}.source.json")),
            value_codec::to_string_pretty(&input).unwrap(),
        )
        .unwrap();
    }
    eprintln!(
        "{name}: original/actual global volume, physical cap/wall/closure G2, exact STEP audit; faces={} caps={}",
        model.faces.len(),
        audited["physicalCapCount"]
    );
}
fn rational_caps(v: &mut Value) {
    for span in [2, 6] {
        v["source"]["profiles"][0][span]["weights"][3] = json!(2.);
        v["material"]["trims"][0][span]["weights"][3] = json!(2.);
    }
}
#[test]
fn planar_caps() {
    roundtrip(support::request(false, false), "planar-caps");
}
#[test]
fn nonplanar_caps() {
    roundtrip(support::request(true, false), "nonplanar-caps");
}
#[test]
fn moving_affine_caps() {
    roundtrip(support::request(true, true), "moving-affine-caps");
}
#[test]
fn reflected_caps() {
    let mut v = support::request(true, true);
    for p in v["source"]["motion"].as_array_mut().unwrap() {
        for c in p["controls"].as_array_mut().unwrap() {
            for col in c.as_array_mut().unwrap() {
                col[0] = json!(-col[0].as_f64().unwrap());
            }
        }
    }
    roundtrip(v, "reflected-caps");
}
#[test]
fn rational_profile_caps() {
    let mut v = support::request(true, false);
    rational_caps(&mut v);
    roundtrip(v, "rational-profile-caps");
}
#[test]
fn rational_profile_affine_caps() {
    let mut v = support::request(true, true);
    rational_caps(&mut v);
    roundtrip(v, "rational-profile-affine-caps");
}
