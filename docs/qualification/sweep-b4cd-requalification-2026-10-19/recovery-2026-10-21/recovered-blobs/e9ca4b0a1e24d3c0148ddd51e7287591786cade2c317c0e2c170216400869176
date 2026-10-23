use nurbs_core::{curve::Curve, curve_distance::nearest_point};
fn line() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![10., 0.]],
        weights: vec![1., 4.],
        periodic: false,
    }
}
#[test]
fn weighted_line_has_analytic_interior_and_endpoint_minima() {
    for (query, expected, parameter) in [
        ([3., 4.], 4., 2. + 15. / 31.),
        ([-3., 4.], 5., 2.),
        ([13., 4.], 5., 7.),
    ] {
        let r = nearest_point(&line(), &query, 1e-5, 10000).unwrap();
        assert!(r.converged, "{r:?}");
        assert!(
            r.distance_interval[0] <= expected && expected <= r.distance_interval[1],
            "{r:?}"
        );
        assert!((r.parameter - parameter).abs() < 0.01);
        for (x, b) in r.point.iter().zip(&r.point_enclosure) {
            assert!(b[0] <= *x && *x <= b[1]);
        }
    }
}
#[test]
fn budget_exhaustion_retains_bounds_and_invalid_requests_fail() {
    let r = nearest_point(&line(), &[3., 4.], 1e-12, 1).unwrap();
    assert!(!r.converged);
    assert!(r.distance_interval[0] <= 4. && 4. <= r.distance_interval[1]);
    for q in [vec![1.], vec![f64::NAN, 0.], vec![f64::INFINITY, 0.]] {
        assert!(nearest_point(&line(), &q, 1e-4, 100).is_err());
    }
    for tolerance in [0., -1., f64::NAN] {
        assert!(nearest_point(&line(), &[0., 0.], tolerance, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(nearest_point(&line(), &[0., 0.], 1e-4, budget).is_err());
    }
}
#[test]
fn stationary_curve_does_not_require_a_regular_tangent() {
    let mut c = line();
    c.control_points = vec![vec![1., 2.], vec![1., 2.]];
    let r = nearest_point(&c, &[4., 6.], 1e-8, 100).unwrap();
    assert!(r.converged);
    assert!(r.distance_interval[0] <= 5. && 5. <= r.distance_interval[1]);
}
