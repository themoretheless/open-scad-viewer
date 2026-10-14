use super::*;

/// Two-span rational quadratic on [0, 1] with nonuniform weights.
fn sample_curve() -> Curve {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 2.],
            vec![3., 2.],
            vec![4., 0.],
        ],
        weights: vec![1., 0.7, 1.4, 2.],
        periodic: false,
    };
    curve.validate().unwrap();
    curve
}

#[test]
fn nonfinite_interior_controls_cannot_receive_a_monotonicity_certificate() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mapping = ParameterMapping::Pieces(vec![MapPiece {
            domain: [0., 1.],
            range: [0., 1.],
            values: vec![0., value, 1.],
            weights: vec![1., 1., 1.],
        }]);
        assert!(certify_reparameterization_report(&mapping, None).is_err());
    }
}

#[test]
fn mobius_reparameterization_preserves_the_sampled_image() {
    let curve = sample_curve();
    // t = (2s + 1)/(s + 3): det = 5 > 0, denominator 3..4 on [0,1].
    let report = mobius_reparameterize_curve_report(&curve, 2., 1., 1., 3.).unwrap();
    assert_eq!(report.old_domain, [0., 1.]);
    let [s0, s1] = report.new_domain;
    assert!((s0 - -0.5).abs() < 1e-12 && (s1 - 2.).abs() < 1e-12);
    assert_eq!(report.curve.degree, curve.degree);
    assert_eq!(report.curve.control_points.len(), curve.control_points.len());
    assert!(report.deviation < 1e-9, "deviation {}", report.deviation);
    for i in 0..=40 {
        let s = s0 + (s1 - s0) * i as f64 / 40.;
        let t = (2. * s + 1.) / (s + 3.);
        let before = curve.evaluate(t).unwrap().point;
        let after = report.curve.evaluate(s).unwrap().point;
        assert!(
            distance(&before, &after) < 1e-9,
            "image mismatch at s={s}: {before:?} vs {after:?}"
        );
    }
}

#[test]
fn mobius_rejects_degenerate_and_nonmonotone_maps() {
    let curve = sample_curve();
    // ad − bc = 0.
    assert!(mobius_reparameterize_curve_report(&curve, 1., 2., 2., 4.).is_err());
    // Denominator root t = 0.5 inside the domain.
    assert!(mobius_reparameterize_curve_report(&curve, 1., 0., 1., -0.5).is_err());
    // Denominator root at the endpoint t = 1.
    assert!(mobius_reparameterize_curve_report(&curve, 1., 0., -1., 1.).is_err());
    // Decreasing map: t = 1 − s swaps the endpoints.
    assert!(mobius_reparameterize_curve_report(&curve, -1., 1., 0., 1.).is_err());
    // Non-finite coefficients.
    assert!(mobius_reparameterize_curve_report(&curve, f64::NAN, 0., 0., 1.).is_err());
}

#[test]
fn normalize_endpoint_weights_unifies_endpoints_and_preserves_image() {
    let curve = sample_curve();
    let report = normalize_endpoint_weights_report(&curve).unwrap();
    assert_eq!(report.curve.weights[0], 1.);
    assert_eq!(*report.curve.weights.last().unwrap(), 1.);
    assert_eq!(report.curve.domain(), curve.domain());
    assert_eq!(report.curve.degree, curve.degree);
    assert!(report.deviation < 1e-9, "deviation {}", report.deviation);
    for i in 0..=40 {
        let s = i as f64 / 40.;
        let [a, b, c, d] = report.coefficients;
        let t = (a * s + b) / (c * s + d);
        let before = curve.evaluate(t).unwrap().point;
        let after = report.curve.evaluate(s).unwrap().point;
        assert!(distance(&before, &after) < 1e-9, "image mismatch at s={s}");
    }
    // Already normalized: the map degenerates to the identity.
    let again = normalize_endpoint_weights_report(&report.curve).unwrap();
    assert_eq!(again.curve.weights[0], 1.);
    assert_eq!(*again.curve.weights.last().unwrap(), 1.);
    assert!(again.deviation < 1e-9);
}
