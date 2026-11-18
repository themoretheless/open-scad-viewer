use nurbs_core::{
    curve::Curve,
    editing::{
        bounded_differential::{self as b, Field, Refusal},
        exact_derivative,
    },
};
fn bezier(points: Vec<Vec<f64>>, weights: Vec<f64>, a: f64, z: f64) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![a; degree + 1], vec![z; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn parabola() -> Curve {
    bezier(
        vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        vec![1.; 3],
        0.,
        1.,
    )
}
#[cfg(feature = "codec")]
fn field_name(f: Field) -> String {
    match f {
        Field::Derivative(n) => format!("derivative{n}"),
        Field::SquaredSpeed => "squared_speed".into(),
        Field::Binormal => "binormal".into(),
        Field::SquaredCurvature => "squared_curvature".into(),
    }
}
fn accepted(source: &Curve, field: Field, tolerance: f64) -> b::Report {
    let before = source.clone();
    let r = b::curve(source, field, tolerance, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?}, work={}", r.refusal, r.work_used);
    assert_eq!(*source, before);
    let spans = (source.degree..source.control_points.len())
        .filter(|&i| source.knots[i] < source.knots[i + 1])
        .collect::<Vec<_>>();
    let pieces = r.pieces.as_ref().unwrap();
    let bounds = r.error_upper.as_ref().unwrap();
    assert_eq!(pieces.len(), spans.len());
    assert_eq!(pieces.len(), bounds.len());
    for ((piece, &bound), span) in pieces.iter().zip(bounds).zip(spans) {
        piece.validate().unwrap();
        assert!(!piece.periodic);
        assert_eq!(piece.domain(), [source.knots[span], source.knots[span + 1]]);
        assert_eq!(
            piece.control_points[0].len(),
            source.control_points[0].len()
        );
        assert!(bound.is_finite() && bound >= 0. && bound <= tolerance);
    }
    #[cfg(feature = "codec")]
    println!(
        "bounded-differential-oracle={}",
        value_codec::json!({"source":source,"field":field_name(field),"tolerance":tolerance,"pieces":pieces,"bounds":bounds})
    );
    r
}
#[test]
fn nonconstant_curvature_field_admits_previously_inexact_binary_storage() {
    let s = parabola();
    assert_eq!(
        exact_derivative::squared_curvature(&s, 1_000_000)
            .unwrap()
            .refusal,
        Some(exact_derivative::Refusal::NotRepresentable)
    );
    let r = accepted(&s, Field::SquaredCurvature, 1e-10);
    assert!(r.error_upper.unwrap()[0] > 0.);
    let p = &r.pieces.unwrap()[0];
    for i in 0..=32 {
        let u = i as f64 / 32.;
        let expected = 4. / (1. + 4. * u * u).powi(3);
        assert!((p.evaluate(u).unwrap().point[0] - expected).abs() < 1e-10);
    }
}
#[test]
fn spatial_rational_fields_cover_all_supported_derivative_orders() {
    let s = bezier(
        vec![vec![0., 0., 0.], vec![2., 0., 1.], vec![2., 2., 0.]],
        vec![1., 2., 1.],
        -2.,
        2.,
    );
    for f in [
        Field::Derivative(1),
        Field::Derivative(2),
        Field::Derivative(3),
        Field::SquaredSpeed,
        Field::Binormal,
    ] {
        accepted(&s, f, 1e-8);
    }
}
#[test]
fn exact_storage_returns_zero_bound_even_at_subnormal_tolerance() {
    let s = bezier(vec![vec![0., 0.], vec![2., 4.]], vec![1., 1.], 0., 1.);
    let r = accepted(&s, Field::Derivative(1), f64::from_bits(1));
    assert_eq!(r.error_upper.unwrap(), vec![0.]);
}
#[test]
fn rational_circle_remains_exact_with_zero_curvature_error() {
    let s = bezier(
        vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        vec![1., 1., 2.],
        -2.,
        2.,
    );
    let r = accepted(&s, Field::SquaredCurvature, 1e-20);
    assert_eq!(r.error_upper.unwrap(), vec![0.]);
}
#[test]
fn reversed_and_stretched_original_clocks_are_kept() {
    let mut s = parabola();
    s.control_points.reverse();
    s.knots = vec![-2., -2., -2., 6., 6., 6.];
    accepted(&s, Field::SquaredCurvature, 1e-10);
    accepted(&s, Field::Derivative(1), 1e-10);
}
#[test]
fn nonuniform_repeated_spans_keep_independent_traces() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 3., 3., 3.],
        control_points: vec![
            vec![0., 0.],
            vec![0.5, 0.],
            vec![1., 1.],
            vec![1.5, 1.],
            vec![2., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let r = accepted(&s, Field::SquaredCurvature, 1e-10);
    assert_eq!(r.pieces.unwrap().len(), 2);
}
#[test]
fn stationary_source_is_not_regularized_by_approximation() {
    let s = bezier(
        vec![vec![0.25, 0.], vec![-0.25, 0.], vec![0.25, 0.]],
        vec![1.; 3],
        0.,
        1.,
    );
    let r = b::curve(&s, Field::SquaredCurvature, 1., 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn too_small_error_refuses_the_entire_result() {
    let r = b::curve(&parabola(), Field::SquaredCurvature, 1e-30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ToleranceExceeded));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn work_exhaustion_including_final_upper_rounding_is_atomic() {
    let s = parabola();
    let full = b::curve(&s, Field::SquaredCurvature, 1e-10, 1_000_000).unwrap();
    assert!(full.pieces.is_some());
    let r = b::curve(&s, Field::SquaredCurvature, 1e-10, full.work_used - 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn invalid_fields_tolerances_and_budgets_are_errors() {
    let s = parabola();
    for t in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(b::curve(&s, Field::Derivative(1), t, 1_000_000).is_err());
    }
    for work in [0, 1_000_001] {
        assert!(b::curve(&s, Field::Derivative(1), 1e-8, work).is_err());
    }
    for f in [Field::Derivative(0), Field::Derivative(4), Field::Binormal] {
        assert!(b::curve(&s, f, 1e-8, 1_000_000).is_err());
    }
    let mut bad = s;
    bad.weights[0] = 0.;
    assert!(b::curve(&bad, Field::Derivative(1), 1e-8, 1_000_000).is_err());
}
#[test]
fn unclamped_source_has_complete_three_span_field_cover() {
    let s = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![-0.5, 0.],
            vec![0.5, 0.],
            vec![1.5, 2.],
            vec![2.5, 6.],
            vec![3.5, 12.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    assert_eq!(
        accepted(&s, Field::SquaredCurvature, 1e-10)
            .pieces
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn periodic_encoding_keeps_independent_original_clock_traces() {
    let s = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: true,
    };
    assert_eq!(
        accepted(&s, Field::Derivative(1), 1e-10)
            .pieces
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn a_late_unproved_source_discards_every_earlier_piece_and_bound() {
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
    let r = b::curve(&s, Field::SquaredCurvature, 1e-8, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn tolerance_cannot_relax_output_geometry_limits() {
    let s = bezier(vec![vec![0., 0.], vec![1e9, 0.]], vec![1., 1.], 0., 1e-9);
    let r = b::curve(&s, Field::Derivative(1), 1e30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn rounded_weights_keep_the_existing_conditioning_limits() {
    let s = bezier(vec![vec![0., 0.], vec![1e-18, 0.]], vec![1., 1e6], 0., 1.);
    let r = b::curve(&s, Field::Derivative(3), 1e-6, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
#[test]
fn positive_subnormal_error_is_not_reported_as_zero() {
    let epsilon = f64::from_bits((1023u64 - 538) << 52);
    let s = bezier(
        vec![vec![0., 0.], vec![0., 0.], vec![0., 0.], vec![epsilon, 0.]],
        vec![1.; 4],
        0.,
        1.,
    );
    assert_eq!(
        exact_derivative::squared_speed(&s, 1_000_000)
            .unwrap()
            .refusal,
        Some(exact_derivative::Refusal::NotRepresentable)
    );
    let r = accepted(&s, Field::SquaredSpeed, f64::from_bits(3));
    assert!(r.error_upper.unwrap()[0] > 0.);
}
