use brep_core::profile_certificate::original_restriction::{self as exact, wire};
use nurbs_core::curve::Curve;
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn qualify(curves: Vec<Curve>) -> nurbs_core::Result<wire::regularity::SourceRegularity> {
    let pieces = curves
        .iter()
        .map(|c| exact::restrict_binary64(c, c.domain()))
        .collect::<nurbs_core::Result<Vec<_>>>()?;
    let mask = vec![false; pieces.len()];
    wire::sew(pieces, mask, ["0".into(), "1".into()])?.qualify_source_regularity()
}
fn rest() -> Vec<Curve> {
    vec![
        line([1., 0.], [1., 1.]),
        line([1., 1.], [0., 1.]),
        line([0., 1.], [0., 0.]),
    ]
}
#[test]
fn authored_corners_are_preserved_but_internal_knot_direction_is_exact() {
    let mut curves =
        vec![Curve::from_polyline(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]]).unwrap()];
    curves.extend(rest());
    let proof = qualify(curves).unwrap();
    assert_eq!(proof.span_count(), 5);
    assert_eq!(proof.internal_joins().len(), 1);
    assert_eq!(
        proof.internal_joins()[0].sine_squared,
        ["0".to_string(), "1".to_string()]
    );
    proof.reverify().unwrap();
    let bad = Curve::from_polyline(vec![vec![0., 0.], vec![0.5, 0.1], vec![1., 0.]]).unwrap();
    let mut curves = vec![bad];
    curves.extend(rest());
    assert!(qualify(curves).is_err());
}
#[test]
fn even_stationary_source_root_is_refused_without_sampling() {
    let first = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let mut curves = vec![first];
    curves.extend(rest());
    assert!(qualify(curves).is_err());
}
