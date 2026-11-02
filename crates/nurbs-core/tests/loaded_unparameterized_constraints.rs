use nurbs_core::{
    constrained_surface::{
        Constraint, adaptive,
        network::BoundaryLocks,
        unparameterized::{self, Continuity, Guide, loading},
        variational::Model,
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_edit,
    surface_join::Boundary,
};
fn seed(d: usize) -> Surface {
    patches::bezier(
        (0..=d)
            .map(|i| {
                (0..=d)
                    .map(|j| {
                        vec![
                            i as f64 / d as f64
                                + if i >= 2 {
                                    0.2 * (i * (i - 1)) as f64 / (d * (d - 1)) as f64
                                } else {
                                    0.
                                },
                            j as f64 / d as f64
                                + if j >= 2 {
                                    0.1 * (j * (j - 1)) as f64 / (d * (d - 1)) as f64
                                } else {
                                    0.
                                },
                            0.,
                        ]
                    })
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn g(c: &Curve) -> Guide<'_> {
    Guide {
        spatial: c,
        continuity: Continuity::Position,
    }
}
fn check(q: &unparameterized::Qualified) {
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted && r.loading.is_some());
    assert!(q.inversions() > 0);
    let e = q.network().loading().unwrap();
    assert_eq!(e.anchor(), q.original());
    assert_eq!(Some(e.source()), q.physical_reference());
    assert!(q.loading_work() >= q.network().loading_work());
    assert!(e.diagnostics().free_gradient_residual < 1e-7);
}
#[test]
fn inferred_loaded_curve_on_nonlinear_chart_keeps_original_rest_and_whole_geometry() {
    let s = seed(2);
    let mut target = s.clone();
    for (i, row) in target.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = [0.01, 0.01, 0.04][i];
        }
    }
    let c = target.iso(Axis::V, 0.4).unwrap();
    let before = (s.clone(), c.clone());
    let mut m = Model::default();
    m.pressure = 0.02;
    let q = loading::solve(&s, &[g(&c)], &[], None, &m, loading::Options::default()).unwrap();
    check(&q);
    assert_eq!(q.physical_reference(), Some(&s));
    let guides = q.network().original_guides();
    let pc = guides[0].parameters;
    let d = pc.domain();
    for t in [0., 0.13, 0.5, 0.89, 1.] {
        let uv = pc.evaluate(d[0] + t * (d[1] - d[0])).unwrap().point;
        let a = c.evaluate(t).unwrap().point;
        let b = q.surface().evaluate(uv[0], uv[1]).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    assert_eq!((s, c), before);
}
#[test]
fn loaded_inferred_g1_g2_supports_use_independent_transposed_opposite_clocks() {
    let s = seed(2);
    let mut support = s.clone();
    support.control_points = (0..3)
        .map(|i| (0..3).map(|j| s.control_points[j][i].clone()).collect())
        .collect();
    for k in &mut support.knots_u {
        *k = 10. + 10. * *k;
    }
    for k in &mut support.knots_v {
        *k = -3. + 4. * *k;
    }
    let c = s.iso(Axis::V, 0.4).unwrap();
    let mut m = Model::default();
    m.pressure = 0.01;
    for continuity in [
        Continuity::Tangent(&support),
        Continuity::Curvature(&support),
    ] {
        let q = loading::solve(
            &s,
            &[Guide {
                spatial: &c,
                continuity,
            }],
            &[],
            None,
            &m,
            loading::Options::default(),
        )
        .unwrap();
        check(&q);
        let guides = q.network().original_guides();
        let p = guides[0].tangent.unwrap().parameters;
        let d = p.domain();
        let a = p.evaluate(d[0]).unwrap().point;
        let b = p.evaluate(d[1]).unwrap().point;
        assert!((a[0] - 14.).abs() < 1e-7 && (b[0] - 14.).abs() < 1e-7);
        assert!((a[1] + 3.).abs() < 1e-7 && (b[1] - 1.).abs() < 1e-7);
    }
}
#[test]
fn inferred_curved_support_requires_actual_transverse_g2_under_load() {
    let s = seed(3);
    let mut support = s.clone();
    for row in &mut support.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = 0.08 * [0.25, -1. / 12., -1. / 12., 0.25][j];
        }
    }
    let c = support.iso(Axis::V, 0.5).unwrap();
    let mut m = Model::default();
    m.pressure = 0.01;
    let mut o = loading::Options::default();
    o.network.curves.network.max_iterations = 8;
    let q = loading::solve(
        &s,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Curvature(&support),
        }],
        &[],
        None,
        &m,
        o,
    )
    .unwrap();
    check(&q);
    assert_eq!(q.network().original_curvature_guides().len(), 1);
    assert!(
        q.surface()
            .evaluate(0.5, 0.5)
            .unwrap()
            .second_derivatives()
            .unwrap()
            .2[2]
            > 0.01
    );
}
#[test]
fn inferred_physical_uvs_and_basis_growth_keep_one_original_rest() {
    let s = seed(1);
    let c = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![i as f64 / 4., 0.5, [0., 0.0125, -1. / 60., 0.0125, 0.][i]])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let mut m = Model::default();
    m.pressure = 0.02;
    let q = adaptive::loading::solve_unparameterized(
        &s,
        &[g(&c)],
        &[],
        None,
        &m,
        adaptive::loading::Options::default(),
        adaptive::Inversion::default(),
    )
    .unwrap();
    assert!(q.surface().degree_u >= 4 && !q.refinements().is_empty());
    assert_eq!(q.network().loading().unwrap().source(), &s);
    assert_eq!(q.network().loading().unwrap().anchor(), q.anchor());
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted && r.network.loading.is_some());
    assert!(q.loading_work() >= q.network().loading_work() && q.loading_work() <= 100000000);
}
#[test]
fn adaptive_inferred_loads_preserve_four_original_g2_seams() {
    let s = seed(3);
    let t = surface_edit::elevate(&s, Axis::U, 6).unwrap();
    let mut t = surface_edit::elevate(&t, Axis::V, 6).unwrap();
    t.control_points[3][3][2] = 0.005;
    let c = t.iso(Axis::V, 0.5).unwrap();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let locks = BoundaryLocks {
        reference: &s,
        boundaries: &sides,
        tolerances: [1e-8, 1e-6, 1e-6],
    };
    let mut m = Model::default();
    m.pressure = 0.01;
    let q = adaptive::loading::solve_unparameterized(
        &s,
        &[g(&c)],
        &[],
        Some(locks),
        &m,
        adaptive::loading::Options::default(),
        adaptive::Inversion::default(),
    )
    .unwrap();
    assert!(q.surface().degree_u >= 6 && q.surface().degree_v >= 6);
    assert_eq!(q.original_seams().len(), 4);
    assert_eq!(q.original_boundary_reference(), Some(&s));
    assert_eq!(q.network().loading().unwrap().source(), &s);
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
}
#[test]
fn inferred_rounds_share_physical_budget_and_freeze_original_inputs() {
    let mut s = seed(2);
    let mut c = s.iso(Axis::V, 0.4).unwrap();
    for p in &mut c.control_points {
        p[2] = 0.01;
    }
    let before = s.clone();
    let mut m = Model::default();
    m.pressure = 0.01;
    let mut o = loading::Options::default();
    o.max_rounds = 1;
    let q = loading::solve(&s, &[g(&c)], &[], None, &m, o).unwrap();
    check(&q);
    let cost = q.loading_work();
    assert!(cost > 0);
    o.network.max_total_assembly_terms = cost - 1;
    assert!(loading::solve(&s, &[g(&c)], &[], None, &m, o).is_err());
    o.network.max_total_assembly_terms = cost;
    let again = loading::solve(&s, &[g(&c)], &[], None, &m, o).unwrap();
    assert_eq!(q.surface(), again.surface());
    s.control_points[0][0][0] = 99.;
    c.control_points[0][2] = 99.;
    m.pressure = 99.;
    assert_eq!(q.physical_reference(), Some(&before));
    assert_eq!(q.network().loading().unwrap().model().pressure, 0.01);
    check(&q);
    let hard = [Constraint::ControlPoint {
        index: [0, 0],
        point: [f64::NAN, 0., 0.],
    }];
    assert!(
        loading::solve(
            &before,
            &[g(&c)],
            &hard,
            None,
            &m,
            loading::Options::default()
        )
        .is_err()
    );
}

