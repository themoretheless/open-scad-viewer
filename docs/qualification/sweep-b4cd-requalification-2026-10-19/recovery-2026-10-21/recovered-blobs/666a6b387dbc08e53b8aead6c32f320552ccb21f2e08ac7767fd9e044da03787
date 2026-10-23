use nurbs_core::{
    surface::Surface,
    surface_measure::{self as measure, StopReason},
};
fn contains(bounds: [f64; 2], value: f64) {
    assert!(
        bounds[0] <= value && value <= bounds[1],
        "{value} outside {bounds:?}"
    );
}
#[test]
fn oblique_plane_area_is_independent_of_domain_and_orientation() {
    let mut s = nurbs_core::patches::plane([3., -2., 5.], [3., 0., 1.], [0., 4., 2.]).unwrap();
    s.knots_u = vec![2., 2., 5., 5.];
    s.knots_v = vec![-4., -4., 2., 2.];
    for s in [
        s.clone(),
        nurbs_core::surface_edit::transpose(&s).unwrap(),
        nurbs_core::surface_edit::reverse(&s, nurbs_core::surface::Axis::U).unwrap(),
    ] {
        let r = measure::area(&s, 1e-9, 1).unwrap();
        contains(r.bounds, 14.);
        assert!(r.within_tolerance, "{r:?}");
        assert_eq!(r.cells, 1);
    }
}
#[test]
fn cylinder_area_matches_analytic_lateral_area() {
    let s = nurbs_core::primitives::cylinder([0.; 3], 2., 3.).unwrap();
    let r = measure::area(&s, 0.03, 32768).unwrap();
    contains(r.bounds, 12. * std::f64::consts::PI);
    assert!(r.within_tolerance, "{r:?}");
    assert!(r.cells <= 32768);
}
#[test]
fn parabolic_extrusion_area_matches_closed_form() {
    let s =
        nurbs_core::polynomial::graph([0., 1., 0., 1.], &[vec![0.], vec![0.], vec![1.]]).unwrap();
    let r = measure::area(&s, 1e-3, 16384).unwrap();
    contains(r.bounds, 5_f64.sqrt() / 2. + 2_f64.asinh() / 4.);
    assert!(r.within_tolerance, "{r:?}");
}
#[test]
fn rational_planar_chart_preserves_rectangle_area() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![3., 0., 0.], vec![3., 4., 0.]],
        ],
        weights: vec![vec![1., 3.], vec![2., 6.]],
        periodic_u: false,
        periodic_v: false,
    };
    let r = measure::area(&s, 0.5, 16384).unwrap();
    contains(r.bounds, 12.);
    assert!(r.within_tolerance, "{r:?}");
}
#[test]
fn folds_are_measured_with_multiplicity() {
    let s = nurbs_core::patches::bezier(
        vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![-1., 0., 0.], vec![-1., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        None,
    )
    .unwrap();
    let r = measure::area(&s, 0.01, 4096).unwrap();
    contains(r.bounds, 2.);
    assert!(r.within_tolerance, "{r:?}");
}
#[test]
fn singular_and_zero_area_patches_have_valid_bounds() {
    let cone = nurbs_core::primitives::cone([0.; 3], 2., 3.).unwrap();
    let r = measure::area(&cone, 1e-12, 4).unwrap();
    contains(r.bounds, 2. * std::f64::consts::PI * 13_f64.sqrt());
    assert!(!r.within_tolerance);
    let zero = nurbs_core::patches::bilinear([[[1., 2., 3.]; 2]; 2]).unwrap();
    let r = measure::area(&zero, 1e-9, 1).unwrap();
    contains(r.bounds, 0.);
    assert!(r.within_tolerance);
}
#[test]
fn budget_refusal_and_stop_do_not_claim_accuracy() {
    let s = nurbs_core::primitives::cylinder([0.; 3], 2., 3.).unwrap();
    assert!(measure::area(&s, 1e-12, 1).is_err());
    let r = measure::area(&s, 1e-12, 4).unwrap();
    contains(r.bounds, 12. * std::f64::consts::PI);
    assert!(!r.within_tolerance);
    assert_eq!(r.cells, 4);
    assert_eq!(r.stop_reason, StopReason::WorkLimit);
}
#[test]
fn invalid_area_requests_are_refused() {
    let s = nurbs_core::patches::plane([0.; 3], [1., 0., 0.], [0., 1., 0.]).unwrap();
    assert!(measure::area(&s, 0., 10).is_err());
    assert!(measure::area(&s, f64::NAN, 10).is_err());
    assert!(measure::area(&s, 1e-3, 0).is_err());
    assert!(measure::area(&s, 1e-3, 100001).is_err());
    let mut malformed = s;
    malformed.weights.clear();
    assert!(measure::area(&malformed, 1e-3, 10).is_err());
}
