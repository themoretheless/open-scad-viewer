use super::*;

fn box_model() -> Value {
    crate::dispatch(json!({"op":"brep_nurbs_box","min":[0.,0.,0.],"max":[1.,2.,3.]})).unwrap()
}
fn request(model: Value, transformed: bool) -> Value {
    let mut nodes = vec![json!({"id":"body","op":"brep_progressive_sweep"})];
    if transformed {
        nodes.push(json!({"id":"placed","op":"transform","input":"body"}));
    }
    json!({"documentJson":json!({"nodes":nodes}).to_string(),"nodeId":if transformed{"placed"}else{"body"},"model":model})
}

#[test]
fn profile_solid_admission_recomputes_full_body_proof_through_transforms() {
    let model = box_model();
    let before = model.clone();
    for transformed in [false, true] {
        let proof = admission(request(model.clone(), transformed)).unwrap();
        assert_eq!(proof["solidGeometryCertified"], json!(true), "{proof}");
        assert_eq!(proof["boundaryEmbeddingCertified"], json!(true));
    }
    assert_eq!(model, before);
}

#[test]
fn profile_solid_admission_rejects_forged_positive_report_and_changed_boundary() {
    let mut model = box_model();
    let point = model["faces"][0]["surface"]["controlPoints"][0][0][0]
        .as_f64()
        .unwrap();
    model["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(point.next_up());
    let mut v = request(model, false);
    v["globalEmbeddingCertified"] = json!(true);
    v["volume"] = json!({"solidGeometryCertified":true});
    assert!(admission(v).is_err());
}

fn line(a: [f64; 3], b: [f64; 3]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[a,b],"weights":[1.,1.],"periodic":false})
}
fn scalar(v: f64) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[v,v],"weights":[1.,1.]})
}
fn vector(v: [f64; 3]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[v,v],"weights":[1.,1.]})
}
#[test]
fn profile_constructor_audits_caps_and_whole_body_for_frame_guide_affine_combinations() {
    let corners = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
    let profiles = (0..4)
        .map(|i| line(corners[i], corners[(i + 1) % 4]))
        .collect::<Vec<_>>();
    for orientation in ["fixed", "rmf", "authored"] {
        for guide in [false, true] {
            for affine in [false, true] {
                let mut options = json!({"normal":[0.,1.,0.],"orientation":orientation,
            "initialSections":2,"maxSections":2,"maxDeviation":1e-6});
                if guide {
                    options["orientationGuide"] = line([0., 2., 0.], [0., 2., 2.]);
                }
                if affine {
                    options["axisScale"] = vector([0.75, 1.25, 1.]);
                    options["centerLaw"] = vector([0., -0.25, 0.]);
                }
                if orientation == "authored" {
                    options["frameAxis"] = vector([0., 0., 1.]);
                    options["frameNormal"] = vector([0., 1., 0.]);
                }
                let v = json!({"loops":[profiles],"path":line([0.,0.,0.],[0.,0.,2.]),
            "scale":scalar(1.),"twist":scalar(0.),"options":options});
                let before = v.clone();
                let outcome = profile_body(v.clone());
                if guide && orientation != "rmf" {
                    let refusal = outcome.unwrap_err();
                    assert_eq!(refusal.code, "NURBS_INVALID_INPUT");
                    assert!(refusal.message.contains("cannot be combined"));
                    assert_eq!(v, before);
                    continue;
                }
                let result = outcome.unwrap();
                assert_eq!(
                    result["volume"]["solidGeometryCertified"],
                    json!(true),
                    "orientation={orientation}, guide={guide}, affine={affine}: {}",
                    result["volume"]
                );
                assert_eq!(result["globalEmbeddingCertified"], json!(false));
                assert_eq!(
                    result["approximation"]["report"]["volume"],
                    result["volume"]
                );
                assert_eq!(
                    result["approximation"]["report"]["wallRegularityCertified"],
                    json!(true)
                );
                assert_eq!(
                    result["approximation"]["report"]["seamContinuity"],
                    json!("C0")
                );
                let proof = admission(request(result["model"].clone(), true)).unwrap();
                assert_eq!(proof["solidGeometryCertified"], json!(true));
                assert_eq!(v, before);
            }
        }
    }
}

