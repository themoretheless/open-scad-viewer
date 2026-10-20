use nurbs_core::involute::{
    BevelGearSpec as Spec, derive_bevel_gear_parameters as derive,
    qualified_nominal_bevel_flank as construct,
};
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{x} outside {b:?}");
}
fn oracle(s: Spec) -> (f64, f64, f64, f64, [f64; 2]) {
    let r = s.module * s.teeth as f64 / (2. * s.pitch_cone_angle.sin());
    let base = (s.pitch_cone_angle.sin() * s.pressure_angle.cos()).asin();
    let tip = s.pitch_cone_angle + (s.addendum / r).atan();
    let root = s.pitch_cone_angle - (s.dedendum / r).atan();
    let roll = |d: f64| (d.cos() / base.cos()).acos() / base.sin();
    (
        r,
        base,
        tip,
        root,
        [if root < base { 0. } else { roll(root) }, roll(tip)],
    )
}
#[test]
fn nominal_boxes_cover_independent_cone_and_unwinding_equations() {
    for s in [
        Spec::default(),
        Spec {
            teeth: 100,
            ..Spec::default()
        },
        Spec {
            pitch_cone_angle: 0.2,
            ..Spec::default()
        },
        Spec {
            pitch_cone_angle: 1.3,
            ..Spec::default()
        },
        Spec {
            module: 0.2,
            teeth: 40,
            face_width: 0.3,
            addendum: 0.2,
            dedendum: 0.25,
            backlash: 0.01,
            ..Spec::default()
        },
    ] {
        let p = derive(s).unwrap();
        let (r, base, tip, root, roll) = oracle(s);
        contains(p.cone_distances[1], r);
        contains(p.cone_distances[0], r - s.face_width);
        contains(p.base_cone_angle, base);
        contains(p.tip_cone_angle, tip);
        contains(p.root_cone_angle, root);
        contains(p.roll[0], roll[0]);
        contains(p.roll[1], roll[1]);
        let t = (s.pitch_cone_angle.cos() / base.cos()).acos() / base.sin();
        contains(p.pitch_roll, t);
        contains(
            p.pitch_involute_azimuth,
            t - ((base.sin() * t).tan() / base.sin()).atan(),
        );
        contains(p.tooth_step, std::f64::consts::TAU / s.teeth as f64);
        contains(
            p.half_pitch_tooth_angle,
            std::f64::consts::PI / (2. * s.teeth as f64) - s.backlash / (s.module * s.teeth as f64),
        );
        assert!(p.base_cone_angle[1] - p.base_cone_angle[0] < 1e-10);
        assert!(p.roll[1][1] - p.roll[1][0] < 1e-9);
    }
}
#[test]
fn nominal_owner_binds_whole_saved_flank_to_original_module_and_cones() {
    for s in [
        Spec::default(),
        Spec {
            teeth: 100,
            ..Spec::default()
        },
        Spec {
            pitch_cone_angle: 0.2,
            ..Spec::default()
        },
    ] {
        let owned = construct(s, 1e-6, 16).unwrap();
        owned.reverify().unwrap();
        let (r, base, _, _, roll) = oracle(s);
        assert!(owned.error_upper() <= 1e-6);
        for surface in owned.patches() {
            let a = surface.knots_u[3];
            let b = surface.knots_u[surface.control_points.len()];
            for u in [a, (a + b) / 2., b] {
                let t = roll[0] + (roll[1] - roll[0]) * u;
                let w = base.sin() * t;
                let q = [
                    base.sin() * w.cos() * t.cos() + w.sin() * t.sin(),
                    base.sin() * w.cos() * t.sin() - w.sin() * t.cos(),
                    base.cos() * w.cos(),
                ];
                for v in [0., 0.5, 1.] {
                    let radius = r - s.face_width * (1. - v);
                    let p = surface.evaluate(u, v).unwrap().point;
                    let error = (p[0] - radius * q[0])
                        .hypot(p[1] - radius * q[1])
                        .hypot(p[2] - radius * q[2]);
                    assert!(error <= owned.error_upper() + 1e-10);
                }
            }
        }
    }
}
#[test]
fn invalid_nominal_gears_and_unprovable_tolerance_are_refused() {
    for s in [
        Spec {
            teeth: 0,
            ..Spec::default()
        },
        Spec {
            module: f64::NAN,
            ..Spec::default()
        },
        Spec {
            face_width: 100.,
            ..Spec::default()
        },
        Spec {
            pitch_cone_angle: 0.,
            ..Spec::default()
        },
        Spec {
            teeth: 6,
            pitch_cone_angle: 1.4,
            ..Spec::default()
        },
        Spec {
            backlash: -1.,
            ..Spec::default()
        },
        Spec {
            dedendum: 0.,
            ..Spec::default()
        },
    ] {
        assert!(derive(s).is_err());
    }
    assert!(construct(Spec::default(), 1e-20, 16).is_err());
    assert!(construct(Spec::default(), 0., 16).is_err());
}
