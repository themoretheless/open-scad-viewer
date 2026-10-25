//! Additional native JSON/STEP cases. This is not an external STEP verifier.
use value_codec::{Value, json};
fn certified(r: &Value, stage: &str) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "continuousBound",
        "topologyPreserved",
        "shellRolesCertified",
        "fullBoundaryG2Certified",
        "idealSourceBoundaryG2Certified",
    ] {
        assert_eq!(
            r[key], true,
            "{stage}/{key}: reason={} original={} binding={}",
            r["reason"], r["original"]["reason"], r["retainedBinding"]
        );
    }
}
fn main() {
    let path = std::env::args().nth(1).expect("request JSON");
    let request: Value = value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let source = geometry_bridge::dispatch(request.clone()).unwrap();
    certified(&source, "construct");
    let exported = geometry_bridge::dispatch(
        json!({"op":"brep_nurbs_export_step_v9","model":source["model"]}),
    )
    .unwrap();
    assert_eq!(exported["certificate"]["complete"], true);
    let imported = geometry_bridge::dispatch(
        json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]}),
    )
    .unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    let mut counts = json!({});
    for key in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
        let n = source["model"][key].as_array().unwrap().len();
        assert_eq!(imported["model"][key].as_array().unwrap().len(), n, "{key}");
        counts[key] = json!(n);
    }
    let mut request = request;
    request["op"] = json!("brep_sweep_smooth_closed_material_audit");
    request["model"] = imported["model"].clone();
    let audited = geometry_bridge::dispatch(request).unwrap();
    certified(&audited, "import-audit");
    println!("{}",value_codec::to_string(&json!({"nativeStepRoundtripPassed":true,"counts":counts,
        "originalBoundaryG2":audited["idealSourceBoundaryG2Certified"],"retainedBoundaryG2":audited["fullBoundaryG2Certified"],
        "boundaryErrorUpper":audited["boundaryErrorUpper"],"originalVolume":audited["original"]["volume"],
        "retainedVolume":audited["retainedVolume"],"sourceSmoothness":audited["original"]["smoothness"],
        "retainedBinding":audited["retainedBinding"],"orientation":audited["orientation"]})).unwrap());
}
