use nurbs_core::{
    curve::Curve,
    curve_self_intersection::{self, Status},
};
#[test]
fn rational_spatial_graph_is_globally_simple() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 2., -1.], vec![1., 0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    let before = c.clone();
    let r = curve_self_intersection::inspect(&c, 1e-7, 10000, 1000).unwrap();
    assert_eq!(r.status, Status::Absent);
    assert_eq!(c, before);
    assert!(r.cells <= 10000);
}
#[test]
fn proper_bow_tie_crossing_is_proven_and_closed_rectangle_is_simple() {
    let bow = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![1., 0.],
        vec![0., 0.],
    ])
    .unwrap();
    let r = curve_self_intersection::inspect(&bow, 1e-7, 100, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let p = r.witness_point.unwrap();
    assert!((p[0] - 0.5).abs() < 1e-6 && (p[1] - 0.5).abs() < 1e-6);
    let rectangle = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    assert_eq!(
        curve_self_intersection::inspect(&rectangle, 1e-7, 10000, 100)
            .unwrap()
            .status,
        Status::Absent
    );
}
#[test]
fn inadequate_budget_and_invalid_input_are_explicit() {
    let c = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![1., 0.],
        vec![0., 0.],
    ])
    .unwrap();
    assert_eq!(
        curve_self_intersection::inspect(&c, 1e-7, 1, 1)
            .unwrap()
            .status,
        Status::Unresolved
    );
    assert!(curve_self_intersection::inspect(&c, 1e-7, 0, 100).is_err());
    assert!(curve_self_intersection::inspect(&c, f64::NAN, 100, 100).is_err());
}

#[test]
fn rational_closed_circle_is_proven_across_curved_span_pairs() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
    let r = curve_self_intersection::inspect(&c, 1e-7, 10000, 1000).unwrap();
    assert_eq!(r.status, Status::Absent, "{r:?}");
    assert!(r.pairs > 0);
}
