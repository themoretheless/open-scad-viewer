use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
use std::{sync::atomic::AtomicBool, time::Instant};
fn run(
    split: f64,
    mode: usize,
    transpose: bool,
    limits: Limits,
    cancel: Option<&AtomicBool>,
) -> BezierIdentityDecision {
    let xs = [0., split / 2., split, (1. + split) / 2., 1.];
    let mut zs = [0., 0., split * split, split, 1.];
    if mode == 1 || mode == 5 {
        zs[4] += (1. - split).powi(2);
    }
    if mode == 2 || mode == 6 {
        zs[3] += 0.0625;
        zs[4] += 0.125;
    }
    let mut net = (0..5)
        .map(|i| {
            (0..3)
                .map(|j| {
                    [
                        xs[i],
                        [0., 0.5, 1.][j],
                        zs[i] + [0., 0., 0.125][j],
                        [1., 2., 3.][j],
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if matches!(mode, 7 | 8) {
        // Both first cross-weight jets vanish at the join, but only the left
        // second weight jet is nonzero. C1 must not be mistaken for C2.
        for p in &mut net[0] {
            p[3] *= 2.;
        }
    }
    if mode == 3 {
        net[3][1][2] = net[3][1][2].next_up();
    }
    let mut ku = vec![
        -3.,
        -3.,
        -3.,
        -3. + 8. * split,
        -3. + 8. * split,
        5.,
        5.,
        5.,
    ];
    let mut kv = vec![7., 7., 7., 11., 11., 11.];
    if transpose {
        net = (0..3)
            .map(|j| (0..5).map(|i| net[i][j]).collect())
            .collect();
        std::mem::swap(&mut ku, &mut kv);
    }
    let mut values = net.iter().flatten().flatten().copied().collect::<Vec<_>>();
    let u = values.len();
    values.extend(&ku);
    let v = values.len();
    values.extend(&kv);
    let arena = SourceArena::authored(
        "original-knot-line-Ck-test",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls = (0..net.len())
        .map(|i| {
            (0..net[0].len())
                .map(|j| {
                    std::array::from_fn(|k| arena.leaf(4 * (i * net[0].len() + j) + k).unwrap())
                })
                .collect()
        })
        .collect::<Vec<_>>();
    let ku = (0..ku.len())
        .map(|i| arena.leaf(u + i).unwrap())
        .collect::<Vec<_>>();
    let kv = (0..kv.len())
        .map(|i| arena.leaf(v + i).unwrap())
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, limits, cancel);
    let order = if matches!(mode, 4 | 5 | 6 | 8) { 1 } else { 2 };
    rational_bspline_tensor_knot_continuity(
        &mut ctx,
        RationalTensorSource {
            degrees: [2, 2],
            controls: &controls,
            knots: [&ku, &kv],
        },
        order,
    )
    .unwrap()
}
#[test]
fn complete_rational_knot_lines_certify_nonuniform_cartesian_c2_in_both_axes() {
    for split in [0.25, 0.5, 0.75] {
        for transpose in [false, true] {
            let r = run(split, 0, transpose, Limits::default(), None);
            assert_eq!(r.outcome, BezierIdentity::Equal, "{r:?}");
            eprintln!(
                "original knot C2 split={split} transpose={transpose}: work={}",
                r.work_used
            );
        }
    }
}
#[test]
fn c1_curvature_jump_and_hidden_interior_along_line_changes_refuse_c2() {
    for transpose in [false, true] {
        assert_eq!(
            run(0.5, 5, transpose, Limits::default(), None).outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            run(0.5, 6, transpose, Limits::default(), None).outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            run(0.5, 1, transpose, Limits::default(), None).outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            run(0.5, 2, transpose, Limits::default(), None).outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            run(0.5, 3, transpose, Limits::default(), None).outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            run(0.5, 4, transpose, Limits::default(), None).outcome,
            BezierIdentity::Equal
        );
    }
}
#[test]
fn knot_line_identity_retains_one_work_budget_and_resource_refusals() {
    let r = run(0.25, 0, false, Limits::default(), None);
    assert_eq!(r.outcome, BezierIdentity::Equal);
    assert_eq!(
        run(
            0.25,
            0,
            false,
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
        let q = run(
            0.25,
            0,
            false,
            Limits {
                max_work,
                ..Default::default()
            },
            None,
        );
        assert_eq!(
            q.outcome,
            BezierIdentity::Indeterminate(Reason::ResourceLimit)
        );
        assert!(q.work_used <= max_work);
    }
    let cancel = AtomicBool::new(true);
    assert_eq!(
        run(0.25, 0, false, Limits::default(), Some(&cancel)).outcome,
        BezierIdentity::Indeterminate(Reason::Cancelled)
    );
    assert_eq!(
        run(
            0.25,
            0,
            false,
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
fn nonpolynomial_rational_c2_crosses_both_original_knot_directions_and_their_vertex() {
    let w = [2., 4., 6.];
    let x = [0., 0.75, 1.];
    let mut values = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            values.extend([x[i], x[j], x[i] * x[j] / 8., w[i] * w[j]]);
        }
    }
    let u = values.len();
    values.extend([-3., -3., 1., 5., 5.]);
    let v = values.len();
    values.extend([7., 7., 9., 11., 11.]);
    let arena = SourceArena::authored(
        "original-rational-C2-knot-crossing",
        1,
        values
            .into_iter()
            .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| std::array::from_fn(|k| arena.leaf(4 * (3 * i + j) + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let ku = (0..5)
        .map(|i| arena.leaf(u + i).unwrap())
        .collect::<Vec<_>>();
    let kv = (0..5)
        .map(|i| arena.leaf(v + i).unwrap())
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let r = rational_bspline_tensor_knot_continuity(
        &mut ctx,
        RationalTensorSource {
            degrees: [1, 1],
            controls: &controls,
            knots: [&ku, &kv],
        },
        2,
    )
    .unwrap();
    assert_eq!(r.outcome, BezierIdentity::Equal, "{r:?}");
    eprintln!(
        "nonpolynomial rational C2 original knot crossing: work={}",
        r.work_used
    );
}

#[test]
fn stationary_first_weight_jet_does_not_hide_a_second_weight_jet_jump() {
    for transpose in [false, true] {
        assert_eq!(
            run(0.5, 8, transpose, Limits::default(), None).outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            run(0.5, 7, transpose, Limits::default(), None).outcome,
            BezierIdentity::Different
        );
    }
}
