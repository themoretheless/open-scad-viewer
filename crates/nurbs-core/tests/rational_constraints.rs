use nurbs_core::{
    constrained_surface::{
        self, Constraint,
        network::{self, BoundaryLocks, CurvatureConstraint, CurveConstraint, SupportTrace},
        rational::{self, Options},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_join::{Boundary, Decision},
};
fn circle_strip() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[1., 0.], [1., 1.], [0., 1.]]
            .into_iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5f64.sqrt(); 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn pc(v: f64) -> Curve {
    Curve::from_polyline(vec![vec![0., v], vec![1., v]]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
fn plane(degree: usize) -> Surface {
    patches::bezier(
        (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| vec![i as f64 / degree as f64, j as f64 / degree as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
#[test]
fn positive_variable_weights_recover_a_circle_unavailable_to_the_fixed_basis() {
    let target = circle_strip();
    let mut seed = target.clone();
    seed.weights = vec![vec![1.; 2]; 3];
    let a = target.iso(Axis::V, 0.).unwrap();
    let b = target.iso(Axis::V, 1.).unwrap();
    let pa = pc(0.);
    let pb = pc(1.);
    let guides = [guide(&a, &pa), guide(&b, &pb)];
    let before = (seed.clone(), target.clone(), a.clone(), b.clone());
    assert!(network::solve(&seed, &guides, &[], network::Options::default()).is_err());
    let mut options = Options::default();
    options.weight_bounds = [0.7, 1.];
    let out = rational::solve(&seed, &guides, &[], &[], None, options).unwrap();
    assert_eq!(out.surface().knots_u, seed.knots_u);
    assert_eq!(out.surface().knots_v, seed.knots_v);
    assert_eq!(out.surface().weights[0][0], 1.);
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .all(|w| *w >= 0.7 && *w <= 1.)
    );
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 1.).abs() > 0.1)
    );
    assert!(out.active_weight_bounds() > 0);
    assert!(out.iterations() > 1);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    for u in [0., 0.13, 0.25, 0.5, 0.72, 1.] {
        for v in [0., 1.] {
            let p = out.surface().evaluate(u, v).unwrap().point;
            assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-7);
            let q = target.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-8));
        }
    }
    assert_eq!((seed, target, a, b), before);
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
}
#[test]
fn weight_optimization_combines_full_original_g2_with_a_rational_parameter_clock() {
    let target = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0.25, 1.]
            .into_iter()
            .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let mut seed = target.clone();
    seed.weights = vec![vec![1.; 2]; 3];
    let p = pc(0.5);
    let c = target.iso(Axis::V, 0.5).unwrap();
    let curv = [CurvatureConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        support: SupportTrace {
            surface: &target,
            parameters: &p,
            reverse: false,
        },
    }];
    let out = rational::solve(&seed, &[], &curv, &[], None, Options::default()).unwrap();
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 1.).abs() > 1e-3)
    );
    let network::Evidence::Curvature(report) = &out.network().evidence().evidence[0] else {
        panic!("Missing whole G2")
    };
    assert_eq!(report.geometric.decision, Decision::WithinTolerance);
    assert!(report.positions.proven && report.geometric.regularity_proven);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}
