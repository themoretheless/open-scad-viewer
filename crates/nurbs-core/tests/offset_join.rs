use nurbs_core::{
    curve::Curve,
    curve_offset_join::{bevel, miter, miter_at_knot},
};
#[test]
fn represented_line_intersection_is_independent_of_tangent_scale() {
    for (u, v) in [([2., -1.], [-1., 2.]), ([20., -10.], [-0.5, 1.])] {
        let j = miter([-2., 3.], u, [4., -3.], v, 5., 1e-9).unwrap();
        assert_eq!(j.points[0], [-2., 3.]);
        assert_eq!(j.points[2], [4., -3.]);
        let p = j.points[1];
        assert!((p[0] - 2.).hypot(p[1] - 1.) <= j.error_upper_mm);
        assert!(j.error_upper_mm <= 1e-9);
    }
    let b = bevel([-2., 3.], [4., -3.]).unwrap();
    assert_eq!(b.points, vec![[-2., 3.], [4., -3.]]);
    assert_eq!(b.error_upper_mm, 0.);
}
#[test]
fn weighted_corner_on_original_nonunit_domain_has_analytic_join() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 7., 7.],
        control_points: vec![vec![0., 0.], vec![10., 0.], vec![10., 10.]],
        weights: vec![0.7, 1.3, 0.9],
        periodic: false,
    };
    for d in [-0.25, 0.25] {
        let j = miter_at_knot(&c, 4., d, 1., 1e-8).unwrap();
        for (p, e) in j.points.iter().zip([[10., d], [10. - d, d], [10. - d, 0.]]) {
            assert!((p[0] - e[0]).hypot(p[1] - e[1]) <= j.error_upper_mm);
        }
        assert!(j.error_upper_mm <= 1e-8);
    }
    assert_eq!(c.weights, vec![0.7, 1.3, 0.9]);
    assert!(miter_at_knot(&c, 3., 0.25, 1., 1e-8).is_err());
}
#[test]
fn parallel_excessive_and_invalid_requests_fail() {
    assert!(miter([-2., 3.], [2., -1.], [4., -3.], [4., -2.], 5., 1e-9).is_err());
    assert!(miter([-2., 3.], [2., -1.], [4., -3.], [-1., 2.], 1., 1e-9).is_err());
    assert!(bevel([f64::NAN, 0.], [0., 0.]).is_err());
    for t in [0., -1., f64::NAN] {
        assert!(miter([0., 0.], [1., 0.], [1., 1.], [0., 1.], 5., t).is_err());
    }
}
