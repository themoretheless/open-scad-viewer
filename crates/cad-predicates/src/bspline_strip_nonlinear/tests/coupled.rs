use super::inverse::{inverse_features, solve_coupled};
use super::*;
use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
#[test]
fn proper_quintic_needs_and_owns_cubic_inverse_features() {
    let source = SourceArena::authored(
        "quintic-inverse-fixture",
        1,
        vec![AuthoredScalar::Binary64Bits(0)],
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let poly = |v: &[f64]| {
        v.iter()
            .map(|&v| Q::scalar(Expansion::scalar(v)))
            .collect::<P>()
    };
    let h = [
        poly(&[0., 1., 1., 2., 1., 1.]),
        poly(&[0., 1., 3., 1., 2., 3.]),
        poly(&[1.]),
        poly(&[1.]),
    ];
    let second = inverse_features(&h, 2, &mut ctx).unwrap();
    assert!(solve_coupled(&second, &mut ctx).unwrap().is_none());
    let (l, m, degree, _) = coupled_coordinate(&h, &mut ctx).unwrap().unwrap();
    assert_eq!(degree, 3);
    let n = poly(&[0., 1.]);
    let d = poly(&[2., -1.]);
    let b: H = std::array::from_fn(|k| compose(&h[k], &n, &d, 5, &mut ctx).unwrap());
    let (nn, dd) = inverse_forms(&b, &l, &m, degree, &mut ctx).unwrap();
    let (nn, dd) = reduce_ratio(nn, dd, &mut ctx).unwrap();
    assert!(zero(
        &sub(
            &mul(&nn, &d, &mut ctx).unwrap(),
            &mul(&dd, &n, &mut ctx).unwrap(),
            &mut ctx
        )
        .unwrap()
    ));
}
#[test]
fn exact_map_cancellation_preserves_identity_and_keeps_genuine_poles() {
    let source = SourceArena::authored(
        "map-cancellation-fixture",
        1,
        vec![AuthoredScalar::Binary64Bits(0)],
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let scalar = |v: f64| Q::scalar(Expansion::scalar(v));
    let n = vec![scalar(0.), scalar(1.), scalar(1.)];
    let d = vec![scalar(1.), scalar(1.)];
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let (nn, dd) = reduce_ratio(n.clone(), d.clone(), &mut ctx).unwrap();
    assert!(zero(
        &sub(&nn, &vec![scalar(0.), scalar(1.)], &mut ctx).unwrap()
    ));
    assert!(zero(&sub(&dd, &vec![scalar(1.)], &mut ctx).unwrap()));
    let mut limits = Limits::default();
    limits.max_work = 1;
    let mut short = PredicateContext::new(&source, &tolerance, limits, None);
    assert!(reduce_ratio(n, d, &mut short).is_err());
    let (pn, pd) = reduce_ratio(
        vec![scalar(0.), scalar(1.)],
        vec![scalar(-1.), scalar(2.)],
        &mut ctx,
    )
    .unwrap();
    assert!(!positive(&pd, &scalar(0.), &scalar(1.), &mut ctx).unwrap());
    assert!(
        !positive(
            &neg(&pd, &mut ctx).unwrap(),
            &scalar(0.),
            &scalar(1.),
            &mut ctx
        )
        .unwrap()
    );
    assert!(zero(
        &sub(
            &mul(&pn, &vec![scalar(-1.), scalar(2.)], &mut ctx).unwrap(),
            &mul(&pd, &vec![scalar(0.), scalar(1.)], &mut ctx).unwrap(),
            &mut ctx
        )
        .unwrap()
    ));
}
