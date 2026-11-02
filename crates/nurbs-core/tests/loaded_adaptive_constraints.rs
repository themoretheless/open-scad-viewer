use nurbs_core::{
    constrained_surface::{
        Constraint,
        adaptive::{self, loading},
        network::{BoundaryLocks, CurveConstraint},
        variational::{self, Material, Model, PointLoad},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
    surface_edit,
    surface_join::Boundary,
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
fn pins(s: &Surface) -> Vec<Constraint> {
    s.control_points
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
        .collect()
}
fn pc() -> Curve {
    Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
fn quartic() -> Curve {
    Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![i as f64 / 4., 0.5, [0., 0.0125, -1. / 60., 0.0125, 0.][i]])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    }
}
fn check(q: &adaptive::Qualified) {
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted && r.network.loading.is_some());
    let e = q.network().loading().unwrap();
    assert_eq!(e.source(), q.original());
    assert_eq!(e.anchor(), q.anchor());
    assert_eq!(e.surface(), q.surface());
    assert!(
        e.diagnostics().numerical_quadrature_only && e.diagnostics().free_gradient_residual < 1e-7
    );
    assert!(q.loading_work() >= q.network().loading_work() && q.loading_work() <= 100000000);
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} versus {b}");
}
#[test]
fn independent_original_metric_retains_affine_membrane_energy_and_load_work() {
    let reference = plane(1);
    let mut trial = plane(2);
    for p in trial.control_points.iter_mut().flatten() {
        p[0] *= 2.;
    }
    let mut m = Model::default();
    m.material = Material {
        membrane: 3.,
        bending: 0.,
        poisson: 0.25,
    };
    m.traction = [0.2, 0., 0.];
    m.points = vec![PointLoad {
        uv: [0.25, 0.5],
        force: [1., 0., 0.],
    }];
    let q = variational::solve_with_reference(
        &trial,
        &reference,
        &pins(&trial),
        &m,
        variational::Options::default(),
    )
    .unwrap();
    close(
        q.diagnostics().elastic_energy,
        0.5 * 3. / (1. - 0.25 * 0.25),
    );
    close(q.diagnostics().load_work, 0.1 + 0.25);
    close(q.diagnostics().anchor_energy, 0.);
    close(q.diagnostics().total_potential, 1.6 - 0.35);
    assert_eq!(q.source(), &reference);
    assert_eq!(q.anchor(), &trial);
    q.reverify(1).unwrap();
}
#[test]
fn independent_original_curvature_retains_bending_and_dead_pressure_offsets() {
    let reference = plane(1);
    let mut trial = plane(2);
    let height = 0.2;
    for (i, row) in trial.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = if i == 2 { height } else { 0. };
        }
    }
    let mut m = Model::default();
    m.material = Material {
        membrane: 0.,
        bending: 2.,
        poisson: 0.2,
    };
    m.pressure = 3.;
    let q = variational::solve_with_reference(
        &trial,
        &reference,
        &pins(&trial),
        &m,
        variational::Options::default(),
    )
    .unwrap();
    close(
        q.diagnostics().elastic_energy,
        0.5 * 2. / (1. - 0.2 * 0.2) * (2. * height).powi(2),
    );
    close(q.diagnostics().load_work, 3. * height / 3.);
    q.reverify(1).unwrap();
}
#[test]
fn original_and_trial_knot_partitions_both_participate_without_chart_inference() {
    let reference = surface_edit::insert(&plane(2), Axis::U, 0.25, 1).unwrap();
    let trial = surface_edit::insert(&plane(3), Axis::V, 0.75, 1).unwrap();
    let q = variational::solve_with_reference(
        &trial,
        &reference,
        &[],
        &Model::default(),
        variational::Options::default(),
    )
    .unwrap();
    assert_eq!(q.diagnostics().quadrature_sites, 64);
    q.reverify(1).unwrap();
    let mut wrong = reference.clone();
    for k in &mut wrong.knots_u {
        *k *= 2.;
    }
    assert!(
        variational::solve_with_reference(
            &trial,
            &wrong,
            &[],
            &Model::default(),
            variational::Options::default()
        )
        .is_err()
    );
    let mut o = variational::Options::default();
    o.max_sites = 63;
    assert!(
        variational::solve_with_reference(&trial, &reference, &[], &Model::default(), o).is_err()
    );
}
#[test]
fn loaded_hidden_quartic_grows_the_basis_but_keeps_original_physical_rest() {
    let seed = plane(1);
    let c = quartic();
    let p = pc();
    let before = (seed.clone(), c.clone(), p.clone());
    let mut m = Model::default();
    m.pressure = 0.05;
    let mut no = loading::Options::default();
    no.max_refinements = 0;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, no).is_err());
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
    assert!(!q.refinements().is_empty());
    assert!(q.surface().degree_u >= 4);
    assert_eq!(q.network().loading().unwrap().source().degree_u, 1);
    check(&q);
    assert!(q.loading_work() > q.network().loading_work());
    for t in [0., 0.13, 0.25, 0.51, 0.87, 1.] {
        let a = c.evaluate(t).unwrap().point;
        let b = q.surface().evaluate(t, 0.5).unwrap().point;
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-8));
    }
    assert_eq!((seed, c, p), before);
}
#[test]
fn physical_basis_insertion_keeps_the_original_metric_and_full_source_curve() {
    let seed = plane(2);
    let knots = vec![0., 0., 0., 0., 0.4, 1., 1., 1., 1.];
    let c = Curve {
        degree: 3,
        control_points: (0..5)
            .map(|i| {
                vec![
                    knots[i + 1..=i + 3].iter().sum::<f64>() / 3.,
                    0.5,
                    [0., 0.02, -0.01, 0.015, 0.][i],
                ]
            })
            .collect(),
        weights: vec![1.; 5],
        knots,
        periodic: false,
    };
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
    assert!(q.surface().knots_u.iter().any(|k| *k == 0.4));
    assert!(!q.original().knots_u.iter().any(|k| *k == 0.4));
    assert!(
        q.network()
            .loading()
            .unwrap()
            .diagnostics()
            .quadrature_sites
            >= 32
    );
    check(&q);
}
#[test]
fn loaded_degree_growth_keeps_original_four_boundary_g2_and_reference_weights() {
    let mut seed = plane(3);
    for (i, row) in seed.weights.iter_mut().enumerate() {
        for (j, w) in row.iter_mut().enumerate() {
            *w = [1., 0.9, 1.3, 1.][i] * [1., 1.1, 0.95, 1.][j];
        }
    }
    let target = surface_edit::elevate(&seed, Axis::U, 6).unwrap();
    let mut target = surface_edit::elevate(&target, Axis::V, 6).unwrap();
    target.control_points[3][3][2] = 0.01;
    let c = target.iso(Axis::V, 0.5).unwrap();
    let p = pc();
    let sides = [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ];
    let locks = BoundaryLocks {
        reference: &seed,
        boundaries: &sides,
        tolerances: [1e-8, 1e-6, 1e-6],
    };
    let mut m = Model::default();
    m.pressure = 0.01;
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
    assert!(q.surface().degree_u >= 6 && q.surface().degree_v >= 6);
    assert_eq!(q.original_seams().len(), 4);
    assert_eq!(q.original_boundary_reference(), Some(&seed));
    assert_eq!(
        q.network().loading().unwrap().source().weights,
        seed.weights
    );
    check(&q);
    assert!(q.reverify(1, 1).is_err());
}
#[test]
fn adaptive_physical_limits_and_original_control_indices_do_not_publish_reinterpreted_results() {
    let seed = plane(1);
    let before = seed.clone();
    let c = quartic();
    let p = pc();
    let m = Model::default();
    let mut o = loading::Options::default();
    o.max_controls = 4;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    o = loading::Options::default();
    o.network.max_total_assembly_terms = 1;
    assert!(loading::solve(&seed, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    let hard = [Constraint::ControlPoint {
        index: [0, 0],
        point: [0., 0., 0.],
    }];
    assert!(
        loading::solve(
            &seed,
            &[guide(&c, &p)],
            &[],
            &hard,
            None,
            &m,
            loading::Options::default()
        )
        .is_err()
    );
    assert_eq!(seed, before);
}
