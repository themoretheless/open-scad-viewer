use geometry_bridge::dispatch;
use value_codec::{Value, from_str, json};

fn fixture() -> Value {
    from_str(include_str!(
        "fixtures/general-frame-cap-certificate-v1.json"
    ))
    .unwrap()
}

#[test]
fn original_frame_and_filled_caps_certify_through_the_public_native_protocol() {
    let data = fixture();
    let owned = dispatch(data["sweep"].clone()).unwrap();
    assert_eq!(owned["certified"], true, "{owned:?}");
    assert_eq!(owned["continuousBound"], true);
    assert_eq!(owned["solidGeometryCertified"], true);
    assert_eq!(owned["regularity"]["proven"], true);
    assert_eq!(
        owned["scope"],
        "original-sweep-law-and-authored-filled-caps"
    );
    assert!(owned["boundaryErrorUpper"].as_f64().unwrap() < 0.5);
    assert_eq!(owned["filledCapErrorUpper"].as_array().unwrap().len(), 2);
    assert_eq!(owned["idealBodyEmbeddingCertified"], false);
    assert_eq!(owned["smoothnessCertified"], false);
    assert_eq!(owned["seamContinuity"], "C0");
    assert!(!owned["model"].is_null());

    let common = dispatch(data["common"].clone()).unwrap();
    assert_eq!(common["certified"], true, "{common:?}");
    assert_eq!(common["referenceVolume"]["proven"], true);
    assert_eq!(common["retainedVolume"]["proven"], true);
    assert_eq!(common["sourceMotionLawCertified"], false);
    assert_eq!(common["smoothnessCertified"], false);
}

#[test]
fn caller_flags_cannot_replace_any_native_certificate_stage() {
    let data = fixture();
    for (key, budget) in [
        ("sourceCells", 0),
        ("sourceProducts", 0),
        ("sourceExactWork", 0),
        ("regularityCells", 0),
    ] {
        let mut request = data["sweep"].clone();
        request["limits"][key] = json!(budget);
        request["certified"] = json!(true);
        request["continuousBound"] = json!(true);
        request["model"] = data["common"]["retained"].clone();
        let refused = dispatch(request).unwrap();
        assert_eq!(refused["certified"], false, "{key}: {refused:?}");
        assert!(refused["model"].is_null(), "{key}");
    }
    let mut request = data["sweep"].clone();
    request["limits"]["volume"]["maxPairs"] = json!(1);
    let refused = dispatch(request).unwrap();
    assert_eq!(refused["certified"], false);
    assert_eq!(refused["solidGeometryCertified"], false);
    assert!(refused["model"].is_null());

    for stage in ["referenceVolume", "retainedVolume"] {
        let mut request = data["common"].clone();
        request["limits"][stage]["maxPairs"] = json!(1);
        request["certified"] = json!(true);
        let refused = dispatch(request).unwrap();
        assert_eq!(refused["continuousBound"], true);
        assert_eq!(refused["certified"], false, "{stage}: {refused:?}");
        assert_eq!(refused[stage]["proven"], false);
    }
}

#[test]
fn malformed_source_is_rejected_before_a_zero_budget_refusal() {
    let data = fixture();
    let mut bad = data["sweep"].clone();
    bad["limits"]["sourceCells"] = json!(0);
    bad["sourceCaps"][0]["surface"]["weights"][0][0] = json!(0.);
    assert!(dispatch(bad).is_err());
    for field in ["frame_axis", "frame_normal"] {
        let mut bad = data["sweep"].clone();
        bad["source"][field] = Value::Null;
        assert!(dispatch(bad).is_err(), "{field}");
    }
    let mut incomplete = data["sweep"].clone();
    incomplete["sourceCaps"] = json!([]);
    assert!(dispatch(incomplete).is_err());
}

