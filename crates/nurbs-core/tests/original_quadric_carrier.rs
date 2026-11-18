use nurbs_core::{
    curve::Curve, intersection::certified_curve_surface::original_quadric_carrier_roots,
};
fn sphere(r2: f64) -> [[f64; 4]; 4] {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., -r2],
    ]
}
fn line(y: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![-2., -2., 5., 5.],
        control_points: vec![vec![-2., y, 0.], vec![2., y, 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn original_sphere_roots_have_natural_parameters_and_world_bounds() {
    let r = original_quadric_carrier_roots(&line(0.), sphere(1.), &[], 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 2);
    for (root, (t, x)) in r.roots.iter().zip([(-0.25, -1.), (3.25, 1.)]) {
        assert!(root.parameter[0] <= t && t <= root.parameter[1]);
        assert!(root.position_bounds[0][0] <= x && x <= root.position_bounds[0][1]);
        assert_eq!(root.multiplicities, [1]);
    }
    assert_eq!(
        r.exact_work,
        r.scalar_proofs.iter().map(|(_, p)| p.work_used).sum()
    );
}
#[test]
fn tangency_and_absence_are_classified_without_a_crossing_assertion() {
    let r = original_quadric_carrier_roots(&line(1.), sphere(1.), &[], 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 1);
    assert_eq!(r.roots[0].multiplicities, [2]);
    let r = original_quadric_carrier_roots(&line(2.), sphere(1.), &[], 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert!(r.roots.is_empty());
}
#[test]
fn rational_source_weights_use_the_original_natural_clock() {
    let mut c = line(0.);
    c.weights = vec![1., 3.];
    let r = original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 2);
    for (root, t) in r.roots.iter().zip([-1.3, 1.5]) {
        assert!(root.parameter[0] <= t && t <= root.parameter[1]);
    }
}
#[test]
fn shared_knot_retains_two_one_sided_residual_orders() {
    // First span X=1+(1-t)^2; second X=1+t. Sphere contact at their
    // shared knot has order two on the left and one on the right.
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![2., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 0., 0.],
            vec![1.5, 0., 0.],
            vec![2., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    assert_eq!(r.roots[0].parameter, [1., 1.]);
    assert_eq!(r.roots[0].source_spans, [2, 4]);
    assert_eq!(r.roots[0].multiplicities, [2, 1]);
    assert_eq!(
        r.roots[0].normalized_original_span_parameters,
        [[1., 1.], [0., 0.]]
    );
    let short = original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 1, 100000).unwrap();
    assert!(!short.complete);
    assert_eq!(short.unresolved_spans, [4]);
    let limited =
        original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, short.exact_work).unwrap();
    assert!(!limited.complete);
    assert_eq!(limited.unresolved_spans, [4]);
    assert!(limited.exact_work <= short.exact_work);
    assert_eq!(
        limited.exact_work,
        limited.scalar_proofs.iter().map(|(_, p)| p.work_used).sum()
    );
}
#[test]
fn coincidence_and_exhausted_budget_preserve_missing_isolated_root_coverage() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![75., 0., 0.], vec![75., 100., 0.], vec![-21., 72., 0.]],
        weights: vec![5., 3., 5.],
        periodic: false,
    };
    let r = original_quadric_carrier_roots(&c, sphere(5625.), &[], 1e-9, 10, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.roots.is_empty());
    assert_eq!(r.unresolved_spans, [2]);
    let r = original_quadric_carrier_roots(&line(0.), sphere(1.), &[], 1e-9, 10, 0).unwrap();
    assert!(!r.complete);
    assert_eq!(r.exact_work, 0);
    assert_eq!(r.unresolved_spans, [1]);
}
#[test]
fn invalid_periodic_dimensional_and_discontinuous_sources_refuse() {
    let mut c = line(0.);
    c.periodic = true;
    assert!(original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, 100000).is_err());
    c.periodic = false;
    for p in &mut c.control_points {
        p.pop();
    }
    assert!(original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, 100000).is_err());
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1., 2., 2.],
        control_points: vec![
            vec![-1., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 0., 0.],
            vec![3., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(original_quadric_carrier_roots(&c, sphere(1.), &[], 1e-9, 10, 100000).is_err());
}
