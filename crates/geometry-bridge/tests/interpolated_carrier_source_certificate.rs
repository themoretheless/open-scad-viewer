//! Distinct original rational carriers through Rust construction and STEP.
use geometry_bridge::dispatch;
use nurbs_core as nurbs;
use value_codec::{Value, json};
#[path = "../../nurbs-core/tests/support/interpolated_carrier_source.rs"]
mod source;
fn request() -> Value {
    let mut v: Value = value_codec::from_str(include_str!(
        "fixtures/general-frame-cap-certificate-v1.json"
    ))
    .unwrap();
    let mut v = v["sweep"].take();
    let caps = source::caps();
    let source_caps: Vec<_> = caps
        .iter()
        .enumerate()
        .map(|(i, s)| json!({"surface":s,"trims":source::trims(s,if i==0 {1.}else{2.})}))
        .collect();
    let retained: Vec<_> = caps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut r = s.clone();
            let scale = if i == 0 { 1. } else { 2. };
            let center = if i == 0 {
                [0.25, 0.125, 0.]
            } else {
                [0.5, -0.25, 2.25]
            };
            for p in r.control_points.iter_mut().flatten() {
                p[0] = 1.5 * scale * p[0] + center[0];
                p[1] = 2. * scale * p[1] + center[1];
                p[2] += center[2];
            }
            json!({"surface":r,"trims":source::trims(s,if i==0 {1.}else{2.})})
        })
        .collect();
    let (path, scale, axes, center, axis, normal, twist) = source::laws();
    v["source"]["profiles"] = value_codec::to_value(source::profiles()).unwrap();
    v["source"]["path"] = value_codec::to_value(path).unwrap();
    v["source"]["scale"] = value_codec::to_value(scale).unwrap();
    v["source"]["axis_scale"] = value_codec::to_value(axes).unwrap();
    v["source"]["center_law"] = value_codec::to_value(center).unwrap();
    v["source"]["frame_axis"] = value_codec::to_value(axis).unwrap();
    v["source"]["frame_normal"] = value_codec::to_value(normal).unwrap();
    v["source"]["twist"] = value_codec::to_value(twist).unwrap();
    v["source"]["spacing"] = json!("arc_length");
    v["source"]["length_tolerance"] = json!(0.001);
    v["source"]["length_max_cells"] = json!(100000);
    v["sourceCaps"] = json!(source_caps);
    v["retainedCaps"] = json!(retained);
    v["idealBody"] = json!({"method":"atlas","projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"maxCells":10000,"maxExactWork":1000000});
    v
}
#[test]
fn original_carrier_homotopy_owns_solid_topology_on_distinct_uv_domains_and_step() {
    let request = request();
    let original = request.clone();
    let built = dispatch(request.clone()).unwrap();
    for k in [
        "certified",
        "continuousBound",
        "solidGeometryCertified",
        "idealBodyEmbeddingCertified",
    ] {
        assert_eq!(built[k], true, "{k}: {built:?}");
    }
    for k in ["volumeRegularityCertified", "parameterTopologyPreserved"] {
        assert_eq!(built["idealBody"][k], true);
    }
    assert_eq!(built["seamContinuity"], "C0");
    assert_eq!(built["smoothnessCertified"], false);
    assert_eq!(request, original);
    let model: brep_core::Model = value_codec::from_value(built["model"].clone()).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let (step, exported, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(exported.complete);
    let (restored, imported, _) = cad_step::import_step_v9(&step).unwrap();
    assert!(imported.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    let cells = built["idealBody"]["cells"].as_u64().unwrap();
    let exact = built["idealBody"]["exactWork"].as_u64().unwrap();
    // Use the successful joint ledger. With spare cells, another sound proof
    // route may need less exact work; that is not budget exhaustion.
    let mut tight = request.clone();
    tight["idealBody"]["maxCells"] = json!(cells);
    tight["idealBody"]["maxExactWork"] = json!(exact);
    let tight_result = dispatch(tight.clone()).unwrap();
    assert_eq!(
        tight_result["certified"], true,
        "{}",
        tight_result["idealBody"]
    );
    for (field, limit) in [
        ("maxCells", 0),
        ("maxCells", cells - 1),
        ("maxExactWork", 0),
        ("maxExactWork", exact - 1),
    ] {
        let mut short = tight.clone();
        short["idealBody"][field] = json!(limit);
        short["model"] = built["model"].clone();
        short["idealBodyEmbeddingCertified"] = json!(true);
        let r = dispatch(short).unwrap();
        assert_eq!(
            r["certified"], false,
            "{field}/{limit}: reason={} ideal={}",
            r["reason"], r["idealBody"]
        );
        assert_eq!(r["idealBodyEmbeddingCertified"], false);
        assert!(r["model"].is_null());
    }
    if let Ok(dir) = std::env::var("OSV_INTERPOLATED_CARRIER_STEP_DIR") {
        let d = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("request.json"),
            value_codec::to_string(&request).unwrap(),
        )
        .unwrap();
        std::fs::write(d.join("body.json"), value_codec::to_string(&built).unwrap()).unwrap();
        std::fs::write(
            d.join("restored-model.json"),
            value_codec::to_string(&restored).unwrap(),
        )
        .unwrap();
        std::fs::write(d.join("body.step"), step).unwrap();
    }
    eprintln!(
        "interpolated original carriers/Solid/STEP: {} faces, source={}",
        model.faces.len(),
        built["idealBody"]
    );
}

#[test]
fn explicit_aggregate_atlas_limit_does_not_widen_primitive_or_invalid_input_limits() {
    let mut v=request();v["idealBody"]["maxCells"]=json!(1000000);
    let result=dispatch(v.clone()).unwrap();assert_eq!(result["certified"],true);
    assert!(result["idealBody"]["cells"].as_u64().unwrap()<=1000000);
    v["idealBody"]["maxCells"]=json!(1000001);assert!(dispatch(v).is_err());
}
