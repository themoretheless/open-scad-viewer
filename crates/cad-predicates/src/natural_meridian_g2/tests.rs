use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn bowl(sign: f64, weighted: bool) -> Vec<Vec<[f64; 4]>> {
    (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    [
                        i as f64 / 2.,
                        sign * (i * j) as f64 / 4.,
                        if i == 2 {
                            [1., 1. + sign * 0.25, 3. + sign * 0.5][j]
                        } else {
                            0.
                        },
                        if weighted {
                            2f64.powi(i as i32 - j as i32)
                        } else {
                            1.
                        },
                    ]
                })
                .collect()
        })
        .collect()
}
fn input(
    a: &[Vec<[f64; 4]>],
    b: &[Vec<[f64; 4]>],
    maps: [[f64; 4]; 2],
) -> (SourceArena, [Vec<Vec<[LeafRef; 4]>>; 2], [[LeafRef; 4]; 2]) {
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .flatten()
        .copied()
        .chain(maps.into_iter().flatten())
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let arena = SourceArena::authored("exact-meridian-g2-tests", 1, values).unwrap();
    let mut at = 0;
    let surfaces = [a, b].map(|s| {
        s.iter()
            .map(|r| {
                r.iter()
                    .map(|_| {
                        let p = std::array::from_fn(|k| arena.leaf(at + k).unwrap());
                        at += 4;
                        p
                    })
                    .collect()
            })
            .collect()
    });
    let refs =
        std::array::from_fn(|side| std::array::from_fn(|k| arena.leaf(at + side * 4 + k).unwrap()));
    (arena, surfaces, refs)
}
fn run(
    a: &[Vec<[f64; 4]>],
    b: &[Vec<[f64; 4]>],
    maps: [[f64; 4]; 2],
    rev: [bool; 2],
) -> BezierIdentityDecision {
    let (arena, s, m) = input(a, b, maps);
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    rational_natural_meridian_g2(&mut c, [&s[0], &s[1]], m, rev, 2).unwrap()
}
#[test]
fn exact_nonzero_graph_jets_follow_original_projective_and_uv_line_weights() {
    let a = bowl(1., false);
    let b = bowl(-1., true);
    for maps in [[[1.; 4], [1., 2., 1., 1.]], [[1.; 4], [1., 4., 1., 2.]]] {
        let r = run(&a, &b, maps, [false; 2]);
        assert!(
            matches!(r.outcome, BezierIdentity::Equal),
            "{:?} work {}",
            r.outcome,
            r.work_used
        );
    }
    assert!(matches!(
        run(&a, &b, [[1.; 4]; 2], [false; 2]).outcome,
        BezierIdentity::Different
    ));
    let mut reversed = bowl(-1., false);
    reversed.reverse();
    for (i, row) in reversed.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[3] = 2f64.powi(i as i32 - j as i32);
        }
    }
    reversed.reverse();
    let r = run(&a, &reversed, [[1.; 4], [1., 2., 1., 1.]], [false, true]);
    assert!(
        matches!(r.outcome, BezierIdentity::Equal),
        "{:?}",
        r.outcome
    );
}
#[test]
fn endpoint_matching_does_not_hide_an_interior_hessian_jump() {
    let source = |perturb: bool| {
        (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let h = if i < 2 {
                            0.
                        } else {
                            (i * (i - 1)) as f64 / 12.
                        };
                        [
                            i as f64 / 4.,
                            (i * j) as f64 / 8.,
                            h * if j == 2 { 3. } else { 1. }
                                + if perturb && i == 3 && j == 2 {
                                    0.25
                                } else {
                                    0.
                                },
                            1.,
                        ]
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
    };
    let r = run(&source(false), &source(true), [[1.; 4]; 2], [false; 2]);
    assert!(
        matches!(r.outcome, BezierIdentity::Different),
        "{:?}",
        r.outcome
    );
}
#[test]
fn exact_work_cancellation_and_foreign_sources_never_admit() {
    let a = bowl(1., false);
    let b = bowl(-1., false);
    let full = run(&a, &b, [[1.; 4]; 2], [false; 2]);
    assert!(matches!(full.outcome, BezierIdentity::Equal));
    let (arena, s, m) = input(&a, &b, [[1.; 4]; 2]);
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(
        &arena,
        &t,
        Limits {
            max_work: full.work_used - 1,
            ..Limits::default()
        },
        None,
    );
    assert!(!matches!(
        rational_natural_meridian_g2(&mut c, [&s[0], &s[1]], m, [false; 2], 2)
            .unwrap()
            .outcome,
        BezierIdentity::Equal
    ));
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), Some(&cancelled));
    assert!(!matches!(
        rational_natural_meridian_g2(&mut c, [&s[0], &s[1]], m, [false; 2], 2)
            .unwrap()
            .outcome,
        BezierIdentity::Equal
    ));
    let (_, foreign, _) = input(&a, &b, [[1.; 4]; 2]);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    assert!(rational_natural_meridian_g2(&mut c, [&s[0], &foreign[1]], m, [false; 2], 2).is_err());
}
#[test]
fn exact_first_order_identity_does_not_require_or_infer_hessian_identity() {
    let a = bowl(1., false);
    let mut b = a.clone();
    b[2][2][2] += 0.125;
    let maps = [[1.; 4]; 2];
    let (arena, s, m) = input(&a, &b, maps);
    let t = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    let full = rational_natural_meridian_g1(&mut c, [&s[0], &s[1]], m, [false; 2], 2).unwrap();
    assert!(matches!(full.outcome, BezierIdentity::Equal));
    assert!(matches!(
        run(&a, &b, maps, [false; 2]).outcome,
        BezierIdentity::Different
    ));
    let mut c = PredicateContext::new(
        &arena,
        &t,
        Limits {
            max_work: full.work_used - 1,
            ..Limits::default()
        },
        None,
    );
    assert!(!matches!(
        rational_natural_meridian_g1(&mut c, [&s[0], &s[1]], m, [false; 2], 2)
            .unwrap()
            .outcome,
        BezierIdentity::Equal
    ));
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), Some(&cancel));
    assert!(!matches!(
        rational_natural_meridian_g1(&mut c, [&s[0], &s[1]], m, [false; 2], 2)
            .unwrap()
            .outcome,
        BezierIdentity::Equal
    ));
    let (_, foreign, _) = input(&a, &b, maps);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    assert!(rational_natural_meridian_g1(&mut c, [&s[0], &foreign[1]], m, [false; 2], 2).is_err());
    // An actual first-derivative jump must still refuse even though C0 agrees.
    b[2][1][2] += 0.125;
    let (arena, s, m) = input(&a, &b, maps);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    assert!(matches!(
        rational_natural_meridian_g1(&mut c, [&s[0], &s[1]], m, [false; 2], 2)
            .unwrap()
            .outcome,
        BezierIdentity::Different
    ));
    // A zero projected Jacobian cannot prove identity through 0/0.
    let mut flat = a.clone();
    for p in flat.iter_mut().flatten() {
        p[2] = 0.;
    }
    let (arena, s, m) = input(&flat, &flat, maps);
    let mut c = PredicateContext::new(&arena, &t, Limits::default(), None);
    assert!(!matches!(
        rational_natural_meridian_g1(&mut c, [&s[0], &s[1]], m, [false; 2], 0)
            .unwrap()
            .outcome,
        BezierIdentity::Equal
    ));
}
