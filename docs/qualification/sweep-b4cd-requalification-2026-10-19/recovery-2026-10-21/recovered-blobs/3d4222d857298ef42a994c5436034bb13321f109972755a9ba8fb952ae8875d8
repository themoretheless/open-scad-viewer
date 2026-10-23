use nurbs_core::{curve::Curve, primitives::certify_circle_arc_radians as certify};
fn arc(radius: f64, start: f64, end: f64) -> Curve {
    let w = ((end - start) / 2.).cos();
    let mid = (start + end) / 2.;
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius * start.cos(), radius * start.sin()],
            vec![radius / w * mid.cos(), radius / w * mid.sin()],
            vec![radius * end.cos(), radius * end.sin()],
        ],
        weights: vec![1., w, 1.],
        periodic: false,
    }
}
#[test]
fn inclusive_rational_family_bound_covers_angles_radius_weights_and_reversal() {
    for (radius, start, end) in [(2., 0.1, 0.9), (5000., 5.7, 6.2), (3., 1.2, -0.2)] {
        let c = arc(radius, start, end);
        let epsilon = 1e-10;
        let rr = [radius - epsilon, radius + epsilon];
        let aa = [start - epsilon, start + epsilon];
        let bb = [end - epsilon, end + epsilon];
        let p = certify(&c, [0.; 3], rr, aa, bb).unwrap();
        assert!(p.rounding_included && p.error_upper < 1e-4);
        for r in rr {
            for a in aa {
                for b in bb {
                    let ideal = arc(r, a, b);
                    for i in 0..=50 {
                        let u = i as f64 / 50.;
                        let q = c.evaluate(u).unwrap().point;
                        // Independent rational quadratic evaluation.
                        let basis = [(1. - u) * (1. - u), 2. * u * (1. - u), u * u];
                        let denominator = (0..3).map(|k| basis[k] * ideal.weights[k]).sum::<f64>();
                        let p0: [f64; 2] = std::array::from_fn(|d| {
                            (0..3)
                                .map(|k| basis[k] * ideal.weights[k] * ideal.control_points[k][d])
                                .sum::<f64>()
                                / denominator
                        });
                        assert!((q[0] - p0[0]).hypot(q[1] - p0[1]) <= p.error_upper + 1e-10);
                    }
                }
            }
        }
    }
}
#[test]
fn saved_weight_and_endpoint_changes_are_measured_and_large_sweeps_refused() {
    let mut c = arc(2., 0.1, 0.9);
    c.weights[1] += 1e-5;
    c.control_points[0][0] += 2e-5;
    let p = certify(&c, [0.; 3], [2.; 2], [0.1; 2], [0.9; 2]).unwrap();
    assert!(p.error_upper >= 2e-5 && p.error_upper < 1e-3);
    assert!(certify(&c, [0.; 3], [2.; 2], [0.; 2], [6.2; 2]).is_err());
    assert!(certify(&c, [0.; 3], [2.; 2], [0.; 2], [12.5; 2]).is_err());
    assert!(certify(&c, [0.; 3], [0.; 2], [0.1; 2], [0.9; 2]).is_err());
    assert!(certify(&c, [0.; 3], [2.; 2], [f64::NAN; 2], [0.9; 2]).is_err());
}
#[test]
fn full_nominal_gear_half_angles_include_independent_involute_phase() {
    use nurbs_core::involute::derive_gear_parameter_bounds as derive;
    for (n, internal) in [(3, false), (20, false), (40, true), (256, false)] {
        let p = derive(1., n, 20., 0.25, 0.02, internal).unwrap();
        let alpha = 20_f64.to_radians();
        let pitch = n as f64 / 2.;
        let base = pitch * alpha.cos();
        let half_base = std::f64::consts::PI / (2. * n as f64)
            + if internal {
                0.02 / (2. * pitch)
            } else {
                -0.02 / (2. * pitch)
            }
            + alpha.tan()
            - alpha;
        let low = if internal { pitch - 1. } else { pitch - 1.25 };
        let high = if internal { pitch + 1.25 } else { pitch + 1. };
        for (radius, bound) in [(low.max(base), p.half_low_angle), (high, p.half_high_angle)] {
            let roll = ((radius / base).powi(2) - 1.).max(0.).sqrt();
            let v = half_base - roll + roll.atan();
            assert!(bound[0] <= v && v <= bound[1], "{v} outside {bound:?}");
        }
    }
}

#[test]
fn radial_transition_certificate_bounds_the_complete_saved_line() {
    use nurbs_core::primitives::certify_radial_segment_radians as certify;
    let angle = 0.7_f64;
    let mut c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![2. * angle.cos(), 2. * angle.sin()],
            vec![3. * angle.cos(), 3. * angle.sin()],
        ],
        weights: vec![2., 2.],
        periodic: false,
    };
    c.control_points[0][0] += 1e-7;
    let r = certify(&c, [0.; 3], [2.; 2], [3.; 2], [angle; 2]).unwrap();
    assert!(r.rounding_included && r.error_upper >= 1e-7 && r.error_upper < 1e-6);
    for i in 0..=100 {
        let u = i as f64 / 100.;
        let p = c.evaluate(u).unwrap().point;
        let radius = 2. + u;
        assert!(
            (p[0] - radius * angle.cos()).hypot(p[1] - radius * angle.sin())
                <= r.error_upper + 1e-14
        );
    }
    c.weights[1] = 1.;
    assert!(certify(&c, [0.; 3], [2.; 2], [3.; 2], [angle; 2]).is_err());
}
