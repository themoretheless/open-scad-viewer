use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn audit_mode(
    points: &[[f64; 4]],
    knots: &[f64],
    degree: usize,
    order: usize,
    work: u64,
    endpoints: bool,
) -> BezierIdentityDecision {
    let mut values = points.iter().flatten().copied().collect::<Vec<_>>();
    let start = values.len();
    values.extend_from_slice(knots);
    let arena = SourceArena::authored(
        "original-internal-jets",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls = (0..points.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect::<Vec<_>>();
    let knots = (0..knots.len())
        .map(|i| arena.leaf(start + i).unwrap())
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: work,
            ..Limits::default()
        },
        None,
    );
    if endpoints {
        rational_bspline_cartesian_endpoint_jet_identity(
            &mut ctx, &controls, &knots, degree, order,
        )
        .unwrap()
    } else {
        rational_bspline_internal_jet_identity(&mut ctx, &controls, &knots, degree, order)
            .unwrap()
    }
}
fn audit(
    points: &[[f64; 4]],
    knots: &[f64],
    degree: usize,
    order: usize,
    work: u64,
) -> BezierIdentityDecision {
    audit_mode(points, knots, degree, order, work, false)
}
#[test]
fn cartesian_endpoints_accept_varying_homogeneous_scale_and_nonclamped_domains() {
    let p = [
        [0., 0., 1., 1.],
        [0.125, 0., 1., 0.5],
        [0.046875, 0.125, 1., 1.],
        [-0.28125, 0.125, 1., 1.],
        [-0.03125, 0., 1., 2.],
        [0., 0., 1., 1.],
    ];
    let knots = [0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.];
    let proof = audit_mode(&p, &knots, 5, 2, 1000000, true);
    assert_eq!(proof.outcome, BezierIdentity::Equal);
    assert_ne!(
        audit_mode(&p, &knots, 5, 2, proof.work_used - 1, true).outcome,
        BezierIdentity::Equal
    );
    let mut bad = p;
    bad[3][0] = bad[3][0].next_up();
    assert_eq!(
        audit_mode(&bad, &knots, 5, 1, 1000000, true).outcome,
        BezierIdentity::Equal
    );
    assert_eq!(
        audit_mode(&bad, &knots, 5, 2, 1000000, true).outcome,
        BezierIdentity::Different
    );
    let c = [
        [1., 2., 3., 1.],
        [1., 2., 3., 2.],
        [1., 2., 3., 3.],
        [1., 2., 3., 4.],
    ];
    let u = [-2., -1., 0., 0.5, 1., 2., 3.];
    assert_eq!(
        audit_mode(&c, &u, 2, 4, 1000000, true).outcome,
        BezierIdentity::Equal
    );
    let mut bad = c;
    bad[0][0] = bad[0][0].next_up();
    assert_eq!(
        audit_mode(&bad, &u, 2, 1, 1000000, true).outcome,
        BezierIdentity::Different
    );
    assert_ne!(
        audit_mode(&c, &u, 2, 2, 0, true).outcome,
        BezierIdentity::Equal
    );
}
#[test]
fn original_simple_knot_rational_c4_requires_every_exact_cartesian_jet() {
    // Original rational quadratic after exact insertion of one midpoint.
    let p = [
        [0., 0., 0., 1.],
        [0., 0., 0.375, 2.],
        [0., 0.25, 0.625, 2.],
        [0., 1., 1., 1.],
    ];
    let knots = [0., 0., 0., 0.5, 1., 1., 1.];
    let proof = audit(&p, &knots, 2, 4, 1000000);
    assert_eq!(proof.outcome, BezierIdentity::Equal);
    assert_ne!(
        audit(&p, &knots, 2, 4, proof.work_used - 1).outcome,
        BezierIdentity::Equal
    );
    assert_ne!(audit(&p, &knots, 2, 4, 0).outcome, BezierIdentity::Equal);
    let mut bad = p;
    bad[2][1] = bad[2][1].next_up();
    assert_eq!(
        audit(&bad, &knots, 2, 1, 1000000).outcome,
        BezierIdentity::Equal
    );
    assert_eq!(
        audit(&bad, &knots, 2, 2, 1000000).outcome,
        BezierIdentity::Different
    );
    bad = p;
    bad[1][3] = 0.;
    assert_ne!(
        audit(&bad, &knots, 2, 4, 1000000).outcome,
        BezierIdentity::Equal
    );
    let changed = [0., 0., 0., 0.25, 1., 1., 1.];
    assert_eq!(
        audit(&p, &changed, 2, 2, 1000000).outcome,
        BezierIdentity::Different
    );
}
#[test]
fn cartesian_identity_does_not_require_constant_homogeneous_scale() {
    let p = [
        [1., 2., 3., 1.],
        [1., 2., 3., 2.],
        [1., 2., 3., 3.],
        [1., 2., 3., 4.],
    ];
    let knots = [0., 0., 0., 0.5, 1., 1., 1.];
    assert_eq!(
        audit(&p, &knots, 2, 4, 1000000).outcome,
        BezierIdentity::Equal
    );
    // A complete basis break with unequal weight derivatives still gives
    // the same constant Cartesian field, not matching homogeneous jets.
    let full = [
        [1., 2., 3., 1.],
        [1., 2., 3., 2.],
        [1., 2., 3., 3.],
        [1., 2., 3., 4.],
        [1., 2., 3., 1.],
        [1., 2., 3., 2.],
    ];
    let breaks = [0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.];
    assert_eq!(
        audit(&full, &breaks, 2, 4, 1000000).outcome,
        BezierIdentity::Equal
    );
    let mut bad = full;
    bad[3][0] = bad[3][0].next_up();
    assert_eq!(
        audit(&bad, &breaks, 2, 1, 1000000).outcome,
        BezierIdentity::Different
    );
}
#[test]
fn mixed_multiplicities_cover_all_original_joins() {
    let p = [
        [0., 0., 0., 1.],
        [0., 0., 0.125, 1.],
        [0., 0.125, 0.375, 1.],
        [0., 0.25, 0.5, 1.],
        [0., 0.5, 0.75, 1.],
        [0., 1., 1., 1.],
    ];
    let knots = [0., 0., 0., 0.25, 0.5, 0.5, 1., 1., 1.];
    let proof = audit(&p, &knots, 2, 4, 1000000);
    assert_eq!(proof.outcome, BezierIdentity::Equal);
    assert_ne!(
        audit(&p, &knots, 2, 4, proof.work_used - 1).outcome,
        BezierIdentity::Equal
    );
    let mut bad = p;
    bad[3][1] = bad[3][1].next_up();
    assert_eq!(
        audit(&bad, &knots, 2, 1, 1000000).outcome,
        BezierIdentity::Different
    );
}
#[test]
fn fourth_jet_is_not_inferred_from_third_jet() {
    let p = [
        [0., 0., 0., 1.],
        [0., 0.125, 0., 1.],
        [0.0625, 0.25, 0.0625, 1.],
        [0.15625, 0.375, 0.125, 1.],
        [0.28125, 0.5, 0.21875, 1.],
        [0.40625, 0.625, 0.3125, 1.],
        [0.5625, 0.75, 0.4375, 1.],
        [0.75, 0.875, 0.625, 1.],
        [1., 1., 1., 1.],
    ];
    let knots = [0., 0., 0., 0., 0., 0.5, 0.5, 0.5, 0.5, 1., 1., 1., 1., 1.];
    assert_eq!(
        audit(&p, &knots, 4, 4, 1000000).outcome,
        BezierIdentity::Equal
    );
    let mut bad = p;
    bad[8][0] = bad[8][0].next_up();
    assert_eq!(
        audit(&bad, &knots, 4, 3, 1000000).outcome,
        BezierIdentity::Equal
    );
    assert_eq!(
        audit(&bad, &knots, 4, 4, 1000000).outcome,
        BezierIdentity::Different
    );
}
