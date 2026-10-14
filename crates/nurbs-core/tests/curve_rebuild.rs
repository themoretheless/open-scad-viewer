use nurbs_core::{curve::Curve, foundation::approximate_edits::rebuild_curve_report};
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
fn rational_curve_rebuild_has_verified_candidate_and_preserves_source() {
    let s = line();
    let r = rebuild_curve_report(&s, 2, 4, 0.2, None).unwrap();
    assert!(r.certificate.accepted);
    assert_eq!(r.curve.degree, 2);
    assert_eq!(r.curve.control_points.len(), 4);
    for i in 0..=300 {
        let t = i as f64 / 300.;
        let p = r.curve.evaluate(2. + 5. * t).unwrap().point;
        assert!((p[0] - 12. * t / (1. + 2. * t)).abs() <= r.certificate.error_upper);
        assert!(p[1].abs() <= r.certificate.error_upper);
    }
    assert_eq!(s, line());
}
#[test]
fn inadequate_rebuild_rolls_back() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 2.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = rebuild_curve_report(&s, 1, 2, 1e-4, None).unwrap();
    assert!(!r.certificate.accepted);
    assert!(r.certificate.error_upper > 1e-4);
    assert_eq!(r.curve, s);
}
#[test]
fn rebuild_rejects_invalid_degree_counts_and_error_budget() {
    let s = line();
    for (degree, count) in [(0, 2), (26, 30), (2, 2), (1, 257)] {
        assert!(rebuild_curve_report(&s, degree, count, 0.1, None).is_err());
    }
    for budget in [-1., f64::NAN, f64::INFINITY] {
        assert!(rebuild_curve_report(&s, 1, 2, budget, None).is_err());
    }
}

#[test]
fn periodic_rebuild_keeps_wrapped_storage_and_bounds_analytic_source() {
    let s = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0., 1.],
            vec![-1., 0.],
            vec![0., -1.],
            vec![1., 0.],
            vec![0., 1.],
        ],
        weights: vec![1., 0.8, 1.2, 1., 1., 0.8],
        periodic: true,
    };
    let r = rebuild_curve_report(&s, 2, 14, 0.5, None).unwrap();
    assert!(r.certificate.accepted);
    assert!(r.curve.periodic);
    let n = r.curve.control_points.len();
    assert_eq!(
        &r.curve.control_points[..2],
        &r.curve.control_points[n - 2..]
    );
    for i in 0..400 {
        let u = 2. + 4. * i as f64 / 400.;
        let span = u.floor() as usize;
        let t = u - span as f64;
        let basis = [0.5 * (1. - t).powi(2), 0.5 + t - t * t, 0.5 * t * t];
        let mut expected = [0.; 2];
        let mut denominator = 0.;
        for j in 0..3 {
            let w = basis[j] * s.weights[span - 2 + j];
            denominator += w;
            for k in 0..2 {
                expected[k] += w * s.control_points[span - 2 + j][k];
            }
        }
        for x in &mut expected {
            *x /= denominator;
        }
        let p = r.curve.evaluate(u).unwrap().point;
        let error = ((p[0] - expected[0]).powi(2) + (p[1] - expected[1]).powi(2)).sqrt();
        assert!(error <= r.certificate.error_upper);
    }
}
