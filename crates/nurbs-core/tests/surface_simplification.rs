use nurbs_core::{
    foundation::approximate_edits::{reduce_surface_degree_report, remove_surface_knot_report},
    surface::{Axis, Surface},
    surface_edit,
};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn verify(s: &Surface, error: f64) {
    for i in 0..=20 {
        for j in 0..=20 {
            let u = i as f64 / 20.;
            let v = j as f64 / 20.;
            let p = s.evaluate(2. + 5. * u, -3. + 4. * v).unwrap().point;
            let e = [12. * u / (1. + 2. * u), 8. * v / (1. + v), 0.];
            let d = p
                .iter()
                .zip(e)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(d <= error + 1e-12);
        }
    }
}
#[test]
fn remove_knots_in_both_axes_with_rational_geometry() {
    for (axis, knot) in [(Axis::U, 4.), (Axis::V, -1.)] {
        let s = surface_edit::insert(&panel(), axis, knot, 1).unwrap();
        let r = remove_surface_knot_report(&s, axis, knot, 1e-3, None).unwrap();
        assert!(r.certificate.accepted);
        assert!(r.certificate.error_upper <= 1e-3);
        verify(&r.surface, r.certificate.error_upper);
        assert_eq!(r.surface.control_points.len(), 2);
        assert_eq!(r.surface.control_points[0].len(), 2);
    }
}
#[test]
fn reduce_degree_in_both_axes_with_rational_geometry() {
    for axis in [Axis::U, Axis::V] {
        let s = surface_edit::elevate(&panel(), axis, 2).unwrap();
        let r = reduce_surface_degree_report(&s, axis, 1, 1e-3, None).unwrap();
        assert!(r.certificate.accepted);
        assert_eq!((r.surface.degree_u, r.surface.degree_v), (1, 1));
        verify(&r.surface, r.certificate.error_upper);
    }
}
#[test]
fn curved_surface_rolls_back_when_budget_cannot_be_proven() {
    let s = nurbs_core::polynomial::graph(
        [0., 1., 0., 1.],
        &[vec![0., 0., 1.], vec![0., 0., 0.], vec![1., 0., 0.]],
    )
    .unwrap();
    for axis in [Axis::U, Axis::V] {
        let r = reduce_surface_degree_report(&s, axis, 1, 1e-6, None).unwrap();
        assert!(!r.certificate.accepted);
        assert_eq!(r.surface, s);
    }
}
#[test]
fn invalid_knots_degrees_and_budgets_are_rejected() {
    let s = panel();
    for knot in [2., 7., 4., f64::NAN] {
        assert!(remove_surface_knot_report(&s, Axis::U, knot, 1., None).is_err());
    }
    for degree in [0, 1, 2] {
        assert!(reduce_surface_degree_report(&s, Axis::U, degree, 1., None).is_err());
    }
    for error in [-1., f64::NAN, f64::INFINITY] {
        assert!(remove_surface_knot_report(&s, Axis::U, 4., error, None).is_err());
        assert!(reduce_surface_degree_report(&s, Axis::U, 1, error, None).is_err());
    }
}
