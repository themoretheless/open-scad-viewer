use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{self as p, Refusal},
};
fn source() -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 1., 1.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn matrix() -> p::Matrix {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}
#[test]
fn exact_rank_has_no_arbitrary_small_determinant_threshold() {
    let mut m = matrix();
    m[0][0] = f64::MIN_POSITIVE;
    let r = p::curve(&source(), m, 1000000).unwrap();
    let out = r.geometry.unwrap();
    assert_eq!(out.control_points[1][0], f64::MIN_POSITIVE);
    assert!(r.refusal.is_none());
    let mut m = matrix();
    m[0][0] = 1e308;
    let mut s = source();
    s.control_points[1][0] = 0.;
    assert!(p::curve(&s, m, 1000000).unwrap().geometry.is_some());
}
#[test]
fn exact_intermediate_cancellation_does_not_overflow_binary64() {
    let mut s = source();
    s.control_points[0] = vec![1., 1., 0.];
    s.weights.fill(1e12);
    let mut m = matrix();
    m[3] = [1e308, -1e308, 0., 1.];
    let r = p::curve(&s, m, 1000000).unwrap();
    assert_eq!(r.geometry.unwrap().control_points, s.control_points);
    assert!(r.refusal.is_none());
}
#[test]
fn subnormal_underflow_at_a_late_pole_refuses_the_whole_result() {
    let mut s = source();
    s.control_points[1][0] = 0.5;
    let mut m = matrix();
    m[0][0] = f64::from_bits(1);
    let r = p::curve(&s, m, 1000000).unwrap();
    assert!(r.geometry.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert_eq!(s.control_points[1][0], 0.5);
}
#[test]
fn exact_geometry_does_not_bypass_existing_weight_range_limits() {
    let mut s = source();
    let tiny = 2f64.powi(-50);
    s.control_points[1][0] = 1. - tiny;
    let mut m = matrix();
    m[3] = [1., 0., 0., tiny];
    let r = p::curve(&s, m, 1000000).unwrap();
    assert!(r.geometry.is_none());
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
}
