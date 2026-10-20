use nurbs_core::worm::{Hand, Spec, certify_patches, derive_parameters, patches_certified};
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{x} outside {b:?}");
}
#[test]
fn nominal_axial_module_pitch_and_trapezoid_match_independent_equations() {
    for spec in [
        Spec::default(),
        Spec {
            starts: 4,
            axial_module: 2.,
            diameter_quotient: 14.,
            backlash: 0.1,
            ..Spec::default()
        },
    ] {
        let p = derive_parameters(spec).unwrap();
        let m = spec.axial_module;
        let pitch = std::f64::consts::PI * m;
        let rp = spec.diameter_quotient * m / 2.;
        let crest = pitch / 2. - spec.backlash - 2. * m * spec.pressure_angle.tan();
        let root =
            pitch / 2. - spec.backlash + 2. * m * spec.dedendum_factor * spec.pressure_angle.tan();
        contains(
            p.lead_angle,
            (spec.starts as f64 / spec.diameter_quotient).atan(),
        );
        contains(p.pitch, pitch);
        contains(p.lead, pitch * spec.starts as f64);
        contains(p.pitch_radius, rp);
        contains(p.crest_radius, rp + m);
        contains(p.root_radius, rp - m * spec.dedendum_factor);
        contains(p.crest_width, crest);
        contains(p.root_width, root);
        for (a, b) in p.profile_z.iter().zip([
            0.,
            crest / 2.,
            root / 2.,
            pitch - root / 2.,
            pitch - crest / 2.,
            pitch,
        ]) {
            contains(*a, b);
        }
    }
}
#[test]
fn every_saved_start_turn_quarter_matches_nominal_za_helicoid_for_both_hands() {
    for hand in [Hand::Right, Hand::Left] {
        let spec = Spec {
            starts: 2,
            turns: 2,
            hand,
            ..Spec::default()
        };
        let owned = patches_certified(spec, 2560).unwrap();
        owned.reverify().unwrap();
        assert!(owned.certificate().error_upper <= spec.error_budget);
        assert_eq!(owned.patches().len(), 80);
        let m = spec.axial_module;
        let pitch = std::f64::consts::PI * m;
        let lead = 2. * pitch;
        let rp = spec.diameter_quotient * m / 2.;
        let rc = rp + m;
        let rr = rp - m * spec.dedendum_factor;
        let wc = pitch / 2. - 2. * m * spec.pressure_angle.tan();
        let wr = pitch / 2. + 2. * m * spec.dedendum_factor * spec.pressure_angle.tan();
        let z = [
            0.,
            wc / 2.,
            wr / 2.,
            pitch - wr / 2.,
            pitch - wc / 2.,
            pitch,
        ];
        let radii = [rc, rc, rr, rr, rc, rc];
        let sign = if hand == Hand::Right { 1. } else { -1. };
        for patch in owned.patches() {
            for u in [0., 0.3, 1.] {
                for v in [0., 0.4, 1.] {
                    let i = patch.segment;
                    let radius = radii[i] + (radii[i + 1] - radii[i]) * u;
                    let angle = sign * std::f64::consts::FRAC_PI_2 * (patch.quarter as f64 + v);
                    let zz = patch.start as f64 * pitch
                        + (patch.turn as f64 + patch.quarter as f64 / 4.) * lead
                        + z[i]
                        + (z[i + 1] - z[i]) * u
                        + lead * v / 4.;
                    let p = patch.surface.evaluate(u, v).unwrap().point;
                    let error = (p[0] - radius * angle.cos())
                        .hypot(p[1] - radius * angle.sin())
                        .hypot(p[2] - zz);
                    assert!(error <= owned.certificate().error_upper + 1e-10);
                }
            }
        }
    }
}
#[test]
fn actual_surface_inventory_and_original_nominal_spec_cannot_reuse_a_receipt() {
    let spec = Spec::default();
    let owned = patches_certified(spec, 2560).unwrap();
    let mut altered = owned.patches().to_vec();
    altered[0].surface.control_points[0][1][0] += 0.01;
    assert!(certify_patches(spec, &altered, 2560).unwrap().error_upper > 0.009);
    altered = owned.patches().to_vec();
    altered.swap(0, 1);
    assert!(certify_patches(spec, &altered, 2560).is_err());
    altered = owned.patches().to_vec();
    altered.pop();
    assert!(certify_patches(spec, &altered, 2560).is_err());
    assert!(
        certify_patches(
            Spec {
                pressure_angle: 0.3,
                ..spec
            },
            owned.patches(),
            2560
        )
        .unwrap()
        .error_upper
            > 0.01
    );
    assert!(patches_certified(spec, 1).is_err());
    assert!(
        derive_parameters(Spec {
            pressure_angle: 0.65,
            ..spec
        })
        .is_err()
    );
    assert!(
        derive_parameters(Spec {
            axial_module: f64::NAN,
            ..spec
        })
        .is_err()
    );
    assert!(derive_parameters(Spec { starts: 0, ..spec }).is_err());
}

#[test]
fn maximum_original_census_and_different_module_pressure_allowances_replay() {
    for spec in [
        Spec {
            axial_module: 0.05,
            diameter_quotient: 6.,
            starts: 8,
            turns: 16,
            ..Spec::default()
        },
        Spec {
            axial_module: 2.,
            diameter_quotient: 14.,
            pressure_angle: 0.25,
            dedendum_factor: 1.3,
            backlash: 0.1,
            starts: 4,
            hand: Hand::Left,
            ..Spec::default()
        },
    ] {
        let owned = patches_certified(spec, 2560).unwrap();
        owned.reverify().unwrap();
        assert_eq!(owned.patches().len(), 20 * spec.starts * spec.turns);
        assert!(owned.certificate().error_upper <= spec.error_budget);
        assert!(patches_certified(spec, owned.patches().len() - 1).is_err());
    }
}
