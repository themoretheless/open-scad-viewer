use brep_core::profile_certificate::original_restriction::{self as r, wire};
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
fn own(c: &Curve) -> r::Restriction {
    r::restrict_binary64(c, c.domain()).unwrap()
}
#[test]
fn complete_rational_circle_checks_all_internal_and_closing_joins() {
    let mut c =
        nurbs_core::primitives::circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 360.).unwrap();
    for p in &mut c.control_points {
        p.truncate(2);
    }
    let wire = wire::sew(vec![own(&c)], vec![true], q(1, 1_000_000)).unwrap();
    let proof = wire.qualify_smooth_edits(q(1, 10_000_000_000)).unwrap();
    assert_eq!(proof.regular_span_count(), wire.pieces()[0].len());
    assert_eq!(proof.junctions().len(), wire.pieces()[0].len());
    proof.reverify().unwrap();
    assert!(wire.qualify_smooth_edits(q(1, 1)).is_err());
}
#[test]
fn reversed_tangent_is_not_geometric_g1_despite_zero_cross_product() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    let wire = wire::sew(vec![own(&a), own(&b)], vec![false, true], q(0, 1)).unwrap();
    let error = wire.qualify_smooth_edits(q(0, 1)).unwrap_err();
    assert!(error.to_string().contains("reversed tangent"));
}
#[test]
fn hidden_even_stationary_point_and_endpoint_stationarity_refuse() {
    for points in [
        vec![vec![0., 0.], vec![1., 0.], vec![0., 0.], vec![1., 0.]],
        vec![vec![0., 0.], vec![0., 0.], vec![1., 0.], vec![1., 0.]],
    ] {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        };
        let closing = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
        let wire = wire::sew(vec![own(&c), own(&closing)], vec![true, false], q(0, 1)).unwrap();
        let error = wire.qualify_smooth_edits(q(1, 10_000_000_000)).unwrap_err();
        assert!(error.to_string().contains("stationary rational span"));
    }
}
#[test]
fn unchanged_original_sharp_corners_remain_explicit_c0() {
    let c =
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.], vec![0., 0.]]).unwrap();
    let wire = wire::sew(vec![own(&c)], vec![false], q(0, 1)).unwrap();
    let proof = wire.qualify_smooth_edits(q(0, 1)).unwrap();
    assert!(proof.junctions().is_empty());
    assert_eq!(proof.regular_span_count(), wire.pieces()[0].len());
    proof.reverify().unwrap();
}
