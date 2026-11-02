use nurbs_core::{curve::Curve, curve_partition_agreement::verify_source_closure};
fn closed() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.13, 0.27, 0.39],
            vec![1., 2., 3.],
            vec![0.13, 0.27, 0.39],
        ],
        weights: vec![0.5, 0.75, 2.],
        periodic: false,
    }
}
#[test]
fn clamped_cartesian_endpoints_close_independently_of_positive_weight_gauges() {
    let c = closed();
    assert!(verify_source_closure(&c).unwrap());
    let mut planar = c.clone();
    for p in &mut planar.control_points {
        p.truncate(2);
    }
    assert!(verify_source_closure(&planar).unwrap());
}
#[test]
fn near_endpoints_and_unclamped_endpoint_controls_cannot_claim_exact_closure() {
    let c = closed();
    let mut near = c.clone();
    near.control_points.last_mut().unwrap()[0] = 0.13f64.next_up();
    assert!(!verify_source_closure(&near).unwrap());
    let mut unclamped = c;
    unclamped.knots = vec![-1., 0., 1., 2., 3., 4.];
    unclamped.validate().unwrap();
    assert!(!verify_source_closure(&unclamped).unwrap());
}

#[test]
fn inactive_exterior_poles_do_not_replace_the_actual_original_endpoints() {
    let c = Curve {
        degree: 2,
        knots: vec![-1., -1., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![9., 9., 9.],
            vec![0.13, 0.27, 0.39],
            vec![1., 2., 3.],
            vec![0.13, 0.27, 0.39],
        ],
        weights: vec![1., 0.5, 0.75, 2.],
        periodic: false,
    };
    c.validate().unwrap();
    assert_ne!(c.control_points[0], *c.control_points.last().unwrap());
    assert!(verify_source_closure(&c).unwrap());
    let mut bad = c;
    bad.control_points.last_mut().unwrap()[0] = 0.13f64.next_up();
    assert!(!verify_source_closure(&bad).unwrap());
}
