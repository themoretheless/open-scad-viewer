use cad_predicates::{
    AuthoredScalar, Limits, OriginalPlaneCommonRootOutcome as Outcome, PredicateContext,
    SourceArena, ToleranceContext, rational_bspline_plane_common_root,
};
fn query(
    x: &[f64],
    y: &[f64],
    weights: &[f64],
    knots: &[f64],
    degree: usize,
    span: usize,
    interval: [f64; 2],
    budget: u64,
) -> Outcome {
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
    let r = rational_bspline_plane_common_root(
        &mut ctx, &controls, &knots, degree, span, planes, interval,
    )
    .unwrap();
    assert!(r.work_used <= budget);
    r.outcome
}
#[test]
fn different_orders_have_the_same_nonbinary_root() {
    assert!(matches!(
        query(
            &[1., -1., 0., 4.],
            &[1., 0., -1., -2.],
            &[1.; 4],
            &[0., 0., 0., 0., 1., 1., 1., 1.],
            3,
            3,
            [0.333, 0.334],
            1_000_000
        ),
        Outcome::Common
    ));
}
#[test]
fn shared_irrational_root_is_not_matched_by_interval_overlap() {
    let knots = [0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    assert!(matches!(
        query(
            &[3., 3., 1., -3., 3.],
            &[-3., -3., -2., 0., 3.],
            &[1.; 5],
            &knots,
            4,
            4,
            [0.707, 0.708],
            1_000_000
        ),
        Outcome::Common
    ));
    // A nearby but distinct quadratic root occupies the SAME broad box.
    assert!(matches!(
        query(
            &[3., 3., 1., -3., 3.],
            &[
                -3.0009765625,
                -3.0009765625,
                -2.0009765625,
                -0.0009765625,
                2.9990234375
            ],
            &[1.; 5],
            &knots,
            4,
            4,
            [0.707, 0.708],
            1_000_000
        ),
        Outcome::Distinct
    ));
}
#[test]
fn original_positive_rational_weights_preserve_shared_root_proof() {
    assert!(matches!(
        query(
            &[1., -1., 0., 4.],
            &[1., 0., -1., -2.],
            &[1., 2., 4., 8.],
            &[0., 0., 0., 0., 1., 1., 1., 1.],
            3,
            3,
            [0.19, 0.21],
            1_000_000
        ),
        Outcome::Common
    ));
}
#[test]
fn unclamped_original_span_proves_nonproportional_common_root() {
    assert!(matches!(
        query(
            &[8., 0., -2., 56.],
            &[-4., -1., 2., 5.],
            &[1.; 4],
            &[-3., -2., -1., 0., 1., 2., 3., 4.],
            3,
            3,
            [0.333, 0.334],
            1_000_000
        ),
        Outcome::Common
    ));
}
#[test]
fn closed_interval_endpoints_and_singletons_are_counted_exactly() {
    for interval in [[0., 0.], [0., 0.1]] {
        assert!(matches!(
            query(
                &[0., 0.5, 1.],
                &[0., 0., 1.],
                &[1.; 3],
                &[0., 0., 0., 1., 1., 1.],
                2,
                2,
                interval,
                100000
            ),
            Outcome::Common
        ));
    }
    for interval in [[1., 1.], [0.9, 1.]] {
        assert!(matches!(
            query(
                &[-1., -0.5, 0.],
                &[1., 0., 0.],
                &[1.; 3],
                &[0., 0., 0., 1., 1., 1.],
                2,
                2,
                interval,
                100000
            ),
            Outcome::Common
        ));
    }
}
#[test]
fn nonunique_or_absent_first_root_and_exhaustion_never_admit() {
    let k = [0., 0., 0., 1., 1., 1.];
    for interval in [[0., 1.], [0.4, 0.6]] {
        assert!(matches!(
            query(
                &[1., -2., 1.],
                &[0., 0., 0.],
                &[1.; 3],
                &k,
                2,
                2,
                interval,
                100000
            ),
            Outcome::Indeterminate(_)
        ));
    }
    assert!(matches!(
        query(
            &[1., -1., 1.],
            &[0., 0., 0.],
            &[1.; 3],
            &k,
            2,
            2,
            [0.49, 0.51],
            0
        ),
        Outcome::Indeterminate(cad_predicates::Reason::ResourceLimit)
    ));
    assert!(matches!(
        query(
            &[0., 0., 0.],
            &[1., -1., 1.],
            &[1.; 3],
            &k,
            2,
            2,
            [0.49, 0.51],
            100000
        ),
        Outcome::Indeterminate(_)
    ));
}
#[test]
fn identically_zero_second_plane_is_common_only_at_unique_first_root() {
    assert!(matches!(
        query(
            &[1., -1., 1.],
            &[0., 0., 0.],
            &[1.; 3],
            &[0., 0., 0., 1., 1., 1.],
            2,
            2,
            [0.49, 0.51],
            100000
        ),
        Outcome::Common
    ));
}

#[test]
fn rounded_nearby_coefficients_never_create_a_false_common_root() {
    let r = query(
        &[3., 3., 1., -3., 3.],
        &[-3.0001, -3.0001, -2.0001, -0.0001, 2.9999],
        &[1.; 5],
        &[0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        4,
        4,
        [0.707, 0.708],
        1_000_000,
    );
    // Independent binary64 rounding can raise the exact degree and exhaust
    // expansion precision. That is an explicit refusal, never root aliasing.
    assert!(!matches!(r, Outcome::Common), "{r:?}");
}
