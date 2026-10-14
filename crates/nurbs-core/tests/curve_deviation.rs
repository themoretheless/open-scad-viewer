use nurbs_core::{curve::Curve, curve_deviation::inspect};
fn line() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    }
}
#[test]
fn rational_translation_has_known_continuous_maximum() {
    let a = line();
    let mut b = a.clone();
    for p in &mut b.control_points {
        p[1] = 3.;
    }
    let r = inspect(&a, &b, 1e-3, 10000).unwrap();
    assert!(r.converged, "{r:?}");
    assert!(r.bounds[0] <= 3. && 3. <= r.bounds[1]);
    assert!(r.cells > 1);
    let q = inspect(&a, &b, 1e-12, 1).unwrap();
    assert!(!q.converged);
    assert!(q.bounds[0] <= 3. && 3. <= q.bounds[1]);
    assert_eq!(inspect(&a, &a, 1e-9, 1).unwrap().bounds, [0., 0.]);
}
#[test]
fn narrow_peak_and_different_bases_are_covered() {
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let b = Curve {
        degree: 1,
        knots: vec![0., 0., 0.0001, 0.0002, 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.], vec![0., 0.], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = inspect(&a, &b, 1e-9, 100).unwrap();
    assert!(r.converged);
    assert!(r.bounds[0] <= 1. && 1. <= r.bounds[1]);
    assert!(inspect(&a, &b, 1e-9, 1).is_err());
}
#[test]
fn domain_dimension_and_request_validation() {
    let a = line();
    for accuracy in [0., -1., f64::NAN] {
        assert!(inspect(&a, &a, accuracy, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(inspect(&a, &a, 1e-4, budget).is_err());
    }
    let mut b = a.clone();
    b.knots = vec![0., 0., 1., 1.];
    assert!(inspect(&a, &b, 1e-4, 100).is_err());
    b = a.clone();
    for p in &mut b.control_points {
        p.push(0.);
    }
    assert!(inspect(&a, &b, 1e-4, 100).is_err());
}

#[test]
fn refinement_and_degree_change_do_not_require_equal_bases() {
    let a = line();
    let b = a.insert(3., 1).unwrap().elevate(2).unwrap();
    let r = inspect(&a, &b, 1e-3, 20000).unwrap();
    assert!(r.converged, "{r:?}");
    assert!(r.bounds[1] <= 1e-3 + 1e-10);
    let discontinuous = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(inspect(&discontinuous, &discontinuous, 1e-4, 100).is_err());
}
