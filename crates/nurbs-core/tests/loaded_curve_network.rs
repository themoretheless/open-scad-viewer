use nurbs_core::{
    constrained_surface::{
        Constraint,
        network::{
            self, BoundaryLocks, CurvatureConstraint, CurveConstraint, SupportTrace, loading,
        },
        variational::Model,
    },
    curve::Curve,
    patches,
    surface::Surface,
    surface_join::{Boundary, Decision},
};
fn plane(d: usize) -> Surface {
    patches::bezier(
        (0..=d)
            .map(|i| {
                (0..=d)
                    .map(|j| vec![i as f64 / d as f64, j as f64 / d as f64, 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
fn pc() -> Curve {
    Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap()
}
fn line() -> Curve {
    Curve::from_polyline(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
fn check(q: &network::Qualified) {
    assert!(q.evidence().admitted && q.evidence().loading.is_some());
    let p = q.loading().unwrap();
    assert_eq!(p.surface(), q.surface());
    assert!(
        p.diagnostics().free_gradient_residual < 1e-7 && p.diagnostics().numerical_quadrature_only
    );
    assert!(q.loading_work() >= 2 * p.diagnostics().assembly_terms);
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted && r.loading.is_some());
}
#[test]
fn loaded_hidden_quartic_guide_refines_before_whole_curve_and_equilibrium_admission() {
    let seed = plane(4);
    let before = seed.clone();
    let p = pc();
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
    m.pressure = 0.04;
    let mut scarce = loading::Options::default();
    scarce.curves.network.max_iterations = 1;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, scarce).is_err());
    let q = loading::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        None,
        &m,
        loading::Options::default(),
    )
    .unwrap();
    assert!(q.iterations() > 1 && q.sites()[0].len() > 3);
    check(&q);
    for t in [0., 0.13, 0.25, 0.51, 0.77, 1.] {
        let a = c.evaluate(t).unwrap().point;
        let b = q.surface().evaluate(t, 0.5).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    let zero = loading::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        None,
        &Model::default(),
        loading::Options::default(),
    )
    .unwrap();
    assert!(
        (q.surface().evaluate(0.5, 0.1).unwrap().point[2]
            - zero.surface().evaluate(0.5, 0.1).unwrap().point[2])
            .abs()
            > 1e-6
    );
    assert_eq!(seed, before);
}
#[test]
fn loaded_g1_and_g2_guides_admit_complete_original_support_fields() {
    let seed = plane(4);
    let support = plane(3);
    let c = line();
    let p = pc();
    let mut m = Model::default();
    m.pressure = 0.05;
    let g1 = CurveConstraint {
        tangent: Some(SupportTrace {
            surface: &support,
            parameters: &p,
            reverse: false,
        }),
        ..guide(&c, &p)
    };
    let q1 = loading::solve(
        &seed,
        &[g1],
        &[],
        &[],
        None,
        &m,
        loading::Options::default(),
    )
    .unwrap();
    check(&q1);
    assert!(
        matches!(&q1.evidence().evidence[0],network::Evidence::TangentPlane(r) if r.decision==Decision::WithinTolerance)
    );
    let g2 = CurvatureConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        support: SupportTrace {
            surface: &support,
            parameters: &p,
            reverse: false,
        },
    };
    let q2 = loading::solve(
        &seed,
        &[],
        &[g2],
        &[],
        None,
        &m,
        loading::Options::default(),
    )
    .unwrap();
    check(&q2);
    assert!(
        matches!(&q2.evidence().evidence[0],network::Evidence::Curvature(r) if r.geometric.decision==Decision::WithinTolerance)
    );
    let (_, _, vv1) = q1
        .surface()
        .evaluate(0.5, 0.5)
        .unwrap()
        .second_derivatives()
        .unwrap();
    let (_, _, vv2) = q2
        .surface()
        .evaluate(0.5, 0.5)
        .unwrap()
        .second_derivatives()
        .unwrap();
    assert!(vv1[2].abs() > 1e-4 && vv2[2].abs() < 1e-8);
    assert!(q2.surface().evaluate(0.5, 0.1).unwrap().point[2].abs() > 1e-6);
}
#[test]
fn curved_g2_force_equilibrium_relinearizes_on_the_actual_saved_tangent_metric() {
    let seed = plane(3);
    let mut support = plane(3);
    for row in &mut support.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = 0.0625 * [0.25, -1. / 12., -1. / 12., 0.25][j];
        }
    }
    let c = Curve::from_polyline((0..=4).map(|i| vec![i as f64 / 4., 0.5, 0.]).collect()).unwrap();
    let p = pc();
    let mut m = Model::default();
    m.pressure = 0.03;
    let g = CurvatureConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        support: SupportTrace {
            surface: &support,
            parameters: &p,
            reverse: false,
        },
    };
    let mut o = loading::Options::default();
    o.curves.network.max_iterations = 8;
    o.curves.curvature_tolerance = 1e-3;
    let mut early = o;
    early.curves.network.max_iterations = 2;
    let reason = loading::solve(&seed, &[], &[g], &[], None, &m, early)
        .err()
        .unwrap()
        .to_string();
    assert!(
        reason.contains("equilibrium") || reason.contains("Constraints inconsistent"),
        "{reason}"
    );
    let q = loading::solve(&seed, &[], &[g], &[], None, &m, o).unwrap();
    check(&q);
    assert!(q.iterations() > 2);
    assert!(
        matches!(&q.evidence().evidence[0],network::Evidence::Curvature(r) if r.geometric.decision==Decision::WithinTolerance && r.geometric.regularity_proven)
    );
    // Its frozen final jets must themselves be satisfied on the ACTUAL surface.
    for c in q.loading().unwrap().constraints() {
        if let Constraint::Jet { equation, .. } = c {
            assert!(equation.second.iter().flatten().any(|x| *x != 0.));
        }
    }
}
#[test]
fn loaded_curve_and_four_original_g2_locks_keep_independent_rest_and_boundary_sources() {
    let seed = plane(8);
    let mut reference = seed.clone();
    for p in reference.control_points.iter_mut().flatten() {
        p[2] = 0.01;
    }
    let mut c = line();
    for p in &mut c.control_points {
        p[2] = 0.01;
    }
    let p = pc();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let locks = BoundaryLocks {
        reference: &reference,
        boundaries: &sides,
        tolerances: [1e-8; 3],
    };
    let mut m = Model::default();
    m.pressure = 0.1;
    let q = loading::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        Some(locks),
        &m,
        loading::Options::default(),
    )
    .unwrap();
    check(&q);
    assert_eq!(q.loading().unwrap().source(), &seed);
    assert_eq!(q.boundary_evidence().unwrap().reference(), &reference);
    assert_eq!(q.evidence().boundary_seams.len(), 4);
    assert!(
        q.evidence()
            .boundary_seams
            .iter()
            .all(|(_, r)| r.decision == Decision::WithinTolerance && r.regularity_proven)
    );
    assert!(q.reverify(1, 1).is_err());
}
#[test]
fn all_loaded_proposals_and_actual_equilibrium_share_one_finite_assembly_budget() {
    let seed = plane(3);
    let c = line();
    let p = pc();
    let mut m = Model::default();
    m.pressure = 0.02;
    let q = loading::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        None,
        &m,
        loading::Options::default(),
    )
    .unwrap();
    check(&q);
    let cost = q.loading().unwrap().diagnostics().assembly_terms;
    assert!(cost > 0 && q.loading_work() == 2 * cost);
    let mut o = loading::Options::default();
    o.max_total_assembly_terms = 2 * cost - 1;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    o.max_total_assembly_terms = 2 * cost;
    let again = loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, o).unwrap();
    assert_eq!(again.surface(), q.surface());
    o.max_total_assembly_terms = cost - 1;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
}
#[test]
fn loaded_network_retains_original_inputs_and_conflicts_publish_no_result() {
    let mut seed = plane(3);
    let before = seed.clone();
    let mut c = line();
    let mut p = pc();
    let mut m = Model::default();
    m.pressure = 0.01;
    let q = loading::solve(
        &seed,
        &[guide(&c, &p)],
        &[],
        &[],
        None,
        &m,
        loading::Options::default(),
    )
    .unwrap();
    seed.control_points[0][0][0] = 99.;
    c.control_points[0][2] = 99.;
    p.control_points[0][0] = 99.;
    m.pressure = 99.;
    assert_eq!(q.loading().unwrap().source(), &before);
    assert_eq!(q.loading().unwrap().model().pressure, 0.01);
    check(&q);
    let c = line();
    let p = pc();
    let conflict = [Constraint::Point {
        uv: [0.5, 0.5],
        point: [0.5, 0.5, 1.],
    }];
    assert!(
        loading::solve(
            &before,
            &[guide(&c, &p)],
            &[],
            &conflict,
            None,
            &m,
            loading::Options::default()
        )
        .is_err()
    );
}

