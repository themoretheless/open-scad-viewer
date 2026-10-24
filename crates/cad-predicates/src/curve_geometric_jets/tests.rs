use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn proof(a: &[[f64; 4]], b: &[[f64; 4]], order: usize, work: u64) -> BezierIdentityDecision {
    let source = SourceArena::authored(
        "original-curve-jets",
        1,
        a.iter()
            .chain(b)
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls = (0..a.len() + b.len())
        .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: work,
            ..Limits::default()
        },
        None,
    );
    rational_bezier_geometric_endpoint_identity(
        &mut ctx,
        &controls[..a.len()],
        &controls[a.len()..],
        order,
    )
    .unwrap()
}
#[test]
fn exact_curve_g2_owns_nonbinary_speed_and_rational_gauge() {
    let a = [[0., 0., 0., 1.], [-3., 0., 0., 1.], [-6., 3., 0., 1.]];
    let b = [[0., 0., 0., 2.], [1., 0., 0., 2.], [2., 1. / 3., 0., 2.]];
    // A binary approximation to 1/3 is not the intended exact curvature.
    assert_eq!(proof(&a, &b, 1, 1000000).outcome, BezierIdentity::Equal);
    assert_ne!(proof(&a, &b, 2, 1000000).outcome, BezierIdentity::Equal);
    let a = [[0., 0., 0., 1.], [-3., 0., 0., 1.], [-6., 9., 0., 1.]];
    let b = [[0., 0., 0., 2.], [1., 0., 0., 2.], [2., 1., 0., 2.]];
    let full = proof(&a, &b, 2, 1000000);
    assert_eq!(full.outcome, BezierIdentity::Equal, "{full:?}");
    assert!(full.work_used > 0);
    assert_ne!(
        proof(&a, &b, 2, full.work_used - 1).outcome,
        BezierIdentity::Equal
    );
    assert_ne!(proof(&a, &b, 2, 0).outcome, BezierIdentity::Equal);
}
#[test]
fn curve_jets_reject_kinks_cusps_gaps_and_curvature_jumps() {
    let a = [[1., 0., 0., 1.], [1., -1., 0., 0.5], [0., -1., 0., 1.]];
    let b = [[1., 0., 0., 1.], [1., 1., 0., 0.5], [0., 1., 0., 1.]];
    assert_eq!(proof(&a, &b, 2, 1000000).outcome, BezierIdentity::Equal);
    let mut b = b;
    b[1][3] = 0.75;
    assert_eq!(proof(&a, &b, 1, 1000000).outcome, BezierIdentity::Equal);
    assert_eq!(proof(&a, &b, 2, 1000000).outcome, BezierIdentity::Different);
    b[1][0] = 1_f64.next_up();
    assert_eq!(proof(&a, &b, 1, 1000000).outcome, BezierIdentity::Different);
    b[0][0] = 1_f64.next_up();
    assert_eq!(proof(&a, &b, 1, 1000000).outcome, BezierIdentity::Different);
    let c = [[0., 0., 0., 1.], [0., 0., 0., 1.], [1., 1., 0., 1.]];
    assert_ne!(proof(&c, &c, 2, 1000000).outcome, BezierIdentity::Equal);
    let a = [[0., 0., 0., 1.], [1., 0., 0., 1.]];
    assert_eq!(proof(&a, &a, 1, 1000000).outcome, BezierIdentity::Different);
}
