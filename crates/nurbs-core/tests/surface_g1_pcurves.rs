use nurbs_core::{curve::Curve, surface::Surface, surface_g1_pcurves, surface_join::Decision};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn whole_pcurve_normals_admit_regular_planes_and_refuse_hidden_interior_tilt() {
    let a = plane();
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let good = surface_g1_pcurves::inspect(
        surface_g1_pcurves::PcurveSeam {
            curve: &c,
            pcurves: [&pc, &pc],
            surfaces: [&a, &a],
            reversed: [false; 2],
        },
        1e-7,
        1e-5,
        10000,
        1000000,
    )
    .unwrap();
    assert_eq!(good.decision, Decision::WithinTolerance);
    let reversed = nurbs_core::surface_edit::reverse(&a, nurbs_core::surface::Axis::V).unwrap();
    let opposite_pc = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
    let opposite = surface_g1_pcurves::inspect(
        surface_g1_pcurves::PcurveSeam {
            curve: &c,
            pcurves: [&pc, &opposite_pc],
            surfaces: [&a, &reversed],
            reversed: [false; 2],
        },
        1e-7,
        1e-5,
        10000,
        1000000,
    )
    .unwrap();
    assert_eq!(opposite.decision, Decision::WithinTolerance);
    assert_eq!(
        opposite.orientation,
        Some(nurbs_core::surface_g2::Orientation::Opposite)
    );
    let mut b = a.clone();
    b.degree_u = 4;
    b.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    b.weights = vec![vec![1.; 2]; 5];
    b.control_points = (0..5)
        .map(|i| {
            vec![
                vec![i as f64 / 4., 0., 0.],
                vec![i as f64 / 4., 1., if i == 2 { 0.01 / 6. } else { 0. }],
            ]
        })
        .collect();
    for u in [0., 1.] {
        let n = b.evaluate(u, 0.).unwrap().unit_normal().unwrap();
        assert!((n[2] - 1.).abs() < 1e-12);
    }
    let n = b.evaluate(0.5, 0.).unwrap().unit_normal().unwrap();
    assert!(n[1].abs() > 1e-4);
    let bad = surface_g1_pcurves::inspect(
        surface_g1_pcurves::PcurveSeam {
            curve: &c,
            pcurves: [&pc, &pc],
            surfaces: [&a, &b],
            reversed: [false; 2],
        },
        1e-7,
        1e-5,
        256,
        1000000,
    )
    .unwrap();
    assert_ne!(bad.decision, Decision::WithinTolerance);
    assert_eq!(bad.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(bad.violation_parameter.is_some());
    assert!(bad.normal_chord_upper.is_none());
    let scarce = surface_g1_pcurves::inspect(
        surface_g1_pcurves::PcurveSeam {
            curve: &c,
            pcurves: [&pc, &pc],
            surfaces: [&a, &a],
            reversed: [false; 2],
        },
        1e-7,
        1e-5,
        good.cells - 1,
        1000000,
    )
    .unwrap();
    assert_ne!(scarce.decision, Decision::WithinTolerance);
}

#[test]
fn first_jet_rectangle_bounds_cover_all_one_sided_multispan_normals() {
    let mut s = plane();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.];
    s.weights = vec![vec![1.; 2]; 5];
    s.control_points = (0..5)
        .map(|i| vec![vec![i as f64 / 4., 0., 0.], vec![i as f64 / 4., 1., 0.]])
        .collect();
    let old = nurbs_core::surface_shape_operator::rectangle_bounds(&s, [[0., 1.], [0., 1.]], 100)
        .unwrap();
    assert!(old.shape.normal.is_none());
    let r = surface_g1_pcurves::rectangle_normal_bounds(&s, [[0., 1.], [0., 1.]], 100).unwrap();
    let n = r.normal.unwrap();
    assert_eq!(r.cells, 2);
    assert!(n[2][0] <= 1. && n[2][1] >= 1. && n[2][1] - n[2][0] < 1e-12);
    assert!(s.evaluate(0.5, 0.37).unwrap().unit_normal().is_none());
    let join =
        surface_g1_pcurves::rectangle_normal_bounds(&s, [[0.5, 0.5], [0.37, 0.37]], 100).unwrap();
    assert!(join.normal.unwrap()[2][0] <= 1.);
    for u in [0., 0.13, 0.49, 0.51, 0.79, 1.] {
        let sample = s.evaluate(u, 0.37).unwrap().unit_normal().unwrap();
        for k in 0..3 {
            assert!(n[k][0] <= sample[k] && sample[k] <= n[k][1]);
        }
    }
    let scarce = surface_g1_pcurves::rectangle_normal_bounds(&s, [[0., 1.], [0., 1.]], 1).unwrap();
    assert!(scarce.normal.is_none());
    assert_eq!(scarce.cells, 1);
}

