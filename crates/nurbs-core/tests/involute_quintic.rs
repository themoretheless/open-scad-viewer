use nurbs_core::involute;
#[test]
fn quintic_involute_matches_analytic_positions_and_endpoint_jets() {
    for (start, end) in [(0., 0.7), (-1., 2.), (1., 4.)] {
        let a = involute::approximate_quintic([3., 4., 5.], 2., start, end, 1e-6).unwrap();
        assert_eq!(a.curve.degree, 5);
        assert!(a.real_arithmetic_error_estimate <= 1e-6);
        for i in 0..=500 {
            let u = i as f64 / 500.;
            let t = start + (end - start) * u;
            let p = a.curve.evaluate(u).unwrap().point;
            let x = 3. + 2. * (t.cos() + t * t.sin());
            let y = 4. + 2. * (t.sin() - t * t.cos());
            assert!((p[0] - x).hypot(p[1] - y) <= a.real_arithmetic_error_estimate + 1e-11);
            assert_eq!(p[2], 5.);
        }
        for (u, t) in [(0., start), (1., end)] {
            let e = a.curve.evaluate(u).unwrap();
            let d = e.d1.unwrap();
            let dd = e.d2.unwrap();
            let range = end - start;
            assert!((d[0] - 2. * t * t.cos() * range).abs() < 1e-9);
            assert!((d[1] - 2. * t * t.sin() * range).abs() < 1e-9);
            assert!((dd[0] - 2. * (t.cos() - t * t.sin()) * range * range).abs() < 1e-8);
            assert!((dd[1] - 2. * (t.sin() + t * t.cos()) * range * range).abs() < 1e-8);
        }
    }
}
#[test]
fn quintic_budget_refusals_and_control_count_are_explicit() {
    let a = involute::approximate_quintic([0.; 3], 20., 0., 0.7, 1e-6).unwrap();
    assert!(a.curve.control_points.len() <= 32);
    assert!(involute::approximate_quintic([0.; 3], 2., 0., 100., 1e-6).is_err());
    for (r, s, e, b) in [
        (0., 0., 1., 1e-6),
        (2., 1., 0., 1e-6),
        (2., 0., 1., 0.),
        (2., 0., 1., f64::NAN),
    ] {
        assert!(involute::approximate_quintic([0.; 3], r, s, e, b).is_err());
    }
}
