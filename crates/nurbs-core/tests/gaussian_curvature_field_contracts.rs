use nurbs_core::{
    editing::{bounded_surface_differential as bounded, exact_surface_derivative as exact},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn saddle() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn bounded_saddle_matches_the_rational_closed_form() {
    let s = saddle();
    // The original exact Gaussian curvature at (1,1) is -1/9 and
    // clamped binary64 poles cannot store that endpoint exactly.
    assert!(
        exact::gaussian_curvature(&s, WORK)
            .unwrap()
            .patches
            .is_none()
    );
    let report = bounded::gaussian_curvature(&s, 1e-10, WORK).unwrap();
    let patch = &report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal))[0];
    let upper = report.error_upper.unwrap()[0];
    assert!((0. ..=1e-10).contains(&upper));
    for (u, v) in [(0., 0.), (0.25_f64, 0.5_f64), (1., 1.)] {
        let expected = -1. / (1. + u * u + v * v).powi(2);
        let p = patch.evaluate(u, v).unwrap().point;
        assert!((p[0] - expected).abs() <= upper + 1e-14);
        assert_eq!(&p[1..], &[0., 0.]);
    }
}

#[test]
fn exact_zero_requires_a_regular_original_normal() {
    let mut s = saddle();
    s.control_points[1][1][2] = 0.;
    let report = exact::gaussian_curvature(&s, WORK).unwrap();
    let patch = &report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal))[0];
    assert!(
        patch
            .control_points
            .iter()
            .flatten()
            .flatten()
            .all(|x| *x == 0.)
    );
    s.control_points[1] = s.control_points[0].clone();
    let exact = exact::gaussian_curvature(&s, WORK).unwrap();
    assert_eq!(exact.refusal, Some(exact::Refusal::UnprovedDenominator));
    let bounded = bounded::gaussian_curvature(&s, 1., WORK).unwrap();
    assert_eq!(bounded.refusal, Some(bounded::Refusal::UnprovedDenominator));
    assert!(bounded.patches.is_none() && bounded.error_upper.is_none());
}

#[test]
fn reversing_uv_orientation_preserves_gaussian_curvature() {
    let mut s = saddle();
    s.control_points.reverse();
    let report = bounded::gaussian_curvature(&s, 1e-10, WORK).unwrap();
    let patch = &report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal))[0];
    let (u, v) = (0.25_f64, 0.5_f64);
    let expected = -1. / (1. + (1. - u).powi(2) + v * v).powi(2);
    let actual = patch.evaluate(u, v).unwrap().point[0];
    assert!((actual - expected).abs() <= report.error_upper.unwrap()[0] + 1e-14);
}

#[test]
fn invalid_limit_and_insufficient_work_do_not_return_fields() {
    let s = saddle();
    assert!(bounded::gaussian_curvature(&s, f64::NAN, WORK).is_err());
    assert!(exact::gaussian_curvature(&s, 0).is_err());
    let report = bounded::gaussian_curvature(&s, 1e-10, 1).unwrap();
    assert_eq!(report.refusal, Some(bounded::Refusal::ResourceLimit));
    assert!(report.patches.is_none() && report.error_upper.is_none());
}

#[test]
fn rational_parameter_change_does_not_change_the_curvature_field_in_space() {
    // Apply u=t/(1+t) before evaluating the same bilinear saddle.
    let mut s = saddle();
    s.weights[1] = vec![2.; 2];
    s.control_points[1][0][0] = 0.5;
    s.control_points[1][1][0] = 0.5;
    s.control_points[1][1][2] = 0.5;
    let report = bounded::gaussian_curvature(&s, 1e-10, WORK).unwrap();
    let patch = &report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal))[0];
    let (t, v) = (0.5_f64, 0.25_f64);
    let u = t / (1. + t);
    let expected = -1. / (1. + u * u + v * v).powi(2);
    let actual = patch.evaluate(t, v).unwrap().point[0];
    assert!((actual - expected).abs() <= report.error_upper.unwrap()[0] + 1e-14);
}

#[test]
fn an_unrepresentable_corner_prevents_a_sub_roundoff_bound() {
    let report = bounded::gaussian_curvature(&saddle(), 1e-30, WORK).unwrap();
    assert_eq!(report.refusal, Some(bounded::Refusal::ToleranceExceeded));
    assert!(report.patches.is_none() && report.error_upper.is_none());
}
