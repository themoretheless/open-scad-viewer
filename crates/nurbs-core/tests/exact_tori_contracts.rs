use nurbs_core::{
    primitives::exact_tori::{Refusal, ring},
    surface::Surface,
};
const UNIT: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn accepted(center: [f64; 3], axes: [[f64; 3]; 3], major: f64, minor: f64) -> Surface {
    let r = ring(center, axes, major, minor, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    let s = r.surface.unwrap();
    s.validate().unwrap();
    println!(
        "exact-torus-oracle={{\"center\":{:?},\"axes\":{:?},\"major\":{:?},\"minor\":{:?},\"surface\":{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        center,
        axes,
        major,
        minor,
        s.degree_u,
        s.degree_v,
        s.knots_u,
        s.knots_v,
        s.control_points,
        s.weights
    );
    s
}
#[test]
fn ring_has_exact_positive_tensor_weights_and_both_seams() {
    let s = accepted([0.; 3], UNIT, 2., 1.);
    assert_eq!((s.degree_u, s.degree_v), (2, 2));
    assert_eq!(s.control_points.len(), 9);
    assert!(!s.periodic_u && !s.periodic_v);
    assert_eq!(s.control_points[0], s.control_points[8]);
    assert_eq!(s.weights[0], s.weights[8]);
    for (p, w) in s.control_points.iter().zip(&s.weights) {
        assert_eq!(p[0], p[8]);
        assert_eq!(w[0], w[8]);
    }
    assert_eq!(s.weights[2], vec![2., 2., 4., 2., 2., 2., 4., 2., 2.]);
    for i in 0..=12 {
        for j in 0..=12 {
            let p = s.evaluate(i as f64 / 12., j as f64 / 12.).unwrap().point;
            let radial = p[0].hypot(p[1]);
            assert!(((radial - 2.).powi(2) + p[2] * p[2] - 1.).abs() < 1e-13);
        }
    }
}
#[test]
fn translated_oblique_frame_retains_authored_cardinals() {
    let s = accepted(
        [2., -3., 5.],
        [[2., 1., 0.5], [1., 3., 2.], [0., 1., 4.]],
        2.,
        0.5,
    );
    for (uv, p) in [
        ([0., 0.], [7., -0.5, 6.25]),
        ([0., 0.25], [6., -0.5, 8.]),
        ([0., 0.5], [5., -1.5, 5.75]),
    ] {
        assert_eq!(s.evaluate(uv[0], uv[1]).unwrap().point, p);
    }
}
#[test]
fn reflected_frame_preserves_phase_and_axis_orientation() {
    let s = accepted(
        [-2., 1., 0.],
        [[0., 2., 0.], [3., 0., 0.], [0., 0., 4.]],
        3.,
        1.,
    );
    assert_eq!(s.evaluate(0., 0.).unwrap().point, [-2., 9., 0.]);
    assert_eq!(s.evaluate(0.25, 0.).unwrap().point, [10., 1., 0.]);
    assert_eq!(s.evaluate(0., 0.25).unwrap().point, [-2., 7., 4.]);
}
#[test]
fn small_positive_hole_is_not_rejected_by_a_geometric_epsilon() {
    let gap = 2f64.powi(-40);
    let s = accepted([0.; 3], UNIT, 1. + gap, 1.);
    assert_eq!(s.evaluate(0., 0.5).unwrap().point, [gap, 0., 0.]);
}
#[test]
fn nearly_dependent_but_invertible_frame_uses_exact_rank() {
    let e = 2f64.powi(-30);
    let s = accepted([0.; 3], [[1., 0., 0.], [1., e, 0.], [1., 1., e]], 2., 1.);
    assert_eq!(s.control_points[1][1], vec![7., 1. + 3. * e, e]);
}
#[test]
fn subnormal_frame_has_no_false_floating_determinant_zero() {
    let t = f64::from_bits(1);
    let s = accepted([0.; 3], [[t, 0., 0.], [0., t, 0.], [0., 0., t]], 2., 1.);
    assert_eq!(s.control_points[0][0], vec![3. * t, 0., 0.]);
}
#[test]
fn dependent_frames_and_unrepresentable_or_overflowed_storage_are_atomic() {
    let r = ring(
        [0.; 3],
        [[1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        2.,
        1.,
        1_000_000,
    )
    .unwrap();
    assert_eq!(r.refusal, Some(Refusal::DependentAxes));
    assert!(r.surface.is_none());
    for (c, axes, major, minor) in [
        (
            [0.3, 0., 0.],
            [[0.1, 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            2.,
            1.,
        ),
        ([0.; 3], UNIT, f64::MAX, f64::MAX / 2.),
        ([0.; 3], UNIT, 1f64.next_up(), 1.),
    ] {
        let r = ring(c, axes, major, minor, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
        assert!(r.surface.is_none());
    }
}
#[test]
fn one_ledger_covers_reference_construction_and_complete_tensor_proof() {
    let full = ring([0.; 3], UNIT, 2., 1., 1_000_000).unwrap();
    assert!(full.surface.is_some());
    for budget in [1, 100, full.work_used - 1] {
        let r = ring([0.; 3], UNIT, 2., 1., budget).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.surface.is_none());
        assert!(r.work_used <= budget);
    }
    assert_eq!(
        ring([0.; 3], UNIT, 2., 1., full.work_used).unwrap().surface,
        full.surface
    );
}
#[test]
fn input_validation_rejects_horn_spindle_and_all_nonfinite_tails() {
    for (major, minor) in [(1., 1.), (0.5, 1.), (1., 0.), (-1., -2.)] {
        assert!(ring([0.; 3], UNIT, major, minor, 1).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(ring([0., 0., bad], UNIT, 2., 1., 1).is_err());
        let mut axes = UNIT;
        axes[2][2] = bad;
        assert!(ring([0.; 3], axes, 2., 1., 1).is_err());
        assert!(ring([0.; 3], UNIT, bad, 1., 1).is_err());
        assert!(ring([0.; 3], UNIT, 2., bad, 1).is_err());
    }
    for budget in [0, 1_000_001] {
        assert!(ring([0.; 3], UNIT, 2., 1., budget).is_err());
    }
}
