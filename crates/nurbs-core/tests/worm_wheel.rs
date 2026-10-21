use nurbs_core::{
    worm,
    worm_wheel::{Flank, Spec, Status, contacts, verify_contact_brackets},
};
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] - 1e-11 <= x && x <= b[1] + 1e-11, "{x} outside {b:?}");
}
#[test]
fn both_original_flanks_and_hands_have_complete_simple_contact_pairs() {
    for hand in [worm::Hand::Right, worm::Hand::Left] {
        for flank in [Flank::Descending, Flank::Ascending] {
            let spec = Spec {
                worm: worm::Spec {
                    hand,
                    ..worm::Spec::default()
                },
                ..Spec::default()
            };
            let u = if flank == Flank::Descending { 0. } else { 1. };
            let report = contacts(spec, flank, u, 0.2, 1e-9).unwrap();
            report.reverify().unwrap();
            assert_eq!(report.status(), Status::TwoSimpleRoots);
            assert_eq!(report.contacts().len(), 2);
            for contact in report.contacts() {
                let b = contact.contact_angle();
                assert!(b[1] - b[0] <= 1e-9);
                let d = contact.derivative();
                assert!(d[0] > 0. || d[1] < 0.);
            }
        }
    }
}
#[test]
fn contact_boxes_match_independent_worm_normals_relative_velocity_and_wheel_transform() {
    for (hand, flank, u, theta) in [
        (worm::Hand::Right, Flank::Descending, 0., 0.2),
        (worm::Hand::Left, Flank::Ascending, 1., -0.2),
    ] {
        let spec = Spec {
            worm: worm::Spec {
                hand,
                ..worm::Spec::default()
            },
            ..Spec::default()
        };
        let report = contacts(spec, flank, u, theta, 1e-9).unwrap();
        assert_eq!(report.status(), Status::TwoSimpleRoots);
        let w = spec.worm;
        let m = w.axial_module;
        let rp = m * w.diameter_quotient / 2.;
        let rc = rp + m;
        let rr = rp - w.dedendum_factor * m;
        let p = std::f64::consts::PI * m;
        let wc = p / 2. - w.backlash - 2. * m * w.pressure_angle.tan();
        let wr = p / 2. - w.backlash + 2. * m * w.dedendum_factor * w.pressure_angle.tan();
        let (r, z0, k) = if flank == Flank::Descending {
            (
                rc + (rr - rc) * u,
                wc / 2. + (wr - wc) * u / 2.,
                -w.pressure_angle.tan(),
            )
        } else {
            (
                rr + (rc - rr) * u,
                p - wr / 2. + (wr - wc) * u / 2.,
                w.pressure_angle.tan(),
            )
        };
        let sign = if hand == worm::Hand::Right { 1. } else { -1. };
        let h = sign * m * w.starts as f64 / 2.;
        let z = z0 + spec.start as f64 * p + h * theta + spec.axial_shift;
        let ratio = -sign * w.starts as f64 / spec.teeth as f64;
        let center = rp + m * spec.teeth as f64 / 2.;
        for c in report.contacts() {
            let b = c.contact_angle();
            let beta = (b[0] + b[1]) / 2.;
            let phi = beta - theta;
            contains(c.worm_rotation(), phi);
            let x = r * beta.cos();
            let y = r * beta.sin();
            let phase = ratio * phi;
            let point = [
                phase.cos() * (x - center) - phase.sin() * z,
                y,
                phase.sin() * (x - center) + phase.cos() * z,
            ];
            for d in 0..3 {
                contains(c.wheel_point()[d], point[d]);
            }
            let normal = [
                h * beta.sin() - k * r * beta.cos(),
                -h * beta.cos() - k * r * beta.sin(),
                r,
            ];
            let velocity = [-y - ratio * z, x, ratio * (x - center)];
            for d in 0..3 {
                contains(c.normal_housing()[d], normal[d]);
                contains(c.relative_velocity()[d], velocity[d]);
            }
            let residual = (0..3).map(|d| normal[d] * velocity[d]).sum::<f64>();
            assert!(residual.abs() < 1e-8);
        }
    }
}
#[test]
fn exclusions_and_tangent_neighborhoods_never_become_fabricated_contacts() {
    let excluded = contacts(Spec::default(), Flank::Descending, 1., 0., 1e-9).unwrap();
    assert_eq!(excluded.status(), Status::Excluded);
    excluded.reverify().unwrap();
    assert!(excluded.contacts().is_empty());
    let tangent = Spec {
        worm: worm::Spec {
            dedendum_factor: 1.,
            ..worm::Spec::default()
        },
        axial_shift: -std::f64::consts::PI / 4.,
        ..Spec::default()
    };
    let report = contacts(tangent, Flank::Descending, 0.5, 0., 1e-9).unwrap();
    assert_eq!(report.status(), Status::Unresolved);
    assert!(report.contacts().is_empty());
    assert!(contacts(Spec::default(), Flank::Descending, 2., 0., 1e-9).is_err());
}
#[test]
fn independent_bracket_replay_refuses_wrong_missing_and_overlapping_roots() {
    let s = Spec::default();
    let good = contacts(s, Flank::Descending, 0., 0., 1e-9).unwrap();
    let brackets = good
        .contacts()
        .iter()
        .map(|c| c.contact_angle())
        .collect::<Vec<_>>();
    verify_contact_brackets(s, Flank::Descending, 0., 0., &brackets)
        .unwrap()
        .reverify()
        .unwrap();
    assert!(verify_contact_brackets(s, Flank::Descending, 0., 0., &brackets[..1]).is_err());
    assert!(
        verify_contact_brackets(s, Flank::Descending, 0., 0., &[brackets[0], brackets[0]]).is_err()
    );
    let changed = brackets
        .iter()
        .map(|b| [b[0] + 0.01, b[1] + 0.01])
        .collect::<Vec<_>>();
    assert!(verify_contact_brackets(s, Flank::Descending, 0., 0., &changed).is_err());
}

