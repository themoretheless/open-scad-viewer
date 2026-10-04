use nurbs_core::{
    curve::Curve,
    curve_measure::{self as measure, StopReason},
};
fn contains(bounds: [f64; 2], expected: f64) {
    assert!(
        bounds[0] <= expected && expected <= bounds[1],
        "{expected} outside {bounds:?}"
    );
}
fn weighted_line() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![10., 0.]],
        weights: vec![1., 4.],
        periodic: false,
    }
}
#[test]
fn weighted_line_length_and_inverse_follow_independent_rational_law() {
    let c = weighted_line();
    let length = measure::length(&c, 1e-5, 16384).unwrap();
    contains(length.bounds, 10.);
    assert!(length.within_tolerance, "{length:?}");
    let p = measure::point_at_length(&c, 3., 1e-4, 32768).unwrap();
    assert!(p.within_tolerance, "{p:?}");
    assert!((p.parameter - (2. + 5. * 3. / 31.)).abs() < 1e-4);
    assert!((p.point[0] - 3.).abs() <= p.residual_upper + 1e-11);
    for (q, b) in p.point.iter().zip(p.point_bounds) {
        contains(b, *q);
    }
}
#[test]
fn c0_polyline_length_and_equal_distance_division() {
    let c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![3., 4., 0.], vec![3., 4., 12.]]).unwrap();
    let l = measure::length(&c, 1e-9, 2).unwrap();
    contains(l.bounds, 17.);
    assert!(l.within_tolerance);
    let d = measure::divide_by_length(&c, 4, 1e-6, 1024).unwrap();
    assert!(d.within_tolerance, "{d:?}");
    assert_eq!(d.points.len(), 5);
    assert!(d.cells <= 1024);
    let expected = [
        [0., 0., 0.],
        [2.55, 3.4, 0.],
        [3., 4., 3.5],
        [3., 4., 7.75],
        [3., 4., 12.],
    ];
    for (p, e) in d.points.iter().zip(expected) {
        for (a, b) in p.point.iter().zip(e) {
            assert!((a - b).abs() < 2e-6);
        }
    }
}
#[test]
fn circle_length_includes_analytic_value_on_nonunit_domain() {
    let c = nurbs_core::primitives::circle([4., -2., 1.], [0., 0., 1.], 3.).unwrap();
    let c = nurbs_core::foundation::reparameterize_curve_report(&c, [-3., 7.], None)
        .unwrap()
        .curve;
    let l = measure::length(&c, 1e-4, 16384).unwrap();
    contains(l.bounds, 6. * std::f64::consts::PI);
    assert!(l.within_tolerance, "{l:?}");
    assert!(l.error_upper <= 1e-4);
}
#[test]
fn equal_circle_quarters_are_independent_geometric_stations() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 3.).unwrap();
    let d = measure::divide_by_length(&c, 4, 1e-3, 32768).unwrap();
    assert!(d.within_tolerance, "{d:?}");
    assert_eq!(d.points.len(), 5);
    for (i, p) in d.points.iter().enumerate() {
        let a = i as f64 * std::f64::consts::FRAC_PI_2;
        assert!((p.point[0] - 3. * a.cos()).abs() < 2e-3);
        assert!((p.point[1] - 3. * a.sin()).abs() < 2e-3);
    }
}
#[test]
fn cusp_and_stationary_curves_do_not_require_regular_tangents() {
    let c =
        nurbs_core::paths::bezier(vec![vec![1., 0.], vec![-1., 0.], vec![1., 0.]], None).unwrap();
    let l = measure::length(&c, 1e-4, 16384).unwrap();
    contains(l.bounds, 2.);
    assert!(l.within_tolerance, "{l:?}");
    let stationary =
        nurbs_core::paths::bezier(vec![vec![1., 2.], vec![1., 2.], vec![1., 2.]], None).unwrap();
    let l = measure::length(&stationary, 1e-10, 8).unwrap();
    contains(l.bounds, 0.);
    assert!(l.within_tolerance);
    let p = measure::point_at_length(&stationary, 0., 1e-8, 8).unwrap();
    assert!(p.within_tolerance);
    assert_eq!(p.point, vec![1., 2.]);
    assert!(measure::point_at_length(&stationary, 1., 1e-8, 8).is_err());
}
#[test]
fn polynomial_parabola_length_matches_closed_form() {
    let c =
        nurbs_core::paths::bezier(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]], None).unwrap();
    let exact = 5_f64.sqrt() / 2. + 2_f64.asinh() / 4.;
    let l = measure::length(&c, 1e-5, 16384).unwrap();
    contains(l.bounds, exact);
    assert!(l.within_tolerance, "{l:?}");
}
#[test]
fn exhausted_work_retains_valid_bounds_without_claiming_accuracy() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 3.).unwrap();
    assert!(measure::length(&c, 1e-12, 1).is_err());
    let l = measure::length(&c, 1e-12, 4).unwrap();
    contains(l.bounds, 6. * std::f64::consts::PI);
    assert!(!l.within_tolerance);
    assert_eq!(l.stop_reason, StopReason::WorkLimit);
    assert_eq!(l.cells, 4);
    let p = measure::point_at_length(&c, 3., 1e-12, 4).unwrap();
    assert!(!p.within_tolerance);
    assert_eq!(p.stop_reason, StopReason::WorkLimit);
    assert!(p.cells <= 4);
    let d = measure::divide_by_length(&c, 8, 1e-12, 4).unwrap();
    assert!(!d.within_tolerance);
    assert!(d.points.len() < 9);
    assert!(d.cells <= 4);
}
#[test]
fn invalid_measure_requests_are_refused() {
    let c = weighted_line();
    assert!(measure::length(&c, 0., 10).is_err());
    assert!(measure::length(&c, f64::NAN, 10).is_err());
    assert!(measure::length(&c, 1e-3, 0).is_err());
    assert!(measure::length(&c, 1e-3, 100001).is_err());
    assert!(measure::point_at_length(&c, -1., 1e-3, 10).is_err());
    assert!(measure::point_at_length(&c, f64::INFINITY, 1e-3, 10).is_err());
    assert!(measure::point_at_length(&c, 100., 1e-3, 10).is_err());
    assert!(measure::divide_by_length(&c, 0, 1e-3, 10).is_err());
    assert!(measure::divide_by_length(&c, 1025, 1e-3, 10).is_err());
    let mut broken = c;
    broken.control_points.clear();
    assert!(measure::length(&broken, 1e-3, 10).is_err());
}

#[test]
fn division_endpoints_keep_exact_correlation_when_total_is_uncertain() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 3.).unwrap();
    let d = measure::divide_by_length(&c, 1, 1e-12, 4).unwrap();
    assert!(!d.total.within_tolerance);
    assert!(d.within_tolerance);
    assert_eq!(d.points.len(), 2);
    assert_eq!(d.points[0].parameter, c.domain()[0]);
    assert_eq!(d.points[1].parameter, c.domain()[1]);
    assert_eq!(d.points[0].residual_upper, 0.);
    assert_eq!(d.points[1].residual_upper, 0.);
    assert_eq!(d.cells, 4);
}
