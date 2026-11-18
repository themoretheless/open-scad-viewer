use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{self as d, Refusal},
};
fn bezier(points: Vec<Vec<f64>>, weights: Vec<f64>, a: f64, b: f64) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![a; degree + 1], vec![b; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn circle(dim: usize, r: f64) -> Curve {
    let mut points = vec![vec![r, 0.], vec![r, r], vec![0., r]];
    if dim == 3 {
        for p in &mut points {
            p.push(2.);
        }
    }
    bezier(points, vec![1., 1., 2.], -2., 2.)
}
fn accepted(s: &Curve, expected: f64) -> Vec<Curve> {
    let before = s.clone();
    let r = d::squared_curvature(s, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    let out = r.pieces.unwrap();
    assert_eq!(*s, before);
    let spans = (s.degree..s.control_points.len())
        .filter(|&i| s.knots[i] < s.knots[i + 1])
        .collect::<Vec<_>>();
    assert_eq!(out.len(), spans.len());
    for (p, k) in out.iter().zip(spans) {
        p.validate().unwrap();
        assert_eq!(p.domain(), [s.knots[k], s.knots[k + 1]]);
        for i in 0..=8 {
            let u = p.domain()[0] + (p.domain()[1] - p.domain()[0]) * i as f64 / 8.;
            let v = p.evaluate(u).unwrap().point;
            assert!((v[0] - expected).abs() < 1e-10);
            assert!(v[1..].iter().all(|x| *x == 0.));
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-squared-curvature-oracle={}",
        value_codec::json!({"source":s,"pieces":out})
    );
    out
}
#[test]
fn rational_circle_and_spatial_circle_have_exact_constant_curvature() {
    for dim in [2, 3] {
        for radius in [1., 2., 4.] {
            accepted(&circle(dim, radius), 1. / (radius * radius));
        }
    }
}
#[test]
fn regular_straight_rational_curve_has_zero_curvature() {
    accepted(
        &bezier(vec![vec![0., 0.], vec![2., 4.]], vec![1., 2.], 0., 1.),
        0.,
    );
}
#[test]
fn parameter_reversal_and_stretch_preserve_curvature() {
    let s = circle(3, 2.);
    let mut reverse = s.clone();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    accepted(&reverse, 0.25);
    let mut stretched = s;
    stretched.knots = stretched.knots.iter().map(|x| 8. * x + 3.).collect();
    accepted(&stretched, 0.25);
}
#[test]
fn translated_and_rotated_circle_retains_scalar() {
    let mut s = circle(3, 2.);
    for p in &mut s.control_points {
        let x = p[0];
        p[0] = -p[1] + 4.;
        p[1] = x - 8.;
        p[2] += 0.125;
    }
    accepted(&s, 0.25);
}
#[test]
fn nonuniform_clock_and_repeated_join_cover_both_arcs() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 3., 3., 3.],
        control_points: vec![
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![-1., 1.],
            vec![-1., 0.],
        ],
        weights: vec![1., 1., 2., 2., 4.],
        periodic: false,
    };
    assert_eq!(accepted(&s, 1.).len(), 2);
}
#[test]
fn endpoint_stationarity_cannot_be_cancelled_into_zero_curvature() {
    let s = bezier(
        vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        vec![1.; 3],
        0.,
        1.,
    );
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none());
}
#[test]
fn interior_stationarity_is_not_regularized() {
    let s = bezier(
        vec![vec![0.25, 0.], vec![-0.25, 0.], vec![0.25, 0.]],
        vec![1.; 3],
        0.,
        1.,
    );
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none());
}
#[test]
fn unresolved_bernstein_regularity_is_a_refusal_not_a_singularity_claim() {
    let s = bezier(
        vec![vec![0., 0.], vec![-0.25, 0.125], vec![0., 0.25]],
        vec![1.; 3],
        0.,
        1.,
    );
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none());
}
#[test]
fn work_limit_and_invalid_input_are_atomic() {
    let s = circle(3, 1.);
    let full = d::squared_curvature(&s, 1_000_000).unwrap();
    assert!(full.pieces.is_some());
    let r = d::squared_curvature(&s, full.work_used - 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.pieces.is_none());
    assert!(d::squared_curvature(&s, 0).is_err());
    assert!(d::squared_curvature(&s, 1_000_001).is_err());
    let mut bad = s;
    bad.weights[0] = f64::NAN;
    assert!(d::squared_curvature(&bad, 1_000_000).is_err());
}
#[test]
fn high_degree_regularity_proof_refuses_before_bernstein_overflow() {
    let s = bezier(
        (0..26).map(|i| vec![i as f64, (i % 2) as f64]).collect(),
        vec![1.; 26],
        0.,
        1.,
    );
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
    assert!(r.pieces.is_none());
}
#[test]
fn late_stationary_span_discards_earlier_regular_zero_curvature() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![-2., 0.],
            vec![-1., 0.],
            vec![0., 0.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none());
}
#[test]
fn exact_field_is_not_rounded_to_fit_binary_storage() {
    let s = bezier(
        vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        vec![1.; 3],
        0.,
        1.,
    );
    let r = d::squared_curvature(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.pieces.is_none());
}
