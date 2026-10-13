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
    assert!(r.certificate.exact_zero_recognized);
    assert_eq!(r.certificate.error_upper, 0.);
    assert_eq!(s.control_points, vec![vec![0., 0.], vec![3., 0.], vec![4., 0.]]);
    assert_eq!(s.weights, vec![1., 2., 3.]);
    assert_eq!(r.curve.control_points, vec![vec![0., 0.], vec![4., 0.]]);
    assert_eq!(r.curve.weights, vec![1., 3.]);
    assert_eq!(r.curve.knots, vec![2., 2., 7., 7.]);
    // Exact homogeneous power coefficients on t=(u-2)/5. Cross multiplication
    // verifies the rational identity without confusing evaluator rounding with
    // geometric deviation (even forming u=2+5*t changes represented t).
    let source_numerator = [0_i64, 12, 0];
    let source_denominator = [1_i64, 2, 0];
    let reduced_numerator = [0_i64, 12];
    let reduced_denominator = [1_i64, 2];
    let mut residual = [0_i64; 4];
    for i in 0..3 {
        for j in 0..2 {
            residual[i+j] += source_numerator[i] * reduced_denominator[j]
                - source_denominator[i] * reduced_numerator[j];
        }
    }
    assert_eq!(residual, [0; 4]);
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
