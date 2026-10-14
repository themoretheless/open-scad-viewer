//! Authored rational saddle caps remain nonplanar under transported frames.
use super::*;
use nurbs_core::{curve::Curve, surface::Surface};
fn line(a: [f64; 3], b: [f64; 3], w: [f64; 2]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[a,b],"weights":w,"periodic":false})
}
fn vector(v: [f64; 3]) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[v,v],"weights":[1.,1.]})
}
fn scalar(v: f64) -> Value {
    json!({"degree":1,"knots":[0.,0.,1.,1.],"values":[v,v],"weights":[1.,1.]})
}
fn wire(points: [[f64; 2]; 4], weights: [[f64; 2]; 4]) -> Vec<Value> {
    let p = points.map(|[x, y]| [x, y, x * y / 256.]);
    (0..4)
        .map(|i| line(p[i], p[(i + 1) % 4], weights[i]))
        .collect()
}
fn section(s: &Surface, j: usize) -> Curve {
    Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        control_points: s.control_points.iter().map(|r| r[j].clone()).collect(),
        weights: s.weights.iter().map(|r| r[j]).collect(),
        periodic: false,
    }
}
fn cap(section: &[Vec<Curve>], hole: bool) -> brep_core::LoftCap {
    let corners: Vec<_> = section[0]
        .iter()
        .map(|c| c.control_points[0].clone())
        .collect();
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![corners[0].clone(), corners[3].clone()],
            vec![corners[1].clone(), corners[2].clone()],
        ],
        weights: vec![vec![1., 2.], vec![2., 4.]],
        periodic_u: false,
        periodic_v: false,
    };
    let mut uv = vec![[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]];
    if hole {
        uv.push([[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]]);
    }
    let trims = uv
        .into_iter()
        .map(|p| {
            (0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()],
                    weights: vec![1., 1.],
                    periodic: false,
                })
                .collect()
        })
        .collect();
    brep_core::LoftCap { surface, trims }
}
fn limits() -> brep_core::volume_validity::Limits {
    use brep_core::{boundary_embedding, face_contacts, volume_validity};
    volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1000,
            contacts: face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        nesting_pairs: 100,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 100,
    }
}
fn intersection_cases() -> Vec<Value> {
    use brep_core::analytic;
    let sphere = analytic::sphere_exact(2.).unwrap();
    let translated = |z| {
        brep_core::transform::affine(
            &sphere,
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., z],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    };
    vec![
        json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":translated(2.)}),
        json!({"op":"brep_intersect_plane_sphere","first":crate::intersections::tests::plane_patch([-4.,-4.,1.],[8.,0.,0.],[0.,8.,0.]),"second":sphere}),
        json!({"op":"brep_intersect_sphere_cylinder","first":translated(3.),"second":analytic::cylinder(1.,6.).unwrap()}),
        json!({"op":"brep_intersect_sphere_cone","first":translated(3.),"second":analytic::frustum(0.5,2.,6.).unwrap()}),
        json!({"op":"brep_intersect_sphere_torus","first":analytic::sphere_exact(2.5).unwrap(),"second":analytic::torus(3.,1.).unwrap()}),
    ]
}
#[test]
fn authored_nonplanar_frame_guide_affine_spacing_hole_matrix() {
    let (mut passed, mut refused) = (0, 0);
    let mut fixtures = Vec::<Value>::new();
    for mode in [
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
                    for hole in [false, true] {
                        let id =
                            format!("{mode}/guide={guide}/affine={affine}/{spacing}/hole={hole}");
                        let mut loops = vec![wire(
                            [[0., 0.], [105., 0.], [105., 105.], [0., 105.]],
                            [[1., 2.], [2., 4.], [4., 2.], [2., 1.]],
                        )];
                        if hole {
                            loops.push(wire(
                                [[42., 42.], [42., 90.], [90., 90.], [90., 42.]],
                                [[5., 7.], [5., 7.], [7., 5.], [7., 5.]],
                            ));
                        }
                        let mut options = json!({"normal":[0.,1.,0.],"orientation":mode,"spacing":spacing,
                "initialSections":2,"maxSections":2,"maxDeviation":0.001});
                        if guide {
                            options["orientationGuide"] =
                                line([0., 3., 0.], [0., 3., 256.], [1., 1.]);
                        }
                        if affine {
                            options["axisScale"] = vector([0.75, 1.25, 1.]);
                            options["centerLaw"] = vector([0., -0.25, 0.]);
                        }
                        if mode == "authored" {
                            options["frameAxis"] = vector([0., 0., 1.]);
                            options["frameNormal"] = vector([0., 1., 0.]);
                        }
                        let policy = json!({"path":line([0.,0.,0.],[0.,0.,256.],[1.,1.]),"scale":scalar(1.),"twist":scalar(0.),"options":options});
                        let request = profile_request(
                            &policy,
                            &json!(loops.iter().flatten().cloned().collect::<Vec<_>>()),
                        )
                        .unwrap();
                        let before = request.clone();
                        let result = call("surface_progressive_sweep_profiles", request.clone());
                        assert_eq!(request, before, "{id}");
                        if (mode == "frenet" && !guide)
                            || (guide && matches!(mode, "fixed" | "authored"))
                        {
                            let e = result
                                .err()
                                .unwrap_or_else(|| panic!("{id}: unsupported frames accepted"));
                            assert_eq!(e.code, "NURBS_INVALID_INPUT", "{id}: {e:?}");
                            fixtures
                                .push(json!({"id":id,"request":request,"expectedError":e.code}));
                            refused += 1;
                            continue;
                        }
                        let result = result.unwrap_or_else(|e| panic!("{id}: {e:?}"));
                        assert_eq!(
                            result["report"]["accepted"], true,
                            "{id}: {}",
                            result["report"]
                        );
                        assert_eq!(
                            result["report"]["continuousBound"], true,
                            "{id}: {}",
                            result["report"]
                        );
                        let patches: Vec<Surface> =
                            value_codec::from_value(result["patches"].clone()).unwrap();
                        assert_eq!(patches.len(), loops.len() * 4, "{id}");
                        let sides: Vec<Vec<Surface>> =
                            patches.chunks(4).map(|s| s.to_vec()).collect();
                        let start: Vec<Vec<Curve>> = sides
                            .iter()
                            .map(|s| s.iter().map(|s| section(s, 0)).collect())
                            .collect();
                        let end: Vec<Vec<Curve>> = sides
                            .iter()
                            .map(|s| {
                                s.iter()
                                    .map(|s| section(s, s.control_points[0].len() - 1))
                                    .collect()
                            })
                            .collect();
                        let caps = [cap(&start, hole), cap(&end, hole)];
                        let model = brep_core::capped_loft_with_caps(
                            &start,
                            &end,
                            &sides,
                            [&caps[0], &caps[1]],
                        )
                        .unwrap_or_else(|e| panic!("{id}: {e:?}"));
                        let original = model.clone();
                        let proof = brep_core::volume_validity::inspect_with_linear(
                            &model,
                            1e-8,
                            limits(),
                            30000,
                        )
                        .unwrap();
                        assert!(
                            proof.proven,
                            "{id}: embedding={} injective={} pairs={:?}",
                            proof.boundary.proven,
                            proof.boundary.intersections.faces.all_faces_injective,
                            proof
                                .boundary
                                .intersections
                                .pairs
                                .pairs
                                .iter()
                                .filter(|p| p.reason == "pair-unresolved")
                                .map(|p| p.faces)
                                .collect::<Vec<_>>()
                        );
                        let regular =
                            brep_core::boundary_embedding::regularity::inspect(&model, 10000)
                                .unwrap();
                        assert!(
                            regular.all_faces_regular,
                            "{id}: {:?}",
                            regular.unresolved_faces
                        );
                        assert_eq!(model, original, "{id}");
                        fixtures.push(json!({"id":id,"request":request,"model":model,"expectedError":Value::Null}));
                        passed += 1;
                    }
                }
            }
        }
    }
    assert_eq!((passed, refused), (72, 24));
    if let Ok(path) = std::env::var("GLOBAL_GEOMETRY_QUALIFICATION_FIXTURES") {
        let mut budgets = volume_budgets();
        budgets["capFaces"] = json!([]);
        budgets["maxRegularityCells"] = json!(10000);
        std::fs::write(
            path,
            value_codec::to_string(
                &json!({"schema":1,"passed":passed,"refused":refused,"cases":fixtures,"volumeBudgets":budgets,"intersectionCases":intersection_cases()}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
