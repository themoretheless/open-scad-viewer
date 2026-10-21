use nurbs_core::wave_surface::cubic;
#[test]
fn graph_matches_independent_hermite_formula() {
    for periods in [1, 3, 5] {
        let s = cubic([2., -3., 7.], 12., 5., 1.5, periods).unwrap();
        for i in 0..=317 {
            let u = i as f64 / 317.;
            let scaled = u * (2 * periods) as f64;
            let seg = (scaled.floor() as usize).min(2 * periods - 1);
            let t = scaled - seg as f64;
            let sign = if seg % 2 == 0 { 1. } else { -1. };
            let expected = 7. + sign * 1.5 * (1. - 6. * t * t + 4. * t * t * t);
            for v in [0., 0.37, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - (2. + 12. * u)).abs() < 1e-11);
                assert!((p[1] - (-3. + 5. * v)).abs() < 1e-11);
                assert!((p[2] - expected).abs() < 1e-11);
            }
        }
        // Analytic zero tangent at all extrema: finite differences approach
        // the same slope on each side, independently of derivative evaluators.
        for k in 1..2 * periods {
            let u = k as f64 / (2 * periods) as f64;
            let h = 1e-6 / (2 * periods) as f64;
            let z = s.evaluate(u, 0.5).unwrap().point[2];
            let left = (z - s.evaluate(u - h, 0.5).unwrap().point[2]) / h;
            let right = (s.evaluate(u + h, 0.5).unwrap().point[2] - z) / h;
            assert!(left.abs() < 0.002 && right.abs() < 0.002);
        }
    }
}
#[test]
fn flat_negative_amplitude_and_invalid_requests() {
    let flat = cubic([0.; 3], 1., 2., 0., 2).unwrap();
    assert_eq!(flat.evaluate(0.23, 0.7).unwrap().point[2], 0.);
    let inverted = cubic([0.; 3], 1., 2., -2., 2).unwrap();
    assert_eq!(inverted.evaluate(0., 0.).unwrap().point[2], -2.);
    assert_eq!(inverted.evaluate(0.25, 0.).unwrap().point[2], 2.);
    for (w, l, a, n) in [
        (0., 1., 1., 1),
        (1., -1., 1., 1),
        (1., 1., f64::NAN, 1),
        (1., 1., 1., 0),
        (1., 1., 1., 6),
    ] {
        assert!(cubic([0.; 3], w, l, a, n).is_err());
    }
    assert!(cubic([f64::MAX, 0., 0.], 1., 1., 1., 1).is_err());
    assert!(cubic([0., 0., f64::MAX], 1., 1., f64::MAX, 1).is_err());
}
