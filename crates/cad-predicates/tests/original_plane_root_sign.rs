use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Reason, Sign, SourceArena, ToleranceContext,
    rational_bspline_plane_root_sign,
};
/// Planar rational B-spline curve given as separate coordinate arrays.
struct Curve<'a> {
    x: &'a [f64],
    y: &'a [f64],
    weights: &'a [f64],
    knots: &'a [f64],
    degree: usize,
    span: usize,
}
fn query(curve: Curve<'_>, interval: [f64; 2], budget: u64) -> Result<Sign, Reason> {
    let Curve {
        x,
        y,
        weights,
        knots,
        degree,
        span,
    } = curve;
    let mut v = Vec::new();
    for i in 0..x.len() {
        v.extend([x[i], y[i], 0., weights[i]]);
    }
    let k = v.len();
    v.extend(knots);
    let p = v.len();
    v.extend([1., 0., 0., 0., 0., 1., 0., 0.]);
    let arena = SourceArena::authored(
        "original-plane-common-root-test",
        1,
        v.into_iter()
            .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls: Vec<_> = (0..x.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let knots: Vec<_> = (0..knots.len())
        .map(|i| arena.leaf(k + i).unwrap())
        .collect();
    let planes =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(p + i * 4 + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: budget,
            ..Limits::default()
        },
        None,
    );
    let r = rational_bspline_plane_root_sign(
        &mut ctx, &controls, &knots, degree, span, planes, interval,
    )
    .unwrap();
    assert!(r.work_used <= budget);
    r.sign
}
#[test]
fn nonbinary_root_sign_resolves_nearby_distinct_edge_roots() {
    for delta in [-2f64.powi(-40), 2f64.powi(-40)] {
        let y = [1., 0., -1., -2.].map(|v| v + delta);
        assert_eq!(
            query(
                Curve {
                    x: &[1., -1., 0., 4.],
                    y: &y,
                    weights: &[1.; 4],
                    knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                    degree: 3,
                    span: 3
                },
                [0.333, 0.334],
                1_000_000
            )
            .unwrap(),
            if delta < 0. {
                Sign::Negative
            } else {
                Sign::Positive
            }
        );
    }
}
#[test]
fn irrational_root_sign_has_no_numeric_root_separation_requirement() {
    let k = [0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    for delta in [-2f64.powi(-40), 0., 2f64.powi(-40)] {
        let y = [-3., -3., -2., 0., 3.].map(|v| v + delta);
        assert_eq!(
            query(
                Curve {
                    x: &[3., 3., 1., -3., 3.],
                    y: &y,
                    weights: &[1.; 5],
                    knots: &k,
                    degree: 4,
                    span: 4
                },
                [0.707, 0.708],
                1_000_000
            )
            .unwrap(),
            if delta < 0. {
                Sign::Negative
            } else if delta > 0. {
                Sign::Positive
            } else {
                Sign::Zero
            }
        );
    }
}
#[test]
fn repeated_first_root_and_higher_degree_query_keep_signed_tarski_result() {
    // At t=1/3, X=(1-3t)^2 and Y=t^3 is strictly positive.
    for y in [[0., 0., 0., 1.], [0., 0., 0., -1.]] {
        assert_eq!(
            query(
                Curve {
                    x: &[1., -1., 0., 4.],
                    y: &y,
                    weights: &[1.; 4],
                    knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                    degree: 3,
                    span: 3
                },
                [0.3, 0.4],
                1_000_000
            )
            .unwrap(),
            if y[3] > 0. {
                Sign::Positive
            } else {
                Sign::Negative
            }
        );
    }
}
#[test]
fn original_rational_weights_and_unclamped_knots_preserve_sign() {
    assert_eq!(
        query(
            Curve {
                x: &[1., -1., 0., 4.],
                y: &[1., 1., 1., 1.],
                weights: &[1., 2., 4., 8.],
                knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                degree: 3,
                span: 3
            },
            [0.19, 0.21],
            1_000_000
        )
        .unwrap(),
        Sign::Positive
    );
    assert_eq!(
        query(
            Curve {
                x: &[8., 0., -2., 56.],
                y: &[-3., 0., 3., 6.],
                weights: &[1.; 4],
                knots: &[-3., -2., -1., 0., 1., 2., 3., 4.],
                degree: 3,
                span: 3
            },
            [0.333, 0.334],
            1_000_000
        )
        .unwrap(),
        Sign::Positive
    );
}
#[test]
fn endpoints_and_singleton_boxes_use_exact_source_sign() {
    for interval in [[0., 0.], [0., 0.5]] {
        assert_eq!(
            query(
                Curve {
                    x: &[0., 0.5, 1.],
                    y: &[-1., 0., 1.],
                    weights: &[1.; 3],
                    knots: &[0., 0., 0., 1., 1., 1.],
                    degree: 2,
                    span: 2
                },
                interval,
                100000
            )
            .unwrap(),
            Sign::Negative
        );
    }
    for interval in [[1., 1.], [0.5, 1.]] {
        assert_eq!(
            query(
                Curve {
                    x: &[-1., -0.5, 0.],
                    y: &[-1., 0., 1.],
                    weights: &[1.; 3],
                    knots: &[0., 0., 0., 1., 1., 1.],
                    degree: 2,
                    span: 2
                },
                interval,
                100000
            )
            .unwrap(),
            Sign::Positive
        );
    }
}
#[test]
fn zero_query_and_budget_nonunique_or_absent_root_fail_closed() {
    let k = [0., 0., 0., 1., 1., 1.];
    assert_eq!(
        query(
            Curve {
                x: &[1., -1., 1.],
                y: &[0.; 3],
                weights: &[1.; 3],
                knots: &k,
                degree: 2,
                span: 2
            },
            [0.49, 0.51],
            100000
        )
        .unwrap(),
        Sign::Zero
    );
    assert!(matches!(
        query(
            Curve {
                x: &[1., -1., 1.],
                y: &[1.; 3],
                weights: &[1.; 3],
                knots: &k,
                degree: 2,
                span: 2
            },
            [0.49, 0.51],
            0
        ),
        Err(Reason::ResourceLimit)
    ));
    for interval in [[0., 1.], [0.4, 0.6]] {
        assert!(
            query(
                Curve {
                    x: &[1., -2., 1.],
                    y: &[1.; 3],
                    weights: &[1.; 3],
                    knots: &k,
                    degree: 2,
                    span: 2
                },
                interval,
                100000
            )
            .is_err()
        );
    }
}

#[test]
fn subnormal_query_has_exact_sign_below_binary64_evaluation_quantum() {
    for tiny in [-f64::from_bits(1), f64::from_bits(1)] {
        assert_eq!(
            query(
                Curve {
                    x: &[1., -1., 0., 4.],
                    y: &[tiny, 0., 0., 0.],
                    weights: &[1.; 4],
                    knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                    degree: 3,
                    span: 3
                },
                [0.333, 0.334],
                1_000_000
            )
            .unwrap(),
            if tiny > 0. {
                Sign::Positive
            } else {
                Sign::Negative
            }
        );
    }
}
#[test]
fn tarski_sign_matches_independent_integer_evaluation_at_one_third() {
    for i in 0..64i32 {
        let y = [
            i % 9 - 4,
            (i * 7 + 3) % 9 - 4,
            (i * 5 + 1) % 9 - 4,
            (i * 2 + 8) % 9 - 4,
        ];
        // At t=1/3, cubic Bernstein weights are [8,12,6,1]/27.
        let numerator = 8 * y[0] + 12 * y[1] + 6 * y[2] + y[3];
        let expected = if numerator < 0 {
            Sign::Negative
        } else if numerator > 0 {
            Sign::Positive
        } else {
            Sign::Zero
        };
        for scale in [-1., 1.] {
            let x = [1., -1., 0., 4.].map(|v| v * scale);
            assert_eq!(
                query(
                    Curve {
                        x: &x,
                        y: &y.map(f64::from),
                        weights: &[1.; 4],
                        knots: &[0., 0., 0., 0., 1., 1., 1., 1.],
                        degree: 3,
                        span: 3
                    },
                    [0.3, 0.4],
                    1_000_000
                )
                .unwrap(),
                expected,
                "query {y:?}, first scale {scale}"
            );
        }
    }
}
