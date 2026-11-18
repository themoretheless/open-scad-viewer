use nurbs_core::{curve::Curve, curve_distance::distance};
fn segment(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![-2., -2., 5., 5.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 3.],
        periodic: false,
    }
}
#[test]
fn analytic_segment_distances_are_globally_enclosed() {
    let a = segment([0., 0., 0.], [4., 0., 0.]);
    for (b, expected) in [
        (segment([0., 3., 0.], [4., 3., 0.]), 3.),
        (segment([7., 4., 0.], [8., 4., 0.]), 5.),
        (segment([2., -1., 0.], [2., 1., 0.]), 0.),
        (segment([2., -1., 2.], [2., 1., 2.]), 2.),
    ] {
        let r = distance(&a, &b, 1e-5, 10000).unwrap();
        assert!(r.converged, "{r:?}");
        assert!(
            r.distance_interval_mm[0] <= expected && expected <= r.distance_interval_mm[1],
            "{r:?}"
        );
        let reverse = distance(&b, &a, 1e-5, 10000).unwrap();
        assert!(reverse.converged);
        assert!(
            reverse.distance_interval_mm[0] <= expected
                && expected <= reverse.distance_interval_mm[1]
        );
    }
}
#[test]
fn invalid_dimensions_and_budgets_are_rejected() {
    let a = segment([0., 0., 0.], [4., 0., 0.]);
    let mut b = a.clone();
    for p in &mut b.control_points {
        p.pop();
    }
    assert!(distance(&a, &b, 1e-5, 100).is_err());
    for budget in [0, 100001] {
        assert!(distance(&a, &a, 1e-5, budget).is_err());
    }
    for tolerance in [0., -1., f64::NAN] {
        assert!(distance(&a, &a, tolerance, 100).is_err());
    }
}

#[test]
fn independently_routed_tubes_include_skew_interiors_and_end_caps() {
    use nurbs_core::analysis::curve_measure::tube_pair_clearance;
    let a = segment([0.,0.,0.],[4.,0.,0.]);
    let b = segment([2.,-1.,2.],[2.,1.,2.]);
    for (r, expected) in [(0.25,0.75), (1.,0.), (1.5,-0.5)] {
        for (first, second) in [((&a,1.),(&b,r)), ((&b,r),(&a,1.))] {
            let report = tube_pair_clearance(first,second,1e-5,10000).unwrap();
            let [lo, hi] = report.signed_margin_mm;
            assert!(lo <= expected && expected <= hi && hi-lo < 0.00002);
            assert!(report.centerline_distance.parameters.iter().all(|&t| -2. < t && t < 5.));
        }
    }
    let beyond = segment([7.,4.,0.],[8.,4.,0.]);
    let report = tube_pair_clearance((&a,2.),(&beyond,4.),1e-5,10000).unwrap();
    assert!(report.signed_margin_mm[0] <= -1. && report.signed_margin_mm[1] >= -1.);
    assert!(tube_pair_clearance((&a,-1.),(&b,0.),1e-5,100).is_err());
}
