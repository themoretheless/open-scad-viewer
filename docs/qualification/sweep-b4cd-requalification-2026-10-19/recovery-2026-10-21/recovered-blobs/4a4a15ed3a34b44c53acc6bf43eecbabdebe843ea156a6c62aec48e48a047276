use nurbs_core::{
    curve::Curve,
    curve_join::{self, Decision, Endpoint, Tolerances},
};
fn tol() -> Tolerances {
    Tolerances {
        position: 1e-8,
        tangent: 1e-8,
        curvature: 1e-8,
    }
}
fn bez(p: Vec<Vec<f64>>) -> Curve {
    nurbs_core::paths::bezier(p, None).unwrap()
}
#[test]
fn unequal_parameter_speeds_have_geometric_second_order_join() {
    let a = bez(vec![vec![-2., 4.], vec![-1., 0.], vec![0., 0.]]);
    let b = bez(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]]);
    let r = curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol()).unwrap();
    assert_eq!(r.g0, Decision::WithinTolerance);
    assert_eq!(r.g1, Decision::WithinTolerance);
    assert_eq!(r.g2, Decision::WithinTolerance);
    let reverse = b.reverse().unwrap();
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &reverse, Endpoint::End, tol())
            .unwrap()
            .g2,
        Decision::WithinTolerance
    );
}
#[test]
fn curvature_and_tangent_departures_are_detected() {
    let a = bez(vec![vec![-2., 4.], vec![-1., 0.], vec![0., 0.]]);
    let b = bez(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 2.]]);
    let r = curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol()).unwrap();
    assert_eq!(r.g1, Decision::WithinTolerance);
    assert_eq!(r.g2, Decision::ExceedsTolerance);
    let b = bez(vec![vec![0., 0.], vec![0., 1.]]);
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol())
            .unwrap()
            .g1,
        Decision::ExceedsTolerance
    );
}
#[test]
fn stationary_endpoint_and_invalid_tolerance_are_explicit() {
    let a = bez(vec![vec![0., 0.], vec![0., 0.]]);
    let b = bez(vec![vec![0., 0.], vec![1., 0.]]);
    let r = curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol()).unwrap();
    assert_eq!(r.g0, Decision::WithinTolerance);
    assert_eq!(r.g1, Decision::RegularityUnproven);
    assert!(curve_join::inspect(
        &a,
        Endpoint::End,
        &b,
        Endpoint::Start,
        Tolerances {
            position: -1.,
            ..tol()
        }
    )
    .is_err());
}

#[test]
fn domains_common_weight_scales_and_position_gaps() {
    let mut a = bez(vec![vec![-2., 4.], vec![-1., 0.], vec![0., 0.]]);
    let mut b = bez(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]]);
    a.weights.fill(4.);
    b.weights.fill(8.);
    let a = nurbs_core::foundation::reparameterize_curve_report(&a, [-8., 4.], None)
        .unwrap()
        .curve;
    let b = nurbs_core::foundation::reparameterize_curve_report(&b, [2., 17.], None)
        .unwrap()
        .curve;
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol())
            .unwrap()
            .g2,
        Decision::WithinTolerance
    );
    let mut gap = b;
    gap.control_points.iter_mut().for_each(|p| p[1] += 0.25);
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &gap, Endpoint::Start, tol())
            .unwrap()
            .g0,
        Decision::ExceedsTolerance
    );
}

#[test]
fn unequal_rational_weights_on_straight_segments_preserve_g2() {
    let a =
        nurbs_core::paths::bezier(vec![vec![-4., 0.], vec![0., 0.]], Some(vec![1., 8.])).unwrap();
    let b =
        nurbs_core::paths::bezier(vec![vec![0., 0.], vec![7., 0.]], Some(vec![3., 1.])).unwrap();
    let r = curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol()).unwrap();
    assert_eq!(r.g2, Decision::WithinTolerance);
    let a = a.reverse().unwrap();
    assert_eq!(
        curve_join::inspect(&a, Endpoint::Start, &b, Endpoint::Start, tol())
            .unwrap()
            .g2,
        Decision::WithinTolerance
    );
}
#[test]
fn spatial_cubic_join_uses_curvature_vector_not_third_derivatives() {
    // Both curves follow [x,x^2,x^3] near x=0, with x=t-1 and x=2t.
    let a = bez(vec![
        vec![-1., 1., -1.],
        vec![-2. / 3., 1. / 3., 0.],
        vec![-1. / 3., 0., 0.],
        vec![0., 0., 0.],
    ]);
    let mut b = bez(vec![
        vec![0., 0., 0.],
        vec![2. / 3., 0., 0.],
        vec![4. / 3., 4. / 3., 0.],
        vec![2., 4., 8.],
    ]);
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol())
            .unwrap()
            .g2,
        Decision::WithinTolerance
    );
    b.control_points[3][2] = 100.; // Changes third derivative only at the start.
    assert_eq!(
        curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol())
            .unwrap()
            .g2,
        Decision::WithinTolerance
    );
}
#[test]
fn proven_gap_takes_precedence_over_stationary_regularity() {
    let a = bez(vec![vec![0., 0.], vec![0., 0.]]);
    let b = bez(vec![vec![1., 0.], vec![2., 0.]]);
    let r = curve_join::inspect(&a, Endpoint::End, &b, Endpoint::Start, tol()).unwrap();
    assert_eq!(r.g0, Decision::ExceedsTolerance);
    assert_eq!(r.g1, Decision::ExceedsTolerance);
    assert_eq!(r.g2, Decision::ExceedsTolerance);
}
