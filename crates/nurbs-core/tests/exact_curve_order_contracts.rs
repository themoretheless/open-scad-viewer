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
fn rational() -> Curve {
    bezier(vec![vec![0., 0.], vec![2., 0.]], vec![1., 2.], -2., 2.)
}
fn accepted(s: Curve, order: usize) -> Vec<Curve> {
    let before = s.clone();
    let r = d::curve_order(&s, order, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    let out = r.pieces.unwrap();
    assert_eq!(s, before);
    let spans = (s.degree..s.control_points.len())
        .filter(|&i| s.knots[i] < s.knots[i + 1])
        .collect::<Vec<_>>();
    assert_eq!(spans.len(), out.len());
    for (p, k) in out.iter().zip(spans) {
        p.validate().unwrap();
        assert!(!p.periodic);
        assert_eq!(p.domain(), [s.knots[k], s.knots[k + 1]]);
        assert_eq!(p.control_points[0].len(), s.control_points[0].len());
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-curve-order-oracle={}",
        value_codec::json!({"order":order,"source":s,"pieces":out})
    );
    out
}
fn close(actual: Vec<f64>, expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.iter().zip(expected) {
        assert!((a - b).abs() < 1e-10, "{a} != {b}");
    }
}
#[test]
fn cubic_fields_have_physical_second_and_third_scales() {
    let s = bezier(
        vec![
            vec![0., 0., 1.],
            vec![0., 0., 1.],
            vec![0., 2., 1.],
            vec![6., 6., 1.],
        ],
        vec![1.; 4],
        2.,
        6.,
    );
    for order in [2, 3] {
        let p = accepted(s.clone(), order);
        for u in [2., 3., 6.] {
            let t: f64 = (u - 2.) / 4.;
            let expected = if order == 2 {
                vec![2.25 * t, 0.75, 0.]
            } else {
                vec![0.5625, 0., 0.]
            };
            close(p[0].evaluate(u).unwrap().point, &expected);
        }
    }
}
#[test]
fn rational_second_and_third_derivatives_survive_degree_one() {
    for order in [2, 3] {
        let p = accepted(rational(), order);
        for u in [-2., -1., 2.] {
            let t: f64 = (u + 2.) / 4.;
            let x = if order == 2 {
                -0.5 / (1. + t).powi(3)
            } else {
                0.375 / (1. + t).powi(4)
            };
            close(p[0].evaluate(u).unwrap().point, &[x, 0.]);
        }
    }
}
#[test]
fn spatial_rational_fields_preserve_all_vector_components() {
    let s = bezier(
        vec![vec![0., 0., 0.], vec![2., -4., 6.]],
        vec![1., 2.],
        -2.,
        2.,
    );
    for order in [2, 3] {
        let p = accepted(s.clone(), order);
        let x = if order == 2 { -0.5 } else { 0.375 };
        close(p[0].evaluate(-2.).unwrap().point, &[x, -2. * x, 3. * x]);
    }
}
#[test]
fn order_one_preserves_previous_output_and_work_ledger() {
    let s = rational();
    let old = d::curve(&s, 1000000).unwrap();
    let new = d::curve_order(&s, 1, 1000000).unwrap();
    assert_eq!(old.work_used, new.work_used);
    assert_eq!(old.refusal, new.refusal);
    assert_eq!(old.pieces, new.pieces);
    assert_eq!(accepted(s, 1), old.pieces.unwrap());
}
#[test]
fn rational_nonuniform_spans_use_original_denominator_and_clock() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 3., 3., 3.],
        control_points: vec![vec![12., 0.], vec![8., 0.], vec![4., 0.], vec![3., 0.]],
        weights: vec![2., 3., 6., 8.],
        periodic: false,
    };
    for order in [2, 3] {
        let out = accepted(s.clone(), order);
        for p in out {
            let [a, b] = p.domain();
            for u in [a, (a + b) / 2., b] {
                let x = if order == 2 {
                    24. / (1. + u).powi(3)
                } else {
                    -72. / (1. + u).powi(4)
                };
                close(p.evaluate(u).unwrap().point, &[x, 0.]);
            }
        }
    }
}
#[test]
fn periodic_rational_spans_own_distinct_third_derivative_traces() {
    let s = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: true,
    };
    let second = accepted(s.clone(), 2);
    close(second[0].evaluate(1.).unwrap().point, &[-1., 0.]);
    close(second[1].evaluate(1.).unwrap().point, &[-1., 0.]);
    let third = accepted(s, 3);
    close(third[0].evaluate(1.).unwrap().point, &[1.5, 0.]);
    close(third[1].evaluate(1.).unwrap().point, &[-1.5, 0.]);
    close(third[0].evaluate(0.).unwrap().point, &[24., 0.]);
    close(third[1].evaluate(2.).unwrap().point, &[-24., 0.]);
}
#[test]
fn unclamped_quadratic_covers_every_fundamental_span() {
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
    for order in [2, 3] {
        let out = accepted(s.clone(), order);
        assert_eq!(out.len(), 3);
        for p in out {
            let [a, b] = p.domain();
            for u in [a, b] {
                close(
                    p.evaluate(u).unwrap().point,
                    &[0., if order == 2 { 2. } else { 0. }],
                );
            }
        }
    }
}
#[test]
fn repeated_knots_preserve_one_sided_acceleration_and_zero_third() {
    let s = Curve {
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
    let p = accepted(s.clone(), 2);
    close(p[0].evaluate(1.).unwrap().point, &[0., -4., 0.]);
    close(p[1].evaluate(1.).unwrap().point, &[2., 0., 0.]);
    for p in accepted(s, 3) {
        assert!(p.control_points.iter().flatten().all(|&x| x == 0.));
    }
}
#[test]
fn degree_twenty_five_linear_source_reduces_to_minimum_zero_field() {
    let s = bezier(
        (0..26).map(|i| vec![i as f64, 3.]).collect(),
        vec![1.; 26],
        0.,
        2.,
    );
    for order in [2, 3] {
        let p = accepted(s.clone(), order);
        assert_eq!(p[0].degree, 1);
        assert!(p[0].control_points.iter().flatten().all(|&x| x == 0.));
    }
}
#[test]
fn weighted_constant_source_cancels_to_zero_without_false_degree_growth() {
    let p = accepted(bezier(vec![vec![6., 8.]; 3], vec![1., 2., 4.], -2., 2.), 3);
    assert_eq!(p[0].degree, 1);
    assert_eq!(p[0].control_points, vec![vec![0., 0.]; 2]);
}
#[test]
fn reversing_parameter_preserves_even_and_reverses_odd_order() {
    let mut s = rational();
    s.control_points.reverse();
    s.weights.reverse();
    for order in [2, 3] {
        let p = accepted(s.clone(), order);
        for u in [-2., 0., 2.] {
            let t: f64 = (-u + 2.) / 4.;
            let x = if order == 2 {
                -0.5 / (1. + t).powi(3)
            } else {
                -0.375 / (1. + t).powi(4)
            };
            close(p[0].evaluate(u).unwrap().point, &[x, 0.]);
        }
    }
}
#[test]
fn late_storage_refusal_never_returns_a_partial_span_cover() {
    let s = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let first = bezier(
        s.control_points[..2].to_vec(),
        s.weights[..2].to_vec(),
        0.,
        1.,
    );
    assert!(d::curve_order(&first, 2, 1000000).unwrap().pieces.is_some());
    let before = s.clone();
    let r = d::curve_order(&s, 2, 1000000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.pieces.is_none());
    assert_eq!(s, before);
}
#[test]
fn final_proof_budget_boundary_is_atomic() {
    let s = rational();
    let r = d::curve_order(&s, 3, 1000000).unwrap();
    assert!(r.pieces.is_some());
    let r = d::curve_order(&s, 3, r.work_used - 1).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
}
#[test]
fn existing_degree_and_coordinate_caps_are_preserved() {
    let mut points = vec![vec![0., 0.]; 8];
    points[7][0] = 1.;
    let mut weights = vec![1.; 8];
    weights[7] = 2.;
    let r = d::curve_order(&bezier(points, weights, 0., 1.), 3, 1000000).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
    let r = d::curve_order(
        &bezier(
            vec![vec![0., 0.], vec![0., 0.], vec![1e9, 0.]],
            vec![1.; 3],
            0.,
            0.5,
        ),
        2,
        1000000,
    )
    .unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
}
#[test]
fn invalid_orders_budgets_and_sources_are_errors() {
    for order in [0, 4, usize::MAX] {
        assert!(d::curve_order(&rational(), order, 1000000).is_err());
    }
    for budget in [0, 1000001, u64::MAX] {
        assert!(d::curve_order(&rational(), 2, budget).is_err());
    }
    let mut s = rational();
    s.weights[0] = 0.;
    assert!(d::curve_order(&s, 2, 1000000).is_err());
    let mut s = rational();
    s.control_points[0][0] = f64::NAN;
    assert!(d::curve_order(&s, 3, 1000000).is_err());
}
