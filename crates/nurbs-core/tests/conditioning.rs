use nurbs_core::{conditioning, curve::Curve};
#[test]
fn independent_weight_and_knot_scale_indicators() {
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 3., 6., 6.],
        control_points: vec![vec![10., 0.], vec![11., 2.], vec![14., 0.]],
        weights: vec![1., 4., 2.],
        periodic: false,
    };
    let r = conditioning::curve(&c).unwrap();
    assert_eq!(r.weight_ratio, 4.);
    assert_eq!(r.min_relative_knot_span, vec![0.25]);
    assert_eq!(r.coordinate_offset_ratio, Some(3.5));
    let mut q = c.clone();
    q.weights.iter_mut().for_each(|x| *x *= 2.);
    q.knots.iter_mut().for_each(|x| *x = 10. + 3. * *x);
    assert_eq!(conditioning::curve(&q).unwrap(), r);
    q.control_points.iter_mut().for_each(|p| p[0] += 100.);
    assert_eq!(
        conditioning::curve(&q).unwrap().coordinate_offset_ratio,
        Some(28.5)
    );
}
#[test]
fn surface_reports_whole_net_and_both_domains() {
    let c = nurbs_core::primitives::line([0.; 3], [4., 0., 0.]).unwrap();
    let mut s = nurbs_core::surface::extrude(&c, [0., 2., 0.]).unwrap();
    s.weights[1][1] = 8.;
    let r = conditioning::surface(&s).unwrap();
    assert_eq!(r.weight_ratio, 8.);
    assert_eq!(r.min_relative_knot_span, vec![1., 1.]);
    assert_eq!(r.coordinate_offset_ratio, Some(1.));
    assert!(!r.zero_extent);
}
#[test]
fn degenerate_geometry_and_invalid_input_are_explicit() {
    let mut c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![2., 3., 4.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let r = conditioning::curve(&c).unwrap();
    assert!(r.zero_extent);
    assert_eq!(r.coordinate_offset_ratio, None);
    c.weights[0] = 0.;
    assert!(conditioning::curve(&c).is_err());
}
