use nurbs_core::{primitives, radial_bounds::curve_radius_error as bound};
#[test]
fn original_rational_arcs_and_nominal_radius_families_have_continuous_bounds() {
    for scale in [1e-100, 1., 10000.] {
        let mut c = primitives::circle_quadrants(1.).unwrap()[0].clone();
        for p in &mut c.control_points {
            for x in p {
                *x *= scale;
            }
        }
        let box_ = [scale * (1. - 1e-8), scale * (1. + 1e-8)];
        let r = bound(&c, [0.; 3], box_).unwrap().unwrap();
        assert!(r.rounding_included && r.error_upper < scale * 1.1e-8);
        for i in 0..=100 {
            let p = c.evaluate(i as f64 / 100.).unwrap().point;
            let actual = p[0].hypot(p[1]);
            for radius in box_ {
                assert!((actual - radius).abs() <= r.error_upper + scale * 1e-14);
            }
        }
    }
}
#[test]
fn canonical_endpoint_changes_and_nonplanar_curves_are_measured() {
    let mut c = primitives::circle_quadrants(2.).unwrap()[0].clone();
    c.control_points[0][0] += 1e-5;
    let r = bound(&c, [0.; 3], [2.; 2]).unwrap().unwrap();
    assert!(r.error_upper >= 1e-5 && r.error_upper < 1e-4);
    for i in 0..=100 {
        let p = c.evaluate(i as f64 / 100.).unwrap().point;
        assert!((p[0].hypot(p[1]) - 2.).abs() <= r.error_upper + 1e-13);
    }
    c.control_points[1][2] = 1.;
    let r = bound(&c, [0.; 3], [2.; 2]).unwrap().unwrap();
    assert!(r.error_upper > 0.01);
}
#[test]
fn unsupported_and_invalid_radius_contracts_never_claim_a_certificate() {
    let c = primitives::circle([0.; 3], [0., 0., 1.], 2.).unwrap();
    assert!(bound(&c, [0.; 3], [2.; 2]).unwrap().is_none());
    let c = primitives::circle_quadrants(2.).unwrap()[0].clone();
    for r in [[0., 1.], [2., 1.], [f64::NAN, 2.], [1., f64::INFINITY]] {
        assert!(bound(&c, [0.; 3], r).is_err());
    }
    assert!(bound(&c, [f64::NAN, 0., 0.], [2.; 2]).is_err());
}
