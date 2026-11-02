//! Same original Rust fixture through JSON and native STEP, with fresh owners.
use geometry_bridge::dispatch;
use nurbs_core as nurbs;
use value_codec::{Value, json};
#[path = "../../nurbs-core/tests/support/arc_bishop_source.rs"]
mod source_case;
fn request() -> Value {
    let mut v: Value =
        value_codec::from_str(include_str!("fixtures/smooth-closed-material-v1.json")).unwrap();
    let (profiles, surface, trims) = source_case::material();
    v["source"]["profiles"] = value_codec::to_value(profiles).unwrap();
    v["source"]["path"] = value_codec::to_value(source_case::path(true)).unwrap();
    v["material"] = json!({"surface":surface,"trims":trims});
    v["source"]["spacing"] = json!("arc_length");
    v["source"]["length_tolerance"] = json!(0.001);
    v["source"]["length_max_cells"] = json!(100000);
    v["source"]["rmf_transport_steps"] = json!(4096);
    v["sections"] = json!(65);
    v["idealBody"]["maxCells"] = json!(1000000);
    v["originalSmoothness"] =
        json!({"order":2,"maxCells":100000,"maxExactWork":1000000,"maxProducts":1000000});
    v
}
fn full(r: &Value) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "continuousBound",
        "jointContinuousGlobalCertified",
        "jointContinuousGlobalG1Certified",
        "jointContinuousGlobalG2Certified",
        "withinBudget",
        "topologyPreserved",
        "shellRolesCertified",
        "fullBoundaryG1Certified",
        "fullBoundaryG2Certified",
        "idealSourceBoundaryG1Certified",
        "idealSourceBoundaryG2Certified",
    ] {
        assert_eq!(
            r[key], true,
            "{key}: reason={} original={}",
            r["reason"], r["original"]["reason"]
        );
    }
    assert_eq!(r["seamContinuity"], "G2");
    assert_eq!(r["physicalCapCount"], 0);
    for p in [&r["original"]["volume"], &r["retainedVolume"]] {
        for key in [
            "embeddingCertified",
            "volumeRegularityCertified",
            "parameterTopologyPreserved",
        ] {
            assert_eq!(p[key], true, "{key}: {p:?}");
        }
    }
    assert_eq!(r["retainedBinding"]["inspectedFaces"], 256);
    assert!(r["boundaryErrorUpper"].as_f64().unwrap() <= 0.05);
}
#[test]
fn original_arc_clock_g2_body_json_and_step_recompute_full_certificates() {
    let request = request();
    let original = request.clone();
    let result = dispatch(request.clone()).unwrap();
    full(&result);
    assert_eq!(request, original);
    let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let (text, exported, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(exported.complete);
    let (restored, imported, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(imported.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    for key in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
        assert_eq!(
            result["model"][key].as_array().unwrap().len(),
            value_codec::to_value(&restored).unwrap()[key]
                .as_array()
                .unwrap()
                .len(),
            "{key}"
        );
    }
    let mut audit = request.clone();
    audit["op"] = json!("brep_sweep_smooth_closed_material_audit");
    audit["model"] = value_codec::to_value(restored).unwrap();
    let audited = dispatch(audit).unwrap();
    full(&audited);
    assert_eq!(result["boundaryErrorUpper"], audited["boundaryErrorUpper"]);
    for parameter in [false, true] {
        let mut refused = request.clone();
        refused["model"] = result["model"].clone();
        refused["certified"] = json!(true);
        refused["jointContinuousGlobalCertified"]=json!(true);
        refused["jointContinuousGlobalG2Certified"]=json!(true);
        refused["idealSourceBoundaryG2Certified"] = json!(true);
        if parameter {
            refused["source"]["spacing"] = json!("parameter");
        } else {
            refused["idealBody"]["maxCells"] = json!(0);
        }
        let r = dispatch(refused).unwrap();
        assert_eq!(r["certified"], false);
        assert_eq!(r["jointContinuousGlobalCertified"],false);
        assert_eq!(r["jointContinuousGlobalG2Certified"],false);
        assert_eq!(r["solidGeometryCertified"], false);
        assert!(r["model"].is_null());
    }
    if let Ok(directory) = std::env::var("OSV_ARC_BISHOP_STEP_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("request.json"),
            value_codec::to_string(&request).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("body.json"),
            value_codec::to_string(&result).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("step-audit.json"),
            value_codec::to_string(&audited).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.join("body.step"), text).unwrap();
    }
    eprintln!(
        "same-source geometric-clock original/actual G2 JSON/STEP:256 faces, E={}",
        result["boundaryErrorUpper"]
    );
}
