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
            let result=profile_body(json!({"loops":[profiles],"path":path,"scale":scalar(1.),"twist":scalar(0.),
            "options":{"normal":[0.,0.,1.],"orientation":"rmf","initialSections":count,"maxSections":count,"maxDeviation":1.}})).unwrap();
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