#[test]
fn oblique_original_graph_normals_keep_correlation_and_cover_curvature() {
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    for curved in [false, true] {
        let mut s = plane();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.weights = vec![vec![1.; 2]; 3];
        s.control_points = (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        let p = [
                            i as f64 / 2.,
                            j as f64 * if i == 2 { 2. } else { 1. },
                            if curved && i == 2 { 0.2 } else { 0. },
                        ];
                        (0..3)
                            .map(|k| 11. + (0..3).map(|a| rotation[k][a] * p[a]).sum::<f64>())
                            .collect()
                    })
                    .collect()
            })
            .collect();
        for reverse in [false, true] {
            let source = if reverse {
                nurbs_core::surface_edit::reverse(&s, nurbs_core::surface::Axis::V).unwrap()
            } else {
                s.clone()
            };
            let n = surface_g1_pcurves::rectangle_normal_bounds(&source, [[0., 1.], [0., 1.]], 100)
                .unwrap()
                .normal
                .unwrap();
            for u in [0., 0.13, 0.5, 0.77, 1.] {
                let raw: [f64; 3] = if curved {
                    [-0.4 * u, 0., 1.]
                } else {
                    [0., 0., 1.]
                };
                let length = (raw[0] * raw[0] + 1.).sqrt();
                for k in 0..3 {
                    let expected = (0..3).map(|a| rotation[k][a] * raw[a]).sum::<f64>() / length
                        * if reverse { -1. } else { 1. };
                    assert!(
                        n[k][0] <= expected && expected <= n[k][1],
                        "{expected} outside {:?}",
                        n[k]
                    );
                    if !curved {
                        assert!(n[k][1] - n[k][0] < 1e-10);
                    }
                }
            }
        }
    }
}

#[test]
fn curved_shared_clock_normal_polynomials_require_fresh_work_and_coherent_orientation() {
    let e = [[1., 0.], [1., 1.], [0., 1.]];
    let w = [1., 0.5f64.sqrt(), 1.];
    let torus = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: e
            .iter()
            .map(|p| {
                [(0., -1.), (1., -1.), (1., 0.)]
                    .iter()
                    .map(|(x, z)| vec![(3. + 0.5 * x) * p[0], (3. + 0.5 * x) * p[1], 0.5 + 0.5 * z])
                    .collect()
            })
            .collect(),
        weights: w
            .iter()
            .map(|u| w.iter().map(|v| u * v).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let cylinder = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: torus.knots_u.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: e
            .iter()
            .map(|p| {
                vec![
                    vec![3.5 * p[0], 3.5 * p[1], 0.],
                    vec![3.5 * p[0], 3.5 * p[1], 1.],
                ]
            })
            .collect(),
        weights: w.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let c = torus.iso(nurbs_core::surface::Axis::V, 1.).unwrap();
    let p = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
    let q = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
    for opposite in [false, true] {
        for reversed in [false, true] {
            let other = if opposite {
                nurbs_core::surface_edit::reverse(&cylinder, nurbs_core::surface::Axis::V).unwrap()
            } else {
                cylinder.clone()
            };
            let p = if reversed {
                p.reverse().unwrap()
            } else {
                p.clone()
            };
            let q = if reversed {
                q.reverse().unwrap()
            } else {
                q.clone()
            };
            let out = surface_g1_pcurves::inspect(
                surface_g1_pcurves::PcurveSeam {
                    curve: &c,
                    pcurves: [&p, &q],
                    surfaces: [&torus, &other],
                    reversed: [reversed; 2],
                },
                1e-7,
                1e-5,
                256,
                100000,
            )
            .unwrap();
            assert_eq!(out.decision, Decision::WithinTolerance);
            assert!(out.positions.proven && out.regularity_proven);
            assert!(out.normal_work > 0 && out.work() <= 100000);
            assert_eq!(
                out.orientation,
                Some(if opposite {
                    nurbs_core::surface_g2::Orientation::Opposite
                } else {
                    nurbs_core::surface_g2::Orientation::Same
                })
            );
            let scarce = surface_g1_pcurves::inspect(
                surface_g1_pcurves::PcurveSeam {
                    curve: &c,
                    pcurves: [&p, &q],
                    surfaces: [&torus, &other],
                    reversed: [reversed; 2],
                },
                1e-7,
                1e-5,
                256,
                out.positions.exact_work,
            )
            .unwrap();
            assert_ne!(scarce.decision, Decision::WithinTolerance);
        }
    }
}
