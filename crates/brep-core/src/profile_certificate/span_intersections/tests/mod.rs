use super::*;
#[test]
fn complete_multi_span_components_and_isolated_knots() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.], vec![4., 4.], vec![0., 4.]])
        .unwrap();
    let b = Curve::from_polyline(vec![vec![2., 0.], vec![4., 0.], vec![4., 2.], vec![4., 4.]])
        .unwrap();
    let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
    let r = verify_span_intersections(&a, &b, &c).unwrap();
    assert!(r.overlaps.len() >= 3);
    let mut bad = c;
    bad.pairs.pop();
    assert!(verify_span_intersections(&a, &b, &bad).is_err());
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 3.], vec![6., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0., 1.], vec![6., 1.]]).unwrap();
    let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
    assert_eq!(
        verify_span_intersections(&a, &b, &c).unwrap().events.len(),
        2
    );
}
