use nurbs_core::{
    curve::Curve,
    editing::exact_knot_insertion::{Refusal, insert},
};
fn line(end: f64, weights: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![end, 0.]],
        weights,
        periodic: false,
    }
}
fn quadratic() -> Curve {
    Curve {
        degree: 2,
        knots: vec![-2., -2., -2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
fn accepted(source: &Curve, u: f64, count: usize) -> Curve {
    let r = insert(source, u, count, 1_000_000).unwrap();
    assert_eq!(r.refusal, None);
    assert_eq!(r.inserted, count);
    assert!(r.work_used <= 1_000_000);
    assert_eq!(r.curve.domain(), source.domain());
    println!(
        "insertion-oracle={{\"source\":{{\"degree\":{},\"knots\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}},\"output\":{{\"degree\":{},\"knots\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}}}",
        source.degree,
        source.knots,
        source.control_points,
        source.weights,
        r.curve.degree,
        r.curve.knots,
        r.curve.control_points,
        r.curve.weights
    );
    r.curve
}
#[test]
fn quadratic_batch_preserves_absolute_clock() {
    let c = accepted(&quadratic(), 2., 2);
    assert_eq!(
        c.control_points,
        vec![
            vec![0., 0.],
            vec![0.5, 0.5],
            vec![1., 0.5],
            vec![1.5, 0.5],
            vec![2., 0.]
        ]
    );
    for i in 0..=32 {
        let t = i as f64 / 32.;
        let p = c.evaluate(-2. + 8. * t).unwrap().point;
        assert!((p[0] - 2. * t).abs() < 1e-12);
        assert!((p[1] - 2. * t * (1. - t)).abs() < 1e-12);
    }
}
#[test]
fn rational_refinement_uses_one_global_weight_scale() {
    let c = accepted(&line(3., vec![1., 2.]), 0.5, 1);
    assert_eq!(
        c.control_points,
        vec![vec![0., 0.], vec![2., 0.], vec![3., 0.]]
    );
    assert_eq!(c.weights, vec![0.5, 0.75, 1.]);
}
#[test]
fn nonrepresentable_cartesian_pole_refuses_atomically() {
    for (c, u) in [(line(2., vec![1., 2.]), 0.5), (line(3., vec![1., 1.]), 0.1)] {
        let r = insert(&c, u, 1, 1_000_000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
        assert_eq!(r.inserted, 0);
        assert_eq!(r.curve, c);
    }
}
#[test]
fn literal_float_knot_is_retained() {
    let c = accepted(&line(1., vec![1., 1.]), 0.1, 1);
    assert_eq!(c.knots, vec![0., 0., 0.1, 1., 1.]);
    assert_eq!(c.control_points[1][0].to_bits(), 0.1f64.to_bits());
}
#[test]
fn existing_internal_knot_and_multiple_spans() {
    let c = accepted(&quadratic(), 2., 1);
    let d = accepted(&c, 2., 1);
    let e = accepted(&d, 0., 1);
    assert_eq!(e.control_points.len(), 6);
}
#[test]
fn budget_refusal_never_publishes_partial_refinement() {
    for budget in [1, 100] {
        let c = quadratic();
        let r = insert(&c, 2., 2, budget).unwrap();
        assert_eq!(r.curve, c);
        assert_eq!(r.inserted, 0);
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.work_used <= budget);
    }
}
#[test]
fn invalid_requests_and_invalid_tail_are_errors() {
    let c = quadratic();
    for (u, n) in [
        (-2., 1),
        (6., 1),
        (f64::NAN, 1),
        (2., 0),
        (2., 3),
        (2., usize::MAX),
    ] {
        assert!(insert(&c, u, n, 1_000_000).is_err());
    }
    assert!(insert(&c, 2., 1, 0).is_err());
    assert!(insert(&c, 2., 1, 1_000_001).is_err());
    let d = accepted(&c, 2., 2);
    assert!(insert(&d, 2., 1, 1_000_000).is_err());
    let mut bad = c;
    bad.weights[2] = f64::NAN;
    assert!(insert(&bad, 2., 1, 1_000_000).is_err());
}
#[test]
fn control_cap_refuses_before_work() {
    let mut c = line(1., vec![1., 1.]);
    c.control_points = (0..256).map(|i| vec![i as f64, 0.]).collect();
    c.weights = vec![1.; 256];
    c.knots = [
        vec![0., 0.],
        (1..255).map(|i| i as f64).collect(),
        vec![255., 255.],
    ]
    .concat();
    c.validate().unwrap();
    let r = insert(&c, 0.5, 1, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert_eq!(r.work_used, 0);
    assert_eq!(r.curve, c);
}
#[test]
fn unclamped_support_and_three_coordinates_are_retained() {
    let source = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2.],
        control_points: vec![vec![0., 2., -1.], vec![2., 4., 3.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    source.validate().unwrap();
    let c = accepted(&source, 0.5, 1);
    assert_eq!(c.knots, vec![-1., 0., 0.5, 1., 2.]);
    assert_eq!(c.control_points[1], vec![1., 3., 1.]);
}
