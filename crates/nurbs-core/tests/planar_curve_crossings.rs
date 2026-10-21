use nurbs_core::{
    curve::Curve,
    intersection::{ContactClass, intersect_curve_curve_certified_report as intersect},
};
fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn exact_transverse_root_is_proved_on_original_nonunit_domains() {
    let mut a = line([0., 0., 3.], [1., 1., 3.]);
    a.knots = vec![2., 2., 5., 5.];
    let mut b = line([0., 1., 3.], [1., 0., 3.]);
    b.knots = vec![-4., -4., 8., 8.];
    let r = intersect(&a, &b, 1e-8, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
    let p = &r.crossings[0];
    assert_eq!(p.contact, ContactClass::Transverse);
    assert!(p.parameters[0][0] <= 3.5 && p.parameters[0][1] >= 3.5);
    assert!(p.parameters[1][0] <= 2. && p.parameters[1][1] >= 2.);
    for (k, x) in [0.5, 0.5, 3.].into_iter().enumerate() {
        assert!(p.position_bounds[k][0] <= x && x <= p.position_bounds[k][1]);
    }
}
#[test]
fn rational_parameterization_has_a_certified_root_without_a_sampled_residual() {
    let mut a = line([2., 0., 0.], [2., 1., 1.]);
    a.weights = vec![1., 2.];
    let b = line([2., 0., 1.], [2., 1., 0.]);
    let r = intersect(&a, &b, 1e-5, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
    let p = &r.crossings[0];
    assert!(p.parameters[0][0] <= 1. / 3. && p.parameters[0][1] >= 1. / 3.);
    assert!(p.parameters[1][0] <= 0.5 && p.parameters[1][1] >= 0.5);
}
#[test]
fn separation_identity_and_unsupported_spatial_pairs_remain_distinct() {
    let a = line([0., 0., 0.], [1., 0., 0.]);
    let b = line([0., 1., 0.], [1., 1., 0.]);
    let r = intersect(&a, &b, 1e-8, 10).unwrap();
    assert!(r.complete && r.crossings.is_empty() && !r.identical_parameter_overlap);
    let r = intersect(&a, &a, 1e-8, 10).unwrap();
    assert!(r.complete && r.identical_parameter_overlap);
    let mut b = line([0., 1., 0.], [1., -1., 1.]).elevate(2).unwrap();
    b.control_points[1][2] += 0.2;
    let r = intersect(&a, &b, 1e-8, 10).unwrap();
    assert!(!r.complete && !r.unresolved.is_empty());
}
#[test]
fn tangency_endpoint_and_limited_work_are_not_false_transverse_crossings() {
    let a = line([0., 0., 0.], [1., 0., 0.]);
    let b = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.25, 0.], vec![0.5, -0.25, 0.], vec![1., 0.25, 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = intersect(&a, &b, 1e-8, 100).unwrap();
    assert!(!r.complete && r.crossings.is_empty());
    let b = line([1., 0., 0.], [1., 1., 0.]);
    let r = intersect(&a, &b, 1e-8, 100).unwrap();
    assert!(r.complete && r.crossings[0].contact == ContactClass::Boundary);
    assert!(intersect(&a, &b, 1e-8, 0).is_err());
    assert!(intersect(&a, &b, 0., 10).is_err());
}

#[test]
fn oblique_spatial_lines_and_skew_projected_crossings_are_decided_exactly() {
    let a = line([0., 0., 0.], [1., 1., 1.]);
    let b = line([0., 1., 1.], [1., 0., 0.]);
    let r = intersect(&a, &b, 1e-12, 10).unwrap();
    assert!(r.complete);
    assert_eq!(r.crossings.len(), 1);
    assert_eq!(r.crossings[0].position_bounds, [[0.5, 0.5]; 3]);
    let b = line([0., 1., 2.], [1., 0., 1.]);
    let r = intersect(&a, &b, 1e-12, 10).unwrap();
    assert!(r.complete && r.crossings.is_empty());
}

#[test]
fn two_curved_transverse_crossings_preserve_full_coverage_and_work_obligations() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.16, 2.], vec![0.5, -0.34, 2.], vec![1., 0.16, 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = line([0., 0., 2.], [1., 0., 2.]);
    let r = intersect(&a, &b, 1e-6, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 2);
    let mut roots: Vec<_> = r
        .crossings
        .iter()
        .map(|p| (p.parameters[0][0] + p.parameters[0][1]) * 0.5)
        .collect();
    roots.sort_by(f64::total_cmp);
    assert!((roots[0] - 0.2).abs() < 1e-6 && (roots[1] - 0.8).abs() < 1e-6);
    for p in &r.crossings {
        assert!(p.position_bounds[1][0] <= 0. && p.position_bounds[1][1] >= 0.);
        assert_eq!(p.contact, ContactClass::Transverse);
    }
    let limited = intersect(&a, &b, 1e-6, 1).unwrap();
    assert!(!limited.complete && !limited.unresolved.is_empty());
    assert!(limited.cells <= 1);
}
