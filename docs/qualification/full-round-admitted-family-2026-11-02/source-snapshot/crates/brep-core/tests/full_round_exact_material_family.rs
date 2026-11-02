use brep_core::feature_ops::full_round_curved::build;
use nurbs_core::curve::Curve;
fn qualify(weight: f64, right: f64, left: f64) {
    let c = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|p| p.to_vec()).to_vec(),
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    let line =
        |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap();
    let source = brep_core::prism::extrude(
        &[vec![
            line([0., 0.], [4., 0.]),
            c([[4., 0.], [right, 4.], [4., 8.]]),
            line([4., 8.], [0., 8.]),
            c([[0., 8.], [left, 4.], [0., 0.]]),
        ]],
        0.,
        6.,
    )
    .unwrap();
    let before = source.clone();
    let r = build(&source, [3, 2, 1]).unwrap();
    let expected = usize::from(right < 4.) + usize::from(left > 0.) + 2;
    assert_eq!(r.exact_radius().spans().len(), expected);
    r.exact_radius().reverify().unwrap();
    assert!(r
        .exact_smoothness()
        .junctions()
        .iter()
        .all(|j| j.sine_squared == ["0".to_string(), "1".to_string()]));
    let relation = r.inspect_exact_source_material(Default::default()).unwrap();
    eprintln!("family weight={weight} right={right} left={left} spans={expected} added={} removed={} rays={}",
        relation.relation().added().count(),relation.relation().removed().count(),relation.relation().ray_cells());
    if let Some(witness) = relation.relation().added().next() {
        eprintln!("unexpected added: {witness:?}");
    }
    let admitted = relation.require_no_addition().unwrap();
    admitted.reverify().unwrap();
    assert_eq!(admitted.source().boundary().source(), &before);
    assert_eq!(source, before);
}
#[test]
fn symmetric_polynomial_one_plus_one() {
    qualify(1., 5., -1.);
}
#[test]
fn symmetric_rational_one_plus_one() {
    qualify(0.7, 5., -1.);
}
#[test]
fn concave_polynomial_two_plus_two() {
    qualify(1., 3., 1.);
}
#[test]
fn concave_rational_two_plus_two() {
    qualify(0.7, 3., 1.);
}
#[test]
fn asymmetric_polynomial_two_plus_one() {
    qualify(1., 3., -1.);
}
#[test]
fn asymmetric_rational_one_plus_two() {
    qualify(0.7, 5., 1.);
}
