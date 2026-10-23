use nurbs_core::{curve::Curve, trim_simplicity};
fn loop_with(last: Curve) -> Vec<Curve> {
    let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut curves = (0..3)
        .map(|i| Curve::from_polyline(vec![points[i].to_vec(), points[i + 1].to_vec()]).unwrap())
        .collect::<Vec<_>>();
    curves.push(last);
    curves
}
#[test]
fn original_rational_multispan_monotonicity_proves_simple_uv_loops_without_decomposition() {
    let last = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![0., 1.], vec![0., 0.5], vec![0., 0.]],
        weights: vec![1., 0.5, 2.],
        periodic: false,
    };
    let curves = loop_with(last);
    let before = curves.clone();
    let report = trim_simplicity::inspect(&curves, 1e-8, 100, 10000).unwrap();
    assert!(report.proven_simple && report.injective.iter().all(|v| *v));
    assert_eq!(curves, before);
}
#[test]
fn flat_spans_and_discontinuous_knot_jumps_never_gain_injectivity() {
    for (knots, points) in [
        (
            vec![0., 0., 0.3, 0.6, 1., 1.],
            vec![vec![0., 1.], vec![0., 0.5], vec![0., 0.5], vec![0., 0.]],
        ),
        (
            vec![0., 0., 0.5, 0.5, 1., 1.],
            vec![vec![0., 1.], vec![0., 0.7], vec![0., 0.3], vec![0., 0.]],
        ),
    ] {
        let c = Curve {
            degree: 1,
            knots,
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        };
        if c.knots[2] == c.knots[3] {
            assert!(trim_simplicity::inspect(&loop_with(c), 1e-8, 100, 10000).is_err());
        } else {
            let r = trim_simplicity::inspect(&loop_with(c), 1e-8, 100, 10000).unwrap();
            assert!(!r.proven_simple && !r.injective[3]);
        }
    }
}