#[test]
fn profile_body_certifies_hole_material_roles_and_refuses_exhausted_exact_work() {
    let outer = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
    let hole = [
        [0.25, 0.25, 0.],
        [0.25, 0.75, 0.],
        [0.75, 0.75, 0.],
        [0.75, 0.25, 0.],
    ];
    let loop_curves = |points: [[f64; 3]; 4]| {
        (0..4)
            .map(|i| line(points[i], points[(i + 1) % 4]))
            .collect::<Vec<_>>()
    };
    let input = json!({"loops":[loop_curves(outer),loop_curves(hole)],
        "path":line([0.,0.,0.],[0.,0.,2.]),"scale":scalar(1.),"twist":scalar(0.),
        "options":{"normal":[0.,1.,0.],"orientation":"rmf","initialSections":2,"maxSections":2,"maxDeviation":1e-6}});
    let result = profile_body(input).unwrap();
    assert_eq!(
        result["volume"]["solidGeometryCertified"],
        json!(true),
        "{}",
        result["volume"]
    );
    assert_eq!(result["volume"]["nesting"]["rolesConsistent"], json!(true));
    assert!(
        result["volume"]["orientations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["outward"] == s["expectedOutward"])
    );
    let model = result["model"].clone();
    let before = model.clone();
    let caps = cap_faces(&model, false).unwrap();
    let mut limits = volume_budgets();
    limits["maxExactWork"] = json!(1);
    let denied = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &limits),
    )
    .unwrap();
    assert_eq!(denied["solidGeometryCertified"], json!(false));
    assert_eq!(result["model"], before);
}

#[test]
fn closed_profile_body_has_fresh_whole_shell_proofs_outside_dyadic_stations() {
    let radius = 3.;
    let half = 0.125;
    let points = [
        [radius - half, 0., -half],
        [radius - half, 0., half],
        [radius + half, 0., half],
        [radius + half, 0., -half],
    ];
    let mut profiles = (0..4)
        .map(|i| line(points[i], points[(i + 1) % 4]))
        .collect::<Vec<_>>();
    let mut path = json!({"degree":2,"knots":[0.,0.,0.,0.25,0.25,0.5,0.5,0.75,0.75,1.,1.,1.],
        "controlPoints":[[3.,0.,0.],[3.,3.,0.],[0.,3.,0.],[-3.,3.,0.],[-3.,0.,0.],[-3.,-3.,0.],[0.,-3.,0.],[3.,-3.,0.],[3.,0.,0.]],
        "weights":[1.,1.,2.,2.,4.,4.,8.,8.,16.],"periodic":false});
    for spatial in [false, true] {
        if spatial {
            path = json!({"degree":3,"knots":[0.,0.,0.,0.,0.25,0.25,0.25,0.5,0.5,0.5,0.75,0.75,0.75,1.,1.,1.,1.],
            "controlPoints":[[3.,0.,0.],[3.,1.,0.125],[1.,3.,0.125],[0.,3.,0.],[-1.,3.,-0.125],[-3.,1.,-0.125],[-3.,0.,0.],[-3.,-1.,0.125],[-1.,-3.,0.125],[0.,-3.,0.],[1.,-3.,-0.125],[3.,-1.,-0.125],[3.,0.,0.]],
            "weights":[1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.],"periodic":false});
            let spatial_points = points.map(|mut p| {
                p[1] = -0.125 * p[2];
                p
            });
            profiles = (0..4)
                .map(|i| line(spatial_points[i], spatial_points[(i + 1) % 4]))
                .collect();
        }
        for count in [6, 7, 10, 12] {
            // This matrix proves the retained shell, independently of a tight
            // ideal-family error budget. The continuous bound can exceed the
            // old sample-only allowance even when the sampled deviation fits.
            // The unequal-weight 12-station case has a conservative bound
            // of 238.284; 256 is explicit here solely for shell qualification.
            let mut source = json!({"loops":[profiles],"path":path,"scale":scalar(1.),"twist":scalar(0.),
                "options":{"normal":[0.,0.,1.],"orientation":"rmf","initialSections":count,"maxSections":count,"maxDeviation":256.}});
            if count == 6 && !spatial {
                source["options"]["maxDeviation"] = json!(1.);
                assert!(profile_body(source.clone()).is_err());
                source["options"]["maxDeviation"] = json!(256.);
            }
            let result = profile_body(source.clone()).unwrap_or_else(|error| {
                let (request, _, _) = bounded_request(&source, false).unwrap();
                let report = nurbs("surface_progressive_sweep_profiles", request).unwrap();
                panic!("spatial={spatial}, count={count}: {error:?}; native approximation={report}")
            });
            assert_eq!(result["approximation"]["report"]["closedPath"], json!(true));
            assert_eq!(
                result["volume"]["solidGeometryCertified"],
                json!(true),
                "count={count}: {}",
                result["volume"]
            );
            let proof = admission(request(result["model"].clone(), true)).unwrap();
            assert_eq!(proof["solidGeometryCertified"], json!(true));
            assert_eq!(result["globalEmbeddingCertified"], json!(false));
            if count == 6 && !spatial {
                let mut changed = result["model"].clone();
                let coordinate = changed["faces"][0]["surface"]["controlPoints"][0][0][0]
                    .as_f64()
                    .unwrap();
                changed["faces"][0]["surface"]["controlPoints"][0][0][0] =
                    json!(coordinate.next_up());
                assert!(admission(request(changed, true)).is_err());
            }
            let mut budget = volume_budgets();
            budget["maxPairs"] = json!(1);
            let denied = call(
                "brep_sweep_volume_audit",
                merge(json!({"model":result["model"],"capFaces":[]}), &budget),
            )
            .unwrap();
            assert_eq!(denied["solidGeometryCertified"], json!(false));
        }
    }
}