#[test]
fn loaded_rational_curve_uses_independent_reversed_clocks_and_original_weights() {
    let mut seed = plane(3);
    for i in 0..4 {
        for j in 0..4 {
            seed.weights[i][j] = 1. + 0.2 * i as f64 + 0.05 * j as f64;
        }
    }
    for k in &mut seed.knots_u {
        *k = 10. + 10. * *k;
    }
    for k in &mut seed.knots_v {
        *k = -3. + 10. * *k;
    }
    let mut c = seed.iso(nurbs_core::surface::Axis::V, 2.).unwrap();
    for p in &mut c.control_points {
        p[2] = 0.02;
    }
    c.control_points.reverse();
    c.weights.reverse();
    for k in &mut c.knots {
        *k = -7. + 20. * (*k - 10.) / 10.;
    }
    let mut p = Curve::from_polyline(vec![vec![10., 2.], vec![20., 2.]]).unwrap();
    for k in &mut p.knots {
        *k = 2. + 3. * *k;
    }
    let g = CurveConstraint {
        spatial: &c,
        parameters: &p,
        reverse: true,
        tangent: None,
    };
    let mut m = Model::default();
    m.pressure = 0.02;
    let q = loading::solve(&seed, &[g], &[], &[], None, &m, loading::Options::default()).unwrap();
    check(&q);
    assert_eq!(q.surface().weights, seed.weights);
    assert_eq!(q.original_guides()[0].spatial, &c);
    for t in [0., 0.17, 0.37, 0.8, 1.] {
        let a = c.evaluate(13. - 20. * t).unwrap().point;
        let b = q.surface().evaluate(10. + 10. * t, 2.).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
}