#[test]
fn independently_differentiated_generating_motion_satisfies_the_envelope_triple_product() {
    fn family(s: Spec, f: Flank, u: f64, t: f64, phi: f64) -> [f64; 3] {
        let w = s.worm;
        let m = w.axial_module;
        let p = std::f64::consts::PI * m;
        let rp = m * w.diameter_quotient / 2.;
        let rc = rp + m;
        let rr = rp - w.dedendum_factor * m;
        let wc = p / 2. - w.backlash - 2. * m * w.pressure_angle.tan();
        let wr = p / 2. - w.backlash + 2. * m * w.dedendum_factor * w.pressure_angle.tan();
        let (r, z) = match f {
            Flank::Descending => (rc * (1. - u) + rr * u, (1. - u) * wc / 2. + u * wr / 2.),
            Flank::Ascending => (
                rr * (1. - u) + rc * u,
                (1. - u) * (p - wr / 2.) + u * (p - wc / 2.),
            ),
        };
        let sign = if w.hand == worm::Hand::Right { 1. } else { -1. };
        let z = z + s.start as f64 * p + sign * m * w.starts as f64 * t / 2. + s.axial_shift;
        let local = [r * t.cos(), r * t.sin(), z];
        let housing = [
            phi.cos() * local[0] - phi.sin() * local[1],
            phi.sin() * local[0] + phi.cos() * local[1],
            z,
        ];
        let relative = [
            housing[0] - (rp + m * s.teeth as f64 / 2.),
            housing[1],
            housing[2],
        ];
        let wheel_angle = -sign * w.starts as f64 * phi / s.teeth as f64;
        [
            wheel_angle.cos() * relative[0] - wheel_angle.sin() * relative[2],
            relative[1],
            wheel_angle.sin() * relative[0] + wheel_angle.cos() * relative[2],
        ]
    }
    for (hand, flank, u, t) in [
        (worm::Hand::Right, Flank::Descending, 0., 0.2),
        (worm::Hand::Left, Flank::Ascending, 1., -0.2),
    ] {
        let spec = Spec {
            worm: worm::Spec {
                hand,
                ..worm::Spec::default()
            },
            ..Spec::default()
        };
        let report = contacts(spec, flank, u, t, 1e-9).unwrap();
        assert_eq!(report.status(), Status::TwoSimpleRoots);
        for contact in report.contacts() {
            let b = contact.worm_rotation();
            let phi = (b[0] + b[1]) / 2.;
            let h = 1e-5;
            let diff = |a: [f64; 3], b: [f64; 3]| {
                std::array::from_fn::<_, 3, _>(|d| (a[d] - b[d]) / (2. * h))
            };
            let du = diff(
                family(spec, flank, u + h, t, phi),
                family(spec, flank, u - h, t, phi),
            );
            let dt = diff(
                family(spec, flank, u, t + h, phi),
                family(spec, flank, u, t - h, phi),
            );
            let dp = diff(
                family(spec, flank, u, t, phi + h),
                family(spec, flank, u, t, phi - h),
            );
            let n = [
                du[1] * dt[2] - du[2] * dt[1],
                du[2] * dt[0] - du[0] * dt[2],
                du[0] * dt[1] - du[1] * dt[0],
            ];
            let norm = |x: [f64; 3]| x[0].hypot(x[1]).hypot(x[2]);
            let residual =
                (0..3).map(|d| n[d] * dp[d]).sum::<f64>().abs() / norm(n) / norm(dp).max(1.);
            assert!(residual < 1e-7, "{residual}");
        }
    }
}

#[test]
fn original_multi_start_phase_is_bound_to_the_contact_equation() {
    let spec = Spec {
        worm: worm::Spec {
            starts: 2,
            ..worm::Spec::default()
        },
        start: 1,
        ..Spec::default()
    };
    let report = contacts(spec, Flank::Descending, 0., -2., 1e-9).unwrap();
    assert_eq!(report.status(), Status::TwoSimpleRoots);
    report.reverify().unwrap();
    let wrong = Spec { start: 0, ..spec };
    let brackets = report
        .contacts()
        .iter()
        .map(|c| c.contact_angle())
        .collect::<Vec<_>>();
    assert!(verify_contact_brackets(wrong, Flank::Descending, 0., -2., &brackets).is_err());
    assert!(contacts(Spec { start: 2, ..spec }, Flank::Descending, 0., -2., 1e-9).is_err());
}
