use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
    rational_bspline_restriction_identity,
};
fn run(
    p: usize,
    a: &[[f64; 4]],
    ad: [f64; 2],
    b: &[[f64; 4]],
    bd: [f64; 2],
    tr: [f64; 3],
    limit: u64,
) -> cad_predicates::BezierIdentityDecision {
    let mut values = a.iter().chain(b).flatten().copied().collect::<Vec<_>>();
    let ak = values.len();
    values.extend([vec![ad[0]; p + 1], vec![ad[1]; p + 1]].concat());
    let bk = values.len();
    values.extend([vec![bd[0]; p + 1], vec![bd[1]; p + 1]].concat());
    let shift = values.len();
    values.extend(tr);
    let arena = SourceArena::authored(
        "whole-clamped-original-nets",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: limit,
            ..Limits::default()
        },
        None,
    );
    let rows = |offset: usize, n: usize| {
        (0..n)
            .map(|i| std::array::from_fn(|j| arena.leaf(offset + 4 * i + j).unwrap()))
            .collect::<Vec<_>>()
    };
    let knots = |offset: usize| {
        (0..2 * (p + 1))
            .map(|i| arena.leaf(offset + i).unwrap())
            .collect::<Vec<_>>()
    };
    rational_bspline_restriction_identity(
        &mut ctx,
        p,
        &rows(0, a.len()),
        &knots(ak),
        p,
        &rows(4 * a.len(), b.len()),
        &knots(bk),
        std::array::from_fn(|i| arena.leaf(shift + i).unwrap()),
    )
    .unwrap()
}
#[test]
fn complete_nonconstant_degree25_nets_keep_the_original_budget() {
    let b = (0..=25)
        .map(|i| {
            [
                i as f64 / 8.,
                (i * i % 7) as f64 / 8.,
                (i % 3) as f64 / 4.,
                (i + 1) as f64,
            ]
        })
        .collect::<Vec<_>>();
    let a = b
        .iter()
        .map(|p| [p[0] + 4., p[1] - 2., p[2] + 1., p[3]])
        .collect::<Vec<_>>();
    let proof = run(25, &a, [-2., 6.], &b, [-2., 6.], [4., -2., 1.], 1_000_000);
    assert_eq!(proof.outcome, BezierIdentity::Equal);
    assert!(proof.work_used < 1_000_000);
    let mut changed = a.clone();
    changed[25][0] = f64::from_bits(changed[25][0].to_bits() + 1);
    assert_eq!(
        run(
            25,
            &changed,
            [-2., 6.],
            &b,
            [-2., 6.],
            [4., -2., 1.],
            1_000_000
        )
        .outcome,
        BezierIdentity::Different
    );
    assert_eq!(
        run(25, &a, [-2., 6.], &b, [-2., 6.], [4., -2., 1.], 1).outcome,
        BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit)
    );
}
#[test]
fn cropped_ranges_still_require_restriction_not_original_whole_controls() {
    let a = [[0., 0., 0., 1.], [0.5, 0., 0., 1.], [1., 0., 0., 1.]];
    let b = [[0.125, 0., 0., 1.], [0.5, 0., 0., 1.], [0.875, 0., 0., 1.]];
    assert_eq!(
        run(2, &a, [-2., 6.], &b, [-1., 5.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Equal
    );
    assert_eq!(
        run(2, &a, [-2., 6.], &a, [-1., 5.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Different
    );
    assert_ne!(
        run(2, &a, [-2., 6.], &a, [-3., 5.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Equal
    );
}
#[test]
fn invalid_tail_weight_does_not_hide_behind_constant_control_coordinates() {
    let good = [[3., 4., 5., 1.]; 3];
    let mut invalid = good;
    invalid[2][3] = 0.;
    assert_ne!(
        run(2, &invalid, [-2., 6.], &good, [-2., 6.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Equal
    );
}
#[test]
fn whole_net_gauge_normalization_keeps_large_finite_homogeneous_products() {
    let a = [
        [1e150, 2e150, 0., 1e150],
        [2e150, 1e150, 0., 1e150],
        [3e150, 2e150, 0., 1e150],
    ];
    assert_eq!(
        run(2, &a, [-2., 6.], &a, [-2., 6.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Equal
    );
    let mut changed = a;
    changed[2][0] = f64::from_bits(changed[2][0].to_bits() + 1);
    assert_eq!(
        run(2, &a, [-2., 6.], &changed, [-2., 6.], [0.; 3], 1_000_000).outcome,
        BezierIdentity::Different
    );
}