#[test]
fn nonlinear_inverse_refinement_charges_failed_physical_rounds_without_moving_locked_controls() {
    let mut s = seed(2);
    for (i, row) in s.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[0] = [0.25, 0.5, 1.][i];
            p[1] = j as f64 / 2.;
        }
    }
    let hard = s
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(i, r)| {
            r.iter()
                .enumerate()
                .map(move |(j, p)| Constraint::ControlPoint {
                    index: [i, j],
                    point: [p[0], p[1], p[2]],
                })
        })
        .collect::<Vec<_>>();
    let c = Curve::from_polyline(vec![vec![0.25, 0.5, 0.], vec![1., 0.5, 0.]]).unwrap();
    let mut o = loading::Options::default();
    o.network.curves.network.fitting.position_tolerance = 1e-3;
    let mut m = Model::default();
    m.pressure = 0.01;
    let q = loading::solve(&s, &[g(&c)], &hard, None, &m, o).unwrap();
    check(&q);
    assert!(q.rounds() > 1);
    assert_eq!(q.surface(), &s);
    assert!(
        q.loading_work() > q.network().loading_work()
            && q.loading_work() <= o.network.max_total_assembly_terms
    );
    let mut scarce = o;
    scarce.max_rounds = 1;
    assert!(loading::solve(&s, &[g(&c)], &hard, None, &m, scarce).is_err());
}
