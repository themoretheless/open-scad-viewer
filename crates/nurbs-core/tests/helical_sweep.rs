use nurbs_core::{curve::Curve, helical_sweep};
fn rational_profile() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![2., 0., 0.], vec![2., 2., 1.], vec![0., 2., 0.]],
        weights: vec![1., 0.5_f64.sqrt(), 1.],
        periodic: false,
    }
}
#[test]
fn rational_helical_sweep_has_continuous_ideal_bound_and_original_u_basis() {
    let c = rational_profile();
    let before = c.clone();
    for twist in [0.7, -1.3, 2.] {
        let a = helical_sweep::approximate(&c, -4., 0.3, twist, 1e-5).unwrap();
        assert_eq!(a.surface.degree_u, 2);
        assert_eq!(a.surface.degree_v, 5);
        assert_eq!(a.surface.knots_u, c.knots);
        assert!(!a.rounding_certified);
        assert!(a.real_arithmetic_error_estimate > 0. && a.real_arithmetic_error_estimate <= 1e-5);
        for i in 0..=40 {
            let t = i as f64 / 40.;
            let b = [(1. - t).powi(2), 2. * t * (1. - t) * 0.5_f64.sqrt(), t * t];
            let w: f64 = b.iter().sum();
            // Independent rational quadratic definition, not surface sampling
            // as the proof: the constructor's analytical remainder is separate.
            let p = [2. * (b[0] + b[1]) / w, 2. * (b[1] + b[2]) / w, b[1] / w];
            for j in 0..=40 {
                let v = j as f64 / 40.;
                let angle = 0.3 + twist * v;
                let expected = [
                    angle.cos() * p[0] - angle.sin() * p[1],
                    angle.sin() * p[0] + angle.cos() * p[1],
                    p[2] - 4. * v,
                ];
                let actual = a.surface.evaluate(2. + 5. * t, v).unwrap().point;
                let error = (0..3)
                    .map(|k| (actual[k] - expected[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(
                    error <= a.real_arithmetic_error_estimate + 1e-11,
                    "{error} > {}",
                    a.real_arithmetic_error_estimate
                );
            }
        }
    }
    assert_eq!(c, before);
}
#[test]
fn zero_twist_is_ruled_and_resource_refusals_do_not_relax_budget() {
    let c = rational_profile();
    let a = helical_sweep::approximate(&c, 3., 0.2, 0., 1e-12).unwrap();
    assert_eq!(a.real_arithmetic_error_estimate, 0.);
    assert_eq!(a.surface.degree_v, 1);
    assert!(helical_sweep::approximate(&c, 3., 0., 100., 1e-6).is_err());
    assert!(helical_sweep::approximate(&c, 3., 0., 1., 0.).is_err());
    assert!(helical_sweep::approximate(&c, f64::NAN, 0., 1., 1e-6).is_err());
    let mut planar = c;
    for p in &mut planar.control_points {
        p.pop();
    }
    assert!(helical_sweep::approximate(&planar, 3., 0., 1., 1e-6).is_err());
}

#[test]
fn periodic_profile_preserves_authored_storage_and_seam_for_every_height() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![1., 0., 2.],
            vec![1., 1., 3.],
            vec![0., 1., 4.],
            vec![0., 0., 1.],
            vec![1., 0., 2.],
        ],
        weights: vec![1., 2., 3., 4., 1., 2.],
        periodic: true,
    };
    for twist in [0., 1., -1.] {
        let a = helical_sweep::approximate(&c, 3., 0.2, twist, 1e-5).unwrap();
        assert!(a.surface.periodic_u);
        assert_eq!(a.surface.knots_u, c.knots);
        for i in 0..c.degree {
            assert_eq!(a.surface.control_points[i], a.surface.control_points[4 + i]);
            assert_eq!(a.surface.weights[i], a.surface.weights[4 + i]);
        }
        // Inspect storage endpoints without periodic evaluation wrapping.
        let mut storage = a.surface.clone();
        storage.periodic_u = false;
        for v in [0., 0.17, 0.5, 0.93, 1.] {
            let lo = storage.evaluate(0., v).unwrap().point;
            let hi = storage.evaluate(4., v).unwrap().point;
            for j in 0..3 {
                assert!((lo[j] - hi[j]).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn axial_profile_does_not_depend_on_unrepresentable_angle_sum() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0., 0., 2.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let a = helical_sweep::approximate(&c, 3., 1e308, 1e308, 1e-12).unwrap();
    assert_eq!(a.real_arithmetic_error_estimate, 0.);
    let p = a.surface.evaluate(0.5, 0.5).unwrap().point;
    assert_eq!(p[0], 0.);
    assert_eq!(p[1], 0.);
    assert!((p[2] - (4. / 3. + 1.5)).abs() < 1e-12);
}
