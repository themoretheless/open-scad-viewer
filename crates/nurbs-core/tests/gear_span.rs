use nurbs_core::involute::{external_spur_span, measurable_spur_spans, tooth_thinning_from_span};

#[test]
fn spinner_span_matches_independent_cartesian_flank_contacts() {
    let angle = 20_f64.to_radians();
    let pitch = 24.;
    let base = pitch * angle.cos();
    let half_base = std::f64::consts::PI / 64. + angle.tan() - angle - 0.1 / (2. * pitch);
    let counts = measurable_spur_spans(1.5, 32, 20., 0.1, 0.1).unwrap();
    assert!(counts.contains(&4));
    for k in 1..=32 {
        let actual = external_spur_span(1.5, 32, 20., 0.1, 0.1, k);
        let t = half_base + (k - 1) as f64 * std::f64::consts::PI / 32.;
        let r = base * t.hypot(1.);
        let admitted = r >= base.max(pitch - 1.6) && r <= pitch + 1.5;
        assert_eq!(actual.is_ok(), admitted);
        assert_eq!(counts.contains(&k), admitted);
        if let Ok([lo, hi]) = actual {
            let theta = half_base - (t - t.atan()) + (k - 1) as f64 * std::f64::consts::PI / 32.;
            let width = 2. * r * theta.sin();
            assert!(lo <= width && width <= hi && hi - lo < 1e-9);
            let loss =
                tooth_thinning_from_span(1.5, 32, 20., 0.1, k, [width - 0.001, width + 0.001])
                    .unwrap();
            for (got, expected) in loss
                .into_iter()
                .zip([0.1 - 0.001 / angle.cos(), 0.1 + 0.001 / angle.cos()])
            {
                assert!((got - expected).abs() < 1e-9);
            }
        }
    }
    assert!(external_spur_span(1.5, 32, 20., 0.1, 0.1, 0).is_err());
    for invalid in [[5., 4.], [f64::NAN, 5.], [100., 101.]] {
        assert!(tooth_thinning_from_span(1.5, 32, 20., 0.1, 4, invalid).is_err());
    }
}