#[test]
fn certified_nonplanar_capped_body_survives_direct_step_topology_exchange() {
    let result = dispatch(fixture()["sweep"].clone()).unwrap();
    let model: brep_core::Model = value_codec::from_value(result["model"].clone()).unwrap();
    let (text, certificate, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(certificate.complete);
    let (restored, certificate, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(certificate.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(restored.bodies.len(), model.bodies.len());
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        let reverse = model.shells[0]
            .faces
            .iter()
            .find(|f| f.face == i)
            .unwrap()
            .reversed;
        let (u, v) = (&a.surface.knots_u, &a.surface.knots_v);
        let [ua, ub] = [u[a.surface.degree_u], u[a.surface.control_points.len()]];
        let [va, vb] = [v[a.surface.degree_v], v[a.surface.control_points[0].len()]];
        for (uf, vf) in [(0., 0.), (0.13, 0.27), (0.61, 0.83), (1., 1.)] {
            let (u, v) = (ua + (ub - ua) * uf, va + (vb - va) * vf);
            let p = a
                .surface
                .evaluate(if reverse { ua + ub - u } else { u }, v)
                .unwrap()
                .point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(
                p.iter().zip(q).all(|(p, q)| (p - q).abs() < 1e-10),
                "face {i}"
            );
        }
    }
    if let Ok(path) = std::env::var("OSV_GENERAL_CERTIFICATE_STEP") {
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn ideal_source_embedding_is_native_owned_and_budgeted_separately_from_retained_solid() {
    let mut request = fixture()["sweep"].clone();
    request["idealBody"] = json!({"projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],"maxCells":100000,"maxExactWork":1000000});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["certified"], true, "{full:?}");
    assert_eq!(full["idealBodyEmbeddingCertified"], true);
    assert_eq!(full["idealBody"]["volumeRegularityCertified"], true);
    assert_eq!(full["idealBody"]["parameterTopologyPreserved"], true);
    assert_eq!(full["smoothnessCertified"], false);
    request["idealBody"]["maxCells"] = json!(full["idealBody"]["cells"].as_u64().unwrap() - 1);
    request["idealBodyEmbeddingCertified"] = json!(true);
    let short = dispatch(request).unwrap();
    assert_eq!(short["certified"], false);
    assert_eq!(short["idealBodyEmbeddingCertified"], false);
    assert!(short["model"].is_null());
    assert_eq!(short["solidGeometryCertified"], true);
}
#[test]
fn complete_smoothness_protocol_cannot_omit_a_cap_or_promote_sharp_joins() {
    let owned = dispatch(fixture()["sweep"].clone()).unwrap();
    for order in [1, 2] {
        let r=dispatch(json!({"op":"brep_complete_boundary_smoothness_audit","model":owned["model"],
            "order":order,"maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
            "capFaces":[],"seams":[],"fullBoundaryG2Certified":true})).unwrap();
        assert_eq!(r["fullBoundaryG1Certified"], false);
        assert_eq!(r["fullBoundaryG2Certified"], false);
        assert_eq!(r["idealSourceSmoothnessCertified"], false);
        assert!(!r["edgeIds"].as_array().unwrap().is_empty());
    }
}

#[test]
fn spatial_bishop_ideal_embedding_survives_owned_public_json_construction() {
    use nurbs_core::progressive_sweep::{MultiSweep, Options, Orientation, Spacing};
    use nurbs_core::{curve::Curve, surface::Surface};
    let mut request = fixture()["sweep"].clone();
    let profiles: Vec<Curve> =
        value_codec::from_value(request["source"]["profiles"].clone()).unwrap();
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0.; 3],
            vec![0.0625, 0.03125, 64. / 3.],
            vec![0.125, 0., 128. / 3.],
            vec![0.1875, 0.03125, 64.],
        ],
        None,
    )
    .unwrap();
    let scale: Curve = value_codec::from_value(request["source"]["scale"].clone()).unwrap();
    let twist: Curve = value_codec::from_value(request["source"]["twist"].clone()).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 6,
            max_sections: 6,
            max_deviation: 0.01,
        },
    )
    .unwrap()
    .with_spatial_rmf_error_limits(512, 100000, 1000000)
    .unwrap();
    let rows = sweep.sections_at(6).unwrap();
    for endpoint in 0..2 {
        let j = endpoint * 5;
        let mut surface: Surface =
            value_codec::from_value(request["retainedCaps"][endpoint]["surface"].clone()).unwrap();
        surface.control_points = vec![
            vec![
                rows[0][j].control_points[0].clone(),
                rows[3][j].control_points[0].clone(),
            ],
            vec![
                rows[1][j].control_points[0].clone(),
                rows[2][j].control_points[0].clone(),
            ],
        ];
        request["retainedCaps"][endpoint]["surface"] = value_codec::to_value(surface).unwrap();
    }
    request["source"]["path"] = value_codec::to_value(path).unwrap();
    request["source"]["orientation"] = json!("rmf");
    request["source"]["frame_axis"] = Value::Null;
    request["source"]["frame_normal"] = Value::Null;
    request["source"]["initial_sections"] = json!(6);
    request["source"]["max_sections"] = json!(6);
    request["sections"] = json!(6);
    request["source"]["rmf_transport_steps"] = json!(512);
    request["source"]["error_max_cells"] = json!(100000);
    request["source"]["error_max_products"] = json!(1000000);
    request["idealBody"] = json!({"projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],"maxCells":100000,"maxExactWork":1000000});
    let result = dispatch(request.clone()).unwrap();
    assert_eq!(result["certified"], true, "{result:?}");
    assert_eq!(result["idealBodyEmbeddingCertified"], true);
    assert_eq!(result["idealBody"]["parameterTopologyPreserved"], true);
    assert_eq!(result["solidGeometryCertified"], true);
    assert_eq!(result["smoothnessCertified"], false);
    request["idealBody"]["maxCells"] = json!(result["idealBody"]["cells"].as_u64().unwrap() - 1);
    let short = dispatch(request).unwrap();
    assert_eq!(short["certified"], false);
    assert!(short["model"].is_null());
    assert_eq!(short["idealBodyEmbeddingCertified"], false);
}

#[test]
fn original_atlas_is_constructor_owned_and_cannot_borrow_monotonicity_or_caller_flags() {
    let mut request = fixture()["sweep"].clone();
    request["idealBody"] = json!({"method":"atlas","projection":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1./64.]],"maxCells":100000,"maxExactWork":1000000});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["certified"],true,"{full:?}");
    assert_eq!(full["idealBody"]["proofMethod"],"atlas");
    assert_eq!(full["idealBodyEmbeddingCertified"],true);
    assert!(full["idealBody"]["monotonicityLower"].is_null());
    assert_eq!(full["smoothnessCertified"],false);
    assert_eq!(full["seamContinuity"],"C0");
    let mut legacy = request.clone(); legacy["idealBody"]["method"] = json!("monotone");
    let legacy = dispatch(legacy).unwrap();
    assert_eq!(full["model"],legacy["model"]);
    for limit in [0,full["idealBody"]["cells"].as_u64().unwrap()-1] {
        let mut short = request.clone(); short["idealBody"]["maxCells"] = json!(limit);
        short["idealBodyEmbeddingCertified"] = json!(true);
        let refused = dispatch(short).unwrap();
        assert_eq!(refused["certified"],false);
        assert_eq!(refused["idealBodyEmbeddingCertified"],false);
        assert_eq!(refused["solidGeometryCertified"],true);
        assert!(refused["model"].is_null());
    }
    request["idealBody"]["method"] = json!("trust-caller");
    assert!(dispatch(request).is_err());
}

