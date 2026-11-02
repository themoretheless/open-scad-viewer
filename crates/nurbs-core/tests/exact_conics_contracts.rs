use nurbs_core::{
    curve::Curve,
    primitives::exact_conics::{Refusal, ellipse},
};
fn accepted(center: [f64; 3], u: [f64; 3], v: [f64; 3]) -> Curve {
    let r = ellipse(center, u, v, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    let c = r.curve.unwrap();
    c.validate().unwrap();
    println!(
        "exact-conic-oracle={{\"center\":{:?},\"axisU\":{:?},\"axisV\":{:?},\"curve\":{{\"degree\":{},\"knots\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        center, u, v, c.degree, c.knots, c.control_points, c.weights
    );
    c
}
#[test]
fn unit_circle_has_only_exact_binary_rational_weights() {
    let c = accepted([0.; 3], [1., 0., 0.], [0., 1., 0.]);
    assert_eq!(c.degree, 2);
    assert_eq!(c.control_points.len(), 9);
    assert_eq!(c.weights, vec![1., 1., 2., 1., 1., 1., 2., 1., 1.]);
    assert_eq!(c.domain(), [0., 1.]);
    assert!(!c.periodic);
    for (t, point) in [
        (0., [1., 0., 0.]),
        (0.25, [0., 1., 0.]),
        (0.5, [-1., 0., 0.]),
        (0.75, [0., -1., 0.]),
        (1., [1., 0., 0.]),
    ] {
        assert_eq!(c.evaluate(t).unwrap().point, point);
    }
    for i in 0..=64 {
        let p = c.evaluate(i as f64 / 64.).unwrap().point;
        assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-14);
    }
}
#[test]
fn translated_oblique_axes_keep_all_three_source_coordinate_laws() {
    let center = [2., -3., 5.];
    let u = [2., 1., 0.5];
    let v = [1., 3., 2.];
    let c = accepted(center, u, v);
    assert_eq!(c.evaluate(0.).unwrap().point, [4., -2., 5.5]);
    assert_eq!(c.evaluate(0.25).unwrap().point, [3., 0., 7.]);
    for i in 0..=16 {
        let t = i as f64 / 16.;
        let d = 1. + t * t;
        let x = (1. - t * t) / d;
        let y = 2. * t / d;
        let p = c.evaluate(t / 4.).unwrap().point;
        for a in 0..3 {
            assert!((p[a] - (center[a] + u[a] * x + v[a] * y)).abs() < 1e-13);
        }
    }
}
#[test]
fn independent_nearly_parallel_axes_are_not_rejected_by_epsilon() {
    let c = accepted([0.; 3], [1., 0., 0.], [1., 2f64.powi(-40), 0.]);
    assert_eq!(c.control_points[1], vec![2., 2f64.powi(-40), 0.]);
}
#[test]
fn winding_follows_the_authored_axes_instead_of_normalization() {
    let c = accepted([-2., 1., 0.], [0., 2., 0.], [3., 0., 0.]);
    assert_eq!(c.evaluate(0.).unwrap().point, [-2., 3., 0.]);
    assert_eq!(c.evaluate(0.25).unwrap().point, [1., 1., 0.]);
}
#[test]
fn nonrepresentable_cartesian_storage_returns_no_partial_curve() {
    let r = ellipse([0.3, 0., 0.], [0.1, 0., 0.], [0., 1., 0.], 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.curve.is_none());
}
#[test]
fn dependent_axes_return_no_conic() {
    for (u, v) in [([1., 2., 3.], [2., 4., 6.]), ([0.; 3], [0., 1., 0.])] {
        let r = ellipse([0.; 3], u, v, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::DependentAxes));
        assert!(r.curve.is_none());
    }
}
#[test]
fn one_ledger_covers_storage_and_the_complete_polynomial_proof() {
    let full = ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], 1_000_000).unwrap();
    assert!(full.curve.is_some());
    for budget in [1, 100, full.work_used - 1] {
        let r = ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], budget).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.curve.is_none());
        assert!(r.work_used <= budget);
    }
    assert_eq!(
        ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], full.work_used)
            .unwrap()
            .curve,
        full.curve
    );
}
#[test]
fn full_input_validation_and_existing_bit_guard_are_retained() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(ellipse([0., 0., bad], [1., 0., 0.], [0., 1., 0.], 1).is_err());
        assert!(ellipse([0.; 3], [1., 0., 0.], [0., 1., bad], 1).is_err());
    }
    for budget in [0, 1_000_001] {
        assert!(ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], budget).is_err());
    }
    let tiny = f64::from_bits(1);
    let r = ellipse([0.; 3], [tiny, 0., 0.], [0., tiny, 0.], 1_000_000).unwrap();
    assert!(r.curve.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.work_used < 1_000_000);
}
