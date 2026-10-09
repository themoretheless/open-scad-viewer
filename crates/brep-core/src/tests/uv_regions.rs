use super::*;
fn curve(points: Vec<Vec<f64>>) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points,
        weights: vec![1., 2., 3.],
        periodic: false,
    }
}
#[test]
fn repeated_endpoint_poles_use_the_first_nonzero_geometric_jet() {
    let source = curve(vec![vec![0., 0.], vec![0., 0.], vec![1., 1.]]);
    assert_eq!(source.evaluate(0.).unwrap().d1.unwrap(), vec![0., 0.]);
    assert!((tangent_angle(&source, true).unwrap() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
    let reverse = curve(vec![vec![1., 1.], vec![0., 0.], vec![0., 0.]]);
    assert!((tangent_angle(&reverse, false).unwrap() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
}
#[test]
fn constant_or_small_nonzero_endpoint_jets_are_not_skipped() {
    assert!(tangent_angle(&curve(vec![vec![0., 0.]; 3]), true).is_err());
    assert!(
        tangent_angle(
            &curve(vec![vec![0., 0.], vec![1e-15, 0.], vec![1., 1.]]),
            true
        )
        .is_err()
    );
}
