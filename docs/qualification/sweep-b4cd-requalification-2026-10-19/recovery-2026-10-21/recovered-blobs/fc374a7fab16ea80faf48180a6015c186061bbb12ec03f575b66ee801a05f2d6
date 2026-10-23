use nurbs_core::{
    curve::Curve,
    helical_sweep::{approximate, certify_stored},
};
fn profile() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![2., 0., -0.5], vec![2., 2., 0.75], vec![0., 2., 1.]],
        weights: vec![1., 0.7, 1.],
        periodic: false,
    }
}
#[test]
fn saved_rational_sweeps_include_poles_knots_and_parameter_uncertainty() {
    let c = profile();
    for (height, start, twist) in [
        (3., 0.2, 0.8),
        (-2., -0.5, -1.2),
        (2., 0.7, 0.),
        (0., 0., 2.7),
    ] {
        let a = approximate(&c, height, start, twist, 1e-4).unwrap();
        let d = 1e-10;
        let h = [height - d, height + d];
        let st = [start - d, start + d];
        let tw = [twist - d, twist + d];
        let r = certify_stored(&c, &a.surface, h, st, tw).unwrap();
        assert!(r.rounding_included && r.error_upper < 2e-4);
        for hh in h {
            for ss in st {
                for tt in tw {
                    for i in 0..=12 {
                        for j in 0..=12 {
                            let u = 2. + 3. * i as f64 / 12.;
                            let v = j as f64 / 12.;
                            let p = c.evaluate(u).unwrap().point;
                            let (sin, cos) = (ss + tt * v).sin_cos();
                            let q = a.surface.evaluate(u, v).unwrap().point;
                            let ideal = [
                                cos * p[0] - sin * p[1],
                                sin * p[0] + cos * p[1],
                                p[2] + hh * v,
                            ];
                            let error = (0..3)
                                .map(|k| (q[k] - ideal[k]).powi(2))
                                .sum::<f64>()
                                .sqrt();
                            assert!(
                                error <= r.error_upper + 1e-12,
                                "{error} > {}",
                                r.error_upper
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn modified_stored_poles_are_accounted_for_and_basis_changes_refused() {
    let c = profile();
    let mut a = approximate(&c, 3., 0.2, 0.8, 1e-4).unwrap().surface;
    let before = certify_stored(&c, &a, [3.; 2], [0.2; 2], [0.8; 2]).unwrap();
    a.control_points[1][2][0] += 0.01;
    let after = certify_stored(&c, &a, [3.; 2], [0.2; 2], [0.8; 2]).unwrap();
    assert!(after.control_difference_upper > 0.009 && after.error_upper > before.error_upper);
    let mut changed_knots = approximate(&c, 3., 0.2, 2., 1e-4).unwrap().surface;
    for t in &mut changed_knots.knots_v {
        if *t == 0.5 {
            *t = 0.375;
        }
    }
    let r = certify_stored(&c, &changed_knots, [3.; 2], [0.2; 2], [2.; 2]).unwrap();
    assert!(r.control_difference_upper > 0.1);
    for i in 0..=20 {
        let v = i as f64 / 20.;
        let p = c.evaluate(3.).unwrap().point;
        let (sin, cos) = (0.2 + 2. * v).sin_cos();
        let q = changed_knots.evaluate(3., v).unwrap().point;
        let error = (q[0] - (cos * p[0] - sin * p[1]))
            .hypot(q[1] - (sin * p[0] + cos * p[1]))
            .hypot(q[2] - p[2] - 3. * v);
        assert!(error <= r.error_upper + 1e-12);
    }
    a.weights[1][2] += 0.01;
    assert!(certify_stored(&c, &a, [3.; 2], [0.2; 2], [0.8; 2]).is_err());
    let a = approximate(&c, 3., 0.2, 0.8, 1e-4).unwrap().surface;
    assert!(certify_stored(&c, &a, [f64::NAN; 2], [0.; 2], [0.; 2]).is_err());
    assert!(certify_stored(&c, &a, [3.; 2], [20.; 2], [0.; 2]).is_err());
}
#[test]
fn nominal_gear_twist_boxes_cover_signed_pressure_and_zero_limit() {
    use nurbs_core::involute::derive_gear_twist_bounds as derive;
    for angle in [-79., -20., 0., 20., 79.] {
        let bound = derive(2., 20, 6., angle).unwrap();
        let ideal = 6. * angle.to_radians().tan() / 20.;
        assert!(bound[0] <= ideal && ideal <= bound[1]);
    }
    assert_eq!(derive(2., 20, 6., 0.).unwrap(), [0.; 2]);
    for angle in [80., f64::NAN, f64::INFINITY] {
        assert!(derive(2., 20, 6., angle).is_err());
    }
}
