use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{self as d, Refusal},
};
fn bezier(points: Vec<Vec<f64>>, weights: Vec<f64>, a: f64, b: f64) -> Curve {
    let p = points.len() - 1;
    Curve {
        degree: p,
        knots: [vec![a; p + 1], vec![b; p + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn weighted() -> Curve {
    bezier(vec![vec![0., 0.], vec![2., 0.]], vec![1., 2.], -2., 2.)
}
fn accepted(s: Curve) -> Vec<Curve> {
    let before = s.clone();
    let r = d::curve(&s, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    let pieces = r.pieces.unwrap();
    assert_eq!(s, before);
    let mut breaks = s.knots[s.degree..=s.control_points.len()].to_vec();
    breaks.dedup();
    assert_eq!(pieces.len() + 1, breaks.len());
    for (i, p) in pieces.iter().enumerate() {
        assert_eq!(p.domain(), [breaks[i], breaks[i + 1]]);
        assert!(!p.periodic);
        p.validate().unwrap();
        for j in 1..8 {
            let u = breaks[i] + (breaks[i + 1] - breaks[i]) * j as f64 / 8.;
            let expected = s.evaluate(u).unwrap().d1.unwrap();
            let actual = p.evaluate(u).unwrap().point;
            for (a, b) in actual.iter().zip(expected) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-derivative-oracle={}",
        value_codec::json!({"source":s,"pieces":pieces})
    );
    pieces
}
#[test]
fn rational_derivative_uses_original_absolute_clock_not_unit_span() {
    let pieces = accepted(weighted());
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].degree, 2);
    assert_eq!(
        pieces[0].control_points,
        vec![vec![1., 0.], vec![0.5, 0.], vec![0.25, 0.]]
    );
    assert_eq!(pieces[0].weights, vec![0.25, 0.5, 1.]);
}
#[test]
fn polynomial_and_constant_derivatives_drop_true_degree_without_changing_domain() {
    let pieces = accepted(bezier(
        vec![vec![0., 0., 1.], vec![1., 2., 1.], vec![2., 0., 1.]],
        vec![1.; 3],
        2.,
        6.,
    ));
    assert_eq!(pieces[0].degree, 1);
    assert_eq!(
        pieces[0].control_points,
        vec![vec![0.5, 1., 0.], vec![0.5, -1., 0.]]
    );
    let pieces = accepted(bezier(vec![vec![3., 5.]; 26], vec![1.; 26], -3., 5.));
    assert_eq!(pieces[0].degree, 1);
    assert!(pieces[0].control_points.iter().flatten().all(|&x| x == 0.));
    let pieces = accepted(bezier(
        (0..26).map(|i| vec![i as f64, 0.]).collect(),
        vec![1.; 26],
        0.,
        1.,
    ));
    assert_eq!(pieces[0].control_points, vec![vec![25., 0.]; 2]);
}
#[test]
fn general_multispan_nonuniform_knots_have_complete_cover_and_correct_scale() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 3., 3., 3.],
        control_points: vec![vec![0., 0.], vec![3., 6.], vec![6., 0.], vec![9., 6.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let out = accepted(s);
    assert_eq!(out[0].control_points, vec![vec![6., 12.], vec![2., -4.]]);
    assert_eq!(out[1].control_points, vec![vec![2., -4.], vec![3., 6.]]);
}
#[test]
fn unclamped_and_periodic_bases_are_not_replaced_with_rounded_trims() {
    let mut s = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![0., 2.],
            vec![0., 0.],
            vec![2., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    accepted(s.clone());
    s.periodic = true;
    accepted(s);
}
#[test]
fn knots_with_derivative_jumps_retain_both_one_sided_traces() {
    let s = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.], vec![2., 3.]]).unwrap();
    let out = accepted(s);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].evaluate(1.).unwrap().point, vec![2., 0.]);
    assert_eq!(out[1].evaluate(1.).unwrap().point, vec![0., 3.]);
}
#[test]
fn common_homogeneous_source_factor_is_cancelled_without_changing_clock() {
    let s = bezier(
        vec![vec![0., 0.], vec![0.25, 0.], vec![1., 0.]],
        vec![1., 2., 3.],
        0.,
        1.,
    );
    accepted(s);
}
#[test]
fn failure_on_a_late_span_does_not_publish_an_earlier_success() {
    let s = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let before = s.clone();
    let r = d::curve(&s, 1000000).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert_eq!(s, before);
}
#[test]
fn output_degree_is_bounded_after_true_factor_cancellation() {
    let mut points = vec![vec![0., 0.]; 14];
    points[13][0] = 1.;
    let mut weights = vec![1.; 14];
    weights[13] = 2.;
    let r = d::curve(&bezier(points, weights, 0., 1.), 1000000).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
}
#[test]
fn work_budget_is_shared_across_extraction_reduction_storage_and_proof() {
    let s = weighted();
    let n = d::curve(&s, 1000000).unwrap().work_used;
    assert!(d::curve(&s, n).unwrap().pieces.is_some());
    for n in [1, n - 1] {
        let r = d::curve(&s, n).unwrap();
        assert!(r.pieces.is_none());
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.work_used <= n);
    }
}
#[test]
fn invalid_input_and_budget_are_errors() {
    for n in [0, 1000001, u64::MAX] {
        assert!(d::curve(&weighted(), n).is_err());
    }
    let mut s = weighted();
    s.weights[1] = 0.;
    assert!(d::curve(&s, 1000000).is_err());
    let mut s = weighted();
    s.control_points[1][0] = f64::NAN;
    assert!(d::curve(&s, 1000000).is_err());
}
