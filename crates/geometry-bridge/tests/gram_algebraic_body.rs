use geometry_bridge::dispatch;
use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::json;
#[path = "../../nurbs-core/tests/support/gram_seam_map.rs"]
mod fixture;

#[test]
fn native_original_gram_seam_keeps_body_guarantees_independent() {
    let (a, b) = fixture::fixture();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],
        "boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,
        "solidCertified":true,"continuousBound":true,"jointContinuousGlobalG2Certified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    for k in [
        "solidCertified",
        "continuousBound",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(full[k], false);
    }
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let (c, d) = fixture::rank_loss();
    let mut bad = request;
    bad["patches"] = json!([c, d]);
    bad["regularityCertified"] = json!(true);
    let refused = dispatch(bad).unwrap();
    assert_eq!(refused["certified"], false);
    assert_eq!(refused["regularityCertified"], false);
}

#[test]
fn quintic_original_profile_body_survives_step_with_sharp_caps() {
    let (a, _) = fixture::fixture();
    let curve = Curve {
        degree: 5,
        knots: a.knots_u.clone(),
        control_points: a
            .control_points
            .iter()
            .map(|row| row[0][..2].to_vec())
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    let chord = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            curve.control_points[5].clone(),
            curve.control_points[0].clone(),
        ],
        weights: vec![1.; 2],
        periodic: false,
    };
    let mut model = brep_core::prism::extrude(&[vec![curve, chord]], 0., 2.).unwrap();
    // Author literal XY cap charts. Dividing by the rectangular control hull
    // would round some pcurve poles (e.g. thirds) and lose exact edge binding.
    // This is a different source UV atlas, with exactly the same physical caps.
    for face in model.faces.len() - 2..model.faces.len() {
        let s = &mut model.faces[face].surface;
        let [a, b] = [
            s.control_points[0][0].clone(),
            s.control_points[1][1].clone(),
        ];
        s.knots_u = vec![a[0], a[0], b[0], b[0]];
        s.knots_v = vec![a[1], a[1], b[1], b[1]];
        let outer = model.faces[face].outer;
        for coedge in &mut model.0.loops[outer].coedges {
            let mut pcurve = model.0.edges[coedge.edge].curve.clone();
            for p in &mut pcurve.control_points {
                p.truncate(2);
            }
            if coedge.reversed {
                pcurve = pcurve.reverse().unwrap();
            }
            coedge.pcurve = pcurve;
        }
    }
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    let (text, exported, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(exported.complete);
    let (restored, imported, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(imported.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    let original_models = (model.clone(), restored.clone());
    let mut common = value_codec::from_str::<value_codec::Value>(include_str!(
        "fixtures/general-frame-cap-certificate-v1.json"
    ))
    .unwrap()["common"]
        .clone();
    common["reference"] = value_codec::to_value(&model).unwrap();
    common["retained"] = value_codec::to_value(&restored).unwrap();
    common["certified"] = json!(true);
    common["jointContinuousGlobalG2Certified"] = json!(true);
    // The source-owned exact chart reversal must bind the original to the
    // imported atlas before the whole-boundary distance owner can run.
    if let Ok(dir) = std::env::var("OSV_GRAM_BODY_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in [
            ("original-diagnostic.json", &model),
            ("imported-diagnostic.json", &restored),
        ] {
            std::fs::write(
                dir.join(name),
                value_codec::to_string(&value_codec::to_value(body).unwrap()).unwrap(),
            )
            .unwrap();
        }
    }
    let transported_boundary = dispatch(common.clone()).unwrap();
    assert_eq!(
        transported_boundary["continuousBound"], true,
        "{transported_boundary:?}"
    );
    assert_eq!(transported_boundary["errorUpper"].as_f64(), Some(0.));
    assert!(
        !transported_boundary["uvTransportFaces"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut short = common.clone();
    short["limits"]["correspondenceProducts"] = json!(0);
    assert_eq!(dispatch(short).unwrap()["continuousBound"], false);
    let mut changed = restored.clone();
    let face = transported_boundary["uvTransportFaces"][0]
        .as_u64()
        .unwrap() as usize;
    let wire = changed.faces[face].outer;
    let lower = changed.faces[face].surface.knots_u[changed.faces[face].surface.degree_u];
    let p = changed.loops[wire]
        .coedges
        .iter_mut()
        .flat_map(|c| c.pcurve.control_points.iter_mut())
        .find(|p| p[0] == lower)
        .unwrap();
    p[0] = p[0].next_up();
    let mut bad = common.clone();
    bad["retained"] = value_codec::to_value(changed).unwrap();
    assert_eq!(dispatch(bad).unwrap()["continuousBound"], false);
    common["retained"] = value_codec::to_value(&model).unwrap();
    let boundary = dispatch(common.clone()).unwrap();
    assert_eq!(boundary["continuousBound"], true, "{boundary:?}");
    assert_eq!(boundary["smoothnessCertified"], false);
    assert_eq!(
        boundary["referenceVolume"]["trimRegionsValid"], true,
        "{boundary:?}"
    );
    common["reference"] = value_codec::to_value(&restored).unwrap();
    common["retained"] = value_codec::to_value(&restored).unwrap();
    let imported_boundary = dispatch(common).unwrap();
    assert_eq!(
        imported_boundary["continuousBound"], true,
        "{imported_boundary:?}"
    );
    assert_eq!(
        imported_boundary["referenceVolume"]["trimRegionsValid"],
        true
    );
    assert_eq!(
        (&model, &restored),
        (&original_models.0, &original_models.1)
    );
    let cap_faces = [model.faces.len() - 2, model.faces.len() - 1];
    let audit = dispatch(
        json!({"op":"brep_complete_boundary_smoothness_audit","model":restored,
        "order":2,"maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
        "capFaces":cap_faces,"seams":[],"fullBoundaryG2Certified":true}),
    )
    .unwrap();
    assert_eq!(audit["fullBoundaryG1Certified"], false);
    assert_eq!(audit["fullBoundaryG2Certified"], false);
    if let Ok(dir) = std::env::var("OSV_GRAM_BODY_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("boundary-imported.json"),
            value_codec::to_string(&imported_boundary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("boundary-transport.json"),
            value_codec::to_string(&transported_boundary).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("boundary.json"),
            value_codec::to_string(&boundary).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("quintic-body.step"), text).unwrap();
        std::fs::write(
            dir.join("body.json"),
            value_codec::to_string(&value_codec::to_value(&model).unwrap()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("smoothness.json"),
            value_codec::to_string(&audit).unwrap(),
        )
        .unwrap();
    }
}
