use super::*;
fn chain(extra_touch: bool) -> Vec<Curve> {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![1., 0.],
            vec![0.5, 0.],
            vec![if extra_touch { 0. } else { 0.25 }, 0.25],
            vec![if extra_touch { 0.25 } else { 0. }, 0.5],
            vec![0., 1.],
        ],
        weights: vec![1., 2., 1., 2., 1.],
        periodic: false,
    };
    vec![
        curve,
        Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
    ]
}
#[test]
fn literal_c0_cusp_support_uses_each_original_span_and_keeps_rational_weights() {
    let c = chain(false);
    let before = c.clone();
    let r = inspect(&c, 1e-8, 100, 10000).unwrap();
    assert!(r.proven_simple);
    assert_eq!(c, before);
}
#[test]
fn a_zero_at_an_internal_c0_endpoint_is_an_extra_contact_not_a_join() {
    let r = inspect(&chain(true), 1e-8, 100, 10000).unwrap();
    assert!(!r.proven_simple);
}
#[test]
fn c0_support_exhaustion_never_grants_simplicity() {
    let r = inspect(&chain(false), 1e-8, 100, 1).unwrap();
    assert!(!r.proven_simple);
    assert!(r.cells <= 1);
}
