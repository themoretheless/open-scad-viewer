use nurbs_core::{
    primitives::{
        exact_conics, exact_frusta,
        exact_planar_regions::{Refusal, RegionKind, radial},
    },
    surface::Surface,
};
const XY: [[f64; 3]; 2] = [[1., 0., 0.], [0., 1., 0.]];
fn accepted(center: [f64; 3], axes: [[f64; 3]; 2], radii: [f64; 2]) -> Surface {
    let r = radial(center, axes, radii, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    assert_eq!(
        r.kind,
        Some(if radii[0] == 0. {
            RegionKind::FilledDisk
        } else {
            RegionKind::Annulus
        })
    );
    let s = r.surface.unwrap();
    s.validate().unwrap();
    println!(
        "exact-planar-region-oracle={{\"center\":{:?},\"axes\":{:?},\"radii\":{:?},\"kind\":\"{}\",\"surface\":{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        center,
        axes,
        radii,
        if radii[0] == 0. {
            "filledDisk"
        } else {
            "annulus"
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
fn disk_has_full_radial_clock_exact_seam_and_smooth_plane_center_image() {
    let s = accepted([0.; 3], XY, [0., 1.]);
    assert_eq!((s.degree_u, s.degree_v), (2, 1));
    assert_eq!(s.control_points.len(), 9);
    assert!(!s.periodic_u && !s.periodic_v);
    assert_eq!(s.control_points[0], s.control_points[8]);
    assert_eq!(s.weights[0], s.weights[8]);
    assert!(s.control_points.iter().all(|r| r[0] == vec![0.; 3]));
    assert_eq!(s.weights[2], vec![2., 2.]);
    for i in 0..=12 {
        for j in 0..=12 {
            let v = j as f64 / 12.;
            let p = s.evaluate(i as f64 / 12., v).unwrap().point;
            assert_eq!(p[2], 0.);
            assert!((p[0] * p[0] + p[1] * p[1] - v * v).abs() < 1e-13);
        }
    }
    let boundary = exact_conics::ellipse([0.; 3], XY[0], XY[1], 1_000_000)
        .unwrap()
        .curve
        .unwrap();
    for i in 0..9 {
        assert_eq!(s.control_points[i][1], boundary.control_points[i]);
        assert_eq!(s.weights[i][1], boundary.weights[i]);
    }
    assert_eq!(s.knots_u, boundary.knots);
}
#[test]
fn annulus_has_a_real_inner_boundary_and_no_center_collapse() {
    let s = accepted([0.; 3], XY, [1., 2.]);
    assert_eq!(s.evaluate(0., 0.).unwrap().point, [1., 0., 0.]);
    assert_eq!(s.evaluate(0.25, 0.5).unwrap().point, [0., 1.5, 0.]);
    assert_ne!(s.control_points[0][0], s.control_points[2][0]);
}
#[test]
fn oblique_plane_retains_all_three_authored_coordinate_laws() {
    let s = accepted([2., -3., 5.], [[2., 1., 0.5], [1., 3., 2.]], [0., 2.]);
    assert_eq!(s.evaluate(0., 1.).unwrap().point, [6., -1., 6.]);
    assert_eq!(s.evaluate(0.25, 1.).unwrap().point, [4., 3., 9.]);
    assert!(s.control_points.iter().all(|r| r[0] == vec![2., -3., 5.]));
}
#[test]
fn reflected_axes_keep_the_authored_angular_phase() {
    let s = accepted([-2., 1., 0.], [[0., 2., 0.], [3., 0., 0.]], [0.5, 1.]);
    assert_eq!(s.evaluate(0., 1.).unwrap().point, [-2., 3., 0.]);
    assert_eq!(s.evaluate(0.25, 1.).unwrap().point, [1., 1., 0.]);
}
#[test]
fn almost_parallel_independent_axes_have_no_epsilon_rejection() {
    let e = 2f64.powi(-80);
    let s = accepted([0.; 3], [[1., 0., 0.], [1., e, 0.]], [0., 2.]);
    assert_eq!(s.control_points[1][1], vec![4., 2. * e, 0.]);
}
#[test]
fn subnormal_frame_uses_a_minor_without_squaring_away_the_bit_budget() {
    let t = f64::from_bits(1);
    let s = accepted([0.; 3], [[t, 0., 0.], [0., t, 0.]], [0., 2.]);
    assert_eq!(s.control_points[0][1], vec![2. * t, 0., 0.]);
    let rotated = accepted([0.; 3], [[t, t, 0.], [t, 0., t]], [0., 2.]);
    assert_eq!(rotated.control_points[1][1], vec![4. * t, 2. * t, 2. * t]);
}
#[test]
fn one_ulp_annular_width_is_not_coalesced_to_a_curve() {
    let outer = 1f64.next_up();
    let s = accepted([0.; 3], XY, [1., outer]);
    assert_eq!(s.control_points[0][1][0].to_bits(), outer.to_bits());
    assert!(s.control_points[0][1][0] > s.control_points[0][0][0]);
}
#[test]
fn two_disk_boundaries_match_the_exact_frustum_end_sections_literally() {
    let axes = [[2., 0., 0.], [0., 3., 0.], [0., 0., 4.]];
    let frustum = exact_frusta::side([0.; 3], axes, [2., 1.], 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    for (end, center, radius) in [(0, [0.; 3], 2.), (1, [0., 0., 4.], 1.)] {
        let cap = accepted(center, [axes[0], axes[1]], [0., radius]);
        assert_eq!(cap.knots_u, frustum.knots_u);
        for i in 0..9 {
            assert_eq!(cap.control_points[i][1], frustum.control_points[i][end]);
            assert_eq!(cap.weights[i][1], frustum.weights[i][end]);
        }
    }
}
#[test]
fn dependent_axes_unrepresentable_poles_and_overflow_are_atomic() {
    let r = radial([0.; 3], [[1., 2., 3.], [2., 4., 6.]], [0., 2.], 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::DependentAxes));
    assert!(r.surface.is_none());
    assert_eq!(r.kind, None);
    for (c, axes, radii) in [
        ([0.3, 0., 0.], [[0.1, 0., 0.], [0., 1., 0.]], [0., 1.]),
        ([0.; 3], [[2., 0., 0.], [0., 1., 0.]], [0., f64::MAX]),
    ] {
        let r = radial(c, axes, radii, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
        assert!(r.surface.is_none());
        assert_eq!(r.kind, None);
    }
}
#[test]
fn one_ledger_covers_reference_and_complete_literal_output_proof() {
    let full = radial([0.; 3], XY, [0., 1.], 1_000_000).unwrap();
    assert!(full.surface.is_some());
    for budget in [1, 100, full.work_used - 1] {
        let r = radial([0.; 3], XY, [0., 1.], budget).unwrap();
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.surface.is_none());
        assert_eq!(r.kind, None);
        assert!(r.work_used <= budget);
    }
    let r = radial([0.; 3], XY, [0., 1.], full.work_used).unwrap();
    assert_eq!(r.surface, full.surface);
    assert_eq!(r.kind, full.kind);
}
#[test]
fn invalid_radii_nonfinite_leaves_and_budgets_are_rejected_before_work() {
    for radii in [[0., 0.], [1., 1.], [2., 1.], [-1., 1.]] {
        assert!(radial([0.; 3], XY, radii, 1).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(radial([0., 0., bad], XY, [0., 1.], 1).is_err());
        let mut axes = XY;
        axes[1][2] = bad;
        assert!(radial([0.; 3], axes, [0., 1.], 1).is_err());
        for radii in [[bad, 1.], [0., bad]] {
            assert!(radial([0.; 3], XY, radii, 1).is_err());
        }
    }
    for budget in [0, 1_000_001] {
        assert!(radial([0.; 3], XY, [0., 1.], budget).is_err());
    }
}