#[test]
fn free_interior_weights_preserve_all_four_original_boundary_two_jets() {
    let seed = plane(6);
    let mut target = seed.clone();
    target.weights[3][3] = 2.;
    target.control_points[3][3][2] = 0.1;
    let p = pc(0.5);
    let c = target.iso(Axis::V, 0.5).unwrap();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let out = rational::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        Some(BoundaryLocks {
            reference: &seed,
            boundaries: &sides,
            tolerances: [1e-8, 1e-6, 1e-6],
        }),
        Options::default(),
    )
    .unwrap();
    assert!((out.surface().weights[3][3] - 2.).abs() < 1e-5);
    let replay = out.reverify(100000, 1000000).unwrap();
    assert!(replay.admitted && replay.boundary_seams.len() == 4);
    for i in 0..7 {
        for j in 0..7 {
            if i != 3 || j != 3 {
                assert_eq!(out.surface().weights[i][j], seed.weights[i][j]);
                assert_eq!(
                    out.surface().control_points[i][j],
                    seed.control_points[i][j]
                );
            }
        }
    }
    for t in [0., 0.13, 0.5, 0.83, 1.] {
        for uv in [[0., t], [1., t], [t, 0.], [t, 1.]] {
            let a = seed.evaluate(uv[0], uv[1]).unwrap();
            let b = out.surface().evaluate(uv[0], uv[1]).unwrap();
            assert!(
                a.point
                    .iter()
                    .zip(b.point)
                    .all(|(a, b)| (a - b).abs() < 1e-11)
            );
            let (au, av) = a.first_derivatives().unwrap();
            let (bu, bv) = b.first_derivatives().unwrap();
            assert!(
                au.iter()
                    .chain(&av)
                    .zip(bu.iter().chain(&bv))
                    .all(|(a, b)| (a - b).abs() < 1e-11)
            );
            let (aa, ab, bb) = a.second_derivatives().unwrap();
            let (cc, cd, dd) = b.second_derivatives().unwrap();
            assert!(
                aa.iter()
                    .chain(&ab)
                    .chain(&bb)
                    .zip(cc.iter().chain(&cd).chain(&dd))
                    .all(|(a, b)| (a - b).abs() < 1e-11)
            );
        }
    }
}
#[test]
fn infeasible_weight_boxes_and_invalid_guides_publish_no_geometry() {
    let target = circle_strip();
    let mut seed = target.clone();
    seed.weights = vec![vec![1.; 2]; 3];
    let before = seed.clone();
    let p = pc(0.);
    let c = target.iso(Axis::V, 0.).unwrap();
    let guides = [guide(&c, &p)];
    let mut options = Options::default();
    options.weight_bounds = [0.8, 1.];
    assert!(rational::solve(&seed, &guides, &[], &[], None, options).is_err());
    options = Options::default();
    options.weight_bounds = [0., 2.];
    assert!(rational::solve(&seed, &guides, &[], &[], None, options).is_err());
    options = Options::default();
    options.gauge = [999, 0];
    assert!(rational::solve(&seed, &guides, &[], &[], None, options).is_err());
    options = Options::default();
    options.network.network.max_cells = 1;
    assert!(rational::solve(&seed, &guides, &[], &[], None, options).is_err());
    let bad = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    assert!(
        rational::solve(
            &seed,
            &[guide(&bad, &p)],
            &[],
            &[],
            None,
            Options::default()
        )
        .is_err()
    );
    assert_eq!(seed, before);
}
#[test]
fn scalar_hard_jet_rows_are_rechecked_after_homogeneous_updates() {
    let seed = plane(3);
    let c = Curve::from_polyline(vec![vec![0., 0.5, 0.1], vec![1., 0.5, 0.2]]).unwrap();
    let p = pc(0.5);
    let hard = [Constraint::Jet {
        uv: [0.5, 0.5],
        equation: constrained_surface::JetEquation {
            first: [[0., 0., 1.], [0.; 3]],
            second: [[0.; 3]; 3],
            value: 0.1,
        },
    }];
    let out = rational::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &hard,
        None,
        Options::default(),
    )
    .unwrap();
    assert!(
        (out.surface()
            .evaluate(0.5, 0.5)
            .unwrap()
            .first_derivatives()
            .unwrap()
            .0[2]
            - 0.1)
            .abs()
            < 1e-8
    );
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
}

#[test]
fn nonlinear_second_jet_is_enforced_on_the_final_rational_geometry() {
    let target = circle_strip();
    let mut seed = target.clone();
    seed.weights = vec![vec![1.; 2]; 3];
    let a = target.iso(Axis::V, 0.).unwrap();
    let b = target.iso(Axis::V, 1.).unwrap();
    let pa = pc(0.);
    let pb = pc(1.);
    let value = target
        .evaluate(0.5, 0.)
        .unwrap()
        .second_derivatives()
        .unwrap()
        .0[0];
    let hard = |value| {
        [Constraint::Jet {
            uv: [0.5, 0.],
            equation: constrained_surface::JetEquation {
                first: [[0.; 3]; 2],
                second: [[1., 0., 0.], [0.; 3], [0.; 3]],
                value,
            },
        }]
    };
    let guides = [guide(&a, &pa), guide(&b, &pb)];
    let out = rational::solve(&seed, &guides, &[], &hard(value), None, Options::default()).unwrap();
    let actual = out
        .surface()
        .evaluate(0.5, 0.)
        .unwrap()
        .second_derivatives()
        .unwrap()
        .0[0];
    assert!((actual - value).abs() < 1e-8);
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    assert!(rational::solve(&seed, &guides, &[], &hard(99.), None, Options::default()).is_err());
}

#[test]
fn reversed_nonunit_clocks_keep_the_original_projective_gauge() {
    let mut target = circle_strip();
    for w in target.weights.iter_mut().flatten() {
        *w *= 3.;
    }
    for u in &mut target.knots_u {
        *u = 10. + 10. * *u;
    }
    let mut seed = target.clone();
    seed.weights = vec![vec![3.; 2]; 3];
    let c = target.iso(Axis::V, 0.5).unwrap().reverse().unwrap();
    let mut p = Curve::from_polyline(vec![vec![10., 0.5], vec![20., 0.5]]).unwrap();
    for t in &mut p.knots {
        *t = -2. + 5. * *t;
    }
    let mut g = guide(&c, &p);
    g.reverse = true;
    let out = rational::solve(&seed, &[g], &[], &[], None, Options::default()).unwrap();
    assert_eq!(out.surface().weights[0][0], 3.);
    assert_eq!(out.surface().knots_u, seed.knots_u);
    assert!(
        out.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 3.).abs() > 0.1)
    );
    assert!(out.reverify(100000, 1000000).unwrap().admitted);
    for i in 0..=20 {
        let u = 10. + i as f64 / 2.;
        let a = out.surface().evaluate(u, 0.5).unwrap();
        let b = target.evaluate(u, 0.5).unwrap();
        assert!(
            a.point
                .iter()
                .zip(&b.point)
                .all(|(a, b)| (a - b).abs() < 1e-6)
        );
    }
}