#[test]
fn profile_bridge_preserves_owned_boundary_evidence_and_authored_budgets() {
    let corners = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
    let profiles = (0..4)
        .map(|i| line(corners[i], corners[(i + 1) % 4]))
        .collect::<Vec<_>>();
    let source = json!({"loops":[profiles],"path":line([0.,0.,0.],[0.,0.,2.]),
        "scale":scalar(1.),"twist":scalar(0.),"options":{"normal":[0.,1.,0.],
        "orientation":"fixed","initialSections":2,"maxSections":2,"maxDeviation":1e-6}});
    let result = profile_body(source.clone()).unwrap();
    assert_eq!(result["boundaryContinuousBound"], json!(true));
    assert_eq!(result["boundaryErrorWithinBudget"], json!(true));
    assert_eq!(
        result["boundaryErrorScope"],
        json!("constructor-owned-retained-wall-and-cap-union")
    );
    assert_eq!(result["volume"]["solidGeometryCertified"], json!(true));
    let mut policy = source.clone();
    policy["options"]["rmfTransportSteps"] = json!(7);
    policy["options"]["errorMaxCells"] = json!(0);
    policy["options"]["errorMaxProducts"] = json!(11);
    policy["options"]["capCorrection"] = json!({"tolerance":0.001,"quantum":1e-9,"maxWork":0});
    let request = profile_request(&policy, &json!(profiles)).unwrap();
    assert_eq!(request["rmf_transport_steps"], json!(7));
    assert_eq!(request["error_max_cells"], json!(0));
    assert_eq!(request["error_max_products"], json!(11));
    assert_eq!(request["cap_correction_max_work"], json!(0));
    assert!(profile_body(policy).is_err());
}

