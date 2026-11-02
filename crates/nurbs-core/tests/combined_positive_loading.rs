use nurbs_core::{
    constrained_surface::{
        adaptive,
        network::{BoundaryLocks, CurveConstraint},
        unparameterized::{self, Continuity, Guide},
        variational::{Material, Model, PointLoad},
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
fn line() -> Curve {
    Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap()
}
fn guide<'a>(c: &'a Curve, p: &'a Curve) -> CurveConstraint<'a> {
    CurveConstraint {
        spatial: c,
        parameters: p,
        reverse: false,
        tangent: None,
    }
}
fn force() -> Model {
    let mut m = Model::default();
    m.material = Material {
        membrane: 0.,
        bending: 0.,
        poisson: 0.3,
    };
    m.points = vec![PointLoad {
        uv: [0.5, 0.5],
        force: [0., 0., 1.],
    }];
    m
}
#[test]
fn inferred_uvs_retain_admitted_positive_weight_merit_search() {
    let s = plane(2);
    let c = line();
    let m = force();
    let mut o = unparameterized::loading::PositiveOptions::default();
    o.weights.variable_weights = vec![[1, 1]];
    o.weights.geometry.weight_bounds = [0.5, 2.];
    o.weights.geometry.network.network.fitting.anchor_weight = 0.2;
    let q = unparameterized::loading::solve_positive_weights(
        &s,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Position,
        }],
        &[],
        None,
        &m,
        o,
    )
    .unwrap();
    let w = q.positive_loading().unwrap();
    assert!(w.report().accepted > 0);
    assert_eq!(q.surface().weights[1][1], 2.);
    assert_eq!(q.physical_reference(), Some(&s));
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
    assert!(q.loading_work() >= w.loading_work());
}
fn arc() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    }
}
#[test]
fn degree_growth_and_rational_weights_recover_an_original_circle_under_load() {
    let s = patches::bezier(
        vec![
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        ],
        None,
    )
    .unwrap();
    let a = arc();
    let mut b = a.clone();
    for p in &mut b.control_points {
        p[2] = 1.;
    }
    let pa = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let pb = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
    let mut o = adaptive::loading::PositiveOptions::default();
    o.weights.max_weight_trials = 0;
    o.weights.geometry.weight_bounds = [0.7, 1.5];
    let mut m = Model::default();
    m.pressure = 0.01;
    let q = adaptive::loading::solve_positive_weights(
        &s,
        &[guide(&a, &pa), guide(&b, &pb)],
        &[],
        &[],
        None,
        &m,
        o,
    )
    .unwrap();
    assert!(q.surface().degree_u >= 2 && !q.refinements().is_empty());
    assert_eq!(q.network().loading().unwrap().source(), &s);
    assert!(
        q.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 1.).abs() > 0.05)
    );
    assert_eq!(q.positive_loading().unwrap().original(), q.anchor());
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
    for t in [0., 0.13, 0.5, 0.83, 1.] {
        let p = q.surface().evaluate(t, 0.).unwrap().point;
        assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-7);
    }
}
fn rational_quartic() -> Curve {
    let weights = (0usize..5)
        .map(|i| 1. + 0.5 * i as f64 / 4.)
        .collect::<Vec<_>>();
    Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0usize..5)
            .map(|i| {
                vec![
                    (i as f64 / 4. + 0.5 * (i * i.saturating_sub(1)) as f64 / 12.) / weights[i],
                    0.5,
                    [0., 0.0125, -1. / 60., 0.0125, 0.][i] / weights[i],
                ]
            })
            .collect(),
        weights,
        periodic: false,
    }
}
#[test]
fn inferred_uvs_basis_growth_and_positive_weights_keep_one_original_physical_reference() {
    let s = plane(1);
    let c = rational_quartic();
    let mut o = adaptive::loading::PositiveOptions::default();
    o.weights.max_weight_trials = 0;
    o.weights.geometry.weight_bounds = [0.5, 2.];
    let mut m = Model::default();
    m.pressure = 0.005;
    let q = adaptive::loading::solve_unparameterized_positive_weights(
        &s,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Position,
        }],
        &[],
        None,
        &m,
        o,
        adaptive::Inversion::default(),
    )
    .unwrap();
    assert!(q.surface().degree_u >= 4 && !q.refinements().is_empty());
    assert!(
        q.surface()
            .weights
            .iter()
            .flatten()
            .any(|w| (*w - 1.).abs() > 0.05)
    );
    assert_eq!(q.network().loading().unwrap().source(), &s);
    assert_eq!(q.positive_loading().unwrap().original(), q.anchor());
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
    assert!(
        q.loading_work() >= q.positive_loading().unwrap().loading_work()
            && q.loading_work() <= 100000000
    );
}
#[test]
fn inferred_positive_weight_merit_survives_adaptive_owner_and_input_changes() {
    let mut s = plane(2);
    let original = s.clone();
    let mut c = line();
    let mut m = force();
    let mut o = adaptive::loading::PositiveOptions::default();
    o.weights.variable_weights = vec![[1, 1]];
    o.weights.geometry.weight_bounds = [0.5, 2.];
    o.weights.geometry.network.network.fitting.anchor_weight = 0.2;
    let q = adaptive::loading::solve_unparameterized_positive_weights(
        &s,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Position,
        }],
        &[],
        None,
        &m,
        o,
        adaptive::Inversion::default(),
    )
    .unwrap();
    assert_eq!(q.surface().weights[1][1], 2.);
    assert!(q.refinements().is_empty());
    assert!(q.positive_loading().unwrap().report().accepted > 0);
    s.control_points[0][0][0] = 99.;
    c.control_points[0][2] = 99.;
    m.points[0].force = [99.; 3];
    assert_eq!(q.original(), &original);
    assert_eq!(
        q.network().loading().unwrap().model().points[0].force,
        [0., 0., 1.]
    );
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
}
#[test]
fn adaptive_inferred_positive_path_preserves_all_four_original_g2_boundaries() {
    let s = plane(3);
    let target = surface_edit::elevate(&s, Axis::U, 6).unwrap();
    let mut target = surface_edit::elevate(&target, Axis::V, 6).unwrap();
    target.control_points[3][3][2] = 0.005;
    let c = target.iso(Axis::V, 0.5).unwrap();
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
    let mut o = adaptive::loading::PositiveOptions::default();
    o.weights.max_weight_trials = 0;
    let mut m = Model::default();
    m.pressure = 0.01;
    let q = adaptive::loading::solve_unparameterized_positive_weights(
        &s,
        &[Guide {
            spatial: &c,
            continuity: Continuity::Position,
        }],
        &[],
        Some(locks),
        &m,
        o,
        adaptive::Inversion::default(),
    )
    .unwrap();
    assert!(q.surface().degree_u >= 6 && q.surface().degree_v >= 6);
    assert_eq!(q.original_seams().len(), 4);
    assert_eq!(q.original_boundary_reference(), Some(&s));
    assert!(q.reverify(100000, 1000000).unwrap().admitted);
    assert_eq!(q.network().loading().unwrap().source(), &s);
}
#[test]
fn original_weight_indices_and_noncorner_gauges_cannot_be_reinterpreted_after_basis_edits() {
    let s = plane(1);
    let before = s.clone();
    let c = rational_quartic();
    let p = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
    let m = Model::default();
    for mask in [true, false] {
        let mut o = adaptive::loading::PositiveOptions::default();
        o.weights.max_weight_trials = 0;
        if mask {
            o.weights.variable_weights = vec![[1, 1]];
        } else {
            o.weights.geometry.gauge = [0, 1];
        }
        assert!(
            adaptive::loading::solve_positive_weights(&s, &[guide(&c, &p)], &[], &[], None, &m, o)
                .is_err()
        );
    }
    let mut o = adaptive::loading::PositiveOptions::default();
    o.weights.max_total_assembly_terms = 1;
    assert!(
        adaptive::loading::solve_unparameterized_positive_weights(
            &s,
            &[Guide {
                spatial: &c,
                continuity: Continuity::Position
            }],
            &[],
            None,
            &m,
            o,
            adaptive::Inversion::default()
        )
        .is_err()
    );
    assert_eq!(s, before);
}
