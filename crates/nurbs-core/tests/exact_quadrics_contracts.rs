use nurbs_core::{
    primitives::exact_quadrics::{Refusal, ellipsoid},
    surface::Surface,
};
const UNIT: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn accepted(center: [f64; 3], axes: [[f64; 3]; 3]) -> Surface {
    let r = ellipsoid(center, axes, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    let s = r.surface.unwrap();
    s.validate().unwrap();
    println!(
        "exact-quadric-oracle={{\"center\":{:?},\"axes\":{:?},\"surface\":{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        center, axes, s.degree_u, s.degree_v, s.knots_u, s.knots_v, s.control_points, s.weights
    );
    s
}
#[test]
fn unit_sphere_has_exact_positive_weights_poles_and_seam() {
    let s = accepted([0.; 3], UNIT);
    assert_eq!((s.degree_u, s.degree_v), (2, 2));
    assert_eq!(s.control_points.len(), 9);
    assert!(s.control_points.iter().all(|r| r.len() == 5));
    assert!(!s.periodic_u && !s.periodic_v);
    assert_eq!(s.control_points[0], s.control_points[8]);
    assert_eq!(s.weights[0], s.weights[8]);
    for row in &s.control_points {
        assert_eq!(row[0], vec![0., 0., -1.]);
        assert_eq!(row[4], vec![0., 0., 1.]);
    }
    assert_eq!(s.weights[2], vec![2., 2., 4., 2., 2.]);
    for i in 0..=12 {
        for j in 0..=12 {
            let p = s.evaluate(i as f64 / 12., j as f64 / 12.).unwrap().point;
            assert!((p.iter().map(|x| x * x).sum::<f64>() - 1.).abs() < 1e-13);
        }
    }
}
#[test]
fn oblique_translated_ellipsoid_keeps_authored_cardinals() {
    let c = [2., -3., 5.];
    let axes = [[2., 1., 0.5], [1., 3., 2.], [0., 1., 4.]];
    let s = accepted(c, axes);
    for (uv, expected) in [
        ([0., 0.], [2., -4., 1.]),
        ([0., 1.], [2., -2., 9.]),
        ([0., 0.5], [4., -2., 5.5]),
        ([0.25, 0.5], [3., 0., 7.]),
        ([0.5, 0.5], [0., -4., 4.5]),
    ] {
        assert_eq!(s.evaluate(uv[0], uv[1]).unwrap().point, expected);
    }
}
#[test]
fn near_dependent_but_exactly_independent_axes_are_admitted() {
    let epsilon = 2f64.powi(-40);
    let s = accepted(
        [0.; 3],
        [[1., 0., 0.], [1., epsilon, 0.], [1., 1., epsilon]],
    );
    assert_eq!(s.control_points[1][1], vec![1., epsilon - 1., -epsilon]);
}
#[test]
fn orientation_reversing_frame_is_not_silently_normalized() {
    let s = accepted([-2., 1., 0.], [[0., 2., 0.], [3., 0., 0.], [0., 0., 4.]]);
    assert_eq!(s.evaluate(0., 0.5).unwrap().point, [-2., 3., 0.]);
    assert_eq!(s.evaluate(0.25, 0.5).unwrap().point, [1., 1., 0.]);
}
#[test]
fn subnormal_axes_are_proved_without_floating_determinant_underflow() {
    let t = f64::from_bits(1);
    let s = accepted([0.; 3], [[t, 0., 0.], [0., t, 0.], [0., 0., t]]);
    assert_eq!(s.control_points[0][2], vec![t, 0., 0.]);
}
#[test]
fn dependent_axes_and_nonrepresentable_poles_publish_no_surface() {
    for axes in [
        [[1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        [[1., 0., 0.], [0., 0., 0.], [0., 0., 1.]],
    ] {
        let r = ellipsoid([0.; 3], axes, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::DependentAxes));
        assert!(r.surface.is_none());
    }
    let r = ellipsoid(
        [0.3, 0., 0.],
        [[0.1, 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        1_000_000,
    )
    .unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.surface.is_none());
}
#[test]
fn a_shared_budget_covers_reference_construction_and_literal_tensor_proof() {
    let full = ellipsoid([0.; 3], UNIT, 1_000_000).unwrap();
    assert!(full.surface.is_some());
    for budget in [1, 100, full.work_used - 1] {
        let r = ellipsoid([0.; 3], UNIT, budget).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.surface.is_none());
        assert!(r.work_used <= budget);
    }
    assert_eq!(
        ellipsoid([0.; 3], UNIT, full.work_used).unwrap().surface,
        full.surface
    );
}
#[test]
fn all_input_leaves_and_budget_are_validated_before_a_small_budget_refusal() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(ellipsoid([0., 0., bad], UNIT, 1).is_err());
        let mut axes = UNIT;
        axes[2][2] = bad;
        assert!(ellipsoid([0.; 3], axes, 1).is_err());
    }
    for budget in [0, 1_000_001] {
        assert!(ellipsoid([0.; 3], UNIT, budget).is_err());
    }
}
