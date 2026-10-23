use nurbs_core::{curve::Curve, foundation::approximate_edits::remove_curve_knot_report};
fn source() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    }
    .insert(4., 1)
    .unwrap()
}
#[test]
fn accepted_candidate_preserves_independent_rational_line_within_budget() {
    let s = source();
    let r = remove_curve_knot_report(&s, 4., 0.1, None).unwrap();
    assert!(r.certificate.accepted);
    assert!(r.certificate.error_upper <= 0.1);
    assert_eq!(r.curve.control_points.len(), 2);
    for i in 0..=300 {
        let u = 2. + 5. * i as f64 / 300.;
        let t = (u - 2.) / 5.;
        let expected = 12. * t / (1. + 2. * t);
        let p = r.curve.evaluate(u).unwrap().point;
        assert!((p[0] - expected).abs() <= r.certificate.error_upper);
        assert!(p[1].abs() <= r.certificate.error_upper);
    }
    assert_eq!(s, source());
}
#[test]
fn zero_budget_rolls_back_when_exact_equality_is_unproven() {
    let s = source();
    let r = remove_curve_knot_report(&s, 4., 0., None).unwrap();
    assert!(!r.certificate.accepted);
    assert!(!r.certificate.exact_zero_recognized);
    assert_eq!(r.curve, s);
}
#[test]
fn invalid_requests_are_rejected() {
    let s = source();
    for knot in [2., 7., 3., f64::NAN] {
        assert!(remove_curve_knot_report(&s, knot, 0.1, None).is_err());
    }
    for budget in [-1., f64::NAN, f64::INFINITY] {
        assert!(remove_curve_knot_report(&s, 4., budget, None).is_err());
    }
}