#[test]
fn profile_body_metadata_does_not_become_miter_viewport_evidence() {
    let model = box_model();
    let volume = json!({"solidGeometryCertified":false});
    let mut packet = json!({"geometry":model,"sweepEvidence":{"volume":volume}});
    let artifact = |packet: &Value| json!({"artifact":{"kind":"brep","nodeId":"body","geometryJson":packet.to_string()}});
    assert!(
        crate::sweep_viewport::read(artifact(&packet))
            .unwrap()
            .is_object()
    );
    packet["sweepBodyBoundaryEvidence"] = json!({"boundaryErrorScope":"constructor-owned-retained-wall-and-cap-union","globalEmbeddingCertified":false});
    let before = packet.clone();
    assert!(
        crate::sweep_viewport::read(artifact(&packet))
            .unwrap()
            .is_null()
    );
    assert_eq!(packet, before);
    // A malformed profile report cannot fall through into a different scope.
    packet["sweepBodyBoundaryEvidence"] = json!({});
    assert!(
        crate::sweep_viewport::read(artifact(&packet))
            .unwrap()
            .is_null()
    );
}

#[test]
fn snapshot_volume_summary_preserves_outcomes_and_pair_accounting() {
    let full = json!({"solidGeometryCertified":false,"allPairsClassified":false,
        "individualPairs":3,"groupedPairs":7,"contactCells":19,"nextPair":[2,4],
        "disjointGroups":[{"face":0,"range":[1,5]},{"face":1,"range":[2,5]}]});
    let before = full.clone();
    let compact = crate::cad_face_contacts::compact_volume_report(full.clone());
    assert_eq!(full, before);
    assert_eq!(compact["groupCertificateCount"], json!(2));
    assert!(compact.get("disjointGroups").is_none());
    for key in [
        "solidGeometryCertified",
        "allPairsClassified",
        "individualPairs",
        "groupedPairs",
        "contactCells",
        "nextPair",
    ] {
        assert_eq!(compact[key], full[key]);
    }
}

#[test]
fn authored_affine_miter_refines_against_complete_corrected_boundary() {
    let source: Value = value_codec::from_str(include_str!(
        "../fixtures/combined-frame-affine-cap-refinement-request.json"
    ))
    .unwrap();
    let before = source.clone();
    let body = progressive_miter(source.clone()).unwrap();
    let report = &body["approximation"]["report"];
    assert_eq!(report["accepted"], json!(true));
    assert!(report["steps"].as_u64().unwrap() > 1);
    assert_eq!(body["boundaryCertificate"]["continuousBound"], json!(true));
    assert_eq!(body["boundaryCertificate"]["withinBudget"], json!(true));
    assert!(body["boundaryCertificate"]["errorUpper"].as_f64().unwrap() <= 2.);
    assert_eq!(body["volume"]["solidGeometryCertified"], json!(true));
    assert!(
        body["approximation"]["levels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|level| level["accepted"] == json!(false)
                && level["errorCertificateReason"] == json!("complete-boundary-refinement"))
    );
    assert_eq!(source, before);
    let mut exhausted = source.clone();
    exhausted["options"]["maxSteps"] = json!(1);
    assert!(progressive_miter(exhausted).is_err());
    exhausted = source;
    exhausted["options"]["capCorrection"]["maxWork"] = json!(1);
    assert!(progressive_miter(exhausted).is_err());
}

#[test]
fn transferred_miter_replay_rejects_changed_affine_provenance() {
    let source: Value = value_codec::from_str(include_str!(
        "../fixtures/combined-frame-affine-cap-refinement-request.json"
    ))
    .unwrap();
    let body = progressive_miter(source).unwrap();
    let matrix = json!([
        [-1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.]
    ]);
    let placed = transform(json!({"owner":body["_nativeOwner"],"source":body,
        "matrix":matrix,"options":{"quantum":2_f64.powi(-40),"maxWork":1000000,"maxDeviation":2.}}))
    .unwrap();
    let mut packet =
        json!({"geometry":placed["model"],"sweepMiterReplay":placed["sweepMiterReplay"]});
    let document = json!({"nodes":[{"id":"body","op":"brep_progressive_miter_sweep"},
        {"id":"placed","op":"transform","input":"body"}]});
    let request = |packet: &Value| {
        json!({"documentJson":document.to_string(),
        "nodeId":"placed","geometryJson":packet.to_string(),"model":placed["model"]})
    };
    assert_eq!(
        admission(request(&packet)).unwrap()["solidGeometryCertified"],
        json!(true)
    );
    packet["sweepMiterReplay"]["matrix"][0][0] = json!(1.);
    assert!(admission(request(&packet)).is_err());
}

