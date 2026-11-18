use nurbs_core::{
    primitives::exact_frusta::{End, Refusal, side},
    surface::Surface,
};
const UNIT: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
fn accepted(center: [f64; 3], axes: [[f64; 3]; 3], radii: [f64; 2]) -> Surface {
    let r = side(center, axes, radii, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    assert_eq!(
        r.collapsed_end,
        if radii[0] == 0. {
            Some(End::Start)
        } else if radii[1] == 0. {
            Some(End::End)
        } else {
            None
        }
    );
    let s = r.surface.unwrap();
    s.validate().unwrap();
    println!(
        "exact-frustum-oracle={{\"center\":{:?},\"axes\":{:?},\"radii\":{:?},\"collapsedEnd\":{},\"surface\":{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        center,
        axes,
        radii,
        match r.collapsed_end {
            None => "null",
            Some(End::Start) => "0",
            Some(End::End) => "1",
        },
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
fn cylinder_has_exact_seam_and_eighteen_positive_weight_poles() {
    let s = accepted([0.; 3], UNIT, [1., 1.]);
    assert_eq!((s.degree_u, s.degree_v), (2, 1));
    assert_eq!(s.control_points.len(), 9);
    assert!(s.control_points.iter().all(|r| r.len() == 2));
    assert!(!s.periodic_u && !s.periodic_v);
    assert_eq!(s.control_points[0], s.control_points[8]);
    assert_eq!(s.weights[0], s.weights[8]);
    assert_eq!(s.weights[2], vec![2., 2.]);
    assert_eq!(s.evaluate(0., 0.).unwrap().point, [1., 0., 0.]);
    assert_eq!(s.evaluate(0., 1.).unwrap().point, [1., 0., 1.]);
}
#[test]
fn frustum_keeps_the_original_linear_height_and_radius_law() {
    let s = accepted([0.; 3], UNIT, [2., 1.]);
    assert_eq!(s.evaluate(0.25, 0.5).unwrap().point, [0., 1.5, 0.5]);
    for i in 0..=12 {
        for j in 0..=12 {
            let v = j as f64 / 12.;
            let p = s.evaluate(i as f64 / 12., v).unwrap().point;
            assert!((p[2] - v).abs() < 1e-14);
            assert!((p[0] * p[0] + p[1] * p[1] - (2. - v).powi(2)).abs() < 1e-13);
        }
    }
}
#[test]
fn both_cone_orientations_have_one_explicit_intrinsically_singular_apex() {
    for (radii, end, point) in [([2., 0.], 1, [0., 0., 1.]), ([0., 2.], 0, [0., 0., 0.])] {
        let s = accepted([0.; 3], UNIT, radii);
        assert!(
            s.control_points
                .iter()
                .all(|r| r[end].as_slice() == point.as_slice())
        );
        for u in [0., 0.125, 0.5, 1.] {
            assert_eq!(s.evaluate(u, end as f64).unwrap().point, point);
        }
    }
}
#[test]
fn translated_oblique_frame_keeps_both_authored_end_sections() {
    let s = accepted(
        [2., -3., 5.],
        [[2., 1., 0.5], [1., 3., 2.], [0., 1., 4.]],
        [2., 0.5],
    );
    assert_eq!(s.evaluate(0., 0.).unwrap().point, [6., -1., 6.]);
    assert_eq!(s.evaluate(0.25, 1.).unwrap().point, [2.5, -0.5, 10.]);
}
#[test]
fn reflected_frame_keeps_authored_phase() {
    let s = accepted(
        [-2., 1., 0.],
        [[0., 2., 0.], [3., 0., 0.], [0., 0., 4.]],
        [3., 1.],
    );
    assert_eq!(s.evaluate(0., 0.).unwrap().point, [-2., 7., 0.]);
    assert_eq!(s.evaluate(0.25, 1.).unwrap().point, [1., 1., 4.]);
}
#[test]
fn almost_dependent_frame_has_no_epsilon_rank_rejection() {
    let e = 2f64.powi(-40);
    let s = accepted([0.; 3], [[1., 0., 0.], [1., e, 0.], [1., 1., e]], [2., 1.]);
    assert_eq!(s.control_points[1][0], vec![4., 2. * e, 0.]);
    assert_eq!(s.control_points[1][1], vec![3., 1. + e, e]);
}
#[test]
fn subnormal_frame_is_not_replaced_by_a_false_zero_height() {
    let t = f64::from_bits(1);
    let s = accepted([0.; 3], [[t, 0., 0.], [0., t, 0.], [0., 0., t]], [2., 1.]);
    assert_eq!(s.control_points[0][0], vec![2. * t, 0., 0.]);
    assert_eq!(s.control_points[0][1], vec![t, 0., t]);
}
#[test]
fn one_ulp_radius_change_is_not_coalesced_to_a_cylinder() {
    let r = 1f64.next_up();
    let s = accepted([0.; 3], UNIT, [1., r]);
    assert_eq!(s.control_points[0][1][0].to_bits(), r.to_bits());
    assert!(s.control_points[0][1][0] > s.control_points[0][0][0]);
}
#[test]
fn dependent_frames_and_unrepresentable_or_overflowing_poles_are_atomic() {
    let r = side(
        [0.; 3],
        [[1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        [2., 1.],
        1_000_000,
    )
    .unwrap();
    assert_eq!(r.refusal, Some(Refusal::DependentAxes));
    assert!(r.surface.is_none());
    assert_eq!(r.collapsed_end, None);
    for (center, axes, radii) in [
        (
            [0.3, 0., 0.],
            [[0.1, 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            [1., 1.],
        ),
        (
            [0.; 3],
            [[2., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            [f64::MAX, 0.],
        ),
    ] {
        let r = side(center, axes, radii, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
        assert!(r.surface.is_none());
        assert_eq!(r.collapsed_end, None);
    }
}
#[test]
fn one_budget_covers_fresh_circle_and_the_complete_actual_output_proof() {
    let full = side([0.; 3], UNIT, [2., 0.], 1_000_000).unwrap();
    assert!(full.surface.is_some());
    for budget in [1, 100, full.work_used - 1] {
        let r = side([0.; 3], UNIT, [2., 0.], budget).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.surface.is_none());
        assert_eq!(r.collapsed_end, None);
        assert!(r.work_used <= budget);
    }
    let r = side([0.; 3], UNIT, [2., 0.], full.work_used).unwrap();
    assert_eq!(r.surface, full.surface);
    assert_eq!(r.collapsed_end, Some(End::End));
}
#[test]
fn invalid_zero_or_signed_radii_all_nonfinite_leaves_and_budgets_are_errors() {
    for radii in [[0., 0.], [-1., 1.], [1., -1.]] {
        assert!(side([0.; 3], UNIT, radii, 1).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(side([0., 0., bad], UNIT, [2., 1.], 1).is_err());
        let mut axes = UNIT;
        axes[2][2] = bad;
        assert!(side([0.; 3], axes, [2., 1.], 1).is_err());
        for radii in [[bad, 1.], [1., bad]] {
            assert!(side([0.; 3], UNIT, radii, 1).is_err());
        }
    }
    for budget in [0, 1_000_001] {
        assert!(side([0.; 3], UNIT, [2., 1.], budget).is_err());
    }
}
