use nurbs_core::involute::{self, derive_gear_parameter_bounds as derive};
fn contains(bound: [f64; 2], value: f64) {
    assert!(
        bound[0] <= value && value <= bound[1],
        "{value} outside {bound:?}"
    )
}
#[test]
fn full_parameter_boxes_enclose_independent_nominal_gear_equations() {
    for (module, teeth, pressure, internal) in [
        (1., 16, 20., false),
        (2.5, 40, 20., true),
        (0.1, 100, 35., false),
        (100., 100, 10., false),
    ] {
        let p = derive(
            module,
            teeth,
            pressure,
            0.25 * module,
            0.02 * module,
            internal,
        )
        .unwrap();
        let alpha: f64 = pressure * std::f64::consts::PI / 180.;
        let pitch = module * teeth as f64 / 2.;
        let base = pitch * alpha.cos();
        let (tip, root) = if internal {
            (pitch - module, pitch + 1.25 * module)
        } else {
            (pitch + module, pitch - 1.25 * module)
        };
        contains(p.pitch_radius, pitch);
        contains(p.base_radius, base);
        contains(p.tip_radius, tip);
        contains(p.root_radius, root);
        let low = if internal { tip } else { root };
        let high = if internal { root } else { tip };
        contains(
            p.roll_start,
            ((low.max(base) / base).powi(2) - 1.).max(0.).sqrt(),
        );
        contains(p.roll_end, ((high / base).powi(2) - 1.).sqrt());
        let half = std::f64::consts::PI / (2. * teeth as f64)
            + if internal {
                0.02 * module / (2. * pitch)
            } else {
                -0.02 * module / (2. * pitch)
            }
            + alpha.tan()
            - alpha;
        contains(p.half_base_angle, half);
        contains(p.tooth_step, std::f64::consts::TAU / teeth as f64);
    }
}
#[test]
fn parameter_family_certificate_bounds_actual_curve_against_all_box_corners() {
    let curve = involute::approximate_quintic([0.; 3], 20., 0.1, 0.7, 1e-6)
        .unwrap()
        .curve;
    let radius = [20. - 1e-8, 20. + 1e-8];
    let start = [0.1 - 1e-8, 0.1 + 1e-8];
    let end = [0.7 - 1e-8, 0.7 + 1e-8];
    let r =
        involute::certify_quintic_parameter_box(&curve, [0.; 3], radius, start, end, [0., 0.], 1.)
            .unwrap();
    assert!(r.rounding_included);
    for rb in radius {
        for a in start {
            for b in end {
                for i in 0..=50 {
                    let u = i as f64 / 50.;
                    let t = a + (b - a) * u;
                    let rho = rb * t.hypot(1.);
                    let phi = t - t.atan();
                    let p = curve.evaluate(u).unwrap().point;
                    assert!(
                        (p[0] - rho * phi.cos()).hypot(p[1] - rho * phi.sin())
                            <= r.error_upper + 1e-12
                    );
                }
            }
        }
    }
}
#[test]
fn invalid_parameter_derivations_return_errors_without_point_panics() {
    for (m, n, p, c, b) in [
        (f64::NAN, 16, 20., 0., 0.),
        (1., 0, 20., 0., 0.),
        (1., 16, 90., 0., 0.),
        (1., 16, 20., f64::NAN, 0.),
        (1., 16, 20., 0., f64::INFINITY),
        (1., 16, 20., -1., 0.),
    ] {
        assert!(derive(m, n, p, c, b, false).is_err());
    }
}
