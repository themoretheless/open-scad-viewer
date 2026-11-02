use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
pub(super) fn inputs() -> [Vec<Vec<[f64; 4]>>; 2] {
    std::array::from_fn(|s| {
        (0..2)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        [
                            0.,
                            j as f64 / 2.,
                            (s + i) as f64 + if j == 2 { 1. } else { 0. },
                            1.,
                        ]
                    })
                    .collect()
            })
            .collect()
    })
}
fn run(data: [Vec<Vec<[f64; 4]>>; 2], max: u64) -> PairedGraphProjectionDecision {
    let values = data
        .iter()
        .flatten()
        .flatten()
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("paired-graph-test", 1, values).unwrap();
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
    let mut at = 0;
    let refs = data.map(|s| {
        s.iter()
            .map(|r| {
                r.iter()
                    .map(|_| {
                        std::array::from_fn(|_| {
                            let r = arena.leaf(at).unwrap();
                            at += 1;
                            r
                        })
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
    });
    binary_paired_graph_projection(
        &mut ctx,
        [&refs[0], &refs[1]],
        [0, 0],
        [true, false],
        false,
        [1, 2],
    )
    .unwrap()
}
#[test]
fn exact_ordered_curved_boundary_projection_replays_one_shared_work_limit() {
    let r = run(inputs(), 1000000);
    assert_eq!(r.directions, Some([Sign::Negative, Sign::Positive]));
    let exact = run(inputs(), r.work_used);
    assert_eq!(exact.directions, r.directions);
    assert_eq!(exact.work_used, r.work_used);
    let short = run(inputs(), r.work_used - 1);
    assert!(short.directions.is_none());
    assert!(short.work_used < r.work_used);
}
#[test]
fn nonseparable_order_or_a_different_boundary_height_never_authorize_contact() {
    let mut bad = inputs();
    bad[0][1][0][1] += 0.125;
    assert!(run(bad, 1000000).directions.is_none());
    let mut bad = inputs();
    for row in &mut bad[1] {
        for p in row {
            p[2] += 0.125;
        }
    }
    assert!(run(bad, 1000000).directions.is_none());
    let mut bad = inputs();
    for s in &mut bad {
        for row in s {
            row[2][1] = 0.;
        }
    }
    assert!(run(bad, 1000000).directions.is_none());
}
