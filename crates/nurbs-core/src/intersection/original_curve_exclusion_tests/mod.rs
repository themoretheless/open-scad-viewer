use super::*;
fn weighted_line(a: [f64; 3], b: [f64; 3], weights: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: weights.to_vec(),
        periodic: false,
    }
}
#[test]
fn original_rational_boxes_keep_a_known_intersection() {
    let a = weighted_line([0., 0., 0.], [2., 0., 0.], [1., 3.]);
    let b = weighted_line([1., -1., 0.], [1., 1., 0.], [1., 1.]);
    // a(1/4) = b(1/2) = [1,0,0] in exact rational arithmetic.
    assert!(!curves_excluded(&a, &b, [0.2, 0.3], [0.4, 0.6]).unwrap());
    let e = a.elevate(3).unwrap();
    assert!(!curves_excluded(&a, &e, [0., 1.], [0., 1.]).unwrap());
}
#[test]
fn disjoint_original_boxes_are_excluded_and_span_crossing_refused() {
    let a = weighted_line([0., 0., 0.], [1., 0., 0.], [0.8, 1.2]);
    let b = weighted_line([0., 2., 0.], [1., 2., 0.], [1.2, 0.8]);
    assert!(curves_excluded(&a, &b, [0., 1.], [0., 1.]).unwrap());
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]])
        .unwrap();
    assert!(curve_box(&c, c.domain()).is_err());
}
