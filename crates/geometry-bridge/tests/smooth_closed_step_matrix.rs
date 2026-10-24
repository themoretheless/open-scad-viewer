//! Fresh native STEP export/import/audit for the original and actual closed body.
//! This matrix is not an independent external STEP-engine qualification.
use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn full(r: &Value, stage: &str) {
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
    assert_eq!(r["physicalCapCount"], 0);
}
fn roundtrip(raw: &str, name: &str) {
    let request: Value = value_codec::from_str(raw).unwrap();
    let source = dispatch(request.clone()).unwrap();
    full(&source, "construct");
    let exported =
        dispatch(json!({"op":"brep_nurbs_export_step_v9","model":source["model"]})).unwrap();
    assert_eq!(exported["certificate"]["complete"], true);
    let imported =
        dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    for key in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
        assert_eq!(
            source["model"][key].as_array().unwrap().len(),
            imported["model"][key].as_array().unwrap().len(),
            "{name}/{key}"
        );
    }
    let model: brep_core::Model = value_codec::from_value(imported["model"].clone()).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let mut request = request;
    request["op"] = json!("brep_sweep_smooth_closed_material_audit");
    request["model"] = imported["model"].clone();
    let audited = dispatch(request).unwrap();
    full(&audited, "import-audit");
    assert_eq!(source["boundaryErrorUpper"], audited["boundaryErrorUpper"]);
    eprintln!(
        "{name}: original/actual G2 STEP audit; faces={} shells={} source joins={} binding work={}",
        model.faces.len(),
        model.shells.len(),
        audited["original"]["smoothness"]["inspectedProfileJoins"],
        audited["retainedBinding"]["exactWork"]
    );
}
macro_rules! case {
    ($name:ident,$file:literal) => {
        #[test]
        fn $name() {
            roundtrip(
                include_str!(concat!(
                    "fixtures/smooth-closed-step-matrix/",
                    $file,
                    ".json"
                )),
                $file,
            );
        }
    };
}
case!(rmf, "rmf");
case!(authored, "authored");
case!(guided, "guided");
case!(hole, "hole");
case!(reversed_hole, "reversed-hole");
case!(nonplanar, "nonplanar");
case!(forty_intervals, "forty-intervals");
