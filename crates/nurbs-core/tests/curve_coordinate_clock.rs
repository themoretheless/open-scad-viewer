use nurbs_core::{curve::Curve, curve_coordinate_clock::inspect};
fn curve(values: &[f64], weights: Vec<f64>, degree: usize, knots: Vec<f64>) -> Curve {
    Curve {
        degree,
        knots,
        control_points: values
            .iter()
            .enumerate()
            .map(|(i, &v)| vec![v, (i % 2) as f64])
            .collect(),
        weights,
        periodic: false,
    }
}
#[test]
fn original_rational_multispan_clock_and_reverse_have_complete_covers() {
    let c = curve(
        &[0., 0.25, 0.75, 1.],
        vec![1., 0.875, 1.125, 1.],
        2,
        vec![0., 0., 0., 0.5, 1., 1., 1.],
    );
    let before = c.clone();
    let p = inspect(&c, 0, 1024).unwrap();
    assert!(p.proven && p.increasing);
    assert_eq!(p.spans, 2);
    assert!(p.covered_cells >= 2);
    let reverse = inspect(&c.reverse().unwrap(), 0, 1024).unwrap();
    assert!(reverse.proven && !reverse.increasing);
    assert_eq!(c, before);
    assert!(!inspect(&c, 0, 0).unwrap().proven);
    assert!(!inspect(&c, 0, p.cells - 1).unwrap().proven);
}
#[test]
fn folded_interior_clock_and_discontinuous_original_knots_cannot_pass() {
    let folded = curve(
        &[0., 1., 0., 0., 1.],
        vec![1.; 5],
        4,
        vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
    );
    assert!(!inspect(&folded, 0, 1024).unwrap().proven);
    let jump = curve(
        &[0., 0.5, 0.4, 1.],
        vec![1.; 4],
        1,
        vec![0., 0., 0.5, 0.5, 1., 1.],
    );
    assert!(inspect(&jump, 0, 1024).is_err());
}
#[test]
fn stationary_and_closed_endpoint_clocks_remain_unresolved() {
    let flat = curve(
        &[0., 0., 0., 1.],
        vec![1.; 4],
        3,
        vec![0., 0., 0., 0., 1., 1., 1., 1.],
    );
    assert!(!inspect(&flat, 0, 64).unwrap().proven);
    let loopback = curve(&[0., 1., 0.], vec![1.; 3], 2, vec![0., 0., 0., 1., 1., 1.]);
    assert!(!inspect(&loopback, 0, 64).unwrap().proven);
}