#[test]
fn reconstructed_station_admission_checks_actual_model_and_snapshot() {
    let model = box_model();
    let packet = json!({"geometry": model, "sweepEvidence":{"solidGeometryCertified":true}});
    let v = json!({"documentJson":json!({"nodes":[{"id":"smooth","op":"brep_smooth_miter_stations"}]}).to_string(),
        "nodeId":"smooth","model":model,"geometryJson":packet.to_string()});
    assert_eq!(
        admission(v.clone()).unwrap()["solidGeometryCertified"],
        json!(true)
    );
    let mut changed = v.clone();
    changed["model"]["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(0.125);
    assert!(
        admission(changed)
            .unwrap_err()
            .message
            .contains("snapshot binding")
    );
    let mut forged = v;
    forged["model"]["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(0.125);
    forged["geometryJson"] = json!(
        json!({"geometry":forged["model"],"sweepEvidence":{"solidGeometryCertified":true}})
            .to_string()
    );
    assert!(admission(forged).is_err());
}

#[test]
fn default_spatial_three_cavity_budget_proves_actual_material_and_refuses_short_work() {
    let source: Value = value_codec::from_str(include_str!(
        "../fixtures/periodic-spatial-three-cavity-request.json"
    ))
    .unwrap();
    let before = source.clone();
    let body = profile_body(source.clone()).unwrap();
    assert_eq!(source, before);
    assert_eq!(body["model"]["faces"].as_array().unwrap().len(), 1024);
    assert_eq!(body["volume"]["solidGeometryCertified"], json!(true));
    assert_eq!(
        body["volume"]["nesting"]["parents"],
        json!([None::<usize>, Some(0usize), Some(0usize), Some(0usize)])
    );
    let model = body["model"].clone();
    let proof = admission(request(model.clone(), false)).unwrap();
    assert_eq!(proof["solidGeometryCertified"], json!(true));
    assert_eq!(proof["nextPair"], Value::Null);
    assert_eq!(
        proof["individualPairs"].as_u64().unwrap() + proof["groupedPairs"].as_u64().unwrap(),
        1024 * 1023 / 2
    );
    let mut limits = volume_budgets();
    assert_eq!(limits["maxLinearCells"], json!(30000));
    limits["model"] = model;
    limits["capFaces"] = json!([]);
    limits["maxLinearCells"] = json!(20000);
    limits["maxPairs"] = json!(1);
    limits["maxCells"] = json!(1);
    let refused = call("brep_sweep_volume_audit", limits).unwrap();
    assert_eq!(refused["allFacesInjective"], json!(false));
    assert_eq!(refused["solidGeometryCertified"], json!(false));
}

#[test]
fn legacy_bare_model_snapshot_preserves_native_admission_and_rejects_forged_metadata() {
    let model = box_model();
    let mut v = request(model.clone(), false);
    v["geometryJson"] = json!(model.to_string());
    assert_eq!(
        admission(v.clone()).unwrap()["solidGeometryCertified"],
        json!(true)
    );
    v["model"]["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(0.125);
    assert!(
        admission(v.clone())
            .unwrap_err()
            .message
            .contains("snapshot binding")
    );
    let mut forged = v["model"].clone();
    forged["volume"] = json!({"solidGeometryCertified":true});
    v["geometryJson"] = json!(forged.to_string());
    let refusal = admission(v.clone()).unwrap_err();
    assert!(
        refusal.message.contains("snapshot model binding"),
        "{refusal:?}"
    );
    v["geometryJson"] = json!(v["model"].to_string());
    assert!(admission(v).is_err());
}
