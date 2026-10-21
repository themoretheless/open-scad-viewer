use nurbs_core::involute::{
    BevelGearSpec as Spec, BevelSide, certify_bevel_teeth as certify,
    qualified_bevel_teeth as construct,
};
fn nominal(s: Spec) -> (f64, f64, [f64; 2], f64) {
    let r = s.module * s.teeth as f64 / (2. * s.pitch_cone_angle.sin());
    let b = (s.pitch_cone_angle.sin() * s.pressure_angle.cos()).asin();
    let tip = s.pitch_cone_angle + (s.addendum / r).atan();
    let root = s.pitch_cone_angle - (s.dedendum / r).atan();
    let roll = |d: f64| (d.cos() / b.cos()).acos() / b.sin();
    let pitch = roll(s.pitch_cone_angle);
    let inv = pitch - ((b.sin() * pitch).tan() / b.sin()).atan();
    let half =
        std::f64::consts::PI / (2. * s.teeth as f64) - s.backlash / (s.module * s.teeth as f64);
    (
        r,
        b,
        [if root < b { 0. } else { roll(root) }, roll(tip)],
        half + inv,
    )
}
#[test]
fn every_left_right_tooth_matches_independent_original_nominal_formula() {
    for s in [
        Spec::default(),
        Spec {
            teeth: 40,
            backlash: 0.05,
            ..Spec::default()
        },
    ] {
        let owned = construct(s, 1e-6, 16, 4096).unwrap();
        owned.reverify().unwrap();
        assert_eq!(owned.flanks().len(), 2 * s.teeth);
        assert!(owned.certificate().world_error_upper <= 1e-6);
        assert!(owned.certificate().nominal_tip_half_angle[0] > 0.);
        let (r, b, roll, offset) = nominal(s);
        for flank in owned.flanks() {
            let sign = if flank.side == BevelSide::Left {
                1.
            } else {
                -1.
            };
            let angle = std::f64::consts::TAU * flank.tooth as f64 / s.teeth as f64 - sign * offset;
            let (sa, ca) = angle.sin_cos();
            for patch in &flank.patches {
                let a = patch.knots_u[3];
                let z = patch.knots_u[patch.control_points.len()];
                for u in [a, (a + z) / 2., z] {
                    let t = roll[0] + (roll[1] - roll[0]) * u;
                    let w = b.sin() * t;
                    let x = b.sin() * w.cos() * t.cos() + w.sin() * t.sin();
                    let y = sign * (b.sin() * w.cos() * t.sin() - w.sin() * t.cos());
                    for v in [0., 1.] {
                        let radius = r - s.face_width * (1. - v);
                        let q = [
                            radius * (ca * x - sa * y),
                            radius * (sa * x + ca * y),
                            radius * b.cos() * w.cos(),
                        ];
                        let p = patch.evaluate(u, v).unwrap().point;
                        let error = (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]);
                        assert!(
                            error <= owned.certificate().world_error_upper + 1e-10,
                            "{error}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn tooth_inventory_original_basis_and_saved_placement_are_requalified() {
    let owned = construct(Spec::default(), 1e-6, 16, 4096).unwrap();
    let mut flanks = owned.flanks().to_vec();
    flanks.swap(0, 1);
    assert!(certify(owned.source(), &flanks).is_err());
    flanks = owned.flanks().to_vec();
    flanks[1].side = BevelSide::Left;
    assert!(certify(owned.source(), &flanks).is_err());
    flanks = owned.flanks().to_vec();
    flanks.pop();
    assert!(certify(owned.source(), &flanks).is_err());
    flanks = owned.flanks().to_vec();
    flanks[0].patches[0].control_points[1][0][0] += 0.01;
    assert!(certify(owned.source(), &flanks).unwrap().world_error_upper > 0.009);
    flanks = owned.flanks().to_vec();
    flanks[0].patches[0].weights[1][0] = 2.;
    assert!(certify(owned.source(), &flanks).is_err());
}
#[test]
fn insufficient_inventory_budgets_and_pointed_nominal_teeth_are_refused() {
    assert!(construct(Spec::default(), 1e-6, 16, 1).is_err());
    assert!(construct(Spec::default(), 1e-6, 0, 4096).is_err());
    assert!(construct(Spec::default(), 0., 16, 4096).is_err());
    assert!(
        construct(
            Spec {
                backlash: 0.99,
                ..Spec::default()
            },
            1e-6,
            16,
            4096
        )
        .is_err()
    );
}