#[test]
fn closed_spatial_bishop_native_json_qualifies_only_the_original_periodic_frame() {
    use nurbs_core::{curve::Curve,progressive_sweep::constant_vector_law};
    let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,1.],
            vec![-3.,3.,-1.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
    let profile=nurbs_core::primitives::line([0.,0.,0.125],[0.,0.,0.25]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    for spacing in ["parameter","arc_length"] {
        for order in [1,2] {
            let request=json!({"op":"surface_progressive_sweep_closed_path_frame_smoothness",
                "profiles":[profile],"path":path,"scale":scale,"twist":twist,"normal":[0.,0.,1.],
                "orientation":"rmf","spacing":spacing,"initial_sections":5,"max_sections":17,"max_deviation":0.01,
                "length_tolerance":0.001,"length_max_cells":100000,
                "rmf_transport_steps":128,"error_max_cells":100000,"error_max_products":1000000,
                "order":order,"maxCells":100000,"maxExactWork":1000000});
            let r=dispatch(request.clone()).unwrap();
            assert_eq!(r["closedSourceFrameSmoothnessCertified"],true,"{r:?}");
            assert_eq!(r["method"],"original-periodic-bishop-ode-with-arc-holonomy");
            assert_eq!(r["scope"],"closed-original-path-frame-only");
            for flag in ["retainedSeamsCertified","capJoinsCertified","profileJoinsCertified","continuousBound","solidCertified"] {
                assert_eq!(r[flag],false,"{flag}");
            }
            for key in ["maxCells","maxExactWork"] {
                let mut short=request.clone();short[key]=json!(0);short["closedSourceFrameSmoothnessCertified"]=json!(true);
                assert_eq!(dispatch(short).unwrap()["closedSourceFrameSmoothnessCertified"],false);
            }
        }
    }
}
