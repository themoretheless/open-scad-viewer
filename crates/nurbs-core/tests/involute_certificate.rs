use nurbs_core::involute::{approximate_quintic, certify_quintic};
#[test]
fn stored_quintic_is_enclosed_including_rounding_and_nonuniform_knots() {
    for (a, b, r) in [
        (0., 1.3, 2.),
        (-2., 0.8, 0.7),
        (5., 7.5, 3.),
        (-8., -7., 0.5),
        (7., 8., 1.),
    ] {
        let center = [3., -2., 4.];
        let approx = approximate_quintic(center, r, a, b, 1e-5).unwrap();
        let cert = certify_quintic(&approx.curve, center, r, a, b).unwrap();
        assert!(cert.rounding_included);
        assert!(cert.error_upper < 1.01e-5, "{cert:?}");
        for i in 0..=300 {
            let u = i as f64 / 300.;
            let t = a + (b - a) * u;
            let expected = [
                center[0] + r * (t.cos() + t * t.sin()),
                center[1] + r * (t.sin() - t * t.cos()),
                center[2],
            ];
            let p = approx.curve.evaluate(u).unwrap().point;
            let error = (0..3)
                .map(|k| (p[k] - expected[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= cert.error_upper + 1e-13);
        }
        let mut shifted = approx.curve.clone();
        shifted.control_points[2][0] += 0.01;
        let bad = certify_quintic(&shifted, center, r, a, b).unwrap();
        assert!(bad.control_difference_upper >= 0.009999999);
        let mut knots = approx.curve.clone();
        if cert.spans > 1 {
            for k in &mut knots.knots {
                if *k == 1. / cert.spans as f64 {
                    *k *= 0.91;
                }
            }
            let altered = certify_quintic(&knots, center, r, a, b).unwrap();
            assert!(altered.control_difference_upper > cert.control_difference_upper);
        }
    }
}
#[test]
fn unsupported_layout_and_invalid_inputs_are_refused() {
    let a = approximate_quintic([0.; 3], 1., 0., 1., 1e-4).unwrap();
    assert!(certify_quintic(&a.curve, [0.; 3], 1., 0., 9.).is_err());
    assert!(certify_quintic(&a.curve, [0.; 3], -1., 0., 1.).is_err());
    let mut c = a.curve.clone();
    c.weights[1] = 2.;
    assert!(certify_quintic(&c, [0.; 3], 1., 0., 1.).is_err());
    let mut c = a.curve.clone();
    c.periodic = true;
    assert!(certify_quintic(&c, [0.; 3], 1., 0., 1.).is_err());
}

#[test]
fn placed_flanks_include_rotation_and_reflection_rounding() {
    use nurbs_core::involute::certify_quintic_placed;
    for angle in [-6.2_f64, 0., 0.7, 6.2] {
        for reflection in [-1., 1.] {
            let approximation = approximate_quintic([0.; 3], 20., 0.1, 0.7, 5e-7).unwrap();
            let mut curve = approximation.curve;
            let (sin, cos) = angle.sin_cos();
            for p in &mut curve.control_points {
                let (x, y) = (p[0], reflection * p[1]);
                *p = vec![cos * x - sin * y, sin * x + cos * y];
            }
            let cert =
                certify_quintic_placed(&curve, [0.; 3], 20., 0.1, 0.7, angle, reflection).unwrap();
            assert!(
                cert.rounding_included && cert.error_upper < 1e-6,
                "{cert:?}"
            );
            let reverse = curve.reverse().unwrap();
            let backwards =
                certify_quintic_placed(&reverse, [0.; 3], 20., 0.7, 0.1, angle, reflection)
                    .unwrap();
            assert!(backwards.error_upper < 1e-6, "{backwards:?}");
            for i in 0..=100 {
                let u = i as f64 / 100.;
                let t = 0.1 + 0.6 * u;
                // Independent polar roll geometry.
                let radius = 20. * t.hypot(1.);
                let phase = angle + reflection * (t - t.atan());
                let q = curve.evaluate(u).unwrap().point;
                assert!(
                    (q[0] - radius * phase.cos()).hypot(q[1] - radius * phase.sin())
                        <= cert.error_upper + 1e-12
                );
            }
        }
    }
}
