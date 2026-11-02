use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
use std::{sync::atomic::AtomicBool, time::Instant};
#[derive(Clone)]
struct Source {
    degrees: [usize; 2],
    net: Vec<Vec<[f64; 4]>>,
    knots: [Vec<f64>; 2],
}
fn source(kind: usize) -> Source {
    let (x, y, weights, ku, kv, degrees) = match kind {
        0 => (
            vec![0., 1.],
            vec![0., 1.],
            vec![1., 1.],
            vec![-3., -3., 5., 5.],
            vec![7., 7., 11., 11.],
            [1, 1],
        ),
        // Same polynomial map with nonconstant homogeneous factors (1+2u)
        // and (1+2v). Stored Cartesian middle controls are EXACT 1/4, not
        // rounded 1/3 from a degree elevation with an incompatible gauge.
        1 => (
            vec![0., 0.25, 1.],
            vec![0., 0.25, 1.],
            vec![1., 2., 3.],
            vec![17.; 3].into_iter().chain([29.; 3]).collect(),
            vec![-7.; 3].into_iter().chain([13.; 3]).collect(),
            [2, 2],
        ),
        2 => (
            vec![0., 0.5, 1.],
            vec![0., 0.25, 0.75, 1.],
            vec![1.; 3],
            vec![-3., -3., 1., 5., 5.],
            vec![7., 7., 8., 10., 11., 11.],
            [1, 1],
        ),
        _ => (
            vec![0., 0.5, 1.],
            vec![0., 0.5, 1.],
            vec![1.; 3],
            vec![-1., 0., 0.5, 1., 2.],
            vec![1., 3., 5., 7., 11.],
            [1, 1],
        ),
    };
    let net = x
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            y.iter()
                .enumerate()
                .map(|(j, &y)| {
                    [
                        x,
                        y,
                        x * y / 8.,
                        if kind == 1 {
                            weights[i] * weights[j]
                        } else {
                            1.
                        },
                    ]
                })
                .collect()
        })
        .collect();
    Source {
        degrees,
        net,
        knots: [ku, kv],
    }
}
fn arena(
    s: &[Source; 2],
) -> (
    SourceArena,
    Vec<(Vec<Vec<[LeafRef; 4]>>, [Vec<LeafRef>; 2])>,
) {
    let mut values = Vec::new();
    let mut indices = Vec::new();
    for s in s {
        let start = values.len();
        values.extend(s.net.iter().flatten().flatten().copied());
        let ku = values.len();
        values.extend(&s.knots[0]);
        let kv = values.len();
        values.extend(&s.knots[1]);
        indices.push((start, ku, kv));
    }
    let arena = SourceArena::authored(
        "original-tensor-identity-test",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let refs = s
        .iter()
        .zip(indices)
        .map(|(s, (base, u, v))| {
            let controls = (0..s.net.len())
                .map(|i| {
                    (0..s.net[0].len())
                        .map(|j| {
                            std::array::from_fn(|k| {
                                arena.leaf(base + 4 * (i * s.net[0].len() + j) + k).unwrap()
                            })
                        })
                        .collect()
                })
                .collect();
            let knots = [
                (0..s.knots[0].len())
                    .map(|i| arena.leaf(u + i).unwrap())
                    .collect(),
                (0..s.knots[1].len())
                    .map(|i| arena.leaf(v + i).unwrap())
                    .collect(),
            ];
            (controls, knots)
        })
        .collect();
    (arena, refs)
}
fn run(s: [Source; 2], limits: Limits, cancel: Option<&AtomicBool>) -> BezierIdentityDecision {
    let (a, r) = arena(&s);
    let tol = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(&a, &tol, limits, cancel);
    let tensor = |i: usize| RationalTensorSource {
        degrees: s[i].degrees,
        controls: &r[i].0,
        knots: [&r[i].1[0], &r[i].1[1]],
    };
    rational_bspline_tensor_normalized_identity(&mut c, tensor(0), tensor(1)).unwrap()
}
#[test]
fn independent_degrees_homogeneous_gauges_and_complete_knot_partitions_are_equal() {
    for (a, b) in [(0, 1), (2, 1), (1, 2), (3, 1), (0, 3)] {
        let r = run(
            [source(a), source(b)],
            Limits {
                max_work: 1000000,
                ..Default::default()
            },
            None,
        );
        assert_eq!(r.outcome, BezierIdentity::Equal, "{a}->{b}: {r:?}");
        eprintln!(
            "original tensor normalized identity {a}->{b}: exact_work={}",
            r.work_used
        );
    }
}
#[test]
fn full_tensor_identity_preserves_aggregate_budget_cancellation_and_deadline() {
    let s = [source(2), source(1)];
    let r = run(s.clone(), Limits::default(), None);
    assert_eq!(r.outcome, BezierIdentity::Equal);
    assert!(r.work_used > 0);
    assert_eq!(
        run(
            s.clone(),
            Limits {
                max_work: r.work_used,
                ..Default::default()
            },
            None
        )
        .outcome,
        BezierIdentity::Equal
    );
    for max_work in [0, 1, r.work_used - 1] {
        let limited = run(
            s.clone(),
            Limits {
                max_work,
                ..Default::default()
            },
            None,
        );
        assert_eq!(
            limited.outcome,
            BezierIdentity::Indeterminate(Reason::ResourceLimit)
        );
        assert!(limited.work_used <= max_work);
    }
    let cancel = AtomicBool::new(true);
    assert_eq!(
        run(s.clone(), Limits::default(), Some(&cancel)).outcome,
        BezierIdentity::Indeterminate(Reason::Cancelled)
    );
    assert_eq!(
        run(
            s,
            Limits {
                deadline: Some(Instant::now()),
                ..Default::default()
            },
            None
        )
        .outcome,
        BezierIdentity::Indeterminate(Reason::DeadlineExceeded)
    );
}
#[test]
fn matching_boundaries_or_geometric_images_do_not_supply_a_map_identity() {
    let a = source(0);
    let mut b = source(1);
    b.net[1][1][2] = b.net[1][1][2].next_up();
    // All boundary controls are unmodified. The exact nonzero interior
    // coefficient must still reject, regardless of sample/evaluation rounding.
    assert_eq!(
        run([a.clone(), b], Limits::default(), None).outcome,
        BezierIdentity::Different
    );
    let mut b = source(1);
    for row in &mut b.net {
        for p in row {
            if p[0] == 0.25 {
                p[0] = 0.5;
            }
        }
    }
    assert_eq!(
        run([a, b], Limits::default(), None).outcome,
        BezierIdentity::Different
    );
    // Every V row separately represents the same Euclidean curve after a
    // positive row gauge. Such independent gauges change the full tensor U
    // parameterization and MUST NOT be normalized away by row extraction.
    let a = source(0);
    let mut b = a.clone();
    for p in &mut b.net[0] {
        p[3] *= 2.;
    }
    assert_eq!(
        run([a, b], Limits::default(), None).outcome,
        BezierIdentity::Different
    );
    let a = source(2);
    let mut b = a.clone();
    b.net[1][2][2] = b.net[1][2][2].next_up();
    assert_eq!(
        run([a, b], Limits::default(), None).outcome,
        BezierIdentity::Different
    );
}
#[test]
fn invalid_weights_knots_and_disconnected_sources_never_supply_identity() {
    for i in 0..4 {
        let mut b = source(1);
        match i {
            0 => b.net[1][1][3] = 0.,
            1 => b.net[1][1][3] = -1.,
            2 => b.knots[0][0] = 100.,
            _ => b.knots[0] = vec![17.; 6],
        }
        assert_eq!(
            run([source(0), b], Limits::default(), None).outcome,
            BezierIdentity::Indeterminate(Reason::MissingProof)
        );
    }
    let mut b = source(2);
    b.net.push(b.net.last().unwrap().clone());
    b.knots[0] = vec![-3., -3., 1., 1., 5., 5.];
    assert_eq!(
        run([source(0), b], Limits::default(), None).outcome,
        BezierIdentity::Indeterminate(Reason::MissingProof)
    );
}
#[test]
fn foreign_original_leaf_is_rejected_before_a_zero_budget_refusal() {
    let s = [source(0), source(1)];
    let (a, mut r) = arena(&s);
    let (foreign, _) = arena(&s);
    r[1].0[1][1][2] = foreign.leaf(0).unwrap();
    let tol = ToleranceContext::default_valid();
    let mut c = PredicateContext::new(
        &a,
        &tol,
        Limits {
            max_work: 0,
            ..Default::default()
        },
        None,
    );
    let tensor = |i: usize| RationalTensorSource {
        degrees: s[i].degrees,
        controls: &r[i].0,
        knots: [&r[i].1[0], &r[i].1[1]],
    };
    assert!(matches!(
        rational_bspline_tensor_normalized_identity(&mut c, tensor(0), tensor(1)),
        Err(InputError::ProvenanceMismatch)
    ));
}
#[test]
fn genuine_rational_refinement_and_curved_higher_degree_maps_keep_exact_identity() {
    let mut a = source(0);
    let mut b = source(3);
    for i in 0..2 {
        for j in 0..2 {
            a.net[i][j][3] = [2., 6.][i] * [2., 6.][j];
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            let (x, y) = ([0., 0.75, 1.][i], [0., 0.75, 1.][j]);
            b.net[i][j] = [x, y, x * y / 8., [2., 4., 6.][i] * [2., 4., 6.][j]];
        }
    }
    let r = run([a, b], Limits::default(), None);
    assert_eq!(r.outcome, BezierIdentity::Equal, "{r:?}");
    let mut a = source(1);
    a.net = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let (x, y) = ([0., 0., 3.][i], [0., 0., 3.][j]);
                    [x, y, x * y / 8., 1.]
                })
                .collect()
        })
        .collect();
    let mut b = a.clone();
    b.degrees = [3, 3];
    b.knots = [
        vec![-3.; 4].into_iter().chain([5.; 4]).collect(),
        vec![7.; 4].into_iter().chain([11.; 4]).collect(),
    ];
    b.net = (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    let (x, y) = ([0., 0., 0.5, 3.][i], [0., 0., 0.5, 3.][j]);
                    [
                        x,
                        y,
                        x * y / 8.,
                        [1., 1.5, 2., 2.5][i] * [1., 1.5, 2., 2.5][j],
                    ]
                })
                .collect()
        })
        .collect();
    let r = run([a, b], Limits::default(), None);
    assert_eq!(r.outcome, BezierIdentity::Equal, "{r:?}");
    eprintln!(
        "original curved quadratic/cubic tensor map: exact_work={}",
        r.work_used
    );
}
