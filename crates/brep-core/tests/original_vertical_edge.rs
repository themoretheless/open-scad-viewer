use brep_core::profile_certificate::original_restriction::vertical;
use nurbs_core::curve::Curve;
fn ratio(n: &str) -> [String; 2] {
    [n.into(), "1".into()]
}
fn curve(z: &[f64]) -> Curve {
    let degree = z.len() - 1;
    Curve {
        degree,
        knots: [vec![0.1; degree + 1], vec![1.9; degree + 1]].concat(),
        control_points: z.iter().map(|z| vec![2., 3., *z]).collect(),
        weights: vec![1.; z.len()],
        periodic: false,
    }
}
fn qualify(c: &Curve) -> nurbs_core::Result<vertical::Vertical> {
    vertical::qualify(c, [ratio("2"), ratio("3")], [ratio("0"), ratio("6")])
}
#[test]
fn exact_rational_vertical_coverage_keeps_original_clock_and_both_directions() {
    let mut c = curve(&[0., 2., 6.]);
    c.weights[1] = 0.7;
    let proof = qualify(&c).unwrap();
    assert_eq!(proof.source(), &c);
    assert_eq!(proof.span_count(), 1);
    proof.reverify().unwrap();
    c.control_points.reverse();
    c.weights.reverse();
    let reverse = qualify(&c).unwrap();
    assert_eq!(reverse.endpoints()[0][2], ratio("6"));
    reverse.reverify().unwrap();
}
#[test]
fn stationary_even_root_endpoints_and_backtracking_refuse() {
    assert!(qualify(&curve(&[0., 6., 0., 6.])).is_err());
    assert!(qualify(&curve(&[0., 0., 6.])).is_err());
    assert!(qualify(&curve(&[0., 9., -3., 6.])).is_err());
    assert!(qualify(&curve(&[0., 2., 5.])).is_err());
}
#[test]
fn tiny_interior_xy_bow_refuses_exact_side_binding() {
    let mut c = curve(&[0., 3., 6.]);
    c.control_points[1][0] += 1e-12;
    assert!(qualify(&c).is_err());
}
