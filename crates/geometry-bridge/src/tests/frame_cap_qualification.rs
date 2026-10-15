//! Explicit Cartesian qualification: modes, guides, affine laws, spacing,
//! endpoint correction and material holes. Every cell has a frozen expectation.
use super::*;
fn line(a: [f64; 3], b: [f64; 3]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[a,b],"weights":[1.,2.],"periodic":false})
}
fn unit_line(a: [f64; 3], b: [f64; 3]) -> Value {
    let mut c = line(a, b);
    c["weights"] = json!([1., 1.]);
    c
}
fn scalar(x: f64) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[x,x],"weights":[1.,1.]})
}
fn vector(x: [f64; 3]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[x,x],"weights":[1.,1.]})
}
fn wire(points: [[f64; 3]; 4]) -> Vec<Value> {
    (0..4)
        .map(|i| line(points[i], points[(i + 1) % 4]))
        .collect()
}
#[test]
fn full_frame_cap_cartesian_contract() {
    let mut passed = 0;
    let mut refused = 0;
    for orientation in [
        "fixed",
        "rmf",
        "authored",
        "fixed_normal",
        "corrected_frenet",
        "frenet",
    ] {
        for guide in [false, true] {
            for affine in [false, true] {
                for spacing in ["parameter", "arc_length"] {
                    for correction in [false, true] {
                        for hole in [false, true] {
                            let id = format!(
                                "{orientation}/guide={guide}/affine={affine}/{spacing}/correction={correction}/hole={hole}"
                            );
                            let mut loops = vec![wire([
                                [0., 0., 0.],
                                [2., 0., 0.],
                                [2., 2., 0.],
                                [0., 2., 0.],
                            ])];
                            if hole {
                                loops.push(wire([
                                    [0.5, 0.5, 0.],
                                    [0.5, 1.5, 0.],
                                    [1.5, 1.5, 0.],
                                    [1.5, 0.5, 0.],
                                ]));
                            }
                            let mut options = json!({"normal":[0.,1.,0.],"orientation":orientation,"spacing":spacing,
                    "initialSections":2,"maxSections":2,"maxDeviation":0.001});
                            if guide {
                                options["orientationGuide"] = unit_line([0., 3., 0.], [0., 3., 4.]);
                            }
                            if affine {
                                options["axisScale"] = vector([0.75, 1.25, 1.]);
                                options["centerLaw"] = vector([0., -0.25, 0.]);
                            }
                            if orientation == "authored" {
                                options["frameAxis"] = vector([0., 0., 1.]);
                                options["frameNormal"] = vector([0., 1., 0.]);
                            }
                            if correction {
                                options["capCorrection"] = json!({"tolerance":1e-9,"quantum":2_f64.powi(-40),"maxWork":1000000});
                            }
                            let source = json!({"loops":loops,"path":unit_line([0.,0.,0.],[0.,0.,4.]),"scale":scalar(1.),"twist":scalar(0.),"options":options});
                            let before = source.clone();
                            let outcome = profile_body(source.clone());
                            assert_eq!(source, before, "{id}");
                            if orientation == "frenet" && !guide {
                                let error = match outcome {
                                    Err(error) => error,
                                    Ok(_) => {
                                        panic!("{id}: zero-curvature Frenet unexpectedly accepted")
                                    }
                                };
                                assert_eq!(error.code, "NURBS_INVALID_INPUT", "{id}: {error:?}");
                                assert!(
                                    error.message.contains("nonzero principal normal"),
                                    "{id}: {error:?}"
                                );
                                refused += 1;
                                continue;
                            }
                            if guide && matches!(orientation, "fixed" | "authored") {
                                let error = match outcome {
                                    Err(error) => error,
                                    Ok(_) => {
                                        panic!("{id}: incompatible frames unexpectedly accepted")
                                    }
                                };
                                assert_eq!(error.code, "NURBS_INVALID_INPUT", "{id}: {error:?}");
                                assert!(
                                    error.message.contains("cannot be combined"),
                                    "{id}: {error:?}"
                                );
                                refused += 1;
                                continue;
                            }
                            let body = outcome.unwrap_or_else(|e| panic!("{id}: {e:?}"));
                            assert_eq!(
                                body["volume"]["solidGeometryCertified"], true,
                                "{id}: {}",
                                body["volume"]
                            );
                            let model: brep_core::Model =
                                value_codec::from_value(body["model"].clone()).unwrap();
                            let regular =
                                brep_core::boundary_embedding::regularity::inspect(&model, 10000)
                                    .unwrap();
                            assert!(
                                regular.all_faces_regular,
                                "{id}: {:?}",
                                regular.unresolved_faces
                            );
                            assert_eq!(
                                body["approximation"]["report"]["seamContinuity"], "C0",
                                "{id}"
                            );
                            passed += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!((passed, refused), (144, 48));
}

#[test]
fn exact_sphere_transport_requires_actual_pole_and_volume_proofs() {
    let model = call("brep_nurbs_sphere_exact", json!({"radius":2.})).unwrap();
    let mut v = volume_budgets();
    v["model"] = model.clone();
    v["capFaces"] = json!([]);
    v["maxRegularityCells"] = json!(10000);
    let proof = call("brep_sweep_volume_audit", v.clone()).unwrap();
    assert_eq!(proof["regularBoundarySolidCertified"], true, "{proof}");
    assert_eq!(proof["boundaryRegularity"]["allFacesRegular"], false);
    assert_eq!(
        proof["boundaryRegularity"]["allFacesRegularModuloPoles"],
        true
    );
    assert_eq!(
        proof["boundaryRegularity"]["poleQuotients"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    let embedding=call("brep_sweep_embedding_audit",v.clone()).unwrap();
    assert_eq!(embedding["boundaryEmbeddingCertified"],true);
    assert_eq!(embedding["unresolvedFaces"],json!([]));
    v["maxRegularityCells"] = json!(0);
    let refused = call("brep_sweep_volume_audit", v).unwrap();
    assert_eq!(refused["solidGeometryCertified"], true);
    assert_eq!(refused["regularBoundarySolidCertified"], false);
    let actual: brep_core::Model = value_codec::from_value(model).unwrap();
    let before = format!("{actual:?}");
    let boundary = brep_core::boundary_agreement::verify_exact(&actual, 1000000).unwrap();
    assert!(boundary.all_equal && boundary.all_joins_exact);
    assert_eq!(format!("{actual:?}"), before);
}
