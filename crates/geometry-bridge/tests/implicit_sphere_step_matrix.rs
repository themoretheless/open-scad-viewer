//! Native STEP preserves the original implicit sphere proof, including poles.
use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[path = "support/implicit_sphere.rs"]
mod support;
fn roundtrip(mut input: Value, name: &str) {
    let source = input.clone();
    let built = dispatch(input.clone()).unwrap();
    support::full(&built, "construct");
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":built["model"]})).unwrap();
    assert_eq!(exported["certificate"]["complete"], true);
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    input["op"] = json!("brep_implicit_sphere_source_audit");
    input["model"] = imported["model"].clone();
    let audited = dispatch(input).unwrap();
    support::full(&audited, "STEP audit");
    assert_eq!(audited["model"], imported["model"]);
    for key in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
        assert_eq!(
            built["model"][key].as_array().unwrap().len(),
            imported["model"][key].as_array().unwrap().len(),
            "{name}/{key}"
        );
    }
    if let Ok(dir) = std::env::var("OSV_IMPLICIT_SPHERE_STEP_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(format!("{name}.step")),
            exported["text"].as_str().unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{name}.source.json")),
            value_codec::to_string_pretty(&source).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{name}.audit.json")),
            value_codec::to_string_pretty(&audited).unwrap(),
        )
        .unwrap();
    }
    eprintln!(
        "{name}: native original sphere equation, global geometry and pole G2 STEP re-audit passed"
    );
}
#[test]
fn exact_radius_two_poles() {
    roundtrip(support::request(2., [0.; 3]), "sphere-two-poles");
}
#[test]
fn exact_nonbinary_radius_square() {
    roundtrip(
        support::request(3_f64.next_up(), [0.; 3]),
        "sphere-nonbinary-radius",
    );
}
#[test]
fn exact_regular_sphere() {
    roundtrip(support::request(3., [0.; 3]), "sphere-regular");
}
#[test]
fn exact_translated_poles() {
    roundtrip(
        support::request(2., [8., -4., 6.]),
        "sphere-translated-poles",
    );
}

#[test]
fn reflected_sheared_source_shell_retains_its_exact_g2_after_step() {
    let mut input = support::request(2., [0.; 3]);
    let built = dispatch(input.clone()).unwrap();
    support::full(&built, "sphere fixture");
    let linear = [[-2., 0.5, 0.], [0., 1., -0.25], [0., 0., 0.5]];
    let center = [4., -2., 1.];
    let [a, b, c] = linear;
    let matrix = [
        [a[0], a[1], a[2], center[0]],
        [b[0], b[1], b[2], center[1]],
        [c[0], c[1], c[2], center[2]],
        [0., 0., 0., 1.],
    ];
    let model = dispatch(json!({
        "op": "brep_nurbs_transform", "matrix": matrix, "model": built["model"]
    }))
    .unwrap();
    input["op"] = json!("brep_implicit_affine_sphere_source_audit");
    input["source"]["center"] = json!(center);
    input["source"]["linear"] = json!(linear);
    input["limits"]["maxPoints"] = json!(1000);
    input["model"] = model;

    let audited = dispatch(input.clone()).unwrap();
    assert_eq!(audited["certified"], true, "{audited}");
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":audited["model"]})).unwrap();
    assert_eq!(exported["certificate"]["complete"], true);
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    input["model"] = imported["model"].clone();
    let audited = dispatch(input).unwrap();
    assert_eq!(audited["certified"], true, "{audited}");
    assert_eq!(audited["model"], imported["model"]);
    assert_eq!(audited["idealSourceBoundaryG2Certified"], true);
    assert_eq!(audited["topologyPreserved"], true);
    assert_eq!(audited["boundaryErrorUpper"], 0.);
}
