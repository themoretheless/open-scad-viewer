use nurbs_core::{curve::Curve, trim_simplicity};
fn arc(y: f64) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, y], vec![1., 0.]],
        weights: vec![1., 0.5, 1.],
        periodic: false,
    }
}
#[test]
fn two_arc_lenses_require_separation_in_addition_to_exact_shared_endpoints() {
    let upper = arc(0.25);
    let chord = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    let lower = arc(-0.25).reverse().unwrap();
    for second in [chord, lower] {
        let curves = vec![upper.clone(), second];
        let before = curves.clone();
        let r = trim_simplicity::inspect(&curves, 1e-8, 10, 100).unwrap();
        assert!(r.proven_simple && r.exact_joins == Some(true));
        assert_eq!(r.pairs.len(), 1);
        assert!(r.pairs[0].proven);
        assert_eq!(curves, before);
    }
    let retraced = vec![upper.clone(), upper.reverse().unwrap()];
    assert!(
        !trim_simplicity::inspect(&retraced, 1e-8, 10, 100)
            .unwrap()
            .proven_simple
    );
    let line = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    assert!(
        !trim_simplicity::inspect(&[line.clone(), line.reverse().unwrap()], 1e-8, 10, 100)
            .unwrap()
            .proven_simple
    );
    let mut crossed = upper.clone();
    crossed.degree = 3;
    crossed.knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    crossed.control_points = vec![
        vec![0., 0.],
        vec![0.25, -0.25],
        vec![0.75, 0.25],
        vec![1., 0.],
    ];
    crossed.weights = vec![1.; 4];
    let chord = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    assert!(
        !trim_simplicity::inspect(&[crossed, chord], 1e-8, 10, 100)
            .unwrap()
            .proven_simple
    );
}
