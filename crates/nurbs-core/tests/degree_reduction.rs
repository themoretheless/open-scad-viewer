use nurbs_core::{curve::Curve, foundation::approximate_edits::reduce_curve_degree_report};
fn line() -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    }
    .elevate(2)
    .unwrap()
}
#[test]
fn elevated_rational_line_reduces_with_verified_error() {
    let s = line();
    let r = reduce_curve_degree_report(&s, 1, 0.1, None).unwrap();
    assert!(r.certificate.accepted);
    assert_eq!(r.curve.degree, 1);
    assert!(r.certificate.error_upper <= 0.1);
    for i in 0..=200 {
        let t = i as f64 / 200.;
        let p = r.curve.evaluate(2. + 5. * t).unwrap().point;
        let oracle = 12. * t / (1. + 2. * t);
        // The certificate bounds the real-arithmetic curves. This separate
        // allowance covers rounding in the parameter transform, evaluator and
        // independently evaluated rational formula; it is not a fit budget.
        let evaluation_roundoff = 64. * f64::EPSILON * (1. + p[0].abs() + oracle.abs());
        assert!((p[0] - oracle).abs() <= r.certificate.error_upper + evaluation_roundoff);
        assert!((p[0] - oracle).abs() <= 0.1);
        let original = s.evaluate(2. + 5. * t).unwrap().point;
        assert!((p[0] - original[0]).abs() <= r.certificate.error_upper + evaluation_roundoff);
        assert!(p[1].abs() <= r.certificate.error_upper);
    }
    assert_eq!(s, line());
}
#[test]
fn curved_profile_rolls_back_for_tight_budget() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 2.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = reduce_curve_degree_report(&s, 1, 1e-4, None).unwrap();
    assert!(!r.certificate.accepted);
    assert!(r.certificate.error_upper > 1e-4);
    assert_eq!(r.curve, s);
}
#[test]
fn invalid_degree_budget_and_source_fail() {
    let s = line();
    for degree in [0, 2, 3] {
        assert!(reduce_curve_degree_report(&s, degree, 0.1, None).is_err());
    }
    for budget in [-1., f64::NAN, f64::INFINITY] {
        assert!(reduce_curve_degree_report(&s, 1, budget, None).is_err());
    }
    let mut invalid = s.clone();
    invalid.weights[0] = 0.;
    assert!(reduce_curve_degree_report(&invalid, 1, 0.1, None).is_err());
}
