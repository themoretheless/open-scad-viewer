use geometry_bridge::dispatch;
use value_codec::{Value, from_str, json};
fn fixture() -> Value {
    from_str(include_str!("fixtures/closed-material-body-v1.json")).unwrap()
}
#[test]
fn closed_material_source_is_certified_by_the_public_native_protocol() {
    let original = fixture();
    let r = dispatch(original.clone()).unwrap();
    assert_eq!(r["certified"], true, "{r:?}");
    for k in [
        "continuousBound",
        "withinBudget",
        "idealBodyEmbeddingCertified",
        "solidGeometryCertified",
    ] {
        assert_eq!(r[k], true, "{k}");
    }
    assert_eq!(r["scope"], "original-closed-sweep-law-and-material-region");
    assert_eq!(r["physicalCapCount"], 0);
    assert!(r["filledCapErrorUpper"].is_null());
    assert_eq!(r["material"]["certified"], true);
    assert_eq!(r["material"]["boundSpans"], 4);
    assert_eq!(r["material"]["trim"]["valid"], true);
    assert_eq!(r["idealBody"]["volumeRegularityCertified"], true);
    assert_eq!(r["idealBody"]["parameterTopologyPreserved"], true);
    assert_eq!(r["retainedWallOwnership"]["certified"], true);
    assert_eq!(r["retainedWallOwnership"]["inspectedFaces"], 128);
    assert_eq!(r["regularity"]["proven"], true);
    assert!(r["boundaryErrorUpper"].as_f64().unwrap() <= 0.05);
    assert_eq!(r["smoothnessCertified"], false);
    assert_eq!(r["seamContinuity"], "C0");
    let model: brep_core::Model = value_codec::from_value(r["model"].clone()).unwrap();
    assert_eq!(model.faces.len(), 128);
    assert_eq!(model.shells.len(), 1);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.bodies[0].inner_shells.is_empty());
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(original, fixture());
}
#[test]
fn caller_models_and_success_flags_cannot_bypass_closed_native_ownership() {
    let base = fixture();
    let full = dispatch(base.clone()).unwrap();
    let mut cases = Vec::new();
    for (key, value) in [
        ("sourceCells", 0),
        ("sourceProducts", 0),
        ("sourceExactWork", 0),
        ("regularityCells", 0),
    ] {
        let mut request = base.clone();
        request["limits"][key] = json!(value);
        cases.push(request);
    }
    let mut request = base.clone();
    request["limits"]["material"]["maxCells"] = json!(0);
    cases.push(request);
    let mut request = base.clone();
    request["idealBody"]["maxCells"] = json!(full["idealBody"]["cells"].as_u64().unwrap() - 1);
    cases.push(request);
    let mut request = base.clone();
    request["limits"]["sourceExactWork"] = json!(full["ownershipExactWork"].as_u64().unwrap() - 1);
    cases.push(request);
    let mut request = base.clone();
    request["limits"]["volume"]["maxPairs"] = json!(1);
    cases.push(request);
    for mut request in cases {
        request["model"] = full["model"].clone();
        request["certified"] = json!(true);
        request["idealBodyEmbeddingCertified"] = json!(true);
        request["smoothnessCertified"] = json!(true);
        let r = dispatch(request).unwrap();
        assert_eq!(r["certified"], false, "{r:?}");
        assert!(r["model"].is_null());
        assert_eq!(r["smoothnessCertified"], false);
        assert!(!r["reason"].is_null());
    }
    let mut request = base.clone();
    request["idealBody"]["method"] = json!("monotone");
    assert!(dispatch(request).is_err());
    let mut request = base.clone();
    request["material"]["surface"]["weights"][0][0] = json!(0.);
    request["limits"]["material"]["maxCells"] = json!(0);
    assert!(dispatch(request).is_err());
    let mut request = base.clone();
    request["material"]["trims"][0][0]["controlPoints"][1][0] = json!(3. - 1e-12);
    let r = dispatch(request).unwrap();
    assert_eq!(r["material"]["certified"], false);
    assert!(r["model"].is_null());
}
fn volume_limits(v: &Value) -> brep_core::volume_validity::Limits {
    let n = |k: &str| v[k].as_u64().unwrap() as usize;
    brep_core::volume_validity::Limits {
        boundary: brep_core::boundary_embedding::Limits {
            exact_work: v["maxExactWork"].as_u64().unwrap(),
            trim_pairs: n("maxTrimPairs"),
            trim_cells: n("maxTrimCells"),
            trim_domain_cells: n("maxTrimDomainCells"),
            spans: n("maxSpans"),
            contacts: brep_core::face_contacts::Limits {
                pairs: n("maxPairs"),
                cells: n("maxCells"),
                domain_cells: n("maxDomainCells"),
                cells_per_pair: n("cellsPerPair"),
                domain_cells_per_pair: n("domainCellsPerPair"),
            },
        },
        nesting_pairs: n("nestingPairs"),
        nesting_cells: n("nestingCells"),
        nesting_domain_cells: n("nestingDomainCells"),
        orientation_cells: n("orientationCells"),
        orientation_domain_cells: n("orientationDomainCells"),
        orientation_spans: n("orientationSpans"),
    }
}
#[test]
fn closed_spatial_nonplanar_solid_roundtrips_native_step_geometry_topology_and_volume() {
    let request = fixture();
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["certified"], true);
    let model: brep_core::Model = value_codec::from_value(r["model"].clone()).unwrap();
    let (text, proof, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(proof.complete);
    let (restored, proof, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(proof.complete);
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(restored.vertices.len(), model.vertices.len());
    assert_eq!(restored.shells.len(), model.shells.len());
    assert_eq!(restored.bodies.len(), model.bodies.len());
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        let reverse = model.shells[0]
            .faces
            .iter()
            .find(|f| f.face == i)
            .unwrap()
            .reversed;
        let [ua, ub] = [
            a.surface.knots_u[a.surface.degree_u],
            a.surface.knots_u[a.surface.control_points.len()],
        ];
        let [va, vb] = [
            a.surface.knots_v[a.surface.degree_v],
            a.surface.knots_v[a.surface.control_points[0].len()],
        ];
        for (uf, vf) in [(0., 0.), (0.13, 0.27), (0.61, 0.83), (1., 1.)] {
            let u = ua + (ub - ua) * uf;
            let v = va + (vb - va) * vf;
            let p = a
                .surface
                .evaluate(if reverse { ua + ub - u } else { u }, v)
                .unwrap()
                .point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(
                p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10),
                "face {i}"
            );
        }
    }
    let p = brep_core::volume_validity::inspect_sweep(
        &restored,
        1e-8,
        volume_limits(&request["limits"]["volume"]),
        20000,
        &[],
        brep_core::sweep_cap_contacts::Budgets {
            max_walls: 0,
            max_exact_work: 0,
            max_chart_cells: 0,
            max_trim_pairs: 0,
            max_trim_cells: 0,
            max_trim_domain_cells: 0,
        },
    )
    .unwrap();
    assert!(
        p.proven,
        "restored Solid requires fresh retained-volume proof"
    );
}
