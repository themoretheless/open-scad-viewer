use nurbs_core::{curve::Curve, editing::exact_derivative};
fn check(source: Curve) -> Vec<Curve> {
    let r = exact_derivative::curve(&source, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    let pieces = r.pieces.unwrap();
    for p in &pieces {
        let [a, b] = p.domain();
        for i in 1..8 {
            let u = a + (b - a) * i as f64 / 8.;
            let expected = source.evaluate(u).unwrap().d1.unwrap();
            let actual = p.evaluate(u).unwrap().point;
            for (a, b) in actual.iter().zip(expected) {
                assert!((a - b).abs() < 1e-12);
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-derivative-oracle={}",
        value_codec::json!({"source":source,"pieces":pieces})
    );
    pieces
}
#[test]
fn rational_nonuniform_quadratic_bspline_extracts_original_denominator_on_every_span() {
    let source = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 3., 3., 3.],
        control_points: vec![vec![12., 0.], vec![8., 0.], vec![4., 0.], vec![3., 0.]],
        weights: vec![2., 3., 6., 8.],
        periodic: false,
    };
    let out = check(source);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].domain(), [0., 1.]);
    assert_eq!(out[1].domain(), [1., 3.]);
    assert_eq!(
        out[0].control_points,
        vec![vec![-12., 0.], vec![-6., 0.], vec![-3., 0.]]
    );
    assert_eq!(
        out[1].control_points,
        vec![vec![-3., 0.], vec![-1.5, 0.], vec![-0.75, 0.]]
    );
    for p in out {
        assert_eq!(p.weights, vec![0.25, 0.5, 1.]);
    }
}
#[test]
fn periodic_rational_aliases_keep_both_fundamental_seam_derivative_traces() {
    let source = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: true,
    };
    let out = check(source);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].domain(), [0., 1.]);
    assert_eq!(out[1].domain(), [1., 2.]);
    assert_eq!(out[0].evaluate(0.).unwrap().point, vec![4., 0.]);
    assert_eq!(out[1].evaluate(2.).unwrap().point, vec![-4., 0.]);
    assert_eq!(out[0].evaluate(1.).unwrap().point, vec![1., 0.]);
    assert_eq!(out[1].evaluate(1.).unwrap().point, vec![-1., 0.]);
    assert!(out.iter().all(|p| !p.periodic));
}
#[test]
fn repeated_quadratic_knots_keep_3d_left_and_right_vector_traces() {
    let source = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![1., 1., 1.],
            vec![2., 0., 1.],
            vec![2., 1., 1.],
            vec![3., 2., 1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let out = check(source);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].degree, 1);
    assert_eq!(out[1].degree, 1);
    assert_eq!(out[0].evaluate(1.).unwrap().point, vec![2., -2., 0.]);
    assert_eq!(out[1].evaluate(1.).unwrap().point, vec![0., 2., 0.]);
}
