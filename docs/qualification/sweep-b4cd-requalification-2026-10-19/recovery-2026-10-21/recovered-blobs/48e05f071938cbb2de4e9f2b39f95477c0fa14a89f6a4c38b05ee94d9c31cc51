use nurbs_core::{
    curve::Curve,
    curve_offset_wire::{self, Role},
};
#[test]
fn entire_cornered_profile_has_connected_bevel_and_known_geometry() {
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![10., 0.], vec![10., 10.]]).unwrap();
    let before = c.clone();
    for d in [-2., 2.] {
        let w = curve_offset_wire::bevel_wire(&c, d, 1e-6, 100).unwrap();
        assert!(!w.closed);
        assert_eq!(w.edges.len(), 3);
        assert!(matches!(w.roles[1], Role::Bevel(_)));
        assert_eq!(w.edges[0].points[1], w.edges[1].points[0]);
        assert_eq!(w.edges[1].points[1], w.edges[2].points[0]);
        for (actual, expected) in w.edges[1].points.iter().zip([[10., d], [10. - d, 0.]]) {
            assert!((actual[0] - expected[0]).hypot(actual[1] - expected[1]) <= w.error_upper_mm);
        }
        assert!(w.error_upper_mm <= 1e-6);
    }
    assert_eq!(c, before);
}
#[test]
fn closed_profile_and_rational_straight_offset_preserve_continuous_contract() {
    let c = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![10., 0.],
        vec![10., 10.],
        vec![0., 10.],
        vec![0., 0.],
    ])
    .unwrap();
    let w = curve_offset_wire::bevel_wire(&c, -1., 1e-6, 100).unwrap();
    assert!(w.closed);
    assert_eq!(w.edges.len(), 8);
    assert_eq!(
        w.edges.first().unwrap().points[0],
        w.edges.last().unwrap().points[1]
    );
    let r = Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let w = curve_offset_wire::bevel_wire(&r, 1., 1e-3, 10000).unwrap();
    assert!(w.error_upper_mm <= 1e-3);
    for edge in &w.edges {
        for p in edge.points {
            assert!((p[1] - 1.).abs() <= w.error_upper_mm);
        }
    }
}
#[test]
fn invalid_normal_or_work_contract_is_refused() {
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]]).unwrap();
    assert!(curve_offset_wire::bevel_wire(&c, 0., 1e-6, 100).is_err());
    assert!(curve_offset_wire::bevel_wire(&c, 1., f64::NAN, 100).is_err());
    assert!(curve_offset_wire::bevel_wire(&c, 1., 1e-6, 0).is_err());
    assert!(curve_offset_wire::bevel_wire(&c, 1., 1e-6, 1).is_err());
}
