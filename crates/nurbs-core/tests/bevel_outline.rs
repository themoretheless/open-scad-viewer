use nurbs_core::involute::{
    BevelGearSpec as Spec, certify_bevel_outline as certify, qualified_bevel_outline as construct,
};
fn closed(curves: &[nurbs_core::curve::Curve]) {
    for i in 0..curves.len() {
        assert_eq!(
            curves[i].control_points.last(),
            curves[(i + 1) % curves.len()].control_points.first()
        );
    }
}
#[test]
fn complete_profile_closes_below_base_and_above_base_root_cones() {
    for teeth in [20, 100] {
        let s = Spec {
            teeth,
            ..Spec::default()
        };
        let owned = construct(s, 1e-6, 16, 4096, 8192).unwrap();
        owned.reverify().unwrap();
        closed(owned.curves());
        assert!(owned.certificate().world_error_upper <= 1e-6);
        let below = owned.source().source().parameters().root_cone_angle[1]
            < owned.source().source().parameters().base_cone_angle[0];
        assert_eq!(below, teeth == 20);
        assert_eq!(
            owned.certificate().transitions,
            teeth * if below { 4 } else { 2 }
        );
        let r = s.module * s.teeth as f64 / (2. * s.pitch_cone_angle.sin());
        for c in owned.curves() {
            let [a, b] = c.domain();
            for u in [0., 0.25, 0.5, 0.75, 1.] {
                let t = if u == 1. { b } else { a + (b - a) * u };
                let p = c.evaluate_de_boor(t).unwrap();
                let radius = p[0].hypot(p[1]).hypot(p[2]);
                assert!((radius - r).abs() <= owned.certificate().world_error_upper + 1e-10);
            }
        }
    }
}
#[test]
fn transition_nominal_poles_and_original_flank_ownership_are_rechecked() {
    let owned = construct(Spec::default(), 1e-6, 16, 4096, 8192).unwrap();
    let mut curves = owned.curves().to_vec();
    curves[0].control_points[1][0] += 0.01;
    assert!(
        certify(owned.source(), &curves)
            .unwrap()
            .transition_error_upper
            > 0.009
    );
    curves = owned.curves().to_vec();
    curves[0].control_points[0][0] += 0.001;
    assert!(certify(owned.source(), &curves).is_err());
    curves = owned.curves().to_vec();
    let i = curves.iter().position(|c| c.degree == 3).unwrap();
    curves[i].control_points[1][0] += 0.001;
    assert!(certify(owned.source(), &curves).is_err());
    curves = owned.curves().to_vec();
    curves.pop();
    assert!(certify(owned.source(), &curves).is_err());
    assert!(construct(Spec::default(), 1e-6, 16, 4096, 1).is_err());
}

fn reference_arc(r: f64, b: [f64; 2], a: [f64; 2], meridian: bool, t: f64) -> [f64; 3] {
    let polar = |beta: f64, phi: f64| {
        [
            r * beta.sin() * phi.cos(),
            r * beta.sin() * phi.sin(),
            r * beta.cos(),
        ]
    };
    let sweep = if meridian { b[1] - b[0] } else { a[1] - a[0] };
    let w = (sweep / 2.).cos();
    let mut middle = polar((b[0] + b[1]) / 2., (a[0] + a[1]) / 2.);
    middle[0] /= w;
    middle[1] /= w;
    if meridian {
        middle[2] /= w;
    }
    let p = [polar(b[0], a[0]), middle, polar(b[1], a[1])];
    let basis = [(1. - t).powi(2), 2. * t * (1. - t) * w, t * t];
    let denominator = basis.iter().sum::<f64>();
    std::array::from_fn(|d| (0..3).map(|i| basis[i] * p[i][d]).sum::<f64>() / denominator)
}
#[test]
fn every_transition_matches_independent_nominal_rational_circle_parameterization() {
    for teeth in [20, 100] {
        let s = Spec {
            teeth,
            ..Spec::default()
        };
        let owned = construct(s, 1e-6, 16, 4096, 8192).unwrap();
        let r = s.module * teeth as f64 / (2. * s.pitch_cone_angle.sin());
        let base = (s.pitch_cone_angle.sin() * s.pressure_angle.cos()).asin();
        let tip = s.pitch_cone_angle + (s.addendum / r).atan();
        let root = s.pitch_cone_angle - (s.dedendum / r).atan();
        let inv = |beta: f64| {
            let t = (beta.cos() / base.cos()).acos() / base.sin();
            t - ((base.sin() * t).tan() / base.sin()).atan()
        };
        let offset = std::f64::consts::PI / (2. * teeth as f64) + inv(s.pitch_cone_angle);
        let ht = offset - inv(tip);
        let hr = if root < base {
            offset
        } else {
            offset - inv(root)
        };
        let n = owned.source().flanks()[0].patches.len();
        let mut cursor = 0;
        let check = |index: usize, b: [f64; 2], a: [f64; 2], meridian: bool| {
            let c = &owned.curves()[index];
            assert_eq!(c.degree, 2);
            for t in [0., 0.2, 0.5, 0.8, 1.] {
                let p = c.evaluate_de_boor(t).unwrap();
                let q = reference_arc(r, b, a, meridian, t);
                let error = (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]);
                assert!(error <= owned.certificate().transition_error_upper + 1e-10);
            }
        };
        for j in 0..teeth {
            let center = std::f64::consts::TAU * j as f64 / teeth as f64;
            if root < base {
                check(cursor, [root, base], [center - offset; 2], true);
                cursor += 1;
            }
            cursor += n;
            check(cursor, [tip; 2], [center - ht, center + ht], false);
            cursor += 1 + n;
            if root < base {
                check(cursor, [base, root], [center + offset; 2], true);
                cursor += 1;
            }
            let next = std::f64::consts::TAU * (j + 1) as f64 / teeth as f64;
            check(cursor, [root; 2], [center + hr, next - hr], false);
            cursor += 1;
        }
        assert_eq!(cursor, owned.curves().len());
    }
}
