use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
fn run(span: [f64; 2], ends: [f64; 2], bad: bool, max: u64) -> CurveGraphProjectionDecision {
    let a = super::super::tests::inputs()[0].clone();
    let b = (0..2)
        .map(|i| {
            (0..3)
                .map(|j| {
                    [
                        0.,
                        [0.25, 0.5, 0.75][j],
                        i as f64
                            + [1.0625, 1.1875, 1.5625][j]
                            + if bad && i == 0 && j == 1 { 0.125 } else { 0. },
                        1.,
                    ]
                })
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let mut values = a
        .iter()
        .chain(&b)
        .flatten()
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect::<Vec<_>>();
    let at = values.len();
    values.extend(
        span.iter()
            .chain(&ends)
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits())),
    );
    let arena = SourceArena::authored("curve-graph-test", 1, values).unwrap();
    let t = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &t,
        Limits {
            max_work: max,
            ..Limits::default()
        },
        None,
    );
    let mut index = 0;
    let refs = [a, b].map(|s| {
        s.iter()
            .map(|row| {
                row.iter()
                    .map(|_| {
                        std::array::from_fn(|_| {
                            let r = arena.leaf(index).unwrap();
                            index += 1;
                            r
                        })
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
    });
    binary_curve_graph_projection(
        &mut ctx,
        [
            ProjectionSource {
                net: &refs[0],
                transverse: 0,
                upper: true,
            },
            ProjectionSource {
                net: &refs[1],
                transverse: 0,
                upper: false,
            },
        ],
        false,
        [1, 2],
        [arena.leaf(at).unwrap(), arena.leaf(at + 1).unwrap()],
        [arena.leaf(at + 2).unwrap(), arena.leaf(at + 3).unwrap()],
    )
    .unwrap()
}
#[test]
fn original_subarc_and_independent_raw_clock_have_exact_shared_budget() {
    for (span, ends) in [([0., 1.], [0.25, 0.75]), ([-3., 5.], [-1., 3.])] {
        let r = run(span, ends, false, 1000000);
        assert!(r.proven, "{:?}", r.reason);
        assert!(run(span, ends, false, r.work_used).proven);
        assert!(!run(span, ends, false, r.work_used - 1).proven);
    }
}
#[test]
fn a_false_curve_match_and_an_outside_restriction_refuse() {
    assert!(!run([0., 1.], [0.25, 0.75], true, 1000000).proven);
    assert!(!run([0., 1.], [-0.25, 0.75], false, 1000000).proven);
}
