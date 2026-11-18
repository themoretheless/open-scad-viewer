use nurbs_core::{
    constrained_surface::{
        network::{self, BoundaryLocks, CurvatureConstraint, CurveConstraint, SupportTrace},
        rational::loading,
        variational::{Material, Model, PointLoad},
    },
    curve::Curve,
    patches,
    surface::{Axis, Surface},
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
fn check(q: &loading::Qualified) {
    let r = q.reverify(100000, 1000000).unwrap();
    assert!(r.admitted && r.loading.is_some());
    assert!(!q.report().joint_weight_stationarity_proven);
    assert!(q.merit_trace().windows(2).all(|w| w[1] < w[0]));
    assert!(q.work().0 <= 100000 && q.work().1 <= 1000000 && q.loading_work() <= 100000000);
}
#[test]
fn admitted_weight_merit_search_matches_an_independent_point_force_objective() {
    let s = plane(2);
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let p = pc(0.);
    let m = Model {
        material: Material {
            membrane: 0.,
            bending: 0.,
            poisson: 0.3,
        },
        points: vec![PointLoad {
            uv: [0.5, 0.5],
            force: [0., 0., 1.],
        }],
        ..Model::default()
    };
    let mut o = loading::Options {
        variable_weights: vec![[1, 1]],
        ..loading::Options::default()
    };
    o.geometry.weight_bounds = [0.5, 2.];
    o.geometry.network.network.fitting.anchor_weight = 0.2;
    let q = loading::solve(&s, &[guide(&c, &p)], &[], &[], None, &m, o).unwrap();
    check(&q);
    assert!(q.report().accepted > 0 && q.report().admitted > 0);
    assert_eq!(q.surface().weights[1][1], 2.);
    for i in 0..3 {
        for j in 0..3 {
            if [i, j] != [1, 1] {
                assert_eq!(q.surface().weights[i][j], 1.);
            }
        }
    }
    // Only v>0 controls can move normally. Their squared rational basis sum
    // at (.5,.5) is ((w/4)^2+7/128)/(3/4+w/4)^2.
    let w = q.surface().weights[1][1];
    let basis2 = (w * w / 16. + 7. / 128.) / (0.75 + w / 4.).powi(2);
    let expected = -0.5 * basis2 / 0.2 + 0.5 * 0.001 * w.ln().powi(2);
    assert!((q.report().final_merit - expected).abs() < 1e-10);
    assert!(q.report().final_merit < q.merit_trace()[0] - 0.1);
    assert!(q.report().final_log_step < 0.015625 && !q.report().trial_budget_exhausted);
}
fn circle() -> Surface {
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
#[test]
fn original_circle_unavailable_to_fixed_weights_receives_original_rest_physics() {
    let target = circle();
    let mut s = target.clone();
    s.weights = vec![vec![1.; 2]; 3];
    let a = target.iso(Axis::V, 0.).unwrap();
    let b = target.iso(Axis::V, 1.).unwrap();
    let pa = pc(0.);
    let pb = pc(1.);
    let mut o = loading::Options {
        variable_weights: vec![[1, 0], [1, 1]],
        max_weight_trials: 0,
        ..loading::Options::default()
    };
    o.geometry.weight_bounds = [0.7, 1.];
    let m = Model {
        pressure: 0.01,
        ..Model::default()
    };
    let q = loading::solve(&s, &[guide(&a, &pa), guide(&b, &pb)], &[], &[], None, &m, o).unwrap();
    check(&q);
    assert!(q.fitting().iterations() > 0);
    assert_eq!(q.network().loading().unwrap().source(), &s);
    assert!(
        q.surface().weights[1]
            .iter()
            .all(|w| (*w - 0.5f64.sqrt()).abs() < 1e-7)
    );
    assert_eq!(q.surface().weights[0], vec![1.; 2]);
    assert_eq!(q.surface().weights[2], vec![1.; 2]);
    assert!(q.network().loading().unwrap().diagnostics().elastic_energy > 1e-6);
}
#[test]
fn variable_weights_combine_original_g2_support_clock_and_loaded_equilibrium() {
    let mut target = plane(2);
    for (i, row) in target.control_points.iter_mut().enumerate() {
        for p in row {
            p[0] = [0., 0.25, 1.][i];
        }
    }
    target.weights = vec![vec![1.; 3], vec![0.5; 3], vec![1.; 3]];
    let mut s = target.clone();
    s.weights = vec![vec![1.; 3]; 3];
    let c = target.iso(Axis::V, 0.5).unwrap();
    let p = pc(0.5);
    let g = CurvatureConstraint {
        spatial: &c,
        parameters: &p,
        reverse: false,
        support: SupportTrace {
            surface: &target,
            parameters: &p,
            reverse: false,
        },
    };
    let mut o = loading::Options {
        max_weight_trials: 0,
        ..loading::Options::default()
    };
    o.geometry.weight_bounds = [0.4, 1.5];
    let m = Model {
        pressure: 0.01,
        ..Model::default()
    };
    let q = loading::solve(&s, &[], &[g], &[], None, &m, o).unwrap();
    check(&q);
    assert!(
        matches!(&q.network().evidence().evidence[0],network::Evidence::Curvature(r) if r.geometric.decision==Decision::WithinTolerance)
    );
    assert!(
        q.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 1.).abs() > 0.1)
    );
}
#[test]
fn free_interior_weights_retain_original_four_boundary_layers_and_g2() {
    let s = plane(6);
    let mut target = s.clone();
    target.weights[3][3] = 0.75;
    target.control_points[3][3][2] = 0.02;
    let c = target.iso(Axis::V, 0.5).unwrap();
    let p = pc(0.5);
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
    let mut o = loading::Options {
        variable_weights: vec![[3, 3]],
        max_weight_trials: 0,
        ..loading::Options::default()
    };
    o.geometry.weight_bounds = [0.5, 1.5];
    let m = Model {
        pressure: 0.01,
        ..Model::default()
    };
    let q = loading::solve(&s, &[guide(&c, &p)], &[], &[], Some(locks), &m, o).unwrap();
    check(&q);
    assert!((q.surface().weights[3][3] - 0.75).abs() < 1e-6);
    assert_eq!(q.network().evidence().boundary_seams.len(), 4);
    for i in 0..7 {
        for j in 0..7 {
            if [i, j] != [3, 3] {
                assert_eq!(q.surface().weights[i][j], 1.);
                assert_eq!(q.surface().control_points[i][j], s.control_points[i][j]);
            }
        }
    }
}
#[test]
fn weight_masks_gauge_boxes_and_exhausted_work_refuse_without_changing_inputs() {
    let s = plane(2);
    let before = s.clone();
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let p = pc(0.);
    let m = Model::default();
    for mask in [vec![[0, 0]], vec![[99, 0]], vec![[1, 1], [1, 1]]] {
        let o = loading::Options {
            variable_weights: mask,
            ..loading::Options::default()
        };
        assert!(loading::solve(&s, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    }
    let o = loading::Options {
        max_total_assembly_terms: 1,
        ..loading::Options::default()
    };
    assert!(loading::solve(&s, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    let mut o = loading::Options::default();
    o.geometry.weight_bounds = [1.1, 2.];
    assert!(loading::solve(&s, &[guide(&c, &p)], &[], &[], None, &m, o).is_err());
    assert_eq!(s, before);
}

#[test]
fn interrupted_merit_search_returns_qualified_geometry_with_explicit_unfinished_status() {
    let mut s = plane(2);
    let original = s.clone();
    let mut c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let p = pc(0.);
    let mut m = Model {
        material: Material {
            membrane: 0.,
            bending: 0.,
            poisson: 0.3,
        },
        points: vec![PointLoad {
            uv: [0.5, 0.5],
            force: [0., 0., 1.],
        }],
        ..Model::default()
    };
    let mut o = loading::Options {
        variable_weights: vec![[1, 1]],
        max_weight_trials: 1,
        ..loading::Options::default()
    };
    o.geometry.weight_bounds = [0.5, 2.];
    let q = loading::solve(&s, &[guide(&c, &p)], &[], &[], None, &m, o).unwrap();
    assert_eq!(q.report().attempted, 1);
    assert!(q.report().trial_budget_exhausted && !q.report().joint_weight_stationarity_proven);
    assert_eq!(q.report().accepted, 1);
    assert!(q.merit_trace().len() == 2);
    s.control_points[0][0][0] = 99.;
    c.control_points[0][2] = 99.;
    m.points[0].force = [99.; 3];
    assert_eq!(q.original(), &original);
    assert_eq!(
        q.network().loading().unwrap().model().points[0].force,
        [0., 0., 1.]
    );
    check(&q);
}
