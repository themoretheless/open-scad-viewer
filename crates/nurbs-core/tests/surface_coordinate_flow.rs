use nurbs_core::{surface::Surface, surface_coordinate_flow};
fn plane(sign: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![6. * sign, 0., 3.], vec![6. * sign, 1., 3.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_nonnormalized_chart_bounds_both_flow_directions() {
    for sign in [-1., 1.] {
        let s = plane(sign);
        let r = surface_coordinate_flow::inspect(&s, 0, 0, 1).unwrap();
        assert!(r.complete);
        assert_eq!(r.direction, Some(if sign > 0. { 1 } else { -1 }));
        let d = r.derivative_interval.unwrap();
        assert!(d[0] <= 2. * sign && 2. * sign <= d[1]);
        let ratio = r.inverse_distance_factor.unwrap();
        assert!(ratio >= 5_f64.sqrt() / 2. && ratio < 1.119);
    }
}
#[test]
fn budget_flat_coordinate_and_invalid_axes_do_not_grant_flow() {
    let s = plane(1.);
    let r = surface_coordinate_flow::inspect(&s, 0, 0, 0).unwrap();
    assert!(!r.complete && r.direction.is_none() && r.inverse_distance_factor.is_none());
    let flat = surface_coordinate_flow::inspect(&s, 0, 1, 1).unwrap();
    assert!(flat.complete && flat.direction.is_none() && flat.inverse_distance_factor.is_none());
    assert!(surface_coordinate_flow::inspect(&s, 2, 0, 1).is_err());
}
#[test]
fn separated_monotone_spans_do_not_prove_global_flow() {
    let mut s = plane(1.);
    s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
    s.control_points = vec![
        vec![vec![0.,0.,0.],vec![0.,1.,0.]],
        vec![vec![1.,0.,0.],vec![1.,1.,0.]],
        vec![vec![-2.,0.,0.],vec![-2.,1.,0.]],
        vec![vec![-1.,0.,0.],vec![-1.,1.,0.]],
    ];
    s.weights = vec![vec![1.;2];4];
    if let Ok(r) = surface_coordinate_flow::inspect(&s,0,0,10) {
        assert!(!r.complete && r.direction.is_none() && r.inverse_distance_factor.is_none());
    }
}
