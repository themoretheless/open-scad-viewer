use nurbs_core::{
    curve::Curve,
    editing::{
        exact_curve_merge::{Refusal, merge},
        exact_knot_removal::simplify,
    },
};
fn quadratic() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0., 0.], vec![3., 6.], vec![6., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
#[test]
fn split_merge_and_exact_compaction_recover_original_nonunit_profile() {
    let source = quadratic();
    let [a, b] = source.split(3.5).unwrap();
    let saved = (a.clone(), b.clone());
    let report = merge(&a, &b).unwrap();
    assert!(report.refusal.is_none());
    let curve = report.curve.unwrap();
    assert_eq!((a, b), saved);
    assert_eq!(curve.knots, vec![2., 2., 2., 3.5, 3.5, 5., 5., 5.]);
    for i in 0..=32 {
        let t = i as f64 / 32.;
        let p = curve.evaluate(2. + 3. * t).unwrap().point;
        assert!((p[0] - 6. * t).abs() < 1e-13);
        assert!((p[1] - 12. * t * (1. - t)).abs() < 1e-13);
    }
    let compact = simplify(&curve, 1_000_000).unwrap();
    assert!(compact.completed_pass);
    assert_eq!(compact.removed, 2);
    assert_eq!(compact.curve, source);
}
fn rational() -> (Curve, Curve) {
    let make = |domain: [f64; 2], x: [f64; 2], w: [f64; 2]| Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: x
            .iter()
            .map(|&x| vec![x, 2. + 2. * x, 4. + 2. * x])
            .collect(),
        weights: w.to_vec(),
        periodic: false,
    };
    (
        make([-2., 2.], [0., 1.], [1., 2.]),
        make([2., 8.], [1., 3.], [2., 4.]),
    )
}
#[test]
fn rational_three_dimensional_curves_preserve_each_absolute_clock() {
    let (a, b) = rational();
    let c = merge(&a, &b).unwrap().curve.unwrap();
    assert_eq!(c.knots, vec![-2., -2., 2., 8., 8.]);
    assert_eq!(c.weights, vec![1., 2., 4.]);
    assert_eq!(
        c.control_points,
        vec![vec![0., 2., 4.], vec![1., 4., 6.], vec![3., 8., 10.]]
    );
    for i in 0..=32 {
        let t = i as f64 / 32.;
        for (u, x) in [
            (-2. + 4. * t, 2. * t / (1. + t)),
            (2. + 6. * t, (1. + 5. * t) / (1. + t)),
        ] {
            let p = c.evaluate(u).unwrap().point;
            let law = [x, 2. + 2. * x, 4. + 2. * x];
            for k in 0..3 {
                assert!((p[k] - law[k]).abs() < 1e-13);
            }
        }
    }
}
#[test]
fn genuine_corner_remains_and_is_not_silently_smoothed() {
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let b = Curve {
        knots: vec![1., 1., 2., 2.],
        control_points: vec![vec![1., 0.], vec![1., 1.]],
        ..a.clone()
    };
    let c = merge(&a, &b).unwrap().curve.unwrap();
    assert_eq!(c.evaluate(0.5).unwrap().point, vec![0.5, 0.]);
    assert_eq!(c.evaluate(1.5).unwrap().point, vec![1., 0.5]);
    let compact = simplify(&c, 1_000_000).unwrap();
    assert_eq!(compact.removed, 0);
    assert_eq!(compact.curve, c);
}
#[test]
fn gauge_changes_and_one_ulp_mismatch_are_refused_without_editing_sources() {
    let (a, b) = rational();
    let mut gauge = b.clone();
    for w in &mut gauge.weights {
        *w *= 2.;
    }
    let mut ulp = b.clone();
    ulp.control_points[0][0] = 1_f64.next_up();
    for right in [gauge, ulp] {
        let saved = right.clone();
        let r = merge(&a, &right).unwrap();
        assert!(r.curve.is_none());
        assert_eq!(r.refusal, Some(Refusal::SeamMismatch));
        assert_eq!(right, saved);
    }
}
#[test]
fn invalid_and_incompatible_storage_are_distinguished() {
    let (a, b) = rational();
    let mut gap = b.clone();
    gap.knots = vec![3., 3., 8., 8.];
    assert_eq!(
        merge(&a, &gap).unwrap().refusal,
        Some(Refusal::NonAdjacentDomain)
    );
    let mut dim = b.clone();
    for p in &mut dim.control_points {
        p.pop();
    }
    assert_eq!(
        merge(&a, &dim).unwrap().refusal,
        Some(Refusal::DimensionMismatch)
    );
    let mut unclamped = b.clone();
    unclamped.knots = vec![0., 2., 8., 9.];
    unclamped.validate().unwrap();
    assert_eq!(
        merge(&a, &unclamped).unwrap().refusal,
        Some(Refusal::NonClamped)
    );
    let elevated = b.elevate(2).unwrap();
    assert_eq!(
        merge(&a, &elevated).unwrap().refusal,
        Some(Refusal::DegreeMismatch)
    );
    let periodic = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    assert_eq!(
        merge(&periodic, &periodic).unwrap().refusal,
        Some(Refusal::Periodic)
    );
    let mut invalid = b;
    invalid.weights[0] = f64::NAN;
    assert!(merge(&a, &invalid).is_err());
}
#[test]
fn structural_merging_keeps_the_original_control_cap() {
    let make = |start: usize| Curve {
        degree: 1,
        knots: [
            vec![start as f64; 2],
            (start + 1..start + 129).map(|u| u as f64).collect(),
            vec![(start + 129) as f64; 2],
        ]
        .concat(),
        control_points: (start..start + 130).map(|x| vec![x as f64, 0.]).collect(),
        weights: vec![1.; 130],
        periodic: false,
    };
    let a = make(0);
    let b = make(129);
    a.validate().unwrap();
    b.validate().unwrap();
    let r = merge(&a, &b).unwrap();
    assert!(r.curve.is_none());
    assert_eq!(r.refusal, Some(Refusal::ControlLimit));
}

#[test]
fn ordered_chain_preserves_all_three_absolute_polynomial_intervals() {
    use nurbs_core::editing::exact_curve_merge::merge_chain;
    let source = quadratic();
    let [a, rest] = source.split(3.5).unwrap();
    let [b, c] = rest.split(4.25).unwrap();
    let sources = vec![a, b, c];
    let saved = sources.clone();
    let r = merge_chain(&sources).unwrap();
    assert_eq!(r.joins_verified, 2);
    assert!(r.refusal.is_none());
    let curve = r.curve.unwrap();
    assert_eq!(sources, saved);
    assert_eq!(curve.degree, 2);
    assert_eq!(
        curve.knots,
        vec![2., 2., 2., 3.5, 3.5, 4.25, 4.25, 5., 5., 5.]
    );
    for i in 0..=64 {
        let t = i as f64 / 64.;
        let p = curve.evaluate(2. + 3. * t).unwrap().point;
        assert!((p[0] - 6. * t).abs() < 1e-13);
        assert!((p[1] - 12. * t * (1. - t)).abs() < 1e-13);
    }
}
#[test]
fn late_chain_refusal_emits_no_partial_profile_and_identifies_the_join() {
    use nurbs_core::editing::exact_curve_merge::{ChainRefusal, merge_chain};
    let [a, rest] = quadratic().split(3.5).unwrap();
    let [b, mut c] = rest.split(4.25).unwrap();
    c.control_points[0][0] = c.control_points[0][0].next_up();
    let sources = vec![a, b, c];
    let saved = sources.clone();
    let r = merge_chain(&sources).unwrap();
    assert!(r.curve.is_none());
    assert_eq!(r.joins_verified, 1);
    assert_eq!(
        r.refusal,
        Some(ChainRefusal {
            right_source: 2,
            reason: Refusal::SeamMismatch
        })
    );
    assert_eq!(sources, saved);
    assert!(merge_chain(&[]).is_err());
    let single = merge_chain(&sources[..1]).unwrap();
    assert_eq!(single.joins_verified, 0);
    assert_eq!(single.curve, Some(sources[0].clone()));
    let mut invalid = sources;
    invalid[2].weights[0] = f64::NAN;
    assert!(merge_chain(&invalid).is_err());
}
#[test]
fn chain_keeps_output_and_total_source_bounds() {
    use nurbs_core::editing::exact_curve_merge::{ChainRefusal, merge_chain};
    let line = |i: usize| Curve {
        degree: 1,
        knots: vec![i as f64, i as f64, (i + 1) as f64, (i + 1) as f64],
        control_points: vec![vec![i as f64, 0.], vec![(i + 1) as f64, 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let sources: Vec<_> = (0..256).map(line).collect();
    let full = merge_chain(&sources[..255]).unwrap();
    assert_eq!(full.curve.unwrap().control_points.len(), 256);
    assert_eq!(full.joins_verified, 254);
    let r = merge_chain(&sources).unwrap();
    assert!(r.curve.is_none());
    assert_eq!(r.joins_verified, 254);
    assert_eq!(
        r.refusal,
        Some(ChainRefusal {
            right_source: 255,
            reason: Refusal::ControlLimit
        })
    );
    let mut too_many = sources;
    too_many.push(line(256));
    assert!(merge_chain(&too_many).is_err());
    let dense = |start: usize| Curve {
        degree: 1,
        knots: [
            vec![start as f64; 2],
            (start + 1..start + 129).map(|u| u as f64).collect(),
            vec![(start + 129) as f64; 2],
        ]
        .concat(),
        control_points: (start..start + 130).map(|u| vec![u as f64, 0.]).collect(),
        weights: vec![1.; 130],
        periodic: false,
    };
    let dense_sources: Vec<_> = (0..4).map(|i| dense(i * 129)).collect();
    assert!(merge_chain(&dense_sources).is_err());
}

#[test]
fn exact_gauge_merge_handles_nonbinary_relative_scaling_with_one_common_gauge() {
    use nurbs_core::editing::exact_curve_merge::merge_with_weight_scaling;
    let left = Curve {
        degree: 1,
        knots: vec![-2., -2., 1., 1.],
        control_points: vec![vec![0., 0.], vec![3., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let right = Curve {
        knots: vec![1., 1., 5., 5.],
        control_points: vec![vec![3., 0.], vec![3., 5.]],
        weights: vec![3., 5.],
        ..left.clone()
    };
    let report = merge_with_weight_scaling(&left, &right, 1_000_000).unwrap();
    let merged = report.curve.unwrap();
    assert_eq!(merged.knots, [-2., -2., 1., 5., 5.]);
    assert_eq!(merged.weights, [0.375, 0.75, 1.25]);
    for (curve, parameters) in [(&left, [-2., 0., 1.]), (&right, [1., 3., 5.])] {
        for parameter in parameters {
            let original = curve.evaluate(parameter).unwrap().point;
            let result = merged.evaluate(parameter).unwrap().point;
            assert!(
                original
                    .iter()
                    .zip(result)
                    .all(|(a, b)| (a - b).abs() < 1e-12)
            );
        }
    }
    assert_eq!(
        merge_with_weight_scaling(&left, &right, report.work_used)
            .unwrap()
            .curve,
        Some(merged)
    );
    assert!(
        merge_with_weight_scaling(&left, &right, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn exact_gauge_merge_refuses_mismatched_geometry_and_unstoreable_common_weights() {
    use nurbs_core::editing::exact_curve_merge::{GaugeMergeRefusal, merge_with_weight_scaling};
    let left = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 100_000_007.],
        periodic: false,
    };
    let mut right = Curve {
        knots: vec![1., 1., 2., 2.],
        control_points: vec![vec![1., 0.], vec![2., 0.]],
        weights: vec![100_000_037., 100_000_039.],
        ..left.clone()
    };
    let result = merge_with_weight_scaling(&left, &right, 1_000_000).unwrap();
    assert!(result.curve.is_none());
    assert!(matches!(result.refusal, Some(GaugeMergeRefusal::Exact(_))));
    right.control_points[0][1] = 0.25;
    let result = merge_with_weight_scaling(&left, &right, 1_000_000).unwrap();
    assert_eq!(
        result.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch))
    );
    assert!(result.curve.is_none());
}

#[test]
fn exact_gauge_merge_chain_composes_all_scale_factors_before_storage() {
    use nurbs_core::editing::exact_curve_merge::merge_chain_with_weight_scaling;
    let segment = |start, end, start_weight, end_weight| Curve {
        degree: 1,
        knots: vec![start, start, end, end],
        control_points: vec![vec![start, 0.], vec![end, 0.]],
        weights: vec![start_weight, end_weight],
        periodic: false,
    };
    let a = segment(0., 1., 1., 2.);
    let b = segment(1., 2., 3., 5.);
    let c = segment(2., 3., 7., 11.);
    let chain = [&a, &b, &c];
    let report = merge_chain_with_weight_scaling(&chain, 1_000_000).unwrap();
    let curve = report.curve.unwrap();
    assert_eq!(curve.weights, [21. / 64., 42. / 64., 70. / 64., 110. / 64.]);
    assert_eq!(curve.knots, [0., 0., 1., 2., 3., 3.]);
    for (source, parameter) in [(&a, 0.5), (&b, 1.5), (&c, 2.5)] {
        let expected = source.evaluate(parameter).unwrap().point[0];
        assert!((curve.evaluate(parameter).unwrap().point[0] - expected).abs() < 1e-12);
    }
    assert_eq!(
        merge_chain_with_weight_scaling(&chain, report.work_used)
            .unwrap()
            .curve,
        Some(curve)
    );
    assert!(
        merge_chain_with_weight_scaling(&chain, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn exact_gauge_merge_chain_rejects_a_late_mismatch_atomically() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, merge_chain_with_weight_scaling,
    };
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let b = Curve {
        knots: vec![1., 1., 2., 2.],
        control_points: vec![vec![1., 0.], vec![2., 0.]],
        ..a.clone()
    };
    let c = Curve {
        knots: vec![2., 2., 3., 3.],
        control_points: vec![vec![2., 1.], vec![3., 1.]],
        ..a.clone()
    };
    let report = merge_chain_with_weight_scaling(&[&a, &b, &c], 1_000_000).unwrap();
    assert!(report.curve.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch))
    );
    assert_eq!(
        merge_chain_with_weight_scaling(&[&a], 1).unwrap().curve,
        Some(a)
    );
    assert!(merge_chain_with_weight_scaling(&[], 1_000_000).is_err());
}

#[test]
fn exact_degree_merge_preserves_mixed_degree_rational_laws_in_both_orders() {
    use nurbs_core::editing::exact_curve_merge::merge_with_exact_degree_elevation;
    let a = Curve {
        degree: 1,
        knots: vec![-2., -2., 1., 1.],
        control_points: vec![vec![0., 0.], vec![3., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let b = Curve {
        degree: 2,
        knots: vec![1., 1., 1., 5., 5., 5.],
        control_points: vec![vec![3., 0.], vec![4., 2.], vec![5., 0.]],
        weights: vec![3., 5., 3.],
        periodic: false,
    };
    for (left, right) in [(&a, &b), (&b.reverse().unwrap(), &a.reverse().unwrap())] {
        // Curve reversal retains the source interval, so put the reversed
        // pieces back on adjacent absolute intervals for the swapped case.
        let left = left.clone();
        let mut right = right.clone();
        if left.domain()[1] != right.domain()[0] {
            let offset = left.domain()[1] - right.domain()[0];
            for knot in &mut right.knots {
                *knot += offset;
            }
        }
        let report = merge_with_exact_degree_elevation(&left, &right, 1_000_000).unwrap();
        let joined = report.curve.unwrap();
        assert_eq!(joined.degree, 2);
        assert_eq!(joined.domain(), [left.domain()[0], right.domain()[1]]);
        for source in [&left, &right] {
            let [lo, hi] = source.domain();
            for parameter in [lo, (lo + hi) / 2., hi] {
                let expected = source.evaluate(parameter).unwrap().point;
                let actual = joined.evaluate(parameter).unwrap().point;
                assert!(
                    expected
                        .iter()
                        .zip(actual)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
            }
        }
        assert!(
            merge_with_exact_degree_elevation(&left, &right, report.work_used)
                .unwrap()
                .curve
                .is_some()
        );
        assert!(
            merge_with_exact_degree_elevation(&left, &right, report.work_used - 1)
                .unwrap()
                .curve
                .is_none()
        );
    }
}

#[test]
fn exact_degree_merge_refuses_an_unstoreable_elevated_pole() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, merge_with_exact_degree_elevation,
    };
    use nurbs_core::editing::rational_factor::Refusal as ExactRefusal;
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let b = Curve {
        degree: 2,
        knots: vec![1., 1., 1., 2., 2., 2.],
        control_points: vec![vec![2., 0.], vec![3., 2.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report = merge_with_exact_degree_elevation(&a, &b, 1_000_000).unwrap();
    assert!(report.curve.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

#[test]
fn compact_exact_seam_merge_recovers_the_original_quadratic() {
    use nurbs_core::editing::{
        exact_curve_merge::merge_and_simplify_exact_seam, exact_knot_insertion::split,
    };
    let source = quadratic();
    let [left, right] = split(&source, 3.5, 1_000_000).unwrap().pieces.unwrap();
    let report = merge_and_simplify_exact_seam(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.seam_multiplicity, Some(0));
    assert_eq!(report.removal_refusal, None);
    assert_eq!(report.curve, Some(source));
    let bounded = merge_and_simplify_exact_seam(&left, &right, report.work_used).unwrap();
    assert_eq!(bounded.curve, report.curve);
    let limited = merge_and_simplify_exact_seam(&left, &right, report.work_used - 1).unwrap();
    assert!(limited.seam_multiplicity.unwrap() > 0);
    assert!(limited.removal_refusal.is_some());
}

#[test]
fn compact_exact_seam_merge_preserves_a_geometrically_necessary_corner() {
    use nurbs_core::editing::exact_curve_merge::merge_and_simplify_exact_seam;
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let b = Curve {
        knots: vec![1., 1., 2., 2.],
        control_points: vec![vec![1., 0.], vec![1., 1.]],
        ..a.clone()
    };
    let report = merge_and_simplify_exact_seam(&a, &b, 1_000_000).unwrap();
    assert_eq!(report.seam_multiplicity, Some(1));
    assert!(report.join_refusal.is_none());
    assert!(report.removal_refusal.is_some());
    assert_eq!(report.curve.unwrap(), merge(&a, &b).unwrap().curve.unwrap());
}

#[test]
fn exact_curve_seam_jets_recover_rational_c2_despite_weight_gauge() {
    use nurbs_core::editing::{
        exact_curve_merge::compare_exact_curve_seam_jets, exact_knot_insertion::split,
    };
    let source = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![3., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let [left, mut right] = split(&source, 0.5, 1_000_000).unwrap().pieces.unwrap();
    right.weights.iter_mut().for_each(|weight| *weight *= 3.);
    let report = compare_exact_curve_seam_jets(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.continuity_order, Some(2));
    assert_eq!(
        compare_exact_curve_seam_jets(&left, &right, report.work_used).unwrap(),
        report
    );
    assert!(
        compare_exact_curve_seam_jets(&left, &right, report.work_used - 1)
            .unwrap()
            .residuals
            .is_none()
    );
}

#[test]
fn exact_curve_seam_jets_distinguish_matching_first_derivatives_from_c2() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_curve_merge::compare_exact_curve_seam_jets;
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let mut b = Curve {
        degree: 2,
        knots: vec![1., 1., 1., 3., 3., 3.],
        control_points: vec![vec![1., 0.], vec![2., 0.], vec![3., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report = compare_exact_curve_seam_jets(&a, &b, 1_000_000).unwrap();
    assert_eq!(report.continuity_order, Some(1));
    assert_eq!(
        report.residuals.unwrap()[2][1],
        Q::from_integer((-1).into())
    );
    b.control_points[1][0] = 2.5;
    assert_eq!(
        compare_exact_curve_seam_jets(&a, &b, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(0)
    );
}

#[test]
fn exact_geometric_curve_seam_proves_g2_with_different_parameter_speeds() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_curve_merge::compare_exact_curve_geometric_seam;
    let left = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut right = Curve {
        knots: vec![10., 10., 10., 12., 12., 12.],
        control_points: vec![vec![1., 1.], vec![1.5, 2.], vec![2., 4.]],
        weights: vec![3.; 3],
        ..left.clone()
    };
    let report = compare_exact_curve_geometric_seam(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.continuity_order, Some(2));
    assert_eq!(report.speed_scale, Some(Q::new(1.into(), 2.into())));
    assert_eq!(report.acceleration_scale, Some(Q::from_integer(0.into())));
    assert_eq!(
        compare_exact_curve_geometric_seam(&left, &right, report.work_used).unwrap(),
        report
    );
    assert!(
        compare_exact_curve_geometric_seam(&left, &right, report.work_used - 1)
            .unwrap()
            .refusal
            .is_some()
    );
    right.control_points[2][1] = 5.;
    assert_eq!(
        compare_exact_curve_geometric_seam(&left, &right, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(1)
    );
}

#[test]
fn exact_geometric_curve_seam_rejects_opposite_and_stationary_tangents() {
    use nurbs_core::editing::exact_curve_merge::compare_exact_curve_geometric_seam;
    let left = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut right = Curve {
        control_points: vec![vec![2., 0.], vec![1., 0.], vec![0., 0.]],
        ..left.clone()
    };
    assert_eq!(
        compare_exact_curve_geometric_seam(&left, &right, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(0)
    );
    right.control_points[1][0] = 2.;
    let report = compare_exact_curve_geometric_seam(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.continuity_order, Some(0));
    assert!(report.speed_scale.is_none());
    right.control_points[0][0] = 3.;
    assert_eq!(
        compare_exact_curve_geometric_seam(&left, &right, 1_000_000)
            .unwrap()
            .continuity_order,
        None
    );
}

fn clock_segment(domain: [f64; 2], x: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: vec![vec![x[0], 0.], vec![x[1], 0.]],
        weights: vec![1.; 2],
        periodic: false,
    }
}

#[test]
fn c1_clock_merge_accepts_a_nondyadic_scale_with_exact_knots() {
    use nurbs_core::editing::exact_curve_merge::merge_with_exact_c1_clock;
    let left = clock_segment([0., 1.], [0., 3.]);
    let right = clock_segment([10., 13.], [3., 9.]);
    let report = merge_with_exact_c1_clock(&left, &right, 1_000_000).unwrap();
    let curve = report.curve.unwrap();
    assert_eq!(report.refusal, None);
    assert_eq!(curve.knots, vec![0., 0., 1., 3., 3.]);
    for t in [0., 0.5, 1., 2., 3.] {
        assert_eq!(curve.evaluate(t).unwrap().point, vec![3. * t, 0.]);
    }
    assert!(
        merge_with_exact_c1_clock(&left, &right, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        merge_with_exact_c1_clock(&left, &right, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn c1_clock_merge_refuses_inexact_knots_and_reversed_tangents() {
    use nurbs_core::editing::{
        exact_curve_merge::{GaugeMergeRefusal, merge_with_exact_c1_clock},
        rational_factor,
    };
    let left = clock_segment([0., 1.], [0., 3.]);
    for (end, expected) in [
        (
            7.,
            GaugeMergeRefusal::Exact(rational_factor::Refusal::NotRepresentable),
        ),
        (-3., GaugeMergeRefusal::Join(Refusal::SeamMismatch)),
    ] {
        let right = clock_segment([10., 12.], [3., end]);
        let report = merge_with_exact_c1_clock(&left, &right, 1_000_000).unwrap();
        assert!(report.curve.is_none());
        assert_eq!(report.refusal, Some(expected));
    }
}

fn quadratic_clock_seam() -> (Curve, Curve) {
    let left = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let right = Curve {
        knots: vec![10., 10., 10., 12., 12., 12.],
        control_points: vec![vec![1., 1.], vec![1.5, 2.], vec![2., 4.]],
        weights: vec![3.; 3],
        ..left.clone()
    };
    (left, right)
}

#[test]
fn c2_clock_merge_preserves_the_quadratic_across_distinct_domains() {
    use nurbs_core::editing::exact_curve_merge::{
        compare_exact_curve_seam_jets, merge_with_exact_c2_clock,
    };
    let (left, right) = quadratic_clock_seam();
    let report = merge_with_exact_c2_clock(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert_eq!(curve.domain(), [0., 2.]);
    let [a, b] = curve.split(1.).unwrap();
    assert_eq!(
        compare_exact_curve_seam_jets(&a, &b, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(2)
    );
    for t in [0., 0.5, 1., 1.5, 2.] {
        assert_eq!(curve.evaluate(t).unwrap().point, vec![t, t * t]);
    }
}

#[test]
fn c2_clock_merge_refuses_g2_with_a_nonzero_tangential_correction() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, compare_exact_curve_geometric_seam, merge_with_exact_c1_clock,
        merge_with_exact_c2_clock,
    };
    let (left, mut right) = quadratic_clock_seam();
    right.control_points[2] = vec![2.5, 5.];
    let seam = compare_exact_curve_geometric_seam(&left, &right, 1_000_000).unwrap();
    assert_eq!(seam.continuity_order, Some(2));
    assert_eq!(
        seam.acceleration_scale,
        Some(num_rational::BigRational::new(1.into(), 4.into()))
    );
    let refused = merge_with_exact_c2_clock(&left, &right, 1_000_000).unwrap();
    assert!(refused.curve.is_none());
    assert_eq!(
        refused.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch))
    );
    assert!(
        merge_with_exact_c1_clock(&left, &right, 1_000_000)
            .unwrap()
            .curve
            .is_some()
    );
}

#[test]
fn projective_c2_clock_matches_nonzero_tangential_acceleration() {
    use nurbs_core::editing::exact_curve_merge::{
        compare_exact_curve_seam_jets, merge_with_exact_projective_c2_clock,
    };
    let (left, mut right) = quadratic_clock_seam();
    right.control_points[2] = vec![2.5, 5.];
    let report = merge_with_exact_projective_c2_clock(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert_eq!(curve.domain(), [0., 3.]);
    let [a, b] = curve.split(1.).unwrap();
    assert_eq!(
        compare_exact_curve_seam_jets(&a, &b, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(2)
    );
    for offset in [0., 0.5, 1., 1.5, 2.] {
        let old = 10. + offset;
        let new = 1. + 0.5 * offset / (1. - 0.25 * offset);
        let expected = right.evaluate(old).unwrap().point;
        let actual = curve.evaluate(new).unwrap().point;
        for (a, b) in expected.into_iter().zip(actual) {
            assert!((a - b).abs() < 1e-13);
        }
    }
    assert!(
        merge_with_exact_projective_c2_clock(&left, &right, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        merge_with_exact_projective_c2_clock(&left, &right, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn projective_c2_clock_refuses_a_pole_at_or_inside_the_right_domain() {
    use nurbs_core::editing::{
        exact_curve_merge::{GaugeMergeRefusal, merge_with_exact_projective_c2_clock},
        exact_projective,
    };
    let (left, mut right) = quadratic_clock_seam();
    for x in [3., 3.5] {
        right.control_points[2] = vec![x, 2. * x];
        let report = merge_with_exact_projective_c2_clock(&left, &right, 1_000_000).unwrap();
        assert!(report.curve.is_none());
        assert_eq!(
            report.refusal,
            Some(GaugeMergeRefusal::Clock(
                exact_projective::Refusal::UnprovedDenominator
            ))
        );
    }
}

fn stationary_clock_seam() -> (Curve, Curve) {
    let left = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![0., 0.], vec![0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let right = Curve {
        knots: vec![10., 10., 10., 12., 12., 12.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        ..left.clone()
    };
    (left, right)
}

#[test]
fn stationary_c2_clock_matches_exact_second_derivatives() {
    use nurbs_core::editing::exact_curve_merge::{
        compare_exact_curve_seam_jets, merge_with_exact_stationary_c2_clock,
    };
    let (left, right) = stationary_clock_seam();
    let report = merge_with_exact_stationary_c2_clock(&left, &right, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert_eq!(curve.domain(), [0., 2.]);
    let [a, b] = curve.split(1.).unwrap();
    assert_eq!(
        compare_exact_curve_seam_jets(&a, &b, 1_000_000)
            .unwrap()
            .continuity_order,
        Some(2)
    );
    for t in [0_f64, 0.5, 1., 1.5, 2.] {
        assert_eq!(curve.evaluate(t).unwrap().point, vec![(t - 1.).powi(2), 0.]);
    }
    assert!(
        merge_with_exact_stationary_c2_clock(&left, &right, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        merge_with_exact_stationary_c2_clock(&left, &right, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn stationary_c2_clock_distinguishes_flat_irrational_and_incompatible_jets() {
    use nurbs_core::editing::{
        exact_curve_merge::{GaugeMergeRefusal, merge_with_exact_stationary_c2_clock},
        rational_factor::Refusal as ExactRefusal,
    };
    let (mut left, mut right) = stationary_clock_seam();
    for (endpoint, refusal) in [
        (
            vec![2., 0.],
            GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable),
        ),
        (
            vec![-1., 0.],
            GaugeMergeRefusal::Join(Refusal::SeamMismatch),
        ),
        (vec![1., 1.], GaugeMergeRefusal::Join(Refusal::SeamMismatch)),
        (vec![0., 0.], GaugeMergeRefusal::Join(Refusal::SeamMismatch)),
    ] {
        right.control_points[2] = endpoint;
        let report = merge_with_exact_stationary_c2_clock(&left, &right, 1_000_000).unwrap();
        assert!(report.curve.is_none());
        assert_eq!(report.refusal, Some(refusal));
    }
    left.control_points[0] = vec![0., 0.];
    let flat = merge_with_exact_stationary_c2_clock(&left, &right, 1_000_000).unwrap();
    assert_eq!(flat.curve.unwrap().domain(), [0., 3.]);
}

#[test]
fn projective_c2_chain_matches_later_seams_in_the_accumulated_clock() {
    use nurbs_core::editing::exact_curve_merge::{
        compare_exact_curve_seam_jets, merge_chain_with_exact_projective_c2_clock,
    };
    let (a, mut b) = quadratic_clock_seam();
    b.control_points[2] = vec![2.5, 5.];
    let c = Curve {
        knots: vec![40., 40., 40., 41., 41., 41.],
        control_points: vec![vec![2.5, 5.], vec![3., 6.5], vec![3.625, 8.5]],
        ..b.clone()
    };
    let sources = [&a, &b, &c];
    let report = merge_chain_with_exact_projective_c2_clock(&sources, 1_000_000).unwrap();
    assert_eq!(report.joins_verified, 2);
    assert_eq!(report.refusal, None);
    let merged = report.curve.unwrap();
    assert_eq!(merged.domain(), [0., 7.]);
    for seam in [1., 3.] {
        let [left, right] = merged.split(seam).unwrap();
        assert_eq!(
            compare_exact_curve_seam_jets(&left, &right, 1_000_000)
                .unwrap()
                .continuity_order,
            Some(2)
        );
    }
    let exact_budget =
        merge_chain_with_exact_projective_c2_clock(&sources, report.work_used).unwrap();
    assert!(exact_budget.curve.is_some());
    let short = merge_chain_with_exact_projective_c2_clock(&sources, report.work_used - 1).unwrap();
    assert!(short.curve.is_none());
    assert_eq!(short.refusal.unwrap().0, 2);
}

#[test]
fn projective_c2_chain_discards_partial_output_on_a_late_mismatch() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, merge_chain_with_exact_projective_c2_clock,
    };
    let (a, b) = quadratic_clock_seam();
    let report = merge_chain_with_exact_projective_c2_clock(&[&a, &b, &a], 1_000_000).unwrap();
    assert!(report.curve.is_none());
    assert_eq!(report.joins_verified, 1);
    assert_eq!(
        report.refusal,
        Some((2, GaugeMergeRefusal::Join(Refusal::SeamMismatch)))
    );
    let singleton = merge_chain_with_exact_projective_c2_clock(&[&b], 1).unwrap();
    assert_eq!(singleton.curve, Some(b));
    assert_eq!(singleton.work_used, 0);
}

#[test]
fn compact_c2_chain_removes_both_redundant_seams_of_a_single_quadratic() {
    use nurbs_core::editing::exact_curve_merge::merge_and_compact_exact_projective_c2_chain;
    let (a, b) = quadratic_clock_seam();
    let c = Curve {
        knots: vec![30., 30., 30., 34., 34., 34.],
        control_points: vec![vec![2., 4.], vec![2.5, 6.], vec![3., 9.]],
        ..b.clone()
    };
    let report = merge_and_compact_exact_projective_c2_chain(&[&a, &b, &c], 1_000_000).unwrap();
    assert_eq!(report.chain.joins_verified, 2);
    assert_eq!(report.seams.len(), 2);
    for (seam, parameter) in report.seams.iter().zip([1., 2.]) {
        assert_eq!(seam.parameter, parameter);
        assert_eq!(seam.removed, 2);
        assert_eq!(seam.remaining_multiplicity, 0);
        assert_eq!(seam.refusal, None);
    }
    let curve = report.chain.curve.unwrap();
    assert_eq!(curve.knots, vec![0., 0., 0., 3., 3., 3.]);
    for t in [0., 0.5, 1., 1.5, 2., 2.5, 3.] {
        let point = curve.evaluate(t).unwrap().point;
        assert!((point[0] - t).abs() < 1e-13);
        assert!((point[1] - t * t).abs() < 1e-13);
    }
}

#[test]
fn compact_c2_chain_retains_verified_geometry_on_a_short_removal_budget() {
    use nurbs_core::editing::{
        exact_curve_merge::{
            merge_and_compact_exact_projective_c2_chain, merge_chain_with_exact_projective_c2_clock,
        },
        exact_knot_removal::Refusal as RemovalRefusal,
    };
    let (a, b) = quadratic_clock_seam();
    let base = merge_chain_with_exact_projective_c2_clock(&[&a, &b], 1_000_000).unwrap();
    let limited = merge_and_compact_exact_projective_c2_chain(&[&a, &b], base.work_used).unwrap();
    assert_eq!(limited.chain.curve, base.curve);
    assert_eq!(limited.chain.work_used, base.work_used);
    assert_eq!(limited.seams[0].removed, 0);
    assert_eq!(
        limited.seams[0].refusal,
        Some(RemovalRefusal::ResourceLimit)
    );
    let failed = merge_and_compact_exact_projective_c2_chain(&[&a, &b, &a], 1_000_000).unwrap();
    assert!(failed.chain.curve.is_none());
    assert!(failed.seams.is_empty());
}

fn closed_cubic_segments() -> Vec<Curve> {
    let poles = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]];
    (0..4)
        .map(|index| {
            let [a, b, c, d] = std::array::from_fn(|offset| poles[(index + offset) % 4]);
            let control = |f: fn(f64, f64, f64, f64) -> f64| {
                (0..2)
                    .map(|axis| f(a[axis], b[axis], c[axis], d[axis]))
                    .collect()
            };
            let start = 10. * index as f64;
            Curve {
                degree: 3,
                knots: [vec![start; 4], vec![start + 1.; 4]].concat(),
                control_points: vec![
                    control(|a, b, c, _| a + 4. * b + c),
                    control(|_, b, c, _| 4. * b + 2. * c),
                    control(|_, b, c, _| 2. * b + 4. * c),
                    control(|_, b, c, d| b + 4. * c + d),
                ],
                weights: vec![1.; 4],
                periodic: false,
            }
        })
        .collect()
}

#[test]
fn closed_c2_chain_proves_exact_wrap_in_the_accumulated_clock() {
    use nurbs_core::editing::exact_curve_merge::merge_closed_exact_projective_c2_chain;
    let segments = closed_cubic_segments();
    let sources: Vec<_> = segments.iter().collect();
    let report = merge_closed_exact_projective_c2_chain(&sources, 1_000_000).unwrap();
    assert_eq!(report.chain.joins_verified, 3);
    assert_eq!(report.chain.curve.unwrap().domain(), [0., 4.]);
    assert_eq!(report.closure.unwrap().continuity_order, Some(2));
    assert!(
        merge_closed_exact_projective_c2_chain(&sources, report.chain.work_used)
            .unwrap()
            .chain
            .curve
            .is_some()
    );
    assert!(
        merge_closed_exact_projective_c2_chain(&sources, report.chain.work_used - 1)
            .unwrap()
            .chain
            .curve
            .is_none()
    );
}

#[test]
fn closed_c2_chain_withholds_a_curve_when_only_internal_joins_pass() {
    use nurbs_core::editing::exact_curve_merge::merge_closed_exact_projective_c2_chain;
    let mut segments = closed_cubic_segments();
    segments[0].control_points[0][0] += 1.;
    let sources: Vec<_> = segments.iter().collect();
    let report = merge_closed_exact_projective_c2_chain(&sources, 1_000_000).unwrap();
    assert_eq!(report.chain.joins_verified, 3);
    assert!(report.chain.curve.is_none());
    let closure = report.closure.unwrap();
    assert_eq!(closure.continuity_order, None);
    assert_eq!(
        closure.residuals.unwrap()[0][0],
        num_rational::BigRational::from_integer((-1).into())
    );
}

#[test]
fn periodic_c2_storage_preserves_the_complete_active_parameter_law() {
    use nurbs_core::{
        analysis::curve_partition_agreement::verify_periodic_source_closure,
        editing::exact_curve_merge::{
            merge_closed_exact_projective_c2_chain, store_exact_periodic_c2_curve,
        },
    };
    let segments = closed_cubic_segments();
    let sources: Vec<_> = segments.iter().collect();
    let source = merge_closed_exact_projective_c2_chain(&sources, 1_000_000)
        .unwrap()
        .chain
        .curve
        .unwrap();
    let report = store_exact_periodic_c2_curve(&source, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert!(verify_periodic_source_closure(&curve).unwrap());
    assert_eq!(curve.domain(), source.domain());
    for i in 0..=32 {
        let t = i as f64 / 8.;
        assert_eq!(
            curve.evaluate(t).unwrap().point,
            source.evaluate(t).unwrap().point
        );
    }
    assert!(
        store_exact_periodic_c2_curve(&source, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        store_exact_periodic_c2_curve(&source, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn periodic_c2_storage_rejects_a_position_only_wrap() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, store_exact_periodic_c2_curve,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 2., 2., 2.],
        control_points: vec![vec![0., 0.], vec![1., 1.], vec![1., -1.], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = store_exact_periodic_c2_curve(&curve, 1_000_000).unwrap();
    assert!(report.curve.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::SeamMismatch))
    );
}

#[test]
fn exact_periodic_knot_rotation_transports_a_weighted_nonuniform_curve() {
    use nurbs_core::editing::exact_curve_merge::rotate_periodic_curve_at_exact_knot;
    let source = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 3., 4., 5., 6., 8.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![2., 1.],
            vec![0., 2.],
            vec![0., 0.],
            vec![2., 0.],
        ],
        weights: vec![1., 2., 1., 4., 1., 2.],
        periodic: true,
    };
    let report = rotate_periodic_curve_at_exact_knot(&source, 3., 1_000_000).unwrap();
    let curve = report.curve.unwrap();
    assert_eq!(curve.domain(), [3., 8.]);
    for t in [3., 3.5, 4., 4.5, 5., 5.5, 6., 7., 8.] {
        let u = if t >= 5. { t - 5. } else { t };
        assert_eq!(
            curve.evaluate(t).unwrap().point,
            source.evaluate(u).unwrap().point
        );
    }
    assert!(
        rotate_periodic_curve_at_exact_knot(&source, 3., report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        rotate_periodic_curve_at_exact_knot(&source, 3., report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn exact_periodic_knot_rotation_refuses_rounded_translations() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, rotate_periodic_curve_at_exact_knot,
    };
    use nurbs_core::editing::rational_factor::Refusal as ExactRefusal;
    let seam = 0.5_f64.next_up();
    let source = Curve {
        degree: 1,
        knots: vec![-0.25, 0., 0.25, seam, 0.75, 1., 1.25],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    assert_eq!(
        rotate_periodic_curve_at_exact_knot(&source, seam, 1_000_000)
            .unwrap()
            .refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

#[test]
fn periodic_seam_insertion_preserves_a_weighted_quadratic_across_the_wrap() {
    use nurbs_core::editing::exact_curve_merge::insert_and_rotate_periodic_curve_seam;
    let source = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1., 1., 1., 2., 1., 1.],
        periodic: true,
    };
    let report = insert_and_rotate_periodic_curve_seam(&source, 0.5, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert!(curve.periodic);
    assert_eq!(curve.domain(), [0.5, 4.5]);
    assert_eq!(curve.control_points.len(), source.control_points.len() + 1);
    for i in 0..=32 {
        let t = 0.5 + i as f64 / 8.;
        let u = if t >= 4. { t - 4. } else { t };
        let expected = source.evaluate(u).unwrap().point;
        let actual = curve.evaluate(t).unwrap().point;
        for (a, b) in expected.into_iter().zip(actual) {
            assert!((a - b).abs() < 1e-13);
        }
    }
    assert!(
        insert_and_rotate_periodic_curve_seam(&source, 0.5, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn periodic_seam_insertion_refuses_nonrepresentable_cartesian_refinement() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, insert_and_rotate_periodic_curve_seam,
    };
    use nurbs_core::editing::rational_factor::Refusal as ExactRefusal;
    let source = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 3., 4., 5., 6., 8.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1.; 6],
        periodic: true,
    };
    let refused = insert_and_rotate_periodic_curve_seam(&source, 0.5, 1_000_000).unwrap();
    assert!(refused.curve.is_none());
    assert_eq!(
        refused.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

#[test]
fn opening_periodic_curve_clamps_single_knots_without_changing_the_clock() {
    use nurbs_core::editing::exact_curve_merge::open_exact_periodic_curve;
    let curve = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1., 1., 2., 2., 1., 1.],
        periodic: true,
    };
    let report = open_exact_periodic_curve(&curve, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let opened = report.curve.unwrap();
    assert!(!opened.periodic);
    assert_eq!(opened.knots, vec![0., 0., 0., 1., 2., 3., 4., 4., 4.]);
    for i in 0..=32 {
        let t = i as f64 / 8.;
        let actual = opened.evaluate(t).unwrap().point;
        let expected = curve.evaluate(t).unwrap().point;
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() < 1e-13);
        }
    }
    assert!(
        open_exact_periodic_curve(&curve, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        open_exact_periodic_curve(&curve, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn opening_periodic_curve_refuses_a_nonrepresentable_endpoint_pole() {
    use nurbs_core::editing::exact_curve_merge::{GaugeMergeRefusal, open_exact_periodic_curve};
    use nurbs_core::editing::rational_factor::Refusal as ExactRefusal;
    let curve = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
            vec![1., 0.],
        ],
        weights: vec![1., 2., 1., 1., 1., 2.],
        periodic: true,
    };
    let report = open_exact_periodic_curve(&curve, 1_000_000).unwrap();
    assert!(report.curve.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

fn periodic_unit_square() -> Curve {
    Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    }
}

#[test]
fn periodic_restriction_preserves_the_unwrapped_clock_across_the_seam() {
    use nurbs_core::editing::exact_curve_merge::restrict_exact_periodic_curve;
    let source = periodic_unit_square();
    let restriction = restrict_exact_periodic_curve(&source, [3.5, 4.5], 1_000_000).unwrap();
    assert_eq!(restriction.refusal, None);
    let curve = restriction.curve.unwrap();
    assert!(!curve.periodic);
    assert_eq!(curve.knots, vec![3.5, 3.5, 4., 4.5, 4.5]);
    for t in [3.5, 3.75, 4., 4.25, 4.5] {
        let original = if t >= 4. { t - 4. } else { t };
        assert_eq!(
            curve.evaluate(t).unwrap().point,
            source.evaluate(original).unwrap().point
        );
    }
    assert!(
        restrict_exact_periodic_curve(&source, [3.5, 4.5], restriction.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        restrict_exact_periodic_curve(&source, [3.5, 4.5], restriction.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn periodic_restriction_accepts_one_turn_and_refuses_more() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, restrict_exact_periodic_curve,
    };
    let source = periodic_unit_square();
    let full = restrict_exact_periodic_curve(&source, [2., 6.], 1_000_000)
        .unwrap()
        .curve
        .unwrap();
    assert_eq!(full.domain(), [2., 6.]);
    assert_eq!(full.control_points.first(), full.control_points.last());
    let too_long = restrict_exact_periodic_curve(&source, [2., 6.5], 1_000_000).unwrap();
    assert!(too_long.curve.is_none());
    assert_eq!(
        too_long.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain))
    );
}

#[test]
fn multiple_periods_keep_their_original_parameter_coordinates() {
    use nurbs_core::editing::exact_curve_merge::restrict_exact_periodic_curve_over_turns;
    let source = periodic_unit_square();
    let interval = [3.5, 13.25];
    let report = restrict_exact_periodic_curve_over_turns(&source, interval, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let curve = report.curve.unwrap();
    assert_eq!(curve.domain(), interval);
    for i in 14..=53 {
        let t = i as f64 / 4.;
        assert_eq!(
            curve.evaluate(t).unwrap().point,
            source.evaluate(t % 4.).unwrap().point
        );
    }
    assert!(
        restrict_exact_periodic_curve_over_turns(&source, interval, report.work_used)
            .unwrap()
            .curve
            .is_some()
    );
    assert!(
        restrict_exact_periodic_curve_over_turns(&source, interval, report.work_used - 1)
            .unwrap()
            .curve
            .is_none()
    );
}

#[test]
fn multiple_periods_refuse_to_exceed_the_curve_control_limit() {
    use nurbs_core::editing::exact_curve_merge::{
        GaugeMergeRefusal, restrict_exact_periodic_curve_over_turns,
    };
    use nurbs_core::editing::rational_factor::Refusal as ExactRefusal;
    let source = periodic_unit_square();
    let report = restrict_exact_periodic_curve_over_turns(&source, [0., 1024.], 1_000_000).unwrap();
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::InvalidOutput))
    );
    assert!(report.curve.is_none());
}

#[test]
fn periodic_partition_covers_the_complete_shifted_period() {
    use nurbs_core::editing::exact_curve_merge::partition_exact_periodic_curve;
    let source = periodic_unit_square();
    let cuts = [4.5, 6.];
    let report = partition_exact_periodic_curve(&source, 3.5, &cuts, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let pieces = report.pieces.unwrap();
    assert_eq!(
        pieces.iter().map(Curve::domain).collect::<Vec<_>>(),
        [[3.5, 4.5], [4.5, 6.], [6., 7.5]]
    );
    for piece in &pieces {
        assert!(!piece.periodic);
        let [lo, hi] = piece.domain();
        for t in [lo, (lo + hi) / 2., hi] {
            assert_eq!(
                piece.evaluate(t).unwrap().point,
                source.evaluate(t % 4.).unwrap().point
            );
        }
    }
    assert!(
        partition_exact_periodic_curve(&source, 3.5, &cuts, report.work_used)
            .unwrap()
            .pieces
            .is_some()
    );
    let limited =
        partition_exact_periodic_curve(&source, 3.5, &cuts, report.work_used - 1).unwrap();
    assert_eq!(limited.refused_piece, Some(2));
    assert!(limited.pieces.is_none());
}

#[test]
fn periodic_partition_rejects_unordered_outside_and_duplicate_cuts() {
    use nurbs_core::editing::exact_curve_merge::partition_exact_periodic_curve;
    let source = periodic_unit_square();
    for cuts in [&[5., 4.][..], &[4., 4.], &[3.5], &[7.5], &[f64::NAN]] {
        assert!(partition_exact_periodic_curve(&source, 3.5, cuts, 1_000_000).is_err());
    }
}

#[test]
fn exact_periodic_surface_rotation_preserves_the_weighted_tensor_in_both_axes() {
    use nurbs_core::{
        editing::exact_periodic_surface::rotate_periodic_surface_at_exact_knot,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    profile.weights = vec![1., 2., 1., 4., 1.];
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let u = rotate_periodic_surface_at_exact_knot(&source, Axis::U, 3., 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert_eq!(u.knots_u, vec![2., 3., 4., 5., 6., 7., 8.]);
    assert_eq!(u.knots_v, source.knots_v);
    assert_eq!(u.weights[0], source.weights[3]);

    let transposed = nurbs_geometry::surface_transpose::transpose(&source).unwrap();
    let v = rotate_periodic_surface_at_exact_knot(&transposed, Axis::V, 3., 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert_eq!(v, nurbs_geometry::surface_transpose::transpose(&u).unwrap());
    for t in [3., 3.5, 4., 5.5, 7.] {
        for transverse in [0., 0.5, 1.] {
            let expected = source.evaluate(t % 4., transverse).unwrap().point;
            assert_eq!(u.evaluate(t, transverse).unwrap().point, expected);
        }
    }
}

#[test]
fn exact_periodic_surface_rotation_refuses_an_inexact_translated_knot() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::GaugeMergeRefusal,
            exact_periodic_surface::rotate_periodic_surface_at_exact_knot,
            rational_factor::Refusal as ExactRefusal,
        },
        surface::{Axis, extrude},
    };
    let seam = 0.5_f64.next_up();
    let mut profile = periodic_unit_square();
    profile.knots = vec![-0.25, 0., 0.25, seam, 0.75, 1., 1.25];
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let report = rotate_periodic_surface_at_exact_knot(&source, Axis::U, seam, 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

#[test]
fn exact_surface_seam_insertion_preserves_relative_weights_between_rows() {
    use nurbs_core::{
        editing::exact_periodic_surface::insert_and_rotate_periodic_surface_seam,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let mut source = extrude(&profile, [0., 0., 2.]).unwrap();
    for weights in &mut source.weights {
        weights[1] = 3.;
    }
    let report = insert_and_rotate_periodic_surface_seam(&source, Axis::U, 0.5, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let moved = report.surface.unwrap();
    assert_eq!(moved.knots_u, vec![0., 0.5, 1., 2., 3., 4., 4.5, 5.]);
    let transposed = nurbs_geometry::surface_transpose::transpose(&source).unwrap();
    let other = insert_and_rotate_periodic_surface_seam(&transposed, Axis::V, 0.5, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert_eq!(
        other,
        nurbs_geometry::surface_transpose::transpose(&moved).unwrap()
    );
    for t in [0.5, 0.75, 2., 4., 4.5] {
        for v in [0.25, 0.5, 0.75] {
            let expected = source.evaluate(t % 4., v).unwrap().point;
            let actual = moved.evaluate(t, v).unwrap().point;
            assert!(
                expected
                    .into_iter()
                    .zip(actual)
                    .all(|(a, b)| (a - b).abs() < 1e-13)
            );
        }
    }
    assert!(
        insert_and_rotate_periodic_surface_seam(&source, Axis::U, 0.5, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn exact_surface_seam_insertion_refuses_inexact_later_rows_atomically() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::GaugeMergeRefusal,
            exact_periodic_surface::insert_and_rotate_periodic_surface_seam,
            rational_factor::Refusal,
        },
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let mut source = extrude(&profile, [0., 0., 2.]).unwrap();
    source.weights[1][1] = 2.;
    let report = insert_and_rotate_periodic_surface_seam(&source, Axis::U, 0.5, 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(Refusal::NotRepresentable))
    );
}

#[test]
fn exact_surface_opening_preserves_both_parameter_domains_and_tensor_weights() {
    use nurbs_core::{
        editing::exact_periodic_surface::open_exact_periodic_surface_axis,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    profile.degree = 2;
    profile.knots = vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.];
    profile.control_points.push(vec![1., 0.]);
    profile.weights.push(1.);
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let mut source = extrude(&profile, [0., 0., 2.]).unwrap();
    for weights in &mut source.weights {
        weights[1] = 3.;
    }
    let report = open_exact_periodic_surface_axis(&source, Axis::U, 1_000_000).unwrap();
    let opened = report.surface.unwrap();
    assert!(!opened.periodic_u);
    assert_eq!(opened.knots_u, vec![0., 0., 0., 1., 2., 3., 4., 4., 4.]);
    assert_eq!(opened.knots_v, source.knots_v);
    let transposed = nurbs_geometry::surface_transpose::transpose(&source).unwrap();
    let opened_v = open_exact_periodic_surface_axis(&transposed, Axis::V, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert_eq!(
        opened_v,
        nurbs_geometry::surface_transpose::transpose(&opened).unwrap()
    );
    for u in [0., 0.25, 1., 2.75, 4.] {
        for v in [0., 0.25, 0.5, 1.] {
            let actual = opened.evaluate(u, v).unwrap().point;
            let expected = source.evaluate(u, v).unwrap().point;
            assert!(
                actual
                    .into_iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-13)
            );
        }
    }
    assert!(
        open_exact_periodic_surface_axis(&source, Axis::U, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn exact_surface_opening_rejects_an_unstorable_endpoint_in_any_row() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::GaugeMergeRefusal,
            exact_periodic_surface::open_exact_periodic_surface_axis, rational_factor::Refusal,
        },
        surface::{Axis, extrude},
    };
    let profile = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
            vec![1., 0., 0.],
        ],
        weights: vec![1.; 6],
        periodic: true,
    };
    let mut source = extrude(&profile, [0., 0., 2.]).unwrap();
    source.weights[1][1] = 2.;
    source.weights[5][1] = 2.;
    let report = open_exact_periodic_surface_axis(&source, Axis::U, 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(Refusal::NotRepresentable))
    );
}

#[test]
fn exact_periodic_surface_strip_crosses_the_seam_in_both_axes() {
    use nurbs_core::{
        editing::exact_periodic_surface::restrict_exact_periodic_surface_axis,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let mut source = extrude(&profile, [0., 0., 2.]).unwrap();
    for weights in &mut source.weights {
        weights[1] = 3.;
    }
    let interval = [3.5, 4.5];
    let report =
        restrict_exact_periodic_surface_axis(&source, Axis::U, interval, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let strip = report.surface.unwrap();
    assert!(!strip.periodic_u);
    assert_eq!(strip.knots_u, vec![3.5, 3.5, 4., 4.5, 4.5]);
    assert_eq!(strip.knots_v, source.knots_v);
    let transposed = nurbs_geometry::surface_transpose::transpose(&source).unwrap();
    let other = restrict_exact_periodic_surface_axis(&transposed, Axis::V, interval, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert_eq!(
        other,
        nurbs_geometry::surface_transpose::transpose(&strip).unwrap()
    );
    for u in [3.5, 3.75, 4., 4.25, 4.5] {
        for v in [0., 0.25, 0.5, 1.] {
            assert_eq!(
                strip.evaluate(u, v).unwrap().point,
                source.evaluate(u % 4., v).unwrap().point
            );
        }
    }
    assert!(
        restrict_exact_periodic_surface_axis(&source, Axis::U, interval, report.work_used)
            .unwrap()
            .surface
            .is_some()
    );
    assert!(
        restrict_exact_periodic_surface_axis(&source, Axis::U, interval, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn exact_periodic_surface_strip_rejects_intervals_longer_than_one_turn() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::{GaugeMergeRefusal, Refusal},
            exact_periodic_surface::restrict_exact_periodic_surface_axis,
        },
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let refused =
        restrict_exact_periodic_surface_axis(&source, Axis::U, [3., 7.5], 1_000_000).unwrap();
    assert!(refused.surface.is_none());
    assert_eq!(
        refused.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain))
    );
}

fn doubly_periodic_tensor() -> nurbs_core::surface::Surface {
    let profile = periodic_unit_square();
    nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: profile.knots.clone(),
        knots_v: profile.knots,
        control_points: profile
            .control_points
            .iter()
            .map(|a| {
                profile
                    .control_points
                    .iter()
                    .map(|b| vec![a[0], b[0], a[1] + b[1]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: true,
        periodic_v: true,
    }
}

#[test]
fn periodic_rectangle_preserves_the_tensor_across_both_seams() {
    use nurbs_core::editing::exact_periodic_surface::restrict_exact_doubly_periodic_surface;
    let source = doubly_periodic_tensor();
    let u = [3.5, 4.5];
    let v = [3.75, 4.25];
    let report = restrict_exact_doubly_periodic_surface(&source, u, v, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let rectangle = report.surface.unwrap();
    assert!(!rectangle.periodic_u && !rectangle.periodic_v);
    assert_eq!(rectangle.knots_u, vec![3.5, 3.5, 4., 4.5, 4.5]);
    assert_eq!(rectangle.knots_v, vec![3.75, 3.75, 4., 4.25, 4.25]);
    for a in [3.5, 3.75, 4., 4.25, 4.5] {
        for b in [3.75, 3.875, 4., 4.125, 4.25] {
            assert_eq!(
                rectangle.evaluate(a, b).unwrap().point,
                source.evaluate(a % 4., b % 4.).unwrap().point
            );
        }
    }
    assert!(
        restrict_exact_doubly_periodic_surface(&source, u, v, report.work_used)
            .unwrap()
            .surface
            .is_some()
    );
    assert!(
        restrict_exact_doubly_periodic_surface(&source, u, v, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn periodic_rectangle_returns_no_partial_strip_on_second_axis_refusal() {
    use nurbs_core::editing::{
        exact_curve_merge::{GaugeMergeRefusal, Refusal},
        exact_periodic_surface::restrict_exact_doubly_periodic_surface,
    };
    let source = doubly_periodic_tensor();
    let report =
        restrict_exact_doubly_periodic_surface(&source, [3.5, 4.5], [3., 7.25], 1_000_000).unwrap();
    assert!(report.work_used > 0);
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Join(Refusal::NonAdjacentDomain))
    );
}

#[test]
fn periodic_surface_partition_covers_one_turn_and_preserves_the_transverse_period() {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_axis, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let report =
        partition_exact_periodic_surface_axis(&source, Axis::V, 3.5, &[4.5, 6.], 1_000_000)
            .unwrap();
    assert_eq!(report.refusal, None);
    let strips = report.strips.unwrap();
    assert_eq!(strips.len(), 3);
    for (strip, [lo, hi]) in strips.iter().zip([[3.5, 4.5], [4.5, 6.], [6., 7.5]]) {
        assert!(strip.periodic_u && !strip.periodic_v);
        assert_eq!(strip.knots_u, source.knots_u);
        assert_eq!(strip.knots_v[1], lo);
        assert_eq!(strip.knots_v[strip.knots_v.len() - 2], hi);
        for v in [lo, (lo + hi) / 2., hi] {
            assert_eq!(
                strip.evaluate(2.25, v).unwrap().point,
                source.evaluate(2.25, v % 4.).unwrap().point
            );
        }
    }
    let short = partition_exact_periodic_surface_axis(
        &source,
        Axis::V,
        3.5,
        &[4.5, 6.],
        report.work_used - 1,
    )
    .unwrap();
    assert!(short.strips.is_none());
    assert_eq!(short.refused_strip, Some(2));
}

#[test]
fn periodic_surface_partition_rejects_invalid_cuts() {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_axis, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    for cuts in [&[6., 4.5][..], &[4.5, 4.5], &[3.5], &[7.5], &[f64::NAN]] {
        assert!(
            partition_exact_periodic_surface_axis(&source, Axis::U, 3.5, cuts, 1_000_000).is_err()
        );
    }
}

#[test]
fn periodic_grid_tiles_both_lifted_axes_exactly() {
    use nurbs_core::editing::exact_periodic_surface::partition_exact_doubly_periodic_surface;
    let source = doubly_periodic_tensor();
    let report = partition_exact_doubly_periodic_surface(
        &source,
        [3.5, 3.75],
        &[4.5],
        &[4.25, 6.],
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.refusal, None);
    let grid = report.patches.unwrap();
    assert_eq!(grid.len(), 2);
    for (row, u) in grid.iter().zip([[3.5, 4.5], [4.5, 7.5]]) {
        assert_eq!(row.len(), 3);
        for (patch, v) in row.iter().zip([[3.75, 4.25], [4.25, 6.], [6., 7.75]]) {
            assert!(!patch.periodic_u && !patch.periodic_v);
            assert_eq!(
                [patch.knots_u[1], patch.knots_u[patch.knots_u.len() - 2]],
                u
            );
            assert_eq!(
                [patch.knots_v[1], patch.knots_v[patch.knots_v.len() - 2]],
                v
            );
            for a in u {
                for b in v {
                    assert_eq!(
                        patch.evaluate(a, b).unwrap().point,
                        source.evaluate(a % 4., b % 4.).unwrap().point
                    );
                }
            }
        }
    }
    let limited = partition_exact_doubly_periodic_surface(
        &source,
        [3.5, 3.75],
        &[4.5],
        &[4.25, 6.],
        report.work_used - 1,
    )
    .unwrap();
    assert_eq!(limited.refused_intervals, [Some(1), Some(2)]);
    assert!(limited.patches.is_none());
}

#[test]
fn periodic_grid_refuses_oversized_layouts() {
    use nurbs_core::editing::exact_periodic_surface::partition_exact_doubly_periodic_surface;
    let source = doubly_periodic_tensor();
    assert!(
        partition_exact_doubly_periodic_surface(&source, [0., 0.], &[1.; 16], &[1.; 16], 1_000_000)
            .is_err()
    );
}

#[test]
fn exact_periodic_surface_assembly_round_trips_strips_on_both_axes() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            assemble_exact_periodic_surface_axis, partition_exact_periodic_surface_axis,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    for axis in [Axis::U, Axis::V] {
        let strips =
            partition_exact_periodic_surface_axis(&source, axis, 3.5, &[4.5, 6.], 1_000_000)
                .unwrap()
                .strips
                .unwrap();
        let report = assemble_exact_periodic_surface_axis(&strips, axis, 1_000_000).unwrap();
        assert_eq!(report.refusal, None);
        let surface = report.surface.unwrap();
        assert!(surface.periodic_u && surface.periodic_v);
        for t in [3.5, 4.25, 5.5, 7.5] {
            let (u, v, old_u, old_v) = match axis {
                Axis::U => (t, 2.25, t % 4., 2.25),
                Axis::V => (2.25, t, 2.25, t % 4.),
            };
            assert_eq!(
                surface.evaluate(u, v).unwrap().point,
                source.evaluate(old_u, old_v).unwrap().point
            );
        }
        assert!(
            assemble_exact_periodic_surface_axis(&strips, axis, report.work_used - 1)
                .unwrap()
                .surface
                .is_none()
        );
    }
}

#[test]
fn exact_periodic_surface_assembly_rejects_a_wrap_weight_mismatch() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            PeriodicAssemblyRefusal, assemble_exact_periodic_surface_axis,
            partition_exact_periodic_surface_axis,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    strips[0].weights[0][2] *= 2.;
    let report = assemble_exact_periodic_surface_axis(&strips, Axis::U, 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(PeriodicAssemblyRefusal::Wrap {
            transverse_index: 2
        })
    );
}

#[test]
fn exact_periodic_grid_assembly_round_trips_both_axes() {
    use nurbs_core::editing::exact_periodic_surface::{
        assemble_exact_doubly_periodic_surface, partition_exact_doubly_periodic_surface,
    };
    let source = doubly_periodic_tensor();
    for (u_cuts, v_cuts) in [(&[4.5][..], &[5., 6.][..]), (&[][..], &[][..])] {
        let patches = partition_exact_doubly_periodic_surface(
            &source,
            [3.5, 3.75],
            u_cuts,
            v_cuts,
            1_000_000,
        )
        .unwrap()
        .patches
        .unwrap();
        let report = assemble_exact_doubly_periodic_surface(&patches, 1_000_000).unwrap();
        assert_eq!(report.refusal, None);
        let result = report.surface.unwrap();
        assert!(result.periodic_u && result.periodic_v);
        for u in [3.5, 4.5, 6., 7.5] {
            for v in [3.75, 4.5, 6., 7.75] {
                assert_eq!(
                    result.evaluate(u, v).unwrap().point,
                    source.evaluate(u % 4., v % 4.).unwrap().point
                );
            }
        }
        assert!(
            assemble_exact_doubly_periodic_surface(&patches, report.work_used - 1)
                .unwrap()
                .surface
                .is_none()
        );
    }
}

#[test]
fn exact_periodic_grid_assembly_withholds_geometry_after_a_late_row_failure() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicAssemblyRefusal, assemble_exact_doubly_periodic_surface,
        partition_exact_doubly_periodic_surface,
    };
    let source = doubly_periodic_tensor();
    let mut patches =
        partition_exact_doubly_periodic_surface(&source, [0., 0.], &[2.], &[2.], 1_000_000)
            .unwrap()
            .patches
            .unwrap();
    patches[1][0].weights[1][0] = 2.;
    let report = assemble_exact_doubly_periodic_surface(&patches, 1_000_000).unwrap();
    assert_eq!(report.refused_row, Some(1));
    assert_eq!(
        report.refusal,
        Some(PeriodicAssemblyRefusal::Wrap {
            transverse_index: 1
        })
    );
    assert!(report.surface.is_none());
}

#[test]
fn periodic_strip_assembly_reconciles_independent_exact_weight_gauges() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            assemble_periodic_surface_axis_with_exact_weight_scaling,
            partition_exact_periodic_surface_axis,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let mut strips =
        partition_exact_periodic_surface_axis(&source, Axis::V, 3.5, &[4.5, 6.], 1_000_000)
            .unwrap()
            .strips
            .unwrap();
    for (strip, scale) in strips.iter_mut().zip([3., 7., 11.]) {
        for weight in strip.weights.iter_mut().flatten() {
            *weight *= scale;
        }
    }
    let report =
        assemble_periodic_surface_axis_with_exact_weight_scaling(&strips, Axis::V, 1_000_000)
            .unwrap();
    assert_eq!(report.refusal, None);
    let result = report.surface.unwrap();
    assert!(result.periodic_u && result.periodic_v);
    for t in [3.5, 4.25, 5.5, 7.5] {
        assert_eq!(
            result.evaluate(2.25, t).unwrap().point,
            source.evaluate(2.25, t % 4.).unwrap().point
        );
    }
    assert!(
        assemble_periodic_surface_axis_with_exact_weight_scaling(
            &strips,
            Axis::V,
            report.work_used - 1
        )
        .unwrap()
        .surface
        .is_none()
    );
}

#[test]
fn periodic_strip_assembly_refuses_nonproportional_boundary_weights() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            PeriodicAssemblyRefusal, assemble_periodic_surface_axis_with_exact_weight_scaling,
            partition_exact_periodic_surface_axis,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    strips[1].weights[0][2] = 2.;
    let report =
        assemble_periodic_surface_axis_with_exact_weight_scaling(&strips, Axis::U, 1_000_000)
            .unwrap();
    assert!(report.surface.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicAssemblyRefusal::Join { strip: 1, .. })
    ));
}

#[test]
fn periodic_grid_assembly_reconciles_patch_gauges_on_both_axes() {
    use nurbs_core::editing::exact_periodic_surface::{
        assemble_doubly_periodic_surface_with_exact_weight_scaling,
        partition_exact_doubly_periodic_surface,
    };
    let mut source = doubly_periodic_tensor();
    for (row, u_weight) in source.weights.iter_mut().zip([1., 1., 2., 4., 1.]) {
        for (weight, v_weight) in row.iter_mut().zip([1., 1., 4., 2., 1.]) {
            *weight = u_weight * v_weight;
        }
    }
    let mut patches =
        partition_exact_doubly_periodic_surface(&source, [0., 0.], &[2.], &[2.], 1_000_000)
            .unwrap()
            .patches
            .unwrap();
    for (patch, scale) in patches.iter_mut().flatten().zip([3., 7., 11., 13.]) {
        for weight in patch.weights.iter_mut().flatten() {
            *weight *= scale;
        }
    }
    let report =
        assemble_doubly_periodic_surface_with_exact_weight_scaling(&patches, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let assembled = report.surface.unwrap();
    assert!(assembled.periodic_u && assembled.periodic_v);
    for u in [0., 0.25, 1.5, 2., 2.75, 4.] {
        for v in [0., 0.75, 2., 3.25, 4.] {
            let a = assembled.evaluate(u, v).unwrap().point;
            let b = source.evaluate(u, v).unwrap().point;
            assert!(a.into_iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-13));
        }
    }
    assert!(
        assemble_doubly_periodic_surface_with_exact_weight_scaling(&patches, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn periodic_grid_gauge_assembly_rejects_nonproportional_final_join() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicAssemblyRefusal, assemble_doubly_periodic_surface_with_exact_weight_scaling,
        partition_exact_doubly_periodic_surface,
    };
    let mut patches = partition_exact_doubly_periodic_surface(
        &doubly_periodic_tensor(),
        [0., 0.],
        &[2.],
        &[2.],
        1_000_000,
    )
    .unwrap()
    .patches
    .unwrap();
    patches[1][0].weights[0][1] = 2.;
    let result =
        assemble_doubly_periodic_surface_with_exact_weight_scaling(&patches, 1_000_000).unwrap();
    assert!(result.surface.is_none());
    assert_eq!(result.refused_row, None);
    assert!(matches!(
        result.refusal,
        Some(PeriodicAssemblyRefusal::Join { strip: 1, .. })
    ));
}

#[test]
fn periodic_strip_assembly_refines_different_transverse_knot_sets() {
    use nurbs_core::{
        editing::{
            exact_periodic_surface::{
                assemble_periodic_surface_axis_with_exact_knot_union,
                partition_exact_periodic_surface_axis,
            },
            exact_surface_knot_insertion,
        },
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    for (strip, knot) in strips.iter_mut().zip([0.25, 0.75]) {
        let report =
            exact_surface_knot_insertion::insert(strip, Axis::V, knot, 1, 1_000_000).unwrap();
        assert_eq!(report.inserted, 1);
        *strip = report.surface;
    }
    let report =
        assemble_periodic_surface_axis_with_exact_knot_union(&strips, Axis::U, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let surface = report.surface.unwrap();
    assert_eq!(surface.knots_v, vec![0., 0., 0.25, 0.75, 1., 1.]);
    assert!(surface.periodic_u);
    for u in [0., 0.5, 2., 3.25, 4.] {
        for v in [0., 0.125, 0.5, 0.75, 1.] {
            assert_eq!(
                surface.evaluate(u, v).unwrap().point,
                source.evaluate(u, v).unwrap().point
            );
        }
    }
    assert!(
        assemble_periodic_surface_axis_with_exact_knot_union(
            &strips,
            Axis::U,
            report.work_used - 1
        )
        .unwrap()
        .surface
        .is_none()
    );
}

#[test]
fn periodic_strip_knot_union_rejects_incompatible_transverse_domains() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            PeriodicAssemblyRefusal, assemble_periodic_surface_axis_with_exact_knot_union,
            partition_exact_periodic_surface_axis,
        },
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    strips[1].knots_v = vec![0., 0., 2., 2.];
    let result =
        assemble_periodic_surface_axis_with_exact_knot_union(&strips, Axis::U, 1_000_000).unwrap();
    assert!(result.surface.is_none());
    assert!(matches!(
        result.refusal,
        Some(PeriodicAssemblyRefusal::Join { strip: 1, .. })
    ));
}

fn strips_with_mixed_transverse_degrees() -> (
    nurbs_core::surface::Surface,
    Vec<nurbs_core::surface::Surface>,
) {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_axis,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    let quadratic = &mut strips[1];
    quadratic.degree_v = 2;
    quadratic.knots_v = vec![0., 0., 0., 1., 1., 1.];
    for (points, weights) in quadratic
        .control_points
        .iter_mut()
        .zip(&mut quadratic.weights)
    {
        let midpoint = vec![points[0][0], points[0][1], 1.];
        points.insert(1, midpoint);
        weights.insert(1, weights[0]);
    }
    (source, strips)
}

#[test]
fn periodic_strip_assembly_elevates_the_transverse_tensor_exactly() {
    use nurbs_core::{
        editing::exact_periodic_surface::assemble_periodic_surface_axis_with_exact_basis_matching,
        surface::Axis,
    };
    let (source, strips) = strips_with_mixed_transverse_degrees();
    let report =
        assemble_periodic_surface_axis_with_exact_basis_matching(&strips, Axis::U, 1_000_000)
            .unwrap();
    assert_eq!(report.refusal, None);
    let surface = report.surface.unwrap();
    assert!(surface.periodic_u);
    assert_eq!(surface.degree_v, 2);
    for u in [0., 1.5, 2., 3.25, 4.] {
        for v in [0., 0.25, 0.5, 1.] {
            assert_eq!(
                surface.evaluate(u, v).unwrap().point,
                source.evaluate(u, v).unwrap().point
            );
        }
    }
    assert!(
        assemble_periodic_surface_axis_with_exact_basis_matching(
            &strips,
            Axis::U,
            report.work_used - 1
        )
        .unwrap()
        .surface
        .is_none()
    );
}

#[test]
fn periodic_basis_matching_refuses_inexact_elevated_control_points() {
    use nurbs_core::{
        editing::{
            exact_periodic_surface::{
                PeriodicAssemblyRefusal, assemble_periodic_surface_axis_with_exact_basis_matching,
            },
            rational_factor::Refusal,
        },
        surface::Axis,
    };
    let (_, mut strips) = strips_with_mixed_transverse_degrees();
    for weights in &mut strips[0].weights {
        weights[1] *= 2.;
    }
    let result =
        assemble_periodic_surface_axis_with_exact_basis_matching(&strips, Axis::U, 1_000_000)
            .unwrap();
    assert!(result.surface.is_none());
    assert_eq!(
        result.refusal,
        Some(PeriodicAssemblyRefusal::Exact(Refusal::NotRepresentable))
    );
}

fn periodic_strips_with_mixed_assembly_degrees() -> (
    nurbs_core::surface::Surface,
    Vec<nurbs_core::surface::Surface>,
) {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_axis,
        surface::{Axis, extrude},
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let mut strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    let high = &mut strips[1];
    high.degree_u = 2;
    high.knots_u = vec![2., 2., 2., 3., 3., 4., 4., 4.];
    for i in [1, 0] {
        let middle = high.control_points[i]
            .iter()
            .zip(&high.control_points[i + 1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) / 2.).collect())
            .collect();
        high.control_points.insert(i + 1, middle);
        high.weights.insert(i + 1, vec![1.; 2]);
    }
    (source, strips)
}

#[test]
fn periodic_strip_assembly_matches_degrees_along_the_assembly_axis() {
    use nurbs_core::{
        editing::exact_periodic_surface::assemble_periodic_surface_axis_with_exact_degree_matching,
        surface::Axis,
    };
    let (source, strips) = periodic_strips_with_mixed_assembly_degrees();
    let report =
        assemble_periodic_surface_axis_with_exact_degree_matching(&strips, Axis::U, 1_000_000)
            .unwrap();
    assert_eq!(report.refusal, None);
    let surface = report.surface.unwrap();
    assert_eq!(surface.degree_u, 2);
    assert!(surface.periodic_u);
    for u in [0., 0.5, 1., 1.5, 2., 2.5, 3.5, 4.] {
        for v in [0., 0.25, 1.] {
            assert_eq!(
                surface.evaluate(u, v).unwrap().point,
                source.evaluate(u, v).unwrap().point
            );
        }
    }
}

#[test]
fn periodic_strip_assembly_refuses_unstorable_elevation_along_the_assembly_axis() {
    use nurbs_core::{
        editing::{
            exact_periodic_surface::{
                PeriodicAssemblyRefusal, assemble_periodic_surface_axis_with_exact_degree_matching,
            },
            rational_factor::Refusal,
        },
        surface::Axis,
    };
    let (_, mut strips) = periodic_strips_with_mixed_assembly_degrees();
    strips[0].weights[1] = vec![2.; 2];
    let report =
        assemble_periodic_surface_axis_with_exact_degree_matching(&strips, Axis::U, 1_000_000)
            .unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(PeriodicAssemblyRefusal::Exact(Refusal::NotRepresentable))
    );
}

#[test]
fn periodic_surface_knot_removal_reduces_a_redundant_split() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            assemble_exact_periodic_surface_axis, partition_exact_periodic_surface_axis,
            remove_exact_periodic_surface_knot,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let strips = partition_exact_periodic_surface_axis(&source, Axis::U, 0., &[0.5, 2.], 1_000_000)
        .unwrap()
        .strips
        .unwrap();
    let refined = assemble_exact_periodic_surface_axis(&strips, Axis::U, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let report = remove_exact_periodic_surface_knot(&refined, Axis::U, 0.5, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let reduced = report.surface.unwrap();
    assert_eq!(reduced, source);
    assert!(
        remove_exact_periodic_surface_knot(&refined, Axis::U, 0.5, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn periodic_surface_knot_removal_preserves_actual_creases() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            PeriodicKnotRemovalRefusal, remove_exact_periodic_surface_knot,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let report = remove_exact_periodic_surface_knot(&source, Axis::V, 1., 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicKnotRemovalRefusal::Remove(_))
    ));
}

#[test]
fn periodic_surface_simplification_removes_multiple_redundant_knots_atomically() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            assemble_exact_periodic_surface_axis, partition_exact_periodic_surface_axis,
            simplify_exact_periodic_surface_axis,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let strips =
        partition_exact_periodic_surface_axis(&source, Axis::V, 0., &[0.5, 2.5], 1_000_000)
            .unwrap()
            .strips
            .unwrap();
    let refined = assemble_exact_periodic_surface_axis(&strips, Axis::V, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let report = simplify_exact_periodic_surface_axis(&refined, Axis::V, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    assert_eq!(report.removed, 2);
    assert_eq!(report.surface, Some(source));
    let short =
        simplify_exact_periodic_surface_axis(&refined, Axis::V, report.work_used - 1).unwrap();
    assert!(short.surface.is_none());
    assert_eq!(short.removed, 0);
}

#[test]
fn periodic_surface_simplification_returns_the_literal_source_if_no_knot_can_be_removed() {
    use nurbs_core::{
        editing::exact_periodic_surface::simplify_exact_periodic_surface_axis, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let result = simplify_exact_periodic_surface_axis(&source, Axis::U, 1_000_000).unwrap();
    assert_eq!(result.refusal, None);
    assert_eq!(result.removed, 0);
    assert_eq!(result.surface, Some(source));
}

#[test]
fn periodic_surface_seam_removal_recovers_the_unsubdivided_cycle() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            insert_and_rotate_periodic_surface_seam, remove_exact_periodic_surface_seam_knot,
            rotate_periodic_surface_at_exact_knot,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    for axis in [Axis::U, Axis::V] {
        let refined = insert_and_rotate_periodic_surface_seam(&source, axis, 0.5, 1_000_000)
            .unwrap()
            .surface
            .unwrap();
        let expected = rotate_periodic_surface_at_exact_knot(&source, axis, 1., 1_000_000)
            .unwrap()
            .surface
            .unwrap();
        let report =
            remove_exact_periodic_surface_seam_knot(&refined, axis, 1., 1_000_000).unwrap();
        assert_eq!(report.refusal, None);
        assert_eq!(report.surface, Some(expected));
        assert!(
            remove_exact_periodic_surface_seam_knot(&refined, axis, 1., report.work_used - 1)
                .unwrap()
                .surface
                .is_none()
        );
    }
}

#[test]
fn periodic_surface_seam_removal_keeps_a_geometric_corner() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            PeriodicKnotRemovalRefusal, remove_exact_periodic_surface_seam_knot,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let report = remove_exact_periodic_surface_seam_knot(&source, Axis::U, 1., 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicKnotRemovalRefusal::Remove(_))
    ));
}

#[test]
fn doubly_periodic_simplification_removes_redundancy_on_both_axes() {
    use nurbs_core::editing::exact_periodic_surface::{
        assemble_exact_doubly_periodic_surface, partition_exact_doubly_periodic_surface,
        simplify_exact_doubly_periodic_surface,
    };
    let source = doubly_periodic_tensor();
    let patches = partition_exact_doubly_periodic_surface(
        &source,
        [0., 0.],
        &[0.5, 2.],
        &[0.5, 2.5],
        1_000_000,
    )
    .unwrap()
    .patches
    .unwrap();
    let refined = assemble_exact_doubly_periodic_surface(&patches, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let report = simplify_exact_doubly_periodic_surface(&refined, 1_000_000).unwrap();
    assert_eq!(report.removed, [1, 2]);
    assert_eq!(report.surface, Some(source));
    assert!(report.refusal.is_none());
    let short = simplify_exact_doubly_periodic_surface(&refined, report.work_used - 1).unwrap();
    assert!(matches!(
        short.refusal,
        Some((nurbs_core::surface::Axis::V, _))
    ));
    assert_eq!(short.removed, [0, 0]);
    assert!(short.surface.is_none());
}

#[test]
fn doubly_periodic_simplification_reports_a_missing_second_period() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            open_exact_periodic_surface_axis, simplify_exact_doubly_periodic_surface,
        },
        surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let strip = open_exact_periodic_surface_axis(&source, Axis::V, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    let report = simplify_exact_doubly_periodic_surface(&strip, 1_000_000).unwrap();
    assert!(matches!(report.refusal, Some((Axis::V, _))));
    assert_eq!(report.removed, [0, 0]);
    assert!(report.surface.is_none());
}

fn surface_with_two_identical_u_turns() -> nurbs_core::surface::Surface {
    let mut source = doubly_periodic_tensor();
    source.knots_u = (-1..=9).map(f64::from).collect();
    source.control_points = source
        .control_points
        .iter()
        .take(4)
        .cycle()
        .take(9)
        .cloned()
        .collect();
    source.weights = source
        .weights
        .iter()
        .take(4)
        .cycle()
        .take(9)
        .cloned()
        .collect();
    source
}

#[test]
fn exact_period_reduction_recovers_one_turn_on_either_surface_axis() {
    use nurbs_core::{
        editing::exact_periodic_surface::reduce_exact_periodic_surface_period, surface::Axis,
    };
    let source = surface_with_two_identical_u_turns();
    let result = reduce_exact_periodic_surface_period(&source, Axis::U, 1_000_000).unwrap();
    assert_eq!(result.turns, 2);
    let surface = result.surface.unwrap();
    assert_eq!(surface, doubly_periodic_tensor());
    assert!(
        reduce_exact_periodic_surface_period(&source, Axis::U, result.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
    let source_v = nurbs_geometry::surface_transpose::transpose(&source).unwrap();
    let result_v = reduce_exact_periodic_surface_period(&source_v, Axis::V, 1_000_000).unwrap();
    assert_eq!(result_v.turns, 2);
    assert_eq!(
        result_v.surface,
        Some(nurbs_geometry::surface_transpose::transpose(&surface).unwrap())
    );
}

#[test]
fn exact_period_reduction_checks_every_transverse_weight() {
    use nurbs_core::{
        editing::exact_periodic_surface::reduce_exact_periodic_surface_period, surface::Axis,
    };
    let mut source = surface_with_two_identical_u_turns();
    source.weights[5][3] = 2.;
    let result = reduce_exact_periodic_surface_period(&source, Axis::U, 1_000_000).unwrap();
    assert_eq!(result.turns, 1);
    assert_eq!(result.surface, Some(source));
}

#[test]
fn exact_period_repetition_round_trips_weighted_surfaces_on_both_axes() {
    use nurbs_core::{
        editing::exact_periodic_surface::{
            reduce_exact_periodic_surface_period, repeat_exact_periodic_surface_period,
        },
        surface::Axis,
    };
    let mut source = doubly_periodic_tensor();
    source.weights[2][2] = 2.;
    for axis in [Axis::U, Axis::V] {
        let report = repeat_exact_periodic_surface_period(&source, axis, 3, 1_000_000).unwrap();
        let repeated = report.surface.unwrap();
        for t in [0., 1.5, 4., 7.25, 11.75, 12.] {
            let (u, v, old_u, old_v) = match axis {
                Axis::U => (t, 2.25, t % 4., 2.25),
                Axis::V => (2.25, t, 2.25, t % 4.),
            };
            assert_eq!(
                repeated.evaluate(u, v).unwrap().point,
                source.evaluate(old_u, old_v).unwrap().point
            );
        }
        let reduced = reduce_exact_periodic_surface_period(&repeated, axis, 1_000_000).unwrap();
        assert_eq!(reduced.turns, 3);
        assert_eq!(reduced.surface, Some(source.clone()));
        assert!(
            repeat_exact_periodic_surface_period(&source, axis, 3, report.work_used - 1)
                .unwrap()
                .surface
                .is_none()
        );
    }
}

#[test]
fn exact_period_repetition_refuses_unstorable_translations_and_oversized_outputs() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::{GaugeMergeRefusal, Refusal},
            exact_periodic_surface::repeat_exact_periodic_surface_period,
            rational_factor::Refusal as ExactRefusal,
        },
        surface::Axis,
    };
    let mut source = doubly_periodic_tensor();
    assert_eq!(
        repeat_exact_periodic_surface_period(&source, Axis::U, usize::MAX, 1_000_000)
            .unwrap()
            .refusal,
        Some(GaugeMergeRefusal::Join(Refusal::ControlLimit))
    );
    source.knots_u = vec![-0.25, 0., 0.25, 0.5_f64.next_up(), 0.75, 1., 1.25];
    let report = repeat_exact_periodic_surface_period(&source, Axis::U, 2, 1_000_000).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(ExactRefusal::NotRepresentable))
    );
}

#[test]
fn multiturn_surface_strip_preserves_absolute_parameters_and_transverse_period() {
    use nurbs_core::{
        editing::exact_periodic_surface::restrict_exact_periodic_surface_over_turns, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    for axis in [Axis::U, Axis::V] {
        let report =
            restrict_exact_periodic_surface_over_turns(&source, axis, [3.5, 13.25], 1_000_000)
                .unwrap();
        assert_eq!(report.refusal, None);
        let strip = report.surface.unwrap();
        match axis {
            Axis::U => assert!(!strip.periodic_u && strip.periodic_v),
            Axis::V => assert!(strip.periodic_u && !strip.periodic_v),
        };
        for q in 14..=53 {
            let t = q as f64 / 4.;
            let (u, v, old_u, old_v) = match axis {
                Axis::U => (t, 2.25, t % 4., 2.25),
                Axis::V => (2.25, t, 2.25, t % 4.),
            };
            assert_eq!(
                strip.evaluate(u, v).unwrap().point,
                source.evaluate(old_u, old_v).unwrap().point
            );
        }
        assert!(
            restrict_exact_periodic_surface_over_turns(
                &source,
                axis,
                [3.5, 13.25],
                report.work_used - 1
            )
            .unwrap()
            .surface
            .is_none()
        );
    }
}

#[test]
fn multiturn_surface_strip_refuses_an_oversized_cover() {
    use nurbs_core::{
        editing::{
            exact_curve_merge::GaugeMergeRefusal,
            exact_periodic_surface::restrict_exact_periodic_surface_over_turns,
            rational_factor::Refusal,
        },
        surface::Axis,
    };
    let report = restrict_exact_periodic_surface_over_turns(
        &doubly_periodic_tensor(),
        Axis::U,
        [0., 1024.],
        1_000_000,
    )
    .unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(Refusal::ResourceLimit))
    );
}

#[test]
fn multiturn_periodic_rectangle_preserves_both_absolute_parameter_axes() {
    use nurbs_core::editing::exact_periodic_surface::restrict_exact_doubly_periodic_surface_over_turns;
    let mut source = doubly_periodic_tensor();
    // The interior rational weight survives both periodic extensions.
    source.weights[2][2] = 2.;
    let u = [3.5, 15.25];
    let v = [3.75, 12.25];
    let report =
        restrict_exact_doubly_periodic_surface_over_turns(&source, u, v, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let patch = report.surface.unwrap();
    assert!(!patch.periodic_u && !patch.periodic_v);
    assert_eq!(
        [patch.knots_u[1], patch.knots_u[patch.control_points.len()]],
        u
    );
    assert_eq!(
        [
            patch.knots_v[1],
            patch.knots_v[patch.control_points[0].len()]
        ],
        v
    );
    for a in [3.5, 4., 5.5, 6.75, 8., 12.5, 15.25] {
        for b in [3.75, 4., 6.25, 8., 9.5, 11.75, 12.25] {
            let p = patch.evaluate(a, b).unwrap().point;
            let expected = source.evaluate(a % 4., b % 4.).unwrap().point;
            assert!(
                p.into_iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 1e-13)
            );
        }
    }
    assert!(
        restrict_exact_doubly_periodic_surface_over_turns(&source, u, v, report.work_used - 1)
            .unwrap()
            .surface
            .is_none()
    );
}

#[test]
fn multiturn_periodic_rectangle_withholds_first_strip_if_second_axis_is_oversized() {
    use nurbs_core::editing::{
        exact_curve_merge::GaugeMergeRefusal,
        exact_periodic_surface::restrict_exact_doubly_periodic_surface_over_turns,
        rational_factor::Refusal,
    };
    let report = restrict_exact_doubly_periodic_surface_over_turns(
        &doubly_periodic_tensor(),
        [3.5, 10.25],
        [0., 1024.],
        1_000_000,
    )
    .unwrap();
    assert!(report.work_used > 0);
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(GaugeMergeRefusal::Exact(Refusal::ResourceLimit))
    );
}

#[test]
fn multiturn_surface_partition_keeps_all_cut_coordinates_and_the_other_period() {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_over_turns, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    let interval = [3.5, 13.25];
    let cuts = [4., 8.5, 12.];
    let report =
        partition_exact_periodic_surface_over_turns(&source, Axis::V, interval, &cuts, 1_000_000)
            .unwrap();
    let strips = report.strips.unwrap();
    for (strip, [lo, hi]) in strips
        .iter()
        .zip([[3.5, 4.], [4., 8.5], [8.5, 12.], [12., 13.25]])
    {
        assert!(strip.periodic_u && !strip.periodic_v);
        assert_eq!(
            [
                strip.knots_v[1],
                strip.knots_v[strip.control_points[0].len()]
            ],
            [lo, hi]
        );
        for t in [lo, (lo + hi) / 2., hi] {
            assert_eq!(
                strip.evaluate(2.25, t).unwrap().point,
                source.evaluate(2.25, t % 4.).unwrap().point
            );
        }
    }
    let short = partition_exact_periodic_surface_over_turns(
        &source,
        Axis::V,
        interval,
        &cuts,
        report.work_used - 1,
    )
    .unwrap();
    assert_eq!(short.refused_strip, Some(3));
    assert!(short.strips.is_none());
}

#[test]
fn multiturn_surface_partition_refuses_out_of_range_and_unordered_cuts() {
    use nurbs_core::{
        editing::exact_periodic_surface::partition_exact_periodic_surface_over_turns, surface::Axis,
    };
    let source = doubly_periodic_tensor();
    for cuts in [&[8., 6.][..], &[8., 8.], &[13.25], &[3.], &[f64::NAN]] {
        assert!(
            partition_exact_periodic_surface_over_turns(
                &source,
                Axis::U,
                [3.5, 13.25],
                cuts,
                1_000_000
            )
            .is_err()
        );
    }
}

#[test]
fn multiturn_periodic_grid_preserves_the_absolute_rectangle_and_every_patch() {
    use nurbs_core::editing::exact_periodic_surface::partition_exact_doubly_periodic_surface_over_turns;
    let source = doubly_periodic_tensor();
    let u = [3.5, 13.25];
    let v = [3.75, 12.25];
    let report = partition_exact_doubly_periodic_surface_over_turns(
        &source,
        u,
        v,
        &[8.5],
        &[4.25, 9.],
        1_000_000,
    )
    .unwrap();
    let grid = report.patches.unwrap();
    assert_eq!(grid.len(), 2);
    for (row, u) in grid.iter().zip([[3.5, 8.5], [8.5, 13.25]]) {
        assert_eq!(row.len(), 3);
        for (patch, v) in row.iter().zip([[3.75, 4.25], [4.25, 9.], [9., 12.25]]) {
            assert!(!patch.periodic_u && !patch.periodic_v);
            assert_eq!(
                [patch.knots_u[1], patch.knots_u[patch.control_points.len()]],
                u
            );
            assert_eq!(
                [
                    patch.knots_v[1],
                    patch.knots_v[patch.control_points[0].len()]
                ],
                v
            );
            for a in [u[0], (u[0] + u[1]) / 2., u[1]] {
                for b in [v[0], (v[0] + v[1]) / 2., v[1]] {
                    assert_eq!(
                        patch.evaluate(a, b).unwrap().point,
                        source.evaluate(a % 4., b % 4.).unwrap().point
                    );
                }
            }
        }
    }
    let limited = partition_exact_doubly_periodic_surface_over_turns(
        &source,
        u,
        v,
        &[8.5],
        &[4.25, 9.],
        report.work_used - 1,
    )
    .unwrap();
    assert_eq!(limited.refused_intervals, [Some(1), Some(2)]);
    assert!(limited.patches.is_none());
}

#[test]
fn multiturn_periodic_grid_rejects_out_of_range_second_axis_cuts() {
    use nurbs_core::editing::exact_periodic_surface::partition_exact_doubly_periodic_surface_over_turns;
    let source = doubly_periodic_tensor();
    assert!(
        partition_exact_doubly_periodic_surface_over_turns(
            &source,
            [3.5, 13.25],
            [3.75, 12.25],
            &[8.5],
            &[12.25],
            1_000_000
        )
        .is_err()
    );
}

#[test]
fn exact_periodic_surface_evaluation_wraps_negative_and_distant_parameters() {
    use nurbs_core::editing::exact_periodic_surface::evaluate_exact_wrapped_surface_point;
    let source = doubly_periodic_tensor();
    for (uv, native) in [
        ([-4.25, 14.25], [3.75, 2.25]),
        ([4., -8.], [0., 0.]),
        ([2f64.powi(60), -2f64.powi(80)], [0., 0.]),
    ] {
        let report = evaluate_exact_wrapped_surface_point(&source, uv, 1_000_000).unwrap();
        assert_eq!(report.native_uv, Some(native));
        assert_eq!(
            report.point,
            Some(source.evaluate(native[0], native[1]).unwrap().point)
        );
        assert!(
            evaluate_exact_wrapped_surface_point(&source, uv, report.work_used - 1)
                .unwrap()
                .point
                .is_none()
        );
    }
}

#[test]
fn exact_periodic_surface_evaluation_refuses_rounding_at_the_upper_seam() {
    use nurbs_core::editing::{
        exact_periodic_surface::{
            evaluate_exact_wrapped_surface_point, open_exact_periodic_surface_axis,
        },
        rational_factor::Refusal,
    };
    use nurbs_core::surface::Axis;
    let source = doubly_periodic_tensor();
    let report =
        evaluate_exact_wrapped_surface_point(&source, [-2f64.powi(-54), 0.], 1_000_000).unwrap();
    assert_eq!(report.refusal, Some(Refusal::NotRepresentable));
    assert!(report.point.is_none());
    let open = open_exact_periodic_surface_axis(&source, Axis::V, 1_000_000)
        .unwrap()
        .surface
        .unwrap();
    assert!(evaluate_exact_wrapped_surface_point(&open, [1., -0.125], 1_000_000).is_err());
}

#[test]
fn wrapped_periodic_surface_batch_matches_individual_evaluation_in_input_order() {
    use nurbs_core::editing::exact_periodic_surface::{
        evaluate_exact_wrapped_surface_point, evaluate_exact_wrapped_surface_points,
    };
    let source = doubly_periodic_tensor();
    let uv = [[-4.25, 14.25], [4., -8.], [3.25, 1.75], [-4.25, 14.25]];
    let report = evaluate_exact_wrapped_surface_points(&source, &uv, 1_000_000).unwrap();
    let samples = report.samples.unwrap();
    assert_eq!(samples.len(), uv.len());
    for (sample, uv) in samples.into_iter().zip(uv) {
        let single = evaluate_exact_wrapped_surface_point(&source, uv, 1_000_000).unwrap();
        assert_eq!(sample, (single.native_uv.unwrap(), single.point.unwrap()));
    }
    let short = evaluate_exact_wrapped_surface_points(&source, &uv, report.work_used - 1).unwrap();
    assert_eq!(short.refused_sample, Some(3));
    assert!(short.samples.is_none());
}

#[test]
fn wrapped_periodic_surface_batch_identifies_unstorable_sample_without_a_prefix() {
    use nurbs_core::editing::{
        exact_periodic_surface::evaluate_exact_wrapped_surface_points, rational_factor::Refusal,
    };
    let source = doubly_periodic_tensor();
    let report = evaluate_exact_wrapped_surface_points(
        &source,
        &[[1., 2.], [-2f64.powi(-54), 0.], [2., 3.]],
        1_000_000,
    )
    .unwrap();
    assert!(report.samples.is_none());
    assert_eq!(report.refused_sample, Some(1));
    assert_eq!(report.refusal, Some(Refusal::NotRepresentable));
}

#[test]
fn periodic_surface_path_lift_tracks_multiple_signed_seam_crossings() {
    use nurbs_core::editing::exact_periodic_surface::lift_exact_periodic_surface_path;
    let source = doubly_periodic_tensor();
    let native = [
        [3.5, 0.5],
        [0.5, 3.5],
        [1.5, 2.5],
        [2.5, 1.5],
        [3.5, 0.5],
        [0.5, 3.5],
    ];
    let result = lift_exact_periodic_surface_path(&source, &native, 1_000_000).unwrap();
    assert_eq!(
        result.parameters,
        Some(vec![
            [3.5, 0.5],
            [4.5, -0.5],
            [5.5, -1.5],
            [6.5, -2.5],
            [7.5, -3.5],
            [8.5, -4.5]
        ])
    );
    let short = lift_exact_periodic_surface_path(&source, &native, result.work_used - 1).unwrap();
    assert_eq!(short.refused_coordinate, Some([5, 1]));
    assert!(short.parameters.is_none());
}

#[test]
fn periodic_surface_path_lift_refuses_exact_half_turns() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicLiftRefusal, lift_exact_periodic_surface_path,
    };
    let report = lift_exact_periodic_surface_path(
        &doubly_periodic_tensor(),
        &[[1., 3.5], [1.5, 0.5], [2., 2.5]],
        1_000_000,
    )
    .unwrap();
    assert_eq!(report.refusal, Some(PeriodicLiftRefusal::AmbiguousHalfTurn));
    assert_eq!(report.refused_coordinate, Some([2, 1]));
    assert!(report.parameters.is_none());
}

#[test]
fn periodic_path_winding_counts_signed_turns_without_storing_lifted_points() {
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_winding;
    let source = doubly_periodic_tensor();
    let tiny = 2f64.powi(-54);
    let native = [[tiny, 0.], [1., 3.], [2., 2.], [3., 1.], [tiny, 0.]];
    let report = exact_periodic_surface_path_winding(&source, &native, 1_000_000).unwrap();
    assert_eq!(report.winding, Some([1, -1]));
    let reversed: Vec<_> = native.into_iter().rev().collect();
    assert_eq!(
        exact_periodic_surface_path_winding(&source, &reversed, 1_000_000)
            .unwrap()
            .winding,
        Some([-1, 1])
    );
    assert!(
        exact_periodic_surface_path_winding(&source, &native, report.work_used - 1)
            .unwrap()
            .winding
            .is_none()
    );
}

#[test]
fn periodic_path_winding_rejects_open_and_half_turn_sequences() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicLiftRefusal, exact_periodic_surface_path_winding,
    };
    let source = doubly_periodic_tensor();
    assert!(
        exact_periodic_surface_path_winding(&source, &[[0., 0.], [1., 0.]], 1_000_000).is_err()
    );
    let report =
        exact_periodic_surface_path_winding(&source, &[[0., 0.], [2., 0.], [0., 0.]], 1_000_000)
            .unwrap();
    assert_eq!(report.refusal, Some(PeriodicLiftRefusal::AmbiguousHalfTurn));
    assert_eq!(report.refused_coordinate, Some([1, 0]));
    assert!(report.winding.is_none());
}

#[test]
fn periodic_path_area_keeps_orientation_across_both_seams() {
    use num_rational::BigRational;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_signed_area;
    let source = doubly_periodic_tensor();
    let path = [[3.5, 3.5], [0.5, 3.5], [0.5, 0.5], [3.5, 0.5], [3.5, 3.5]];
    let report = exact_periodic_surface_path_signed_area(&source, &path, 1_000_000).unwrap();
    assert_eq!(
        report.signed_area,
        Some(BigRational::from_integer(1.into()))
    );
    let reverse: Vec<_> = path.into_iter().rev().collect();
    assert_eq!(
        exact_periodic_surface_path_signed_area(&source, &reverse, 1_000_000)
            .unwrap()
            .signed_area,
        Some(BigRational::from_integer((-1).into()))
    );
    assert!(
        exact_periodic_surface_path_signed_area(&source, &path, report.work_used - 1)
            .unwrap()
            .signed_area
            .is_none()
    );
}

#[test]
fn periodic_path_area_refuses_a_loop_with_nonzero_winding() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathAreaRefusal, exact_periodic_surface_path_signed_area,
    };
    let source = doubly_periodic_tensor();
    let report = exact_periodic_surface_path_signed_area(
        &source,
        &[[0., 0.], [1., 0.], [2., 0.], [3., 0.], [0., 0.]],
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.refusal,
        Some(PeriodicPathAreaRefusal::NonzeroWinding)
    );
    assert!(report.signed_area.is_none());
}

#[test]
fn periodic_path_centroid_keeps_exact_rational_coordinates_across_seams() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_centroid;
    let source = doubly_periodic_tensor();
    let path = [[3.5, 3.5], [0.5, 3.5], [3.5, 0.5], [3.5, 3.5]];
    let report = exact_periodic_surface_path_centroid(&source, &path, 1_000_000).unwrap();
    let centroid = Some([Q::new(23.into(), 6.into()), Q::new(23.into(), 6.into())]);
    assert_eq!(report.centroid, centroid);
    assert_eq!(report.signed_area, Some(Q::new(1.into(), 2.into())));
    let reverse: Vec<_> = path.into_iter().rev().collect();
    let reversed = exact_periodic_surface_path_centroid(&source, &reverse, 1_000_000).unwrap();
    assert_eq!(reversed.centroid, centroid);
    assert_eq!(reversed.signed_area, Some(Q::new((-1).into(), 2.into())));
    assert!(
        exact_periodic_surface_path_centroid(&source, &path, report.work_used - 1)
            .unwrap()
            .refusal
            .is_some()
    );
}

#[test]
fn periodic_path_centroid_is_undefined_for_zero_algebraic_area() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_centroid;
    let path = [[3.5, 0.], [0.5, 0.], [1., 0.], [3.5, 0.]];
    // Split the return edge into two unambiguous shortest steps.
    let path = [path[0], path[1], path[2], path[1], path[3]];
    let report =
        exact_periodic_surface_path_centroid(&doubly_periodic_tensor(), &path, 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    assert_eq!(report.signed_area, Some(Q::from_integer(0.into())));
    assert!(report.centroid.is_none());
}

#[test]
fn periodic_path_inertia_proves_signed_triangle_moments_across_both_seams() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_inertia;
    let source = doubly_periodic_tensor();
    let path = [[3.5, 3.5], [0.5, 3.5], [3.5, 0.5], [3.5, 3.5]];
    let report = exact_periodic_surface_path_inertia(&source, &path, 1_000_000).unwrap();
    let expected = [
        Q::new(1.into(), 36.into()),
        Q::new((-1).into(), 72.into()),
        Q::new(1.into(), 36.into()),
    ];
    assert_eq!(report.central_moments, Some(expected.clone()));
    let reverse: Vec<_> = path.into_iter().rev().collect();
    let reverse = exact_periodic_surface_path_inertia(&source, &reverse, 1_000_000).unwrap();
    assert_eq!(reverse.central_moments, Some(expected.map(|q| -q)));
    assert!(
        exact_periodic_surface_path_inertia(&source, &path, report.geometry.work_used - 1)
            .unwrap()
            .central_moments
            .is_none()
    );
}

#[test]
fn periodic_path_inertia_is_undefined_for_zero_algebraic_area() {
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_path_inertia;
    let path = [[3.5, 0.], [0.5, 0.], [1., 0.], [0.5, 0.], [3.5, 0.]];
    let report =
        exact_periodic_surface_path_inertia(&doubly_periodic_tensor(), &path, 1_000_000).unwrap();
    assert_eq!(report.geometry.refusal, None);
    assert!(report.geometry.centroid.is_none());
    assert!(report.central_moments.is_none());
}

#[test]
fn periodic_point_classification_tests_wrapped_interior_edges_and_vertices() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathLocation as L, classify_exact_periodic_surface_path_point,
    };
    let source = doubly_periodic_tensor();
    let path = [[3.5, 3.5], [0.5, 3.5], [0.5, 0.5], [3.5, 0.5], [3.5, 3.5]];
    for (point, expected) in [
        ([0., 0.], L::Inside),
        ([0.5, 0.], L::Boundary),
        ([3.5, 3.5], L::Boundary),
        ([2., 0.], L::Outside),
    ] {
        let report =
            classify_exact_periodic_surface_path_point(&source, &path, point, 1_000_000).unwrap();
        assert_eq!(report.location, Some(expected));
        assert!(
            classify_exact_periodic_surface_path_point(&source, &path, point, report.work_used - 1)
                .unwrap()
                .location
                .is_none()
        );
    }
}

#[test]
fn periodic_point_classification_refuses_nonclosing_lifts() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathAreaRefusal, classify_exact_periodic_surface_path_point,
    };
    let source = doubly_periodic_tensor();
    let path = [[0., 0.], [1., 0.], [2., 0.], [3., 0.], [0., 0.]];
    let report =
        classify_exact_periodic_surface_path_point(&source, &path, [0., 0.], 1_000_000).unwrap();
    assert_eq!(
        report.refusal,
        Some(PeriodicPathAreaRefusal::NonzeroWinding)
    );
    assert!(report.location.is_none());
}

#[test]
fn periodic_point_batch_reuses_the_contour_and_preserves_order() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathLocation as L, classify_exact_periodic_surface_path_point,
        classify_exact_periodic_surface_path_points,
    };
    let source = doubly_periodic_tensor();
    let path = [[3.5, 3.5], [0.5, 3.5], [0.5, 0.5], [3.5, 0.5], [3.5, 3.5]];
    let queries = [[0., 0.], [0.5, 0.], [2., 0.], [0., 0.]];
    let report =
        classify_exact_periodic_surface_path_points(&source, &path, &queries, 1_000_000).unwrap();
    assert_eq!(
        report.locations,
        Some(vec![L::Inside, L::Boundary, L::Outside, L::Inside])
    );
    let separate_work: u64 = queries
        .into_iter()
        .map(|q| {
            classify_exact_periodic_surface_path_point(&source, &path, q, 1_000_000)
                .unwrap()
                .work_used
        })
        .sum();
    assert!(report.work_used < separate_work);
    let short =
        classify_exact_periodic_surface_path_points(&source, &path, &queries, report.work_used - 1)
            .unwrap();
    assert_eq!(short.refused_query, Some(3));
    assert!(short.locations.is_none());
}

#[test]
fn periodic_batch_classification_rejects_nonpolygon_inputs() {
    use nurbs_core::editing::exact_periodic_surface::classify_exact_periodic_surface_path_points;
    let source = doubly_periodic_tensor();
    for path in [&[][..], &[[0., 0.], [0., 0.]][..]] {
        assert!(
            classify_exact_periodic_surface_path_points(&source, path, &[[0., 0.]], 1_000_000)
                .is_err()
        );
    }
}

#[test]
fn periodic_region_classification_preserves_a_hole_across_both_seams() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathLocation as L, classify_exact_periodic_surface_region_points,
    };
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [3.75, 3.75],
        [0.25, 3.75],
        [0.25, 0.25],
        [3.75, 0.25],
        [3.75, 3.75],
    ];
    let queries = [[0., 0.], [0.5, 0.], [0.25, 0.], [0.75, 0.], [2., 0.]];
    let report = classify_exact_periodic_surface_region_points(
        &source,
        &[&outer, &hole],
        &queries,
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.locations,
        Some(vec![
            L::Outside,
            L::Inside,
            L::Boundary,
            L::Boundary,
            L::Outside
        ])
    );
    let reversed: Vec<_> = hole.into_iter().rev().collect();
    assert_eq!(
        classify_exact_periodic_surface_region_points(
            &source,
            &[&reversed, &outer],
            &queries,
            1_000_000
        )
        .unwrap()
        .locations,
        report.locations
    );
    let short = classify_exact_periodic_surface_region_points(
        &source,
        &[&outer, &hole],
        &queries,
        report.work_used - 1,
    )
    .unwrap();
    assert_eq!(short.refused_loop, Some(1));
    assert_eq!(short.refused_query, Some(4));
    assert!(short.locations.is_none());
}

#[test]
fn periodic_region_withholds_results_on_an_unclosed_lift() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathAreaRefusal, classify_exact_periodic_surface_region_points,
    };
    let source = doubly_periodic_tensor();
    let valid = [[0., 0.], [1., 0.], [1., 1.], [0., 0.]];
    let winding = [[0., 0.], [1., 0.], [2., 0.], [3., 0.], [0., 0.]];
    let report = classify_exact_periodic_surface_region_points(
        &source,
        &[&valid, &winding],
        &[[0., 0.]],
        1_000_000,
    )
    .unwrap();
    assert_eq!(
        report.refusal,
        Some(PeriodicPathAreaRefusal::NonzeroWinding)
    );
    assert_eq!(report.refused_loop, Some(1));
    assert!(report.locations.is_none());
}

#[test]
fn periodic_path_simplicity_distinguishes_a_valid_wrap_from_a_crossing() {
    use nurbs_core::editing::exact_periodic_surface::certify_exact_periodic_surface_path_simplicity;
    let source = doubly_periodic_tensor();
    let rectangle = [[3.5, 3.5], [0.5, 3.5], [0.5, 0.5], [3.5, 0.5], [3.5, 3.5]];
    let report =
        certify_exact_periodic_surface_path_simplicity(&source, &rectangle, 1_000_000).unwrap();
    assert_eq!(report.simple, Some(true));
    assert!(
        certify_exact_periodic_surface_path_simplicity(&source, &rectangle, report.work_used - 1)
            .unwrap()
            .simple
            .is_none()
    );
    let crossed = [[3.5, 3.5], [0.5, 0.5], [3.5, 0.5], [0.5, 3.5], [3.5, 3.5]];
    let crossed =
        certify_exact_periodic_surface_path_simplicity(&source, &crossed, 1_000_000).unwrap();
    assert_eq!(crossed.simple, Some(false));
    assert_eq!(crossed.contact.unwrap().edges, [0, 2]);
}

#[test]
fn periodic_path_simplicity_detects_overlap_with_a_translated_copy() {
    use nurbs_core::editing::exact_periodic_surface::certify_exact_periodic_surface_path_simplicity;
    let source = doubly_periodic_tensor();
    let path = [
        [0., 0.],
        [1.5, 0.],
        [3., 0.],
        [0.5, 0.],
        [1., 0.],
        [1., 1.],
        [0.5, 1.],
        [3., 1.],
        [1.5, 1.],
        [0., 1.],
        [0., 0.],
    ];
    let report = certify_exact_periodic_surface_path_simplicity(&source, &path, 1_000_000).unwrap();
    assert_eq!(report.simple, Some(false));
    assert_eq!(report.contact.unwrap().turns, [1, 0]);
}

#[test]
fn periodic_contour_pair_detects_contact_only_after_translation() {
    use nurbs_core::editing::exact_periodic_surface::check_exact_periodic_surface_path_contact;
    let source = doubly_periodic_tensor();
    let a = [
        [0.125, 0.5],
        [0.5, 0.5],
        [0.5, 1.5],
        [0.125, 1.5],
        [0.125, 0.5],
    ];
    let b = [
        [3.75, 0.75],
        [0.125, 0.75],
        [0.125, 1.25],
        [3.75, 1.25],
        [3.75, 0.75],
    ];
    let report = check_exact_periodic_surface_path_contact(&source, &a, &b, 1_000_000).unwrap();
    assert_eq!(report.disjoint, Some(false));
    let hit = report.contact.unwrap();
    assert_eq!(hit.turns, [-1, 0]);
    assert_eq!(hit.edges, [3, 0]);
    assert!(
        check_exact_periodic_surface_path_contact(&source, &a, &b, report.work_used - 1)
            .unwrap()
            .disjoint
            .is_none()
    );
}

#[test]
fn periodic_contour_pair_certifies_disjoint_boundaries_of_nested_loops() {
    use nurbs_core::editing::exact_periodic_surface::check_exact_periodic_surface_path_contact;
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let inner = [
        [3.75, 3.75],
        [0.25, 3.75],
        [0.25, 0.25],
        [3.75, 0.25],
        [3.75, 3.75],
    ];
    let report =
        check_exact_periodic_surface_path_contact(&source, &outer, &inner, 1_000_000).unwrap();
    assert_eq!(report.disjoint, Some(true));
    assert!(report.contact.is_none());
}

#[test]
fn periodic_path_relation_certifies_strict_nesting_across_both_seams() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathRelation as R, relate_exact_periodic_surface_paths,
    };
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let inner = [
        [3.75, 3.75],
        [0.25, 3.75],
        [0.25, 0.25],
        [3.75, 0.25],
        [3.75, 3.75],
    ];
    let result = relate_exact_periodic_surface_paths(&source, &outer, &inner, 1_000_000).unwrap();
    assert_eq!(result.relation, Some(R::FirstContainsSecond));
    assert_eq!(
        relate_exact_periodic_surface_paths(&source, &inner, &outer, 1_000_000)
            .unwrap()
            .relation,
        Some(R::SecondContainsFirst)
    );
    assert!(
        relate_exact_periodic_surface_paths(&source, &outer, &inner, result.work_used - 1)
            .unwrap()
            .relation
            .is_none()
    );
}

#[test]
fn periodic_path_relation_distinguishes_contact_and_self_intersection() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicPathRelation as R, relate_exact_periodic_surface_paths,
    };
    let source = doubly_periodic_tensor();
    let square = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let adjacent = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let far = [[2., 2.], [3., 2.], [3., 3.], [2., 3.], [2., 2.]];
    let bow = [[0., 0.], [1., 1.], [0., 1.], [1., 0.], [0., 0.]];
    assert!(matches!(
        relate_exact_periodic_surface_paths(&source, &square, &adjacent, 1_000_000)
            .unwrap()
            .relation,
        Some(R::BoundaryContact(_))
    ));
    assert_eq!(
        relate_exact_periodic_surface_paths(&source, &square, &far, 1_000_000)
            .unwrap()
            .relation,
        Some(R::Separate)
    );
    assert!(matches!(
        relate_exact_periodic_surface_paths(&source, &square, &bow, 1_000_000)
            .unwrap()
            .relation,
        Some(R::NonSimple { contour: 1, .. })
    ));
}

#[test]
fn periodic_contour_forest_tracks_holes_and_islands_in_input_order() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourNode, exact_periodic_surface_contour_forest,
    };
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [3.75, 3.75],
        [0.25, 3.75],
        [0.25, 0.25],
        [3.75, 0.25],
        [3.75, 3.75],
    ];
    let island = [
        [3.875, 3.875],
        [0.125, 3.875],
        [0.125, 0.125],
        [3.875, 0.125],
        [3.875, 3.875],
    ];
    let separate = [[1., 1.], [2., 1.], [2., 2.], [1., 2.], [1., 1.]];
    let contours = [&island[..], &outer[..], &separate[..], &hole[..]];
    let report = exact_periodic_surface_contour_forest(&source, &contours, 1_000_000).unwrap();
    assert_eq!(
        report.nodes,
        Some(vec![
            PeriodicContourNode {
                parent: Some(3),
                depth: 2
            },
            PeriodicContourNode {
                parent: None,
                depth: 0
            },
            PeriodicContourNode {
                parent: None,
                depth: 0
            },
            PeriodicContourNode {
                parent: Some(1),
                depth: 1
            }
        ])
    );
    assert!(
        exact_periodic_surface_contour_forest(&source, &contours, report.work_used - 1)
            .unwrap()
            .nodes
            .is_none()
    );
}

#[test]
fn periodic_contour_forest_withholds_the_result_for_touching_boundaries() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, exact_periodic_surface_contour_forest,
    };
    let source = doubly_periodic_tensor();
    let a = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let b = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let report = exact_periodic_surface_contour_forest(&source, &[&a, &b], 1_000_000).unwrap();
    assert!(report.nodes.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact {
            contours: [0, 1],
            ..
        })
    ));
}

#[test]
fn periodic_region_area_adds_islands_and_subtracts_holes_across_seams() {
    use num_rational::BigRational;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_region_area;
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [3.75, 3.75],
        [0.25, 3.75],
        [0.25, 0.25],
        [3.75, 0.25],
        [3.75, 3.75],
    ];
    let island = [
        [3.875, 3.875],
        [0.125, 3.875],
        [0.125, 0.125],
        [3.875, 0.125],
        [3.875, 3.875],
    ];
    let separate = [[1., 1.], [2., 1.], [2., 2.], [1., 2.], [1., 1.]];
    let contours = [&hole[..], &island[..], &separate[..], &outer[..]];
    let report = exact_periodic_surface_region_area(&source, &contours, 1_000_000).unwrap();
    assert_eq!(report.area, Some(BigRational::new(49.into(), 16.into())));
    assert!(report.refusal.is_none());
    let reverse: Vec<_> = outer.into_iter().rev().collect();
    let other = [&reverse[..], &separate[..], &hole[..], &island[..]];
    assert_eq!(
        exact_periodic_surface_region_area(&source, &other, 1_000_000)
            .unwrap()
            .area,
        report.area
    );
    assert!(
        exact_periodic_surface_region_area(&source, &contours, report.work_used - 1)
            .unwrap()
            .area
            .is_none()
    );
}

#[test]
fn periodic_region_area_refuses_tangent_boundaries() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, exact_periodic_surface_region_area,
    };
    let source = doubly_periodic_tensor();
    let outer = [[0., 0.], [1.5, 0.], [1.5, 1.5], [0., 1.5], [0., 0.]];
    let touching = [[0., 0.5], [0.5, 0.5], [0.5, 1.], [0., 1.], [0., 0.5]];
    let report =
        exact_periodic_surface_region_area(&source, &[&outer, &touching], 1_000_000).unwrap();
    assert!(report.area.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact {
            contours: [0, 1],
            ..
        })
    ));
}

#[test]
fn periodic_region_centroid_aligns_holes_to_the_outer_lift() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_region_component_centroids;
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let island = [
        [0.25, 0.25],
        [0.5, 0.25],
        [0.5, 0.5],
        [0.25, 0.5],
        [0.25, 0.25],
    ];
    let contours = [&hole[..], &outer[..], &island[..]];
    let report =
        exact_periodic_surface_region_component_centroids(&source, &contours, 1_000_000).unwrap();
    let components = report.components.unwrap();
    assert_eq!(components.len(), 2);
    assert_eq!(components[0].outer_contour, 1);
    assert_eq!(components[0].area, Q::from_integer(2.into()));
    assert_eq!(
        components[0].centroid,
        [Q::new(253.into(), 64.into()), Q::new(253.into(), 64.into())]
    );
    assert_eq!(components[1].outer_contour, 2);
    assert_eq!(
        components[1].centroid,
        [Q::new(3.into(), 8.into()), Q::new(3.into(), 8.into())]
    );
    assert!(
        exact_periodic_surface_region_component_centroids(&source, &contours, report.work_used - 1)
            .unwrap()
            .components
            .is_none()
    );
}

#[test]
fn periodic_region_centroid_rejects_touching_components() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, exact_periodic_surface_region_component_centroids,
    };
    let source = doubly_periodic_tensor();
    let a = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let b = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let report =
        exact_periodic_surface_region_component_centroids(&source, &[&a, &b], 1_000_000).unwrap();
    assert!(report.components.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact { .. })
    ));
}

#[test]
fn periodic_region_inertia_uses_the_shifted_hole_and_component_centroid() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::exact_periodic_surface_region_component_inertia;
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let report =
        exact_periodic_surface_region_component_inertia(&source, &[&outer, &hole], 1_000_000)
            .unwrap();
    assert!(report.refusal.is_none());
    let components = report.components.unwrap();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].geometry.area, Q::from_integer(2.into()));
    let expected = [
        Q::new(2317.into(), 6144.into()),
        Q::new((-81).into(), 2048.into()),
        Q::new(2317.into(), 6144.into()),
    ];
    assert_eq!(components[0].central_moments, expected);
    let reverse: Vec<_> = hole.into_iter().rev().collect();
    assert_eq!(
        exact_periodic_surface_region_component_inertia(&source, &[&outer, &reverse], 1_000_000)
            .unwrap()
            .components
            .unwrap()[0]
            .central_moments,
        expected
    );
    assert!(
        exact_periodic_surface_region_component_inertia(
            &source,
            &[&outer, &hole],
            report.work_used - 1
        )
        .unwrap()
        .components
        .is_none()
    );
}

#[test]
fn periodic_region_inertia_rejects_contact_between_contours() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, exact_periodic_surface_region_component_inertia,
    };
    let a = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let b = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let report = exact_periodic_surface_region_component_inertia(
        &doubly_periodic_tensor(),
        &[&a, &b],
        1_000_000,
    )
    .unwrap();
    assert!(report.components.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact { .. })
    ));
}

#[test]
fn periodic_region_lift_exports_holes_in_the_outer_chart_and_islands_separately() {
    use num_rational::BigRational as Q;
    use nurbs_core::editing::exact_periodic_surface::lift_exact_periodic_surface_region_components;
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let island = [
        [0.25, 0.25],
        [0.5, 0.25],
        [0.5, 0.5],
        [0.25, 0.5],
        [0.25, 0.25],
    ];
    let contours = [&island[..], &outer[..], &hole[..]];
    let report =
        lift_exact_periodic_surface_region_components(&source, &contours, 1_000_000).unwrap();
    let charts = report.components.unwrap();
    assert_eq!(charts.len(), 2);
    assert_eq!(charts[0].outer.contour, 0);
    assert!(charts[0].holes.is_empty());
    assert_eq!(charts[1].outer.contour, 1);
    let hole = &charts[1].holes[0];
    assert_eq!(hole.contour, 2);
    assert_eq!(hole.translation, [1, 1]);
    assert_eq!(
        hole.vertices[0],
        [Q::new(33.into(), 8.into()), Q::new(33.into(), 8.into())]
    );
    assert_eq!(hole.vertices.first(), hole.vertices.last());
    assert!(
        lift_exact_periodic_surface_region_components(&source, &contours, report.work_used - 1)
            .unwrap()
            .components
            .is_none()
    );
}

#[test]
fn periodic_region_lift_refuses_touching_boundaries() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, lift_exact_periodic_surface_region_components,
    };
    let a = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let b = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let report = lift_exact_periodic_surface_region_components(
        &doubly_periodic_tensor(),
        &[&a, &b],
        1_000_000,
    )
    .unwrap();
    assert!(report.components.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact { .. })
    ));
}

#[test]
fn periodic_trim_components_feed_the_uv_classifier_with_aligned_holes() {
    use nurbs_core::{
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_components,
        trim_domain::{Location, TrimDomain, exact_loop_joins},
    };
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [3.25, 0.75],
        [0.75, 0.75],
        [0.75, 3.25],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let report =
        build_exact_periodic_surface_trim_components(&source, &[&hole, &outer], 1_000_000).unwrap();
    let components = report.components.unwrap();
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].contours, vec![1, 0]);
    let loops = &components[0].loops;
    for contour in loops {
        assert_eq!(exact_loop_joins(contour).unwrap(), Some(true));
    }
    let trim = TrimDomain::new(loops, 1e-8).unwrap();
    assert_eq!(
        trim.classify_point([4., 4.], 1024).unwrap().location,
        Location::Inside
    );
    assert_eq!(
        trim.classify_point([4.25, 4.25], 1024).unwrap().location,
        Location::Outside
    );
    assert!(
        build_exact_periodic_surface_trim_components(
            &source,
            &[&hole, &outer],
            report.work_used - 1
        )
        .unwrap()
        .components
        .is_none()
    );
}

#[test]
fn periodic_trim_components_refuse_touching_contours() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicContourForestRefusal, build_exact_periodic_surface_trim_components,
    };
    let a = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    let b = [[1., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 0.]];
    let report = build_exact_periodic_surface_trim_components(
        &doubly_periodic_tensor(),
        &[&a, &b],
        1_000_000,
    )
    .unwrap();
    assert!(report.components.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicContourForestRefusal::Contact { .. })
    ));
}

#[test]
fn periodic_trim_patches_provide_exact_support_and_holes_across_both_seams() {
    use num_rational::BigRational as Q;
    use nurbs_core::{
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::{Location, TrimDomain},
    };
    let source = doubly_periodic_tensor();
    // Start on the near side so the lift initially has negative minima.
    let outer = [
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let report =
        build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000).unwrap();
    assert!(report.refusal.is_none());
    let patches = report.patches.unwrap();
    let patch = &patches[0];
    assert_eq!(
        patch.uv_translation,
        [Q::from_integer(4.into()), Q::from_integer(4.into())]
    );
    assert!(!patch.surface.periodic_u && !patch.surface.periodic_v);
    assert_eq!(patch.surface.knots_u[1], 3.25);
    assert_eq!(
        patch.surface.knots_v[patch.surface.control_points[0].len()],
        4.75
    );
    let domain = TrimDomain::new(&patch.trim.loops, 1e-8).unwrap();
    for (uv, location) in [
        ([4., 4.], Location::Inside),
        ([4.25, 4.25], Location::Outside),
    ] {
        assert_eq!(domain.classify_point(uv, 1024).unwrap().location, location);
        assert_eq!(
            patch.surface.evaluate(uv[0], uv[1]).unwrap().point,
            source.evaluate(uv[0] % 4., uv[1] % 4.).unwrap().point
        );
    }
    let short =
        build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], report.work_used - 1)
            .unwrap();
    assert!(short.patches.is_none());
}

#[test]
fn periodic_trim_patches_support_a_nonperiodic_transverse_axis() {
    use nurbs_core::{
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        surface::extrude,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [[3.5, 0.], [0.5, 0.], [0.5, 0.75], [3.5, 0.75], [3.5, 0.]];
    let report = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000).unwrap();
    let patch = &report.patches.unwrap()[0];
    assert_eq!(patch.surface.knots_v, vec![0., 0., 0.75, 0.75]);
    assert_eq!(
        patch.surface.evaluate(4.25, 0.5).unwrap().point,
        source.evaluate(0.25, 0.5).unwrap().point
    );
}

#[test]
fn periodic_trim_edges_share_exact_uv_and_world_curves_across_chart_cells() {
    use nurbs_core::{
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_edges,
        trim_domain::exact_loop_joins,
    };
    let source = doubly_periodic_tensor();
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let report =
        build_exact_periodic_surface_trim_edges(&source, &[&outer, &hole], 1_000_000).unwrap();
    assert_eq!(report.refusal, None);
    let result = report.patches.unwrap();
    assert_eq!(result[0].loops[0].len(), 8);
    assert_eq!(result[0].loops[1].len(), 4);
    for path in &result[0].loops {
        assert_eq!(
            exact_loop_joins(
                &path
                    .iter()
                    .map(|edge| edge.pcurve.clone())
                    .collect::<Vec<_>>()
            )
            .unwrap(),
            Some(true)
        );
        for (edge, next) in path.iter().zip(path.iter().cycle().skip(1)) {
            assert_eq!(
                edge.curve.control_points.last(),
                next.curve.control_points.first()
            );
            let [lo, hi] = edge.curve.domain();
            assert_eq!(edge.pcurve.domain(), [lo, hi]);
            for t in [lo, (lo + hi) / 2., hi] {
                let uv = edge.pcurve.evaluate(t).unwrap().point;
                let edge_point = edge.curve.evaluate(t).unwrap().point;
                let support_point = result[0]
                    .patch
                    .surface
                    .evaluate(uv[0], uv[1])
                    .unwrap()
                    .point;
                assert!(
                    edge_point
                        .into_iter()
                        .zip(support_point)
                        .all(|(a, b)| (a - b).abs() < 1e-13)
                );
            }
        }
    }
    let limited =
        build_exact_periodic_surface_trim_edges(&source, &[&outer, &hole], report.work_used - 1)
            .unwrap();
    assert!(limited.patches.is_none());
    assert_eq!(limited.refused_curve, Some([0, 1, 0]));
}

#[test]
fn periodic_trim_edges_withhold_all_results_on_invalid_contour_topology() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimEdgeRefusal, build_exact_periodic_surface_trim_edges,
    };
    let source = doubly_periodic_tensor();
    let bow = [[0., 0.], [1., 1.], [0., 1.], [1., 0.], [0., 0.]];
    let report = build_exact_periodic_surface_trim_edges(&source, &[&bow], 1_000_000).unwrap();
    assert!(report.patches.is_none());
    assert!(matches!(
        report.refusal,
        Some(PeriodicTrimEdgeRefusal::Patch(_))
    ));
}


#[test]
fn periodic_trim_edge_sources_partition_oriented_segments() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_edges;
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let report = build_exact_periodic_surface_trim_edges(
        &doubly_periodic_tensor(),
        &[&hole, &outer],
        1_000_000,
    )
    .unwrap();
    let mut patch = report.patches.unwrap().remove(0);
    let sources = patch.edge_sources().unwrap();
    for (segment, pieces) in sources[0].chunks_exact(2).enumerate() {
        assert_eq!(pieces[0].contour, 1);
        assert_eq!(pieces[1].contour, 1);
        assert_eq!(pieces[0].segment, segment);
        assert_eq!(pieces[1].segment, segment);
        assert_eq!(pieces[0].interval, [0., 0.5]);
        assert_eq!(pieces[1].interval, [0.5, 1.]);
    }
    assert_eq!(sources[0].len(), 8);
    assert_eq!(sources[1].len(), 4);
    for (segment, edge) in sources[1].iter().enumerate() {
        assert_eq!(
            (edge.contour, edge.segment, edge.interval),
            (0, segment, [0., 1.])
        );
    }
    patch.loops[0].remove(0);
    assert!(patch.edge_sources().is_err());
}

#[test]
fn periodic_trim_edge_sources_continue_segment_indices_across_chunk_clocks() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_edges;
    let outer = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let mut patch =
        build_exact_periodic_surface_trim_edges(&doubly_periodic_tensor(), &[&outer], 1_000_000)
            .unwrap()
            .patches
            .unwrap()
            .remove(0);
    let first_chunk = patch.patch.trim.loops[0][0].clone();
    patch.patch.trim.loops[0].push(first_chunk);
    let repeated_clock = patch.loops[0].clone();
    patch.loops[0].extend(repeated_clock);
    let sources = patch.edge_sources().unwrap();
    assert_eq!(
        sources[0]
            .iter()
            .map(|edge| edge.segment)
            .collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
}

#[test]
fn periodic_trim_input_edges_restore_original_segment_indices_and_direction() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [3.25, 0.75],
        [0.75, 0.75],
        [0.75, 3.25],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&hole, &outer],
        1_000_000,
    )
    .unwrap();
    let paths = &result.sources[0];
    assert_eq!(paths[0].len(), 8);
    for (oriented_segment, pieces) in paths[0].chunks_exact(2).enumerate() {
        for (edge, expected) in pieces.iter().zip([[1., 0.5], [0.5, 0.]]) {
            assert_eq!((edge.contour, edge.segment), (1, 3 - oriented_segment));
            assert_eq!(
                edge.parameters,
                expected.map(|t| num_rational::BigRational::from_float(t).unwrap()),
            );
        }
    }
    for (i, edge) in paths[1].iter().enumerate() {
        assert_eq!((edge.contour, edge.segment), (0, 3 - i));
        assert_eq!(
            edge.parameters,
            [1., 0.].map(|t| num_rational::BigRational::from_float(t).unwrap())
        );
    }
}

#[test]
fn periodic_trim_input_edges_withhold_sources_on_refusal() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let source = doubly_periodic_tensor();
    let result = build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1).unwrap();
    assert!(result.geometry.patches.is_none());
    assert!(result.sources.is_empty());
    let accepted =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    assert_eq!(accepted.sources[0][0][0].segment, 0);
    assert_eq!(
        accepted.sources[0][0][0].parameters,
        [0., 0.5].map(|t| num_rational::BigRational::from_float(t).unwrap()),
    );
}

#[test]
fn periodic_trim_input_point_evaluates_exact_rational_coordinates_across_seam() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let sample = result
        .evaluate_input_point(0, 0, &q(1, 3), 1_000_000)
        .unwrap();
    assert_eq!(sample.point, Some([q(0, 1), q(1, 6), q(1, 4)]));
    let seam = result
        .evaluate_input_point(0, 0, &q(1, 2), 1_000_000)
        .unwrap();
    assert_eq!(seam.point, Some([q(0, 1), q(0, 1), q(1, 4)]));
    let none = result
        .evaluate_input_point(3, 0, &q(1, 2), 1_000_000)
        .unwrap();
    assert!(none.point.is_none() && none.refusal.is_none());

    result.geometry.patches.as_mut().unwrap()[0].loops[0][1]
        .curve
        .control_points[0][0] = 0.125;
    let disagreement = result
        .evaluate_input_point(0, 0, &q(1, 2), 1_000_000)
        .unwrap();
    assert!(disagreement.point.is_none() && disagreement.refusal.is_some());
}

#[test]
fn periodic_trim_input_point_withholds_results_when_work_is_exhausted() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    use nurbs_core::editing::rational_factor::Refusal;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let seam = num_rational::BigRational::new(1.into(), 2.into());
    let report = result.evaluate_input_point(0, 0, &seam, 1).unwrap();
    assert!(report.point.is_none());
    assert_eq!(report.refusal, Some(Refusal::ResourceLimit));
}

#[test]
fn periodic_trim_tangents_distinguish_periodic_corner_and_follow_input_direction() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let seam = result
        .evaluate_input_tangents(0, 3, &q(1, 2), 1_000_000)
        .unwrap()
        .tangents
        .unwrap();
    assert_eq!(seam.before, Some([q(-1, 1), q(0, 1), q(0, 1)]));
    assert_eq!(seam.after, Some([q(0, 1), q(1, 1), q(0, 1)]));
    let interior = result
        .evaluate_input_tangents(0, 3, &q(1, 3), 1_000_000)
        .unwrap()
        .tangents
        .unwrap();
    assert_eq!(interior.before, seam.before);
    assert_eq!(interior.after, seam.before);
    let end = result
        .evaluate_input_tangents(0, 3, &q(1, 1), 1_000_000)
        .unwrap()
        .tangents
        .unwrap();
    assert_eq!(end.before, seam.after);
    assert_eq!(end.after, None);
}

#[test]
fn periodic_trim_tangents_withhold_values_under_exhausted_budget() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    use nurbs_core::editing::rational_factor::Refusal;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let seam = num_rational::BigRational::new(1.into(), 2.into());
    let report = result.evaluate_input_tangents(0, 0, &seam, 1).unwrap();
    assert!(report.tangents.is_none());
    assert_eq!(report.refusal, Some(Refusal::ResourceLimit));
}

#[test]
fn periodic_trim_tangent_join_classifies_exact_corner_and_endpoints() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimTangentJoin, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (n, d, expected) in [
        (1, 2, PeriodicTrimTangentJoin::Corner),
        (1, 3, PeriodicTrimTangentJoin::C1),
        (1, 1, PeriodicTrimTangentJoin::OneSided),
    ] {
        let t = num_rational::BigRational::new(n.into(), d.into());
        assert_eq!(
            result
                .classify_input_tangent_join(0, 0, &t, 1_000_000)
                .unwrap()
                .join,
            Some(expected)
        );
    }
}

#[test]
fn periodic_trim_tangent_join_distinguishes_collinear_speeds_and_reversal() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimTangentJoin, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let seam = num_rational::BigRational::new(1.into(), 2.into());
    for (y, expected) in [
        (-1., PeriodicTrimTangentJoin::G1),
        (-0.5, PeriodicTrimTangentJoin::C1),
        (0.5, PeriodicTrimTangentJoin::Reversal),
        (0., PeriodicTrimTangentJoin::Stationary),
    ] {
        let curve = &mut result.geometry.patches.as_mut().unwrap()[0].loops[0][1].curve;
        *curve.control_points.last_mut().unwrap() = vec![0., y, 0.25];
        let report = result
            .classify_input_tangent_join(0, 0, &seam, 1_000_000)
            .unwrap();
        assert_eq!(report.join, Some(expected));
    }
}

#[test]
fn periodic_trim_vertex_join_uses_adjacent_segments_and_wraps_closed_contour() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimTangentJoin, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [0.125, 0.125],
        [0.25, 0.125],
        [0.375, 0.125],
        [0.375, 0.875],
        [0.125, 0.875],
        [0.125, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (vertex, expected) in [
        (0, Some(PeriodicTrimTangentJoin::Corner)),
        (1, Some(PeriodicTrimTangentJoin::C1)),
        (2, Some(PeriodicTrimTangentJoin::Corner)),
        (5, None),
    ] {
        let report = result
            .classify_input_vertex_join(0, vertex, 1_000_000)
            .unwrap();
        assert_eq!(report.join, expected, "{report:?}");
        assert!(report.refusal.is_none());
    }
}

#[test]
fn periodic_trim_vertex_join_refuses_disconnected_geometry_and_exhaustion() {
    use nurbs_core::editing::{
        exact_periodic_surface::build_exact_periodic_surface_trim_input_edges,
        rational_factor::Refusal,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [0.125, 0.125],
        [0.375, 0.125],
        [0.375, 0.875],
        [0.125, 0.875],
        [0.125, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let exhausted = result.classify_input_vertex_join(0, 0, 1).unwrap();
    assert_eq!(exhausted.refusal, Some(Refusal::ResourceLimit));
    assert!(exhausted.join.is_none());

    result.geometry.patches.as_mut().unwrap()[0].loops[0][0]
        .curve
        .control_points[0][0] = 0.25;
    let disconnected = result.classify_input_vertex_join(0, 0, 1_000_000).unwrap();
    assert_eq!(disconnected.refusal, Some(Refusal::InvalidOutput));
    assert!(disconnected.join.is_none());
}

#[test]
fn periodic_trim_contour_audit_lists_vertices_and_seam_splits_in_input_order() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimTangentJoin, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let joins = result
        .audit_input_contour_joins(0, 1_000_000)
        .unwrap()
        .joins
        .unwrap();
    let q = |n, d| {
        num_rational::BigRational::new(num_bigint::BigInt::from(n), num_bigint::BigInt::from(d))
    };
    let expected = [
        (0, q(0, 1)),
        (1, q(0, 1)),
        (1, q(1, 2)),
        (2, q(0, 1)),
        (3, q(0, 1)),
        (3, q(1, 2)),
    ];
    assert_eq!(joins.len(), expected.len());
    for (join, (segment, parameter)) in joins.iter().zip(expected) {
        assert_eq!((join.segment, &join.parameter), (segment, &parameter));
        assert_eq!(join.join, PeriodicTrimTangentJoin::Corner);
    }
}

#[test]
fn periodic_trim_contour_audit_withholds_results_on_gap_or_exhaustion() {
    use nurbs_core::editing::{
        exact_periodic_surface::build_exact_periodic_surface_trim_input_edges,
        rational_factor::Refusal,
    };
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let mut result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let limited = result.audit_input_contour_joins(0, 1).unwrap();
    assert!(limited.joins.is_none());
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    result.sources[0][0][1].parameters[0] = num_rational::BigRational::new(3.into(), 4.into());
    let gap = result.audit_input_contour_joins(0, 1_000_000).unwrap();
    assert!(gap.joins.is_none());
    assert_eq!(gap.refusal, Some(Refusal::InvalidOutput));
}

#[test]
fn periodic_trim_contour_length_encloses_seam_perimeter_and_hole_separately() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&hole, &outer], 1_000_000)
            .unwrap();
    for (contour, expected, minimum) in [(0, 2.5, 4), (1, 5., 6)] {
        let report = result
            .measure_input_contour_length(contour, 1e-8, 100)
            .unwrap();
        assert_eq!(report.minimum_cells, minimum);
        let length = report.length.unwrap();
        assert!(
            length.bounds[0] <= expected && expected <= length.bounds[1],
            "{length:?}"
        );
        assert!(length.within_tolerance);
        assert!(length.cells <= 100);
    }
    assert!(
        result
            .measure_input_contour_length(2, 1e-8, 100)
            .unwrap()
            .length
            .is_none()
    );
}

#[test]
fn periodic_trim_contour_length_withholds_partial_perimeter_and_reports_uncertainty() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let limited = result.measure_input_contour_length(0, 1e-12, 7).unwrap();
    assert_eq!(limited.minimum_cells, 8);
    assert!(limited.length.is_none());
    let tight = result
        .measure_input_contour_length(0, 1e-30, 8)
        .unwrap()
        .length
        .unwrap();
    assert_eq!(tight.cells, 8);
    assert!(!tight.within_tolerance);
    assert!(tight.bounds[0] <= tight.bounds[1]);
}

#[test]
fn periodic_trim_segment_length_encloses_reversed_split_segment() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (segment, expected, minimum) in [(0, 1.5, 1), (3, 1., 2)] {
        let report = result
            .measure_input_segment_length(0, segment, 1e-8, 100)
            .unwrap();
        assert_eq!(report.minimum_cells, minimum);
        let length = report.length.unwrap();
        assert!(length.bounds[0] <= expected && expected <= length.bounds[1]);
        assert!(length.within_tolerance);
    }
    let limited = result.measure_input_segment_length(0, 3, 1e-8, 1).unwrap();
    assert_eq!(limited.minimum_cells, 2);
    assert!(limited.length.is_none());
}

#[test]
fn periodic_trim_segment_length_rejects_incomplete_bindings() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let mut result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    assert!(
        result
            .measure_input_segment_length(0, 4, 1e-8, 100)
            .unwrap()
            .length
            .is_none()
    );
    result.sources[0][0][1].parameters[0] = num_rational::BigRational::new(3.into(), 4.into());
    assert!(
        result
            .measure_input_segment_length(0, 0, 1e-8, 100)
            .is_err()
    );
}

#[test]
fn periodic_trim_range_length_counts_only_requested_pieces_in_either_direction() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (range, expected, minimum) in [
        ([0.125, 0.75], 0.625, 2),
        ([0.75, 0.125], 0.625, 2),
        ([0.5, 0.75], 0.25, 1),
    ] {
        let report = result
            .measure_input_segment_range_length(0, 3, range, 1e-8, 100)
            .unwrap();
        assert_eq!(report.minimum_cells, minimum);
        let length = report.length.unwrap();
        assert!(length.bounds[0] <= expected && expected <= length.bounds[1]);
        assert!(length.within_tolerance);
    }
}

#[test]
fn periodic_trim_range_length_refuses_unstorable_clock_and_insufficient_budget() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let limited = result
        .measure_input_segment_range_length(0, 0, [0.25, 0.75], 1e-8, 1)
        .unwrap();
    assert_eq!(limited.minimum_cells, 2);
    assert!(limited.length.is_none());
    assert!(
        result
            .measure_input_segment_range_length(0, 1, [0.1, 0.25], 1e-8, 100)
            .is_err()
    );
    assert!(
        result
            .measure_input_segment_range_length(0, 0, [0.5, 0.5], 1e-8, 100)
            .is_err()
    );
}

#[test]
fn periodic_trim_rational_range_encloses_unstorable_cuts_across_seam() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .measure_rational_input_segment_range_length(0, 3, [&q(4, 5), &q(1, 3)], 1e-8, 100)
        .unwrap();
    assert_eq!(report.minimum_cells, 4);
    let length = report.length.unwrap();
    assert!(num_rational::BigRational::from_float(length.bounds[0]).unwrap() <= q(7, 15));
    assert!(num_rational::BigRational::from_float(length.bounds[1]).unwrap() >= q(7, 15));
    assert!(length.within_tolerance && length.cells <= 100);
}

#[test]
fn periodic_trim_rational_range_reserves_cells_for_both_enclosures() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let limited = result
        .measure_rational_input_segment_range_length(0, 0, [&q(1, 3), &q(2, 3)], 1e-8, 3)
        .unwrap();
    assert_eq!(limited.minimum_cells, 4);
    assert!(limited.length.is_none());
    let exact = result
        .measure_rational_input_segment_range_length(0, 0, [&q(1, 4), &q(3, 4)], 1e-8, 2)
        .unwrap();
    assert_eq!(exact.minimum_cells, 2);
    assert_eq!(exact.length.unwrap().cells, 2);
}

#[test]
fn periodic_trim_length_location_brackets_split_and_reversed_input_parameters() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for distance in [0.125, 0.5, 0.75] {
        let report = result
            .locate_input_segment_at_length(0, 3, distance, 1e-10, 1e-6, 1024)
            .unwrap();
        let [lo, hi] = report.parameter_bounds.unwrap();
        assert!(lo <= distance && distance <= hi);
        assert!(hi - lo <= 1e-6);
        assert!(report.cells <= 1024);
        assert_eq!(report.status, PeriodicTrimLengthLocationStatus::Tolerance);
    }
}

#[test]
fn periodic_trim_length_location_distinguishes_outside_and_cell_limit() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (distance, max_cells, status, expected) in [
        (0., 16, Status::Tolerance, Some([0., 0.])),
        (0.5, 2, Status::WorkLimit, Some([0., 1.])),
        (0.5, 1, Status::WorkLimit, None),
        (2., 16, Status::Outside, None),
    ] {
        let report = result
            .locate_input_segment_at_length(0, 0, distance, 1e-10, 1e-6, max_cells)
            .unwrap();
        assert_eq!(report.parameter_bounds, expected);
        assert_eq!(report.status, status);
        assert!(report.cells <= max_cells);
    }
}

#[test]
fn periodic_trim_prefix_lengths_follow_input_order_across_seam() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .measure_input_contour_prefix_lengths(0, 1e-8, 100)
        .unwrap();
    assert_eq!(report.minimum_cells, 6);
    assert!(report.cells <= 100);
    let prefixes = report.vertex_prefix_bounds.unwrap();
    assert_eq!(prefixes.len(), 5);
    assert_eq!(prefixes[0], [0., 0.]);
    for ([lo, hi], expected) in prefixes.into_iter().zip([0., 1.5, 2.5, 4., 5.]) {
        assert!(lo <= expected && expected <= hi);
        assert!(hi - lo < 1e-8);
    }
}

#[test]
fn periodic_trim_prefix_lengths_reject_missing_coverage_and_omit_partial_tables() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let mut result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let limited = result
        .measure_input_contour_prefix_lengths(0, 1e-8, 7)
        .unwrap();
    assert!(limited.vertex_prefix_bounds.is_none());
    assert_eq!(limited.minimum_cells, 8);
    assert_eq!(limited.cells, 0);
    result.sources[0][0][1].parameters[0] = num_rational::BigRational::new(3.into(), 4.into());
    assert!(
        result
            .measure_input_contour_prefix_lengths(0, 1e-8, 100)
            .is_err()
    );
}

#[test]
fn periodic_trim_contour_inversion_locates_vertices_and_splits() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (distance, expected) in [(0., 0.), (0.75, 0.5), (1.5, 1.), (2., 1.5), (4.5, 3.5)] {
        let report = result
            .locate_input_contour_at_length(0, distance, 1e-10, 1e-6, 1024)
            .unwrap();
        let [lo, hi] = report.parameter_bounds.unwrap();
        assert!(lo <= expected && expected <= hi, "{report:?}");
        assert_eq!(report.status, Status::Tolerance);
        assert!(hi - lo <= 1e-6 && report.cells <= 1024);
    }
}

#[test]
fn periodic_trim_contour_inversion_retains_proved_bracket_on_cell_limit() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .locate_input_contour_at_length(0, 2., 1e-10, 1e-6, 6)
        .unwrap();
    assert_eq!(report.parameter_bounds, Some([1., 2.]));
    assert_eq!(report.status, Status::WorkLimit);
    assert_eq!(report.cells, 6);
    let outside = result
        .locate_input_contour_at_length(0, 6., 1e-10, 1e-6, 6)
        .unwrap();
    assert!(outside.parameter_bounds.is_none());
    assert_eq!(outside.status, Status::Outside);
}

#[test]
fn periodic_trim_station_bounds_enclose_seams_and_original_vertices() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    for (distance, expected) in [
        (0., [0., 0.5, 0.25]),
        (0.5, [0., 0., 0.25]),
        (1., [0.5, 0., 0.25]),
    ] {
        let report = result
            .enclose_input_contour_station(0, distance, 1e-10, 1e-6, 1024)
            .unwrap();
        for ([lo, hi], value) in report.point_bounds.unwrap().into_iter().zip(expected) {
            assert!(lo <= value && value <= hi);
            assert!(hi - lo < 1e-5);
        }
    }
}

#[test]
fn periodic_trim_station_bounds_cover_coarse_bracket_and_withhold_outside() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .enclose_input_contour_station(0, 0.5, 1e-10, 1e-6, 6)
        .unwrap();
    assert_eq!(report.location.status, Status::WorkLimit);
    let [x, y, z] = report.point_bounds.unwrap();
    assert!(x[0] <= 0. && x[1] >= 0.5 && y[0] <= 0. && y[1] >= 0.5);
    assert!(z[0] <= 0.25 && z[1] >= 0.25);
    let outside = result
        .enclose_input_contour_station(0, 6., 1e-10, 1e-6, 6)
        .unwrap();
    assert_eq!(outside.location.status, Status::Outside);
    assert!(outside.point_bounds.is_none());
}

#[test]
fn periodic_trim_station_batch_reuses_table_and_duplicate_results() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .locate_input_contour_at_lengths(0, &[1., 0.5, 1., 6.], 1e-10, 1e-6, 1024)
        .unwrap();
    assert_eq!(report.table_cells, 6);
    assert_eq!(report.stations.len(), 4);
    for (station, target) in report.stations[..3].iter().zip([1., 0.5, 1.]) {
        let [lo, hi] = station.parameter_bounds.unwrap();
        assert!(lo <= target && target <= hi);
        assert_eq!(station.status, Status::Tolerance);
    }
    assert_eq!(
        report.stations[0].parameter_bounds,
        report.stations[2].parameter_bounds
    );
    assert_eq!(report.stations[2].cells, 0);
    assert_eq!(report.stations[3].status, Status::Outside);
    assert_eq!(
        report.cells,
        report.table_cells + report.stations.iter().map(|s| s.cells).sum::<usize>()
    );
}

#[test]
fn periodic_trim_station_batch_preserves_proved_brackets_under_shared_limit() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let report = result
        .locate_input_contour_at_lengths(0, &[0.125, 0., 0.125], 1e-8, 1e-6, 8)
        .unwrap();
    assert_eq!(report.cells, 8);
    assert_eq!(report.stations[0].status, Status::WorkLimit);
    assert!(report.stations[0].parameter_bounds.is_some());
    assert_eq!(report.stations[1].parameter_bounds, Some([0., 0.]));
    assert_eq!(
        report.stations[0].parameter_bounds,
        report.stations[2].parameter_bounds
    );
}

#[test]
fn periodic_trim_equal_length_division_encloses_rational_stations() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let report = result
        .divide_input_contour_by_length(0, 3, 1e-10, 1e-6, 1024)
        .unwrap();
    assert_eq!(report.stations.len(), 4);
    for (station, expected) in report.stations.iter().zip([0., 13. / 9., 17. / 6., 4.]) {
        let [lo, hi] = station.parameter_bounds.unwrap();
        assert!(lo <= expected && expected <= hi, "{station:?}");
        assert!(hi - lo <= 1e-6);
        assert_eq!(station.status, Status::Tolerance);
    }
    assert!(report.cells <= 1024);
}

#[test]
fn periodic_trim_equal_length_division_preserves_endpoints_under_cell_limit() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let report = result
        .divide_input_contour_by_length(0, 2, 1e-8, 1e-6, 8)
        .unwrap();
    assert_eq!(report.cells, 8);
    assert_eq!(report.stations[0].parameter_bounds, Some([0., 0.]));
    assert_eq!(report.stations[1].status, Status::WorkLimit);
    assert!(report.stations[1].parameter_bounds.is_some());
    assert_eq!(report.stations[2].parameter_bounds, Some([4., 4.]));
}

#[test]
fn periodic_trim_division_boxes_enclose_points_including_closure() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let division = result
        .enclose_input_contour_divisions(0, 5, 1e-10, 1e-6, 2048)
        .unwrap();
    assert_eq!(division.stations.len(), 6);
    assert!(division.cells <= 2048);
    assert_eq!(division.table_cells, 6);
    for (station, expected) in division.stations.into_iter().zip([
        [0., 0.5, 0.25],
        [0.5, 0., 0.25],
        [0.5, 0., 1.25],
        [0., 0., 1.75],
        [0., 0.5, 1.25],
        [0., 0.5, 0.25],
    ]) {
        for ([lo, hi], value) in station.point_bounds.unwrap().into_iter().zip(expected) {
            assert!(lo <= value && value <= hi, "{station:?}");
            assert!(hi - lo < 1e-5);
        }
    }
}

#[test]
fn periodic_trim_division_boxes_cover_coarse_stations_and_skip_unproved_ones() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimLengthLocationStatus as Status, build_exact_periodic_surface_trim_input_edges,
    };
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let unresolved = result
        .enclose_input_contour_divisions(0, 2, 1e-8, 1e-6, 7)
        .unwrap();
    assert!(
        unresolved
            .stations
            .iter()
            .all(|station| station.point_bounds.is_none())
    );
    let coarse = result
        .enclose_input_contour_divisions(0, 2, 1e-8, 1e-6, 8)
        .unwrap();
    assert_eq!(coarse.stations[1].location.status, Status::WorkLimit);
    assert!(
        coarse
            .stations
            .iter()
            .all(|station| station.point_bounds.is_some())
    );
}

#[test]
fn periodic_trim_projected_area_is_exact_and_respects_input_direction() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let mut outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    for sign in [-1, 1] {
        let result =
            build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
        let report = result
            .exact_input_contour_projected_area(0, 1_000_000)
            .unwrap();
        assert_eq!(
            report.projected_area,
            Some([q(sign * 3, 4), q(sign * 3, 4), q(0, 1)])
        );
        outer.reverse();
    }
}

#[test]
fn periodic_trim_projected_area_refuses_exhaustion_and_disconnected_edges() {
    use nurbs_core::editing::{
        exact_periodic_surface::{
            PeriodicTrimProjectedAreaRefusal as Failure,
            build_exact_periodic_surface_trim_input_edges,
        },
        rational_factor::Refusal,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let limited = result.exact_input_contour_projected_area(0, 1).unwrap();
    assert!(limited.projected_area.is_none());
    assert_eq!(
        limited.refusal,
        Some(Failure::Arithmetic(Refusal::ResourceLimit))
    );
    result.geometry.patches.as_mut().unwrap()[0].loops[0][0]
        .curve
        .control_points[0][0] = 0.25;
    let invalid = result
        .exact_input_contour_projected_area(0, 1_000_000)
        .unwrap();
    assert!(invalid.projected_area.is_none());
    assert_eq!(
        invalid.refusal,
        Some(Failure::Arithmetic(Refusal::InvalidOutput))
    );
}

#[test]
fn periodic_trim_polynomial_projected_area_integrates_curved_bezier_exactly() {
    use nurbs_core::editing::exact_periodic_surface::{
        PeriodicTrimProjectedAreaRefusal as Failure, build_exact_periodic_surface_trim_input_edges,
    };
    let q = |n: i64, d: i64| num_rational::BigRational::new(n.into(), d.into());
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    {
        let first = &mut result.geometry.patches.as_mut().unwrap()[0].loops[0][0].curve;
        first.degree = 2;
        first.knots = vec![0., 0., 0., 0.5, 0.5, 0.5];
        first.control_points.insert(1, vec![0.375, 0.25, 0.25]);
        first.weights = vec![2.; 3];
    }
    assert_eq!(
        result
            .exact_input_contour_projected_area(0, 1_000_000)
            .unwrap()
            .refusal,
        Some(Failure::NonlinearEdge)
    );
    let area = result
        .exact_input_contour_polynomial_projected_area(0, 1_000_000)
        .unwrap();
    assert_eq!(area.projected_area, Some([q(-3, 4), q(-3, 4), q(-1, 16)]));

    result.geometry.patches.as_mut().unwrap()[0].loops[0][0]
        .curve
        .weights[1] = 1.;
    let rational = result
        .exact_input_contour_polynomial_projected_area(0, 1_000_000)
        .unwrap();
    assert!(rational.projected_area.is_none());
    assert_eq!(rational.refusal, Some(Failure::NonlinearEdge));
}

#[test]
fn periodic_trim_rational_area_bounds_enclose_a_weighted_curved_edge() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let first = &mut result.geometry.patches.as_mut().unwrap()[0].loops[0][0].curve;
    first.degree = 2;
    first.knots = vec![0., 0., 0., 0.5, 0.5, 0.5];
    first.control_points.insert(1, vec![0.375, 0.25, 0.25]);
    first.weights = vec![1., 0.5, 1.];
    let mut xy = 0.;
    let mut last = first.evaluate(0.).unwrap().point;
    for i in 1..=8192 {
        let point = first.evaluate(0.5 * i as f64 / 8192.).unwrap().point;
        xy += (last[0] * point[1] - last[1] * point[0]) * 0.5;
        last = point;
    }
    let report = result
        .bound_input_contour_projected_area(0, 1e-4, 20_000)
        .unwrap();
    assert!(report.within_tolerance, "{report:?}");
    for ([lo, hi], expected) in report.bounds.unwrap().into_iter().zip([-0.75, -0.75, xy]) {
        assert!(
            lo <= expected && expected <= hi,
            "{lo} <= {expected} <= {hi}"
        );
    }
}

#[test]
fn periodic_trim_rational_area_bounds_cover_entire_contour_at_cell_limit() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let limited = result
        .bound_input_contour_projected_area(0, 1e-30, 6)
        .unwrap();
    assert_eq!(limited.cells, 6);
    assert!(!limited.within_tolerance);
    for ([lo, hi], expected) in limited.bounds.unwrap().into_iter().zip([-0.75, -0.75, 0.]) {
        assert!(lo <= expected && expected <= hi);
    }
    let short = result
        .bound_input_contour_projected_area(0, 1e-6, 5)
        .unwrap();
    assert!(short.bounds.is_none());
    assert_eq!(short.minimum_cells, 6);
}

#[test]
fn periodic_trim_patch_projected_area_subtracts_holes_independently_of_input_order() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let mut outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    for _ in 0..2 {
        let result =
            build_exact_periodic_surface_trim_input_edges(&source, &[&hole, &outer], 1_000_000)
                .unwrap();
        let report = result
            .bound_input_patch_projected_area(1, 1e-8, 100)
            .unwrap();
        assert_eq!(report.minimum_cells, 10);
        assert!(report.within_tolerance);
        for ([lo, hi], expected) in report.bounds.unwrap().into_iter().zip([-0.75, -0.5, 0.]) {
            assert!(lo <= expected && expected <= hi, "{report:?}");
        }
        assert!(
            result
                .bound_input_patch_projected_area(0, 1e-8, 100)
                .unwrap()
                .bounds
                .is_none()
        );
        outer.reverse();
        hole.reverse();
    }
}

#[test]
fn periodic_trim_patch_projected_area_reserves_initial_cells_for_every_wire() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let hole = [
        [0.125, 0.125],
        [0.625, 0.125],
        [0.625, 0.625],
        [0.125, 0.625],
        [0.125, 0.125],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer, &hole],
        1_000_000,
    )
    .unwrap();
    let limited = result
        .bound_input_patch_projected_area(0, 1e-8, 11)
        .unwrap();
    assert_eq!(limited.minimum_cells, 12);
    assert_eq!(limited.cells, 0);
    assert!(limited.bounds.is_none());
    let coarse = result
        .bound_input_patch_projected_area(0, 1e-30, 12)
        .unwrap();
    assert!(coarse.bounds.is_some());
    assert!(!coarse.within_tolerance);
    assert_eq!(coarse.cells, 12);
}

#[test]
fn periodic_trim_directional_area_encloses_oblique_projection_with_hole() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer, &hole], 1_000_000)
            .unwrap();
    let expected = -1.25 / 2.0_f64.sqrt();
    for (normal, expected) in [
        ([1., 1., 0.], expected),
        ([f64::MAX, f64::MAX, 0.], expected),
        ([f64::from_bits(1), f64::from_bits(1), 0.], expected),
        ([-1., -1., 0.], -expected),
    ] {
        let report = result
            .bound_input_patch_area_along_normal(0, normal, 1e-8, 100)
            .unwrap();
        let [lo, hi] = report.bounds.unwrap();
        assert!(lo <= expected && expected <= hi);
        assert!(report.within_tolerance);
    }
}

#[test]
fn periodic_trim_directional_area_rejects_null_normal_and_preserves_budget() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    assert!(
        result
            .bound_input_patch_area_along_normal(0, [0.; 3], 1e-8, 100)
            .is_err()
    );
    let report = result
        .bound_input_patch_area_along_normal(0, [0., 0., 1.], 1e-8, 7)
        .unwrap();
    assert_eq!(report.minimum_cells, 8);
    assert!(report.bounds.is_none());
    assert!(!report.within_tolerance);
}

#[test]
fn periodic_trim_max_projection_encloses_analytic_optimum_with_hole() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&hole, &outer], 1_000_000)
            .unwrap();
    let report = result
        .bound_input_patch_max_projected_area(1, 1e-8, 100)
        .unwrap();
    let [lo, hi] = report.maximum_bounds.unwrap();
    let expected = 13.0_f64.sqrt() / 4.;
    assert!(0.75 < lo && lo <= expected && expected <= hi);
    assert!(report.within_tolerance);
    let optimal = result
        .bound_input_patch_area_along_normal(1, [-3., -2., 0.], 1e-8, 100)
        .unwrap();
    let [direction_lo, direction_hi] = optimal.bounds.unwrap();
    assert!(direction_lo <= hi && lo <= direction_hi);
    assert!(
        result
            .bound_input_patch_max_projected_area(0, 1e-8, 100)
            .unwrap()
            .maximum_bounds
            .is_none()
    );
}

#[test]
fn periodic_trim_max_projection_preserves_complete_bounds_under_cell_limit() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let outer = [
        [3.25, 3.25],
        [0.75, 3.25],
        [0.75, 0.75],
        [3.25, 0.75],
        [3.25, 3.25],
    ];
    let result = build_exact_periodic_surface_trim_input_edges(
        &doubly_periodic_tensor(),
        &[&outer],
        1_000_000,
    )
    .unwrap();
    let short = result
        .bound_input_patch_max_projected_area(0, 1e-8, 7)
        .unwrap();
    assert!(short.maximum_bounds.is_none());
    assert_eq!(short.minimum_cells, 8);
    let coarse = result
        .bound_input_patch_max_projected_area(0, 1e-30, 8)
        .unwrap();
    let [lo, hi] = coarse.maximum_bounds.unwrap();
    assert!(0. <= lo && lo <= hi);
    assert_eq!(coarse.cells, 8);
    assert!(!coarse.within_tolerance);
}

#[test]
fn periodic_trim_max_projection_direction_encloses_optimal_unit_normal() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&hole, &outer], 1_000_000)
            .unwrap();
    let optimal = result
        .bound_input_patch_max_projection_direction(1, 1e-8, 100)
        .unwrap();
    let r = 13.0_f64.sqrt();
    for ([lo, hi], expected) in
        optimal
            .normal_bounds
            .unwrap()
            .into_iter()
            .zip([-3. / r, -2. / r, 0.])
    {
        assert!(lo <= expected && expected <= hi);
        assert!(hi - lo < 1e-8);
    }
    let previous = result
        .bound_input_patch_max_projected_area(1, 1e-8, 100)
        .unwrap();
    assert_eq!(optimal.maximum.maximum_bounds, previous.maximum_bounds);
}

#[test]
fn periodic_trim_max_projection_direction_requires_nonzero_area_vector() {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_input_edges;
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let mut result =
        build_exact_periodic_surface_trim_input_edges(&source, &[&outer], 1_000_000).unwrap();
    let short = result
        .bound_input_patch_max_projection_direction(0, 1e-8, 5)
        .unwrap();
    assert!(short.normal_bounds.is_none() && short.maximum.maximum_bounds.is_none());
    for edge in &mut result.geometry.patches.as_mut().unwrap()[0].loops[0] {
        for point in &mut edge.curve.control_points {
            point[2] = 0.25;
        }
    }
    let zero = result
        .bound_input_patch_max_projection_direction(0, 1e-8, 100)
        .unwrap();
    assert!(zero.normal_bounds.is_none());
    assert_eq!(zero.maximum.maximum_bounds.unwrap()[0], 0.);
}

#[test]
fn periodic_trim_surface_area_encloses_true_area_across_seam_and_hole() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_area,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    let report = periodic_trim_patch_area(&patches[0], 0.05, 1e-10, 10_000, 1_000_000).unwrap();
    let area = report.area.unwrap();
    assert!(area.bounds[0] <= 1.25 && 1.25 <= area.bounds[1], "{area:?}");
    assert!(area.within_tolerance, "{area:?}");
    assert!(report.trim_cells <= 1_000_000 && area.cells <= 10_000);
}

#[test]
fn periodic_trim_surface_area_retains_conservative_bounds_when_trim_budget_runs_out() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_area,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    let insufficient = periodic_trim_patch_area(&patches[0], 0.01, 1e-10, 1, 1).unwrap();
    assert_eq!(insufficient.minimum_surface_cells, 2);
    assert!(insufficient.area.is_none());
    let coarse = periodic_trim_patch_area(&patches[0], 0.01, 1e-10, 2, 1).unwrap();
    let area = coarse.area.unwrap();
    assert_eq!(area.bounds[0], 0.);
    assert!(area.bounds[1] >= 1.5);
    assert!(!area.within_tolerance);
    assert_eq!(coarse.trim_cells, 1);
}

#[test]
fn trimmed_surface_area_reuses_proved_inside_cells_after_trim_budget_exhaustion() {
    use nurbs_core::{
        analysis::surface_measure::{area, trimmed_surface_area},
        surface::Surface,
        trim_domain::{Location, TrimDomain},
    };
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let wire = Curve::from_polyline(vec![
        vec![-1., -1.],
        vec![2., -1.],
        vec![2., 2.],
        vec![-1., 2.],
        vec![-1., -1.],
    ])
    .unwrap();
    let trim = TrimDomain::new(&[vec![wire]], 1e-10).unwrap();
    let initial = trim.classify([[0., 1.], [0., 1.]], 100).unwrap();
    assert_eq!(initial.location, Location::Inside);
    let proved = trimmed_surface_area(&surface, &trim, 1e-4, 5000, initial.cells).unwrap();
    let bounds = proved.area.unwrap();
    assert_eq!(proved.trim_cells, initial.cells);
    assert!(bounds.cells > 1 && bounds.within_tolerance, "{bounds:?}");
    let reference = area(&surface, 1e-5, 20_000).unwrap().bounds;
    assert!(bounds.bounds[0] <= reference[1] && reference[0] <= bounds.bounds[1]);
}

#[test]
fn trimmed_surface_area_accepts_reusable_domain_for_disjoint_support() {
    use nurbs_core::{analysis::surface_measure::trimmed_surface_area, trim_domain::TrimDomain};
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let surface = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let wire = Curve::from_polyline(vec![
        vec![5., 5.],
        vec![6., 5.],
        vec![6., 6.],
        vec![5., 6.],
        vec![5., 5.],
    ])
    .unwrap();
    let trim = TrimDomain::new(&[vec![wire]], 1e-10).unwrap();
    for _ in 0..2 {
        let report = trimmed_surface_area(&surface, &trim, 1e-8, 100, 1000).unwrap();
        let area = report.area.unwrap();
        assert!(area.bounds[0] == 0. && area.bounds[1] < 1e-8);
        assert!(area.within_tolerance);
    }
}

#[test]
fn trimmed_surface_area_window_clips_seam_spans_and_hole() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_area_in_window,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    let patch = &patches[0];
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_area_in_window(
        &patch.surface,
        &trim,
        [[3.75, 4.25], [0.375, 0.625]],
        0.01,
        5000,
        500_000,
    )
    .unwrap();
    assert_eq!(report.minimum_surface_cells, 2);
    let area = report.area.unwrap();
    assert!(
        area.bounds[0] <= 0.1875 && 0.1875 <= area.bounds[1],
        "{area:?}"
    );
    assert!(area.within_tolerance);
}

#[test]
fn trimmed_surface_area_window_returns_zero_for_disjoint_and_clips_partial_support() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_area_in_window,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for point in &mut profile.control_points {
        point.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    let patch = &patches[0];
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let empty =
        trimmed_surface_area_in_window(&patch.surface, &trim, [[5., 6.], [0.25, 0.5]], 1e-6, 1, 1)
            .unwrap();
    assert_eq!(empty.area.unwrap().bounds, [0.; 2]);
    assert_eq!(empty.trim_cells, 0);
    let partial = trimmed_surface_area_in_window(
        &patch.surface,
        &trim,
        [[3.25, 3.75], [0.25, 0.5]],
        0.01,
        5000,
        500_000,
    )
    .unwrap()
    .area
    .unwrap();
    assert!(partial.bounds[0] <= 0.125 && 0.125 <= partial.bounds[1]);
    assert!(partial.within_tolerance);
}

#[test]
fn trimmed_surface_centroid_encloses_nonplanar_centroid_with_a_hole() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_centroid,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_centroid(&patch.surface, &trim, 0.005, 10_000, 1_000_000).unwrap();
    let bounds = report.centroid_bounds.unwrap();
    for ([lo, hi], expected) in bounds.into_iter().zip([0.1, 0.15, 1.]) {
        assert!(lo <= expected && expected <= hi, "{bounds:?}");
        assert!(hi - lo < 0.6, "{bounds:?}");
    }
    assert!(report.measure.area.unwrap().bounds[0] > 0.);
}

#[test]
fn trimmed_surface_centroid_requires_proved_positive_area() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_centroid,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_centroid(&patch.surface, &trim, 0.01, 2, 1).unwrap();
    assert!(report.centroid_bounds.is_none());
    assert_eq!(report.measure.area.unwrap().bounds[0], 0.);
}

#[test]
fn trimmed_surface_inertia_encloses_nonplanar_tensor_with_hole() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_area_inertia,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report =
        trimmed_surface_area_inertia(&patch.surface, &trim, [0.; 3], 0.005, 10_000, 1_000_000)
            .unwrap();
    let tensor = report.inertia_bounds.unwrap();
    for (i, j, expected) in [
        (2, 2, 83. / 768.),
        (0, 1, 0.),
        (0, 2, -0.125),
        (1, 2, -0.1875),
    ] {
        let [lo, hi] = tensor[i][j];
        assert!(lo <= expected && expected <= hi, "{tensor:?}");
        assert_eq!(tensor[i][j], tensor[j][i]);
    }
    assert!(tensor[2][2][0] > 0.);
    assert!(tensor[2][2][1] - tensor[2][2][0] < 0.2);
}

#[test]
fn trimmed_surface_inertia_withholds_incomplete_basis_coverage() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_area_inertia,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let short = trimmed_surface_area_inertia(&patch.surface, &trim, [0.; 3], 0.01, 1, 1).unwrap();
    assert!(short.inertia_bounds.is_none());
    let coarse = trimmed_surface_area_inertia(&patch.surface, &trim, [0.; 3], 0.01, 2, 1).unwrap();
    assert!(coarse.inertia_bounds.is_some());
    assert_eq!(coarse.measure.area.unwrap().bounds[0], 0.);
}

#[test]
fn trimmed_surface_central_inertia_encloses_analytic_values_on_one_partition() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_central_inertia,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report =
        trimmed_surface_central_inertia(&patch.surface, &trim, 0.002, 20_000, 1_000_000).unwrap();
    let tensor = report.central_inertia_bounds.unwrap();
    for (i, j, expected) in [
        (2, 2, 259. / 3840.),
        (0, 1, 3. / 160.),
        (0, 2, 0.),
        (1, 2, 0.),
    ] {
        let [lo, hi] = tensor[i][j];
        assert!(lo <= expected && expected <= hi, "{tensor:?}");
        assert_eq!(tensor[i][j], tensor[j][i]);
    }
    assert!(tensor[2][2][1] - tensor[2][2][0] < 0.2);
    assert!(report.centroid.centroid_bounds.is_some());
    assert!(report.centroid.measure.area.unwrap().cells <= 20_000);
}

#[test]
fn trimmed_surface_central_inertia_requires_positive_area() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_central_inertia,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_central_inertia(&patch.surface, &trim, 0.01, 2, 1).unwrap();
    assert!(report.centroid.centroid_bounds.is_none());
    assert!(report.central_inertia_bounds.is_none());
    assert_eq!(report.centroid.measure.area.unwrap().bounds[0], 0.);
}

#[test]
fn trimmed_principal_moments_bracket_rotation_and_repeated_eigenvalues() {
    use nurbs_core::analysis::surface_measure::{
        TrimmedAreaReport, TrimmedCentralInertiaReport, TrimmedCentroidReport,
    };
    let central = TrimmedCentralInertiaReport {
        centroid: TrimmedCentroidReport {
            measure: TrimmedAreaReport {
                area: None,
                minimum_surface_cells: 0,
                trim_cells: 0,
            },
            centroid_bounds: None,
        },
        central_inertia_bounds: Some(
            [[2., 1., 0.], [1., 2., 0.], [0., 0., 3.]].map(|row| row.map(|x| [x; 2])),
        ),
    };
    let report = central.bound_principal_moments(1e-8).unwrap();
    assert!(report.within_moment_tolerance);
    for ([lo, hi], expected) in report
        .principal_bounds
        .unwrap()
        .into_iter()
        .zip([1., 3., 3.])
    {
        assert!(lo <= expected && expected <= hi);
        assert!(hi - lo <= 1e-8);
    }
}

#[test]
fn trimmed_principal_moments_keep_uncertain_eigenvalue_ranges() {
    use nurbs_core::analysis::surface_measure::{
        TrimmedAreaReport, TrimmedCentralInertiaReport, TrimmedCentroidReport,
    };
    let mut central = TrimmedCentralInertiaReport {
        centroid: TrimmedCentroidReport {
            measure: TrimmedAreaReport {
                area: None,
                minimum_surface_cells: 0,
                trim_cells: 0,
            },
            centroid_bounds: None,
        },
        central_inertia_bounds: None,
    };
    assert!(
        central
            .clone()
            .bound_principal_moments(1e-8)
            .unwrap()
            .principal_bounds
            .is_none()
    );
    let mut tensor = [[[0.; 2]; 3]; 3];
    for (i, row) in tensor.iter_mut().enumerate() {
        row[i] = [1., 2.];
    }
    central.central_inertia_bounds = Some(tensor);
    let report = central.bound_principal_moments(1e-8).unwrap();
    assert!(!report.within_moment_tolerance);
    assert!(
        report
            .principal_bounds
            .unwrap()
            .iter()
            .all(|b| b[0] <= 1. && b[1] >= 2.)
    );
}

#[test]
fn trimmed_surface_window_inertia_encloses_analytic_properties_after_clipping() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_central_inertia_in_window,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_central_inertia_in_window(
        &patch.surface,
        &trim,
        [[3.75, 4.25], [0.375, 0.625]],
        0.001,
        10_000,
        500_000,
    )
    .unwrap();
    let area = report.centroid.measure.area.unwrap();
    assert!(area.bounds[0] <= 0.1875 && 0.1875 <= area.bounds[1]);
    for ([lo, hi], expected) in
        report
            .centroid
            .centroid_bounds
            .unwrap()
            .into_iter()
            .zip([1. / 48., 1. / 12., 1.])
    {
        assert!(lo <= expected && expected <= hi);
    }
    let tensor = report.central_inertia_bounds.unwrap();
    for (i, j, expected) in [(2, 2, 19. / 12288.), (0, 1, 1. / 3072.)] {
        let [lo, hi] = tensor[i][j];
        assert!(lo <= expected && expected <= hi, "{tensor:?}");
    }
}

#[test]
fn trimmed_surface_window_inertia_withholds_for_disjoint_window() {
    use nurbs_core::{
        analysis::surface_measure::trimmed_surface_central_inertia_in_window,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
        trim_domain::TrimDomain,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patch = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
        .remove(0);
    let trim = TrimDomain::new(&patch.trim.loops, 1e-10).unwrap();
    let report = trimmed_surface_central_inertia_in_window(
        &patch.surface,
        &trim,
        [[5., 6.], [0.25, 0.5]],
        0.01,
        1,
        1,
    )
    .unwrap();
    assert!(report.centroid.centroid_bounds.is_none() && report.central_inertia_bounds.is_none());
    assert_eq!(report.centroid.measure.area.unwrap().bounds, [0.; 2]);
}

#[test]
fn periodic_trim_patch_set_area_includes_island_under_shared_budget() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_set_area,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let island = [
        [0.1875, 0.375],
        [0.3125, 0.375],
        [0.3125, 0.625],
        [0.1875, 0.625],
        [0.1875, 0.375],
    ];
    let patches =
        build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole, &island], 1_000_000)
            .unwrap()
            .patches
            .unwrap();
    let result = periodic_trim_patch_set_area(&patches, 0.075, 1e-10, 10_000, 1_000_000).unwrap();
    let total = result.total.area.unwrap();
    assert!(
        total.bounds[0] <= 1.3125 && 1.3125 <= total.bounds[1],
        "{total:?}"
    );
    assert!(
        total.within_tolerance && total.cells <= 10_000 && result.total.trim_cells <= 1_000_000,
        "{total:?}; {:?}", result.components,
    );
    assert_eq!(result.components.len(), 2);
    for (area, expected) in result.components.iter().zip([1.25, 0.0625]) {
        assert!(area.bounds[0] <= expected && expected <= area.bounds[1]);
    }
    assert_eq!(
        total.cells,
        result
            .components
            .iter()
            .map(|area| area.cells)
            .sum::<usize>()
    );
}

#[test]
fn periodic_trim_patch_set_area_never_returns_partial_component_coverage() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_set_area,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let a = [
        [0.125, 0.125],
        [0.375, 0.125],
        [0.375, 0.375],
        [0.125, 0.375],
        [0.125, 0.125],
    ];
    let b = [
        [2.125, 0.625],
        [2.375, 0.625],
        [2.375, 0.875],
        [2.125, 0.875],
        [2.125, 0.625],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&a, &b], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    for (surface_cells, trim_cells) in [(1, 100), (100, 1)] {
        let short =
            periodic_trim_patch_set_area(&patches, 0.01, 1e-10, surface_cells, trim_cells).unwrap();
        assert!(short.total.area.is_none() && short.components.is_empty());
        assert_eq!(short.total.minimum_surface_cells, 2);
    }
    let coarse = periodic_trim_patch_set_area(&patches, 0.01, 1e-10, 2, 2).unwrap();
    assert_eq!(coarse.components.len(), 2);
    assert_eq!(coarse.total.trim_cells, 2);
    assert!(coarse.total.area.unwrap().bounds[1] >= 0.25);
}

#[test]
fn periodic_trim_patch_set_centroid_weights_islands_by_area() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_set_centroid,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let island = [
        [0.1875, 0.375],
        [0.3125, 0.375],
        [0.3125, 0.625],
        [0.1875, 0.625],
        [0.1875, 0.375],
    ];
    let patches =
        build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole, &island], 1_000_000)
            .unwrap()
            .patches
            .unwrap();
    let report =
        periodic_trim_patch_set_centroid(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    for ([lo, hi], expected) in
        report
            .centroid_bounds
            .unwrap()
            .into_iter()
            .zip([3. / 28., 1. / 7., 1.])
    {
        assert!(lo <= expected && expected <= hi);
        assert!(hi - lo < 0.6);
    }
    assert_eq!(report.measure.components.len(), 2);
}

#[test]
fn periodic_trim_patch_set_centroid_withholds_when_total_area_is_uncertain() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_set_centroid,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    let report = periodic_trim_patch_set_centroid(&patches, 0.01, 1e-10, 2, 1).unwrap();
    assert!(report.centroid_bounds.is_none());
    assert_eq!(report.measure.total.area.unwrap().bounds[0], 0.);
}

fn periodic_trim_patches_with_nested_island()
-> Vec<nurbs_core::editing::exact_periodic_surface::PeriodicTrimPatch> {
    use nurbs_core::editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches;

    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let island = [
        [0.1875, 0.375],
        [0.3125, 0.375],
        [0.3125, 0.625],
        [0.1875, 0.625],
        [0.1875, 0.375],
    ];
    build_exact_periodic_surface_trim_patches(&source, &[&outer, &hole, &island], 1_000_000)
        .unwrap()
        .patches
        .unwrap()
}

#[test]
fn periodic_trim_patch_set_central_inertia_includes_offset_components() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let report =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let tensor = report.central_inertia_bounds.unwrap();
    let [lo, hi] = tensor[2][2];
    let expected = 83. / 768. + 49. / 12288. - 75. / 1792.;
    assert!(lo <= expected && expected <= hi, "{tensor:?}");
    let expected_xy = 1.3125 * (3. / 28.) * (1. / 7.);
    assert!(tensor[0][1][0] <= expected_xy && expected_xy <= tensor[0][1][1]);
    assert_eq!(tensor[0][1], tensor[1][0]);
    assert!(report.centroid.measure.total.area.unwrap().cells <= 20_000);
}

#[test]
fn periodic_trim_patch_set_central_inertia_requires_full_positive_area_proof() {
    use nurbs_core::{
        analysis::surface_measure::periodic_trim_patch_set_central_inertia,
        editing::exact_periodic_surface::build_exact_periodic_surface_trim_patches,
    };
    let mut profile = periodic_unit_square();
    for pole in &mut profile.control_points {
        pole.push(0.);
    }
    let source = nurbs_core::surface::extrude(&profile, [0., 0., 2.]).unwrap();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let patches = build_exact_periodic_surface_trim_patches(&source, &[&outer], 1_000_000)
        .unwrap()
        .patches
        .unwrap();
    for (cells, area_present) in [(1, false), (2, true)] {
        let report =
            periodic_trim_patch_set_central_inertia(&patches, 0.01, 1e-10, cells, 1).unwrap();
        assert!(report.central_inertia_bounds.is_none());
        assert!(report.centroid.centroid_bounds.is_none());
        assert_eq!(report.centroid.measure.total.area.is_some(), area_present);
    }
}

#[test]
fn periodic_trim_patch_set_principal_moments_bracket_analytic_island_case() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let central =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let coarse = central.clone().bound_principal_moments(10.).unwrap();
    assert!(coarse.within_moment_tolerance);
    let report = central.bound_principal_moments(1e-8).unwrap();
    let area: f64 = 1.3125;
    let cx = 3. / 28.;
    let cy = 1. / 7.;
    let var_x = 1. / 16. - 13. / 768. + 49. / 12288. - area * cx * cx;
    let var_y = 1. / 16. - area * cy * cy;
    let var_z = 1.5 * 1.5_f64.powi(2) / 12. - 0.25 / 12. + 0.0625 * 0.25 / 12.;
    let coupling = area * cx * cy;
    let split = ((var_x - var_y) * 0.5).hypot(coupling);
    let average = var_z + (var_x + var_y) * 0.5;
    let mut expected = [var_x + var_y, average - split, average + split];
    expected.sort_by(f64::total_cmp);
    let bounds = report.principal_bounds.unwrap();
    for ([lo, hi], reference) in bounds.into_iter().zip(expected) {
        assert!(
            lo <= reference && reference <= hi,
            "{bounds:?}, {expected:?}"
        );
    }
    assert!(!report.within_moment_tolerance);
}

#[test]
fn periodic_trim_patch_set_principal_moments_require_complete_inertia() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let central =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    for bad_tolerance in [0., -1., f64::INFINITY, f64::NAN] {
        assert!(
            central
                .clone()
                .bound_principal_moments(bad_tolerance)
                .is_err()
        );
    }
    let report = central.bound_principal_moments(0.1).unwrap();
    assert!(report.principal_bounds.is_none());
    assert!(!report.within_moment_tolerance);
}

#[test]
fn periodic_trim_patch_set_principal_moments_resolve_repeated_exact_values() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let mut central =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    central.central_inertia_bounds = Some([
        [[3.; 2], [1.; 2], [0.; 2]],
        [[1.; 2], [3.; 2], [0.; 2]],
        [[0.; 2], [0.; 2], [4.; 2]],
    ]);
    let report = central.bound_principal_moments(1e-8).unwrap();
    assert!(report.within_moment_tolerance);
    for ([lo, hi], reference) in report
        .principal_bounds
        .unwrap()
        .into_iter()
        .zip([2., 4., 4.])
    {
        assert!(lo <= reference && reference <= hi);
    }
}

#[test]
fn interval_tensor_eigenvalues_keep_individual_bounds_under_entry_uncertainty() {
    use nurbs_core::analysis::surface_measure::bound_principal_moments_from_tensor;

    let tensor = [
        [[12., 20.], [-0.5, 1.], [0.; 2]],
        [[-0.5, 1.], [1., 8.], [0.; 2]],
        [[0.; 2], [0.; 2], [6., 11.]],
    ];
    // A large stopping tolerance bypasses spectral bisection, isolating
    // the individual bounds from the diagonal order statistics.
    let ranges = bound_principal_moments_from_tensor(tensor, 100.).unwrap();
    for (range, expected) in ranges.into_iter().zip([[0., 9.], [5., 12.], [11., 21.]]) {
        for axis in 0..2 {
            assert!((range[axis] - expected[axis]).abs() < 1e-12, "{ranges:?}");
        }
    }
    for a in [12., 20.] {
        for b in [1., 8.] {
            for c in [6., 11.] {
                for off in [-0.5, 0., 1.] {
                    let split = ((a - b) / 2.0_f64).hypot(off);
                    let mut eigenvalues = [(a + b) / 2. - split, (a + b) / 2. + split, c];
                    eigenvalues.sort_by(f64::total_cmp);
                    for ([lo, hi], eigenvalue) in ranges.into_iter().zip(eigenvalues) {
                        assert!(lo <= eigenvalue && eigenvalue <= hi);
                    }
                }
            }
        }
    }
}

#[test]
fn interval_tensor_eigenvalues_validate_bounds_and_tolerance() {
    use nurbs_core::analysis::surface_measure::bound_principal_moments_from_tensor;

    let mut tensor = [[[0.; 2]; 3]; 3];
    assert!(bound_principal_moments_from_tensor(tensor, 0.).is_err());
    tensor[0][1] = [0., 1.];
    assert!(bound_principal_moments_from_tensor(tensor, 1.).is_err());
    tensor[1][0] = tensor[0][1];
    tensor[1][1] = [2., 1.];
    assert!(bound_principal_moments_from_tensor(tensor, 1.).is_err());
    tensor[1][1] = [1., f64::INFINITY];
    assert!(bound_principal_moments_from_tensor(tensor, 1.).is_err());
}

#[test]
fn directional_inertia_brackets_coupled_interval_tensor_at_all_axis_scales() {
    use nurbs_core::analysis::surface_measure::bound_directional_inertia;

    let tensor = [
        [[2., 4.], [0.5, 1.], [0.; 2]],
        [[0.5, 1.], [6., 8.], [0.; 2]],
        [[0.; 2], [0.; 2], [9.; 2]],
    ];
    for scale in [f64::from_bits(1), 1., f64::MAX] {
        for (direction, expected) in [
            ([scale, scale, 0.], [4.5, 7.]),
            ([-scale, scale, 0.], [3., 5.5]),
        ] {
            let [lo, hi] = bound_directional_inertia(tensor, direction).unwrap();
            assert!(lo <= expected[0] && hi >= expected[1]);
            assert!((lo - expected[0]).abs() < 1e-12 && (hi - expected[1]).abs() < 1e-12);
        }
    }
    assert_eq!(
        bound_directional_inertia(tensor, [0., 0., -2.]).unwrap(),
        [9.; 2]
    );
    for direction in [[0.; 3], [f64::NAN, 0., 0.], [0., f64::INFINITY, 0.]] {
        assert!(bound_directional_inertia(tensor, direction).is_err());
    }
}

#[test]
fn periodic_trim_directional_inertia_includes_island_about_common_centroid() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let report =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let [lo, hi] = report.bound_axis_moment([1., 1., 0.]).unwrap().unwrap();
    let area = 1.3125;
    let cx = 3. / 28.;
    let cy = 1. / 7.;
    let var_x = 1. / 16. - 13. / 768. + 49. / 12288. - area * cx * cx;
    let var_y = 1. / 16. - area * cy * cy;
    let var_z = 1.5 * 1.5_f64.powi(2) / 12. - 0.25 / 12. + 0.0625 * 0.25 / 12.;
    let expected = var_z + (var_x + var_y) * 0.5 + area * cx * cy;
    assert!(lo <= expected && expected <= hi);

    let incomplete =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    assert_eq!(incomplete.bound_axis_moment([1., 1., 0.]).unwrap(), None);
}

#[test]
fn inertia_translation_brackets_uncertain_centroid_and_preserves_symmetry() {
    use nurbs_core::analysis::surface_measure::bound_inertia_about_point;

    let central = [[2., 1., 0.], [1., 4., 0.], [0., 0., 7.]].map(|row| row.map(|x| [x; 2]));
    let target = [1., 1., 5.];
    let translated =
        bound_inertia_about_point(central, [2., 3.], [[2., 3.], [-1., 0.], [5.; 2]], target)
            .unwrap();
    for mass in [2., 3.] {
        for dx in [1., 2.] {
            for dy in [-2., -1.] {
                let shifted = [
                    [2. + mass * dy * dy, 1. - mass * dx * dy, 0.],
                    [1. - mass * dx * dy, 4. + mass * dx * dx, 0.],
                    [0., 0., 7. + mass * (dx * dx + dy * dy)],
                ];
                for (i, row) in shifted.iter().enumerate() {
                    for (j, &reference) in row.iter().enumerate() {
                        let [lo, hi] = translated[i][j];
                        assert!(lo <= reference && reference <= hi);
                        assert_eq!(translated[i][j], translated[j][i]);
                    }
                }
            }
        }
    }
    let coincident = target.map(|x| [x; 2]);
    assert_eq!(
        bound_inertia_about_point(central, [2., 3.], coincident, target).unwrap(),
        central
    );
    assert!(bound_inertia_about_point(central, [-1., 2.], coincident, target).is_err());
    assert!(bound_inertia_about_point(central, [2., 3.], coincident, [f64::NAN; 3]).is_err());
}

#[test]
fn periodic_trim_inertia_about_fixed_point_includes_nested_island() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let report =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let translated = report.bound_inertia_about_point([0.; 3]).unwrap().unwrap();
    let uncentered_z = 83. / 768. + 49. / 12288.;
    assert!(translated[2][2][0] <= uncentered_z && uncentered_z <= translated[2][2][1]);
    assert!(translated[0][1][0] <= 0. && 0. <= translated[0][1][1]);
    assert_eq!(translated[0][1], translated[1][0]);

    let incomplete =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    assert!(
        incomplete.bound_inertia_about_point([0.; 3]).unwrap().is_none()
    );
}

#[test]
fn line_inertia_is_invariant_under_axis_reversal_and_motion_along_axis() {
    use nurbs_core::analysis::surface_measure::bound_inertia_about_line;

    let central = [[2., 1., 0.], [1., 4., 0.], [0., 0., 7.]].map(|row| row.map(|x| [x; 2]));
    let centroid = [[2.; 2], [0.; 2], [3.; 2]];
    for shift in [-10., 0., 100.] {
        let result =
            bound_inertia_about_line(central, [2.; 2], centroid, [shift, shift, 3.], [-2., -2., 0.])
                .unwrap();
        assert!(result[0] <= 8. && result[1] >= 8.);
        assert!(result[1] - result[0] < 1e-10);
    }
    let result =
        bound_inertia_about_line(central, [2.; 2], centroid, [0., 0., f64::MAX], [0., 0., 1.])
            .unwrap();
    assert!(result[0] <= 15. && result[1] >= 15.);
    assert!(result[1] - result[0] < 1e-12);
    assert!(bound_inertia_about_line(central, [2.; 2], centroid, [0.; 3], [0.; 3]).is_err());
}

#[test]
fn periodic_trim_line_inertia_brackets_offset_axis_with_nested_island() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let report =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let [lo, hi] = report
        .bound_inertia_about_line([1., 0., 5.], [0., 0., 1.])
        .unwrap()
        .unwrap();
    let area = 1.3125;
    let first_x = area * 3. / 28.;
    let at_origin = 83. / 768. + 49. / 12288.;
    let expected = at_origin - 2. * first_x + area;
    assert!(lo <= expected && expected <= hi);

    let incomplete =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    assert_eq!(
        incomplete.bound_inertia_about_line([1., 0., 5.], [0., 0., 1.]).unwrap(),
        None
    );
}

#[test]
fn radius_of_gyration_encloses_positive_mass_and_avoids_intermediate_overflow() {
    use nurbs_core::analysis::surface_measure::bound_radius_of_gyration;

    for (moment, mass, expected) in [
        ([8., 16.], [1., 2.], [2., 4.]),
        ([-1e-10, 16.], [1., 2.], [0., 4.]),
        ([1e300; 2], [1e-300; 2], [1e300; 2]),
        ([f64::from_bits(1); 2], [f64::from_bits(1); 2], [1.; 2]),
    ] {
        let bounds = bound_radius_of_gyration(moment, mass).unwrap();
        assert!(bounds[0] <= expected[0] && expected[1] <= bounds[1]);
        assert!(bounds[0] >= 0.);
        assert!((bounds[0] - expected[0]).abs() <= expected[0] * 1e-12);
        assert!((bounds[1] - expected[1]).abs() <= expected[1] * 1e-12);
    }
    assert_eq!(bound_radius_of_gyration([0.; 2], [1.; 2]).unwrap(), [0.; 2]);
    assert!(bound_radius_of_gyration([8.; 2], [0., 1.]).is_err());
    assert!(bound_radius_of_gyration([-8.; 2], [1.; 2]).is_err());
}

#[test]
fn periodic_trim_line_radius_of_gyration_encloses_offset_axis_with_nested_island() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_central_inertia;

    let patches = periodic_trim_patches_with_nested_island();
    let report =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000).unwrap();
    let [lo, hi] = report
        .bound_line_radius_of_gyration([1., 0., 5.], [0., 0., 1.])
        .unwrap()
        .unwrap();
    let area = 1.3125;
    let at_origin = 83. / 768. + 49. / 12288.;
    let expected = (at_origin / area + 1. - 2. * 3. / 28_f64).sqrt();
    assert!(lo <= expected && expected <= hi);
    assert!(lo > 0.);

    let incomplete =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    assert_eq!(
        incomplete
            .bound_line_radius_of_gyration([1., 0., 5.], [0., 0., 1.])
            .unwrap(),
        None
    );
}

#[test]
fn combined_surface_mass_properties_account_for_separated_centers() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceMassPropertiesBounds, combine_surface_mass_properties,
    };
    let pieces = [(2., 0.), (1., 3.)].map(|(area, x)| SurfaceMassPropertiesBounds {
        area_bounds: [area; 2],
        centroid_bounds: [[x; 2], [0.; 2], [0.; 2]],
        central_inertia_bounds: [[1., 0., 0.], [0., 2., 0.], [0., 0., 3.]]
            .map(|row| row.map(|x| [x; 2])),
    });
    let combined = combine_surface_mass_properties(&pieces).unwrap();
    for (interval, expected) in [
        (combined.area_bounds, 3.),
        (combined.centroid_bounds[0], 1.),
        (combined.central_inertia_bounds[0][0], 2.),
        (combined.central_inertia_bounds[1][1], 10.),
        (combined.central_inertia_bounds[2][2], 12.),
    ] {
        assert!(interval[0] <= expected && expected <= interval[1]);
        assert!(interval[1] - interval[0] < 1e-12);
    }
    assert!(combine_surface_mass_properties(&[]).is_err());
    let mut bad = pieces;
    bad[1].area_bounds[0] = 0.;
    assert!(combine_surface_mass_properties(&bad).is_err());
}

#[test]
fn combine_periodic_trim_reports_preserves_island_mass_properties() {
    use nurbs_core::analysis::surface_measure::{
        combine_surface_mass_properties, periodic_trim_patch_set_central_inertia,
    };

    let patches = periodic_trim_patches_with_nested_island();
    let pieces = patches
        .iter()
        .map(|patch| {
            periodic_trim_patch_set_central_inertia(
                std::slice::from_ref(patch),
                0.005,
                1e-10,
                20_000,
                1_000_000,
            )
            .unwrap()
            .mass_properties_bounds()
            .unwrap()
        })
        .collect::<Vec<_>>();
    let combined = combine_surface_mass_properties(&pieces).unwrap();
    let expected_zz = 83. / 768. + 49. / 12288. - 75. / 1792.;
    for (interval, expected) in [
        (combined.area_bounds, 1.3125),
        (combined.centroid_bounds[0], 3. / 28.),
        (combined.central_inertia_bounds[2][2], expected_zz),
    ] {
        assert!(interval[0] <= expected && expected <= interval[1]);
    }
    let incomplete =
        periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 2, 1_000_000).unwrap();
    assert!(incomplete.mass_properties_bounds().is_none());
}

#[test]
fn density_weighted_assembly_brackets_mass_center_and_parallel_axis_correction() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceMassPropertiesBounds, combine_surface_mass_properties_with_density,
    };
    let pieces = [(2., 0., [1., 2.]), (1., 3., [3., 4.])].map(|(area, x, density)| {
        (
            SurfaceMassPropertiesBounds {
                area_bounds: [area; 2],
                centroid_bounds: [[x; 2], [0.; 2], [0.; 2]],
                central_inertia_bounds: [[1., 0., 0.], [0., 2., 0.], [0., 0., 3.]]
                    .map(|row| row.map(|x| [x; 2])),
            },
            density,
        )
    });
    let assembled = combine_surface_mass_properties_with_density(&pieces).unwrap();
    for first in [1., 2.] {
        for second in [3., 4.] {
            let mass = 2. * first + second;
            let center = 3. * second / mass;
            let correction = 18. * first * second / mass;
            for (bounds, expected) in [
                (assembled.mass_bounds, mass),
                (assembled.centroid_bounds[0], center),
                (assembled.central_inertia_bounds[0][0], first + second),
                (assembled.central_inertia_bounds[1][1], 2. * (first + second) + correction),
                (assembled.central_inertia_bounds[2][2], 3. * (first + second) + correction),
            ] {
                assert!(bounds[0] <= expected && expected <= bounds[1]);
            }
        }
    }
}

#[test]
fn density_weighted_assembly_rejects_invalid_density_and_matches_unit_density() {
    use nurbs_core::analysis::surface_measure::{
        combine_surface_mass_properties, combine_surface_mass_properties_with_density,
        periodic_trim_patch_set_central_inertia,
    };
    let patches = periodic_trim_patches_with_nested_island();
    let piece = periodic_trim_patch_set_central_inertia(&patches, 0.005, 1e-10, 20_000, 1_000_000)
        .unwrap()
        .mass_properties_bounds()
        .unwrap();
    let unit = combine_surface_mass_properties(&[piece]).unwrap();
    let weighted = combine_surface_mass_properties_with_density(&[(piece, [1.; 2])]).unwrap();
    assert!(weighted.mass_bounds[0] <= unit.area_bounds[0]);
    assert!(weighted.mass_bounds[1] >= unit.area_bounds[1]);
    for density in [[0., 1.], [2., 1.], [-1., 1.], [1., f64::INFINITY]] {
        assert!(combine_surface_mass_properties_with_density(&[(piece, density)]).is_err());
    }
}

#[test]
fn periodic_trim_set_measures_density_weighted_mass_on_a_shared_partition_budget() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;

    let patches = periodic_trim_patches_with_nested_island();
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &[[2.; 2], [6.; 2]], 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let properties = report.mass_properties.unwrap();
    let mass = 2.875;
    let center_x = (2. * 0.125 + 6. * 0.015625) / mass;
    let center_y = 2. * 0.1875 / mass;
    let inertia_z = 2. * 83. / 768. + 6. * 49. / 12288.
        - mass * (center_x * center_x + center_y * center_y);
    for ([lo, hi], expected) in [
        (properties.mass_bounds, mass),
        (properties.centroid_bounds[0], center_x),
        (properties.centroid_bounds[1], center_y),
        (properties.central_inertia_bounds[2][2], inertia_z),
    ] {
        assert!(lo <= expected && expected <= hi, "{lo} .. {hi}: {expected}");
    }
    assert_eq!(report.measure.components.len(), 2);
    assert!(report.measure.total.area.unwrap().cells <= 20_000);
    assert!(report.measure.total.trim_cells <= 1_000_000);
}

#[test]
fn periodic_trim_set_density_mass_requires_aligned_inputs_and_full_coverage() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;

    let patches = periodic_trim_patches_with_nested_island();
    for densities in [&[[1.; 2]][..], &[[1.; 2], [0., 1.]][..]] {
        assert!(
            periodic_trim_patch_set_mass_with_density(
                &patches, densities, 0.005, 1e-10, 20_000, 1_000_000,
            )
            .is_err()
        );
    }
    for (surface_cells, trim_cells) in [(2, 1_000_000), (20_000, 1)] {
        let report = periodic_trim_patch_set_mass_with_density(
            &patches, &[[2.; 2], [6.; 2]], 0.005, 1e-10, surface_cells, trim_cells,
        )
        .unwrap();
        assert!(report.mass_properties.is_none());
        assert!(report.measure.total.area.is_none());
        assert!(report.measure.components.is_empty());
    }
}

#[test]
fn periodic_trim_set_reports_mass_of_each_component_under_density_ranges() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;

    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2., 3.], [6., 8.]];
    let report =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 20_000, 1_000_000)
            .unwrap();
    assert_eq!(report.component_mass_bounds.len(), 2);
    for ([lo, hi], [minimum, maximum]) in report
        .component_mass_bounds
        .into_iter()
        .zip([[2.5, 3.75], [0.375, 0.5]])
    {
        assert!(lo <= minimum && maximum <= hi);
    }
    let [lo, hi] = report.total_mass_bounds.unwrap();
    assert!(lo <= 2.875 && 4.25 <= hi);

    let incomplete =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 2, 1_000_000)
            .unwrap();
    assert!(incomplete.component_mass_bounds.is_empty());
    assert!(incomplete.total_mass_bounds.is_none());
}

#[test]
fn component_mass_fractions_preserve_shared_denominator_dependence() {
    use nurbs_core::analysis::surface_measure::bound_component_mass_fractions;

    let bounds = bound_component_mass_fractions(&[[2., 4.], [1., 2.], [0., 0.]])
        .unwrap()
        .unwrap();
    for (actual, expected) in bounds.into_iter().zip([[0.5, 0.8], [0.2, 0.5], [0., 0.]]) {
        assert!(actual[0] <= expected[0] && expected[1] <= actual[1]);
        for (endpoint, reference) in actual.into_iter().zip(expected) {
            assert!((endpoint - reference).abs() < 1e-14);
        }
    }
    assert_eq!(
        bound_component_mass_fractions(&[[0., 4.], [0., 2.]]).unwrap(),
        None
    );
    assert!(bound_component_mass_fractions(&[[-1., 2.]]).is_err());
    let tiny = f64::from_bits(1);
    let tiny_bounds = bound_component_mass_fractions(&[[tiny; 2], [tiny; 2]])
        .unwrap()
        .unwrap();
    assert!(tiny_bounds.iter().all(|[lo, hi]| *lo <= 0.5 && 0.5 <= *hi));
}

#[test]
fn periodic_trim_component_mass_fractions_follow_patch_order() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;

    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2.; 2], [6.; 2]];
    let report =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 20_000, 1_000_000)
            .unwrap();
    let fractions = report.component_mass_fractions().unwrap().unwrap();
    for ([lo, hi], reference) in fractions.into_iter().zip([20. / 23., 3. / 23.]) {
        assert!(lo <= reference && reference <= hi);
    }
    let incomplete =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 2, 1_000_000)
            .unwrap();
    assert!(incomplete.component_mass_fractions().unwrap().is_none());
}

#[test]
fn weighted_centroid_brackets_exact_interval_corner_extrema_tightly() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_mass_weighted_centroid,
    };

    let pieces = [
        ([6., 12.], [[10., 11.], [0.; 2], [-10., -9.]]),
        ([2., 8.], [[-2., 0.], [0.; 2], [3., 4.]]),
        ([1., 3.], [[2., 3.], [0.; 2], [1., 2.]]),
    ]
    .map(|(mass_bounds, centroid_bounds)| SurfaceAssemblyMassBounds {
        mass_bounds,
        centroid_bounds,
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    let bounds = bound_mass_weighted_centroid(&pieces).unwrap();
    for (axis, [lo, hi]) in bounds.into_iter().enumerate() {
        let mut reference = [f64::INFINITY, f64::NEG_INFINITY];
        for side in 0..2 {
            for mask in 0..8 {
                let mut sum = 0.;
                let mut mass = 0.;
                for (i, piece) in pieces.iter().enumerate() {
                    let weight = piece.mass_bounds[(mask >> i) & 1];
                    sum += weight * piece.centroid_bounds[axis][side];
                    mass += weight;
                }
                reference[0] = reference[0].min(sum / mass);
                reference[1] = reference[1].max(sum / mass);
            }
        }
        assert!(lo <= reference[0] && hi >= reference[1]);
        assert!((lo - reference[0]).abs() < 1e-12 && (hi - reference[1]).abs() < 1e-12);
    }
}

#[test]
fn weighted_centroid_handles_one_piece_and_rejects_unproved_mass() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_mass_weighted_centroid,
    };
    let mut piece = SurfaceAssemblyMassBounds {
        mass_bounds: [f64::from_bits(1); 2],
        centroid_bounds: [[-3., 1.], [4., 5.], [7.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    };
    assert_eq!(
        bound_mass_weighted_centroid(&[piece]).unwrap(),
        piece.centroid_bounds
    );
    piece.mass_bounds = [0., 1.];
    assert!(bound_mass_weighted_centroid(&[piece]).is_err());
    assert!(bound_mass_weighted_centroid(&[]).is_err());
}

#[test]
fn weighted_center_plane_distance_retains_cancellation_across_world_axes() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_mass_weighted_centroid_plane_distance,
    };
    let pieces = [1., 10.].map(|x| SurfaceAssemblyMassBounds {
        mass_bounds: [1., 9.],
        centroid_bounds: [[x; 2], [x; 2], [0.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    for scale in [f64::from_bits(1), 1., f64::MAX] {
        let [lo, hi] = bound_mass_weighted_centroid_plane_distance(
            &pieces, [2., 0., 0.], [scale, -scale, 0.],
        )
        .unwrap();
        let reference = -2_f64.sqrt();
        assert!(lo <= reference && reference <= hi);
        assert!(hi - lo < 1e-12);
    }
}

#[test]
fn weighted_center_plane_distance_brackets_uncertain_positions_and_masses() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_mass_weighted_centroid_plane_distance,
    };
    let pieces = [([1., 2.], [2., 3.]), ([3., 4.], [10., 12.])].map(|(mass, x)| {
        SurfaceAssemblyMassBounds {
            mass_bounds: mass,
            centroid_bounds: [x, [0.; 2], [5.; 2]],
            central_inertia_bounds: [[[0.; 2]; 3]; 3],
        }
    });
    let [lo, hi] =
        bound_mass_weighted_centroid_plane_distance(&pieces, [5., 0., 0.], [-1., 0., 0.])
            .unwrap();
    let expected_lo = 5. - (3. + 4. * 12.) / 5.;
    let expected_hi = 5. - (2. * 2. + 3. * 10.) / 5.;
    assert!(lo <= expected_lo && expected_hi <= hi);
    assert!((lo - expected_lo).abs() < 1e-12 && (hi - expected_hi).abs() < 1e-12);
    assert!(
        bound_mass_weighted_centroid_plane_distance(&pieces, [0.; 3], [0.; 3]).is_err()
    );
}

#[test]
fn assembly_line_inertia_adds_separate_offsets_without_shared_center_uncertainty() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_inertia_about_line,
    };
    let pieces = [([1., 2.], 2.), ([2., 4.], -3.)].map(|(mass, x)| SurfaceAssemblyMassBounds {
        mass_bounds: mass,
        centroid_bounds: [[x; 2], [0.; 2], [0.; 2]],
        central_inertia_bounds: [[1., 0., 0.], [0., 1., 0.], [0., 0., 2.]]
            .map(|row| row.map(|value| [value; 2])),
    });
    let [lo, hi] =
        bound_assembly_inertia_about_line(&pieces, [0., 0., 100.], [0., 0., -2.]).unwrap();
    assert!(lo <= 26. && 48. <= hi);
    assert!((lo - 26.).abs() < 1e-12 && (hi - 48.).abs() < 1e-12);
    assert!(bound_assembly_inertia_about_line(&pieces, [0.; 3], [0.; 3]).is_err());
    assert!(bound_assembly_inertia_about_line(&[], [0.; 3], [0., 0., 1.]).is_err());
}

#[test]
fn weighted_periodic_trim_inertia_about_line_accounts_for_nested_island() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;

    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2.; 2], [6.; 2]];
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &densities, 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let [lo, hi] = report
        .bound_inertia_about_line([1., 0., 5.], [0., 0., 1.])
        .unwrap()
        .unwrap();
    let expected = 2. * 83. / 768. + 6. * 49. / 12288. - 2. * 0.34375 + 2.875;
    assert!(lo <= expected && expected <= hi);

    let incomplete =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 2, 1_000_000)
            .unwrap();
    assert_eq!(
        incomplete.bound_inertia_about_line([1., 0., 5.], [0., 0., 1.]).unwrap(),
        None
    );
}

#[test]
fn assembly_tensor_about_point_adds_all_shifted_components_and_cross_terms() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_inertia_about_point,
    };

    let mut pieces = [([1., 2.], 2.), ([2., 4.], -3.)].map(|(mass, x)| SurfaceAssemblyMassBounds {
        mass_bounds: mass,
        centroid_bounds: [[x; 2], [1.; 2], [0.; 2]],
        central_inertia_bounds: [[1., 0., 0.], [0., 1., 0.], [0., 0., 2.]]
            .map(|row| row.map(|value| [value; 2])),
    });
    let tensor = bound_assembly_inertia_about_point(&pieces, [0.; 3]).unwrap();
    for (interval, expected) in [
        (tensor[0][0], [5., 8.]),
        (tensor[1][1], [24., 46.]),
        (tensor[2][2], [29., 54.]),
        (tensor[0][1], [2., 10.]),
    ] {
        assert!(interval[0] <= expected[0] && expected[1] <= interval[1]);
        assert!((interval[0] - expected[0]).abs() < 1e-12);
        assert!((interval[1] - expected[1]).abs() < 1e-12);
    }
    assert_eq!(tensor[0][1], tensor[1][0]);
    pieces[1].mass_bounds = [-1., 2.];
    assert!(bound_assembly_inertia_about_point(&pieces, [0.; 3]).is_err());
}

#[test]
fn weighted_periodic_trim_tensor_about_point_encloses_analytic_offset_inertia() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;
    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2.; 2], [6.; 2]];
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &densities, 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let tensor = report.bound_inertia_about_point([1., 0., 0.]).unwrap().unwrap();
    let expected_zz = 2. * 83. / 768. + 6. * 49. / 12288. - 2. * 0.34375 + 2.875;
    assert!(tensor[2][2][0] <= expected_zz && expected_zz <= tensor[2][2][1]);
    assert!(tensor[0][1][0] <= 0.375 && 0.375 <= tensor[0][1][1]);
    assert_eq!(tensor[0][1], tensor[1][0]);

    let incomplete =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 2, 1_000_000)
            .unwrap();
    assert_eq!(incomplete.bound_inertia_about_point([1., 0., 0.]).unwrap(), None);
}

#[test]
fn assembly_first_mass_moment_preserves_signed_contributions() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_first_mass_moment_about_plane,
    };
    let pieces = [([1., 2.], -2.), ([3., 4.], 3.)].map(|(mass, x)| SurfaceAssemblyMassBounds {
        mass_bounds: mass,
        centroid_bounds: [[x; 2], [0.; 2], [0.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    for (normal, expected) in [([2., 0., 0.], [5., 10.]), ([-1., 0., 0.], [-10., -5.])] {
        let bounds =
            bound_assembly_first_mass_moment_about_plane(&pieces, [0.; 3], normal).unwrap();
        assert!(bounds[0] <= expected[0] && expected[1] <= bounds[1]);
        assert!((bounds[0] - expected[0]).abs() < 1e-12);
        assert!((bounds[1] - expected[1]).abs() < 1e-12);
    }
    assert!(bound_assembly_first_mass_moment_about_plane(&pieces, [0.; 3], [0.; 3]).is_err());
}

#[test]
fn weighted_trim_first_mass_moment_measures_oblique_balance() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;
    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2.; 2], [6.; 2]];
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &densities, 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let point = [1., 0., 0.];
    let normal = [1., -1., 0.];
    let [lo, hi] = report.first_mass_moment_about_plane(point, normal).unwrap().unwrap();
    let expected = (0.34375 - 0.375 - 2.875) / 2_f64.sqrt();
    assert!(lo <= expected && expected <= hi && hi < 0.);
    let limited = periodic_trim_patch_set_mass_with_density(
        &patches, &densities, 0.005, 1e-10, 2, 1_000_000,
    )
    .unwrap();
    assert_eq!(limited.first_mass_moment_about_plane(point, normal).unwrap(), None);
}

#[test]
fn physical_principal_moments_prune_impossible_largest_moment() {
    use nurbs_core::analysis::surface_measure::{
        bound_physical_principal_moments, bound_principal_moments_from_tensor,
    };
    let tensor = [
        [[3.; 2], [0.; 2], [0.; 2]],
        [[0.; 2], [4.; 2], [0.; 2]],
        [[0.; 2], [0.; 2], [4., 12.]],
    ];
    let generic = bound_principal_moments_from_tensor(tensor, 100.).unwrap();
    let physical = bound_physical_principal_moments(tensor, 100.).unwrap();
    assert!(generic[2][1] >= 12.);
    assert!(physical[2][1] < 7. + 1e-12);
    for z in [4., 5., 7.] {
        for ([lo, hi], expected) in physical.into_iter().zip([3., 4., z]) {
            assert!(lo <= expected && expected <= hi);
        }
    }
}

#[test]
fn physical_principal_moments_preserve_zero_and_reject_impossible_tensors() {
    use nurbs_core::analysis::surface_measure::bound_physical_principal_moments;
    let moments = bound_physical_principal_moments([[[0.; 2]; 3]; 3], 1e-6).unwrap();
    assert!(moments.into_iter().all(|[lo, hi]| lo == 0. && hi < 1e-12));
    let impossible = [[1., 0., 0.], [0., 1., 0.], [0., 0., 5.]]
        .map(|row| row.map(|entry| [entry; 2]));
    assert!(bound_physical_principal_moments(impossible, 1e-6).is_err());
}

#[test]
fn physical_principal_gaps_certify_separation_and_preserve_repeated_roots() {
    use nurbs_core::analysis::surface_measure::bound_physical_principal_moment_gaps;
    for (diagonal, expected) in [([2., 3., 5.], [1., 2.]), ([3., 3., 5.], [0., 2.])] {
        let tensor = std::array::from_fn(|i| {
            std::array::from_fn(|j| [if i == j { diagonal[i] } else { 0. }; 2])
        });
        let gaps = bound_physical_principal_moment_gaps(tensor, 1e-8).unwrap();
        for ([lo, hi], exact) in gaps.into_iter().zip(expected) {
            assert!(0. <= lo && lo <= exact && exact <= hi);
            assert!(hi - lo < 2e-8);
        }
        assert_eq!(gaps[0][0] > 0., expected[0] > 0.);
    }
}

#[test]
fn physical_upper_gap_is_capped_by_smallest_moment_under_uncertainty() {
    use nurbs_core::analysis::surface_measure::bound_physical_principal_moment_gaps;
    let tensor = [
        [[2.; 2], [0.; 2], [0.; 2]],
        [[0.; 2], [3., 6.], [0.; 2]],
        [[0.; 2], [0.; 2], [6., 8.]],
    ];
    let [first, second] = bound_physical_principal_moment_gaps(tensor, 100.).unwrap();
    assert!(first[0] > 0. && first[0] <= 2. && 4. <= first[1]);
    assert!(second[0] == 0. && 2. <= second[1] && second[1] < 2. + 1e-12);
}

#[test]
fn principal_axis_residual_certifies_a_rotated_direction() {
    use nurbs_core::analysis::surface_measure::certify_principal_axis_direction;
    let tensor = [[3., 1., 0.], [1., 3., 0.], [0., 0., 5.]]
        .map(|row| row.map(|value| [value; 2]));
    let direction = [1., 0.99, 0.];
    let bound = certify_principal_axis_direction(tensor, 1, direction, 1e-9)
        .unwrap()
        .unwrap();
    let expected = 0.01 / (2_f64 * (1. + 0.99_f64.powi(2))).sqrt();
    assert!(expected <= bound && bound < 0.015);
    for scale in [f64::from_bits(1), -1., f64::MAX] {
        assert!(
            certify_principal_axis_direction(tensor, 1, [scale, scale, 0.], 1e-9)
                .unwrap()
                .unwrap()
                < 1e-8
        );
    }
}

#[test]
fn principal_axis_residual_refuses_repeated_moments_and_wrong_candidates() {
    use nurbs_core::analysis::surface_measure::certify_principal_axis_direction;
    let tensor = [[3., 0., 0.], [0., 3., 0.], [0., 0., 5.]]
        .map(|row| row.map(|value| [value; 2]));
    assert_eq!(
        certify_principal_axis_direction(tensor, 0, [1., 0., 0.], 1e-9).unwrap(),
        None
    );
    assert_eq!(
        certify_principal_axis_direction(tensor, 2, [1., 0., 0.], 1e-9).unwrap(),
        None
    );
    assert!(certify_principal_axis_direction(tensor, 3, [1., 0., 0.], 1e-9).is_err());
    assert!(certify_principal_axis_direction(tensor, 2, [0.; 3], 1e-9).is_err());
}

#[test]
fn automatic_principal_axes_certify_the_three_rotated_directions() {
    use nurbs_core::analysis::surface_measure::find_certified_principal_axis;
    let tensor = [[3., 1., 0.], [1., 3., 0.], [0., 0., 5.]]
        .map(|row| row.map(|value| [value; 2]));
    for (index, expected) in [[1., -1., 0.], [1., 1., 0.], [0., 0., 1.]].into_iter().enumerate() {
        let result = find_certified_principal_axis(tensor, index, 1e-9).unwrap().unwrap();
        let [x, y, z] = result.direction;
        let [a, b, c] = expected;
        let cosine = (x * a + y * b + z * c).abs()
            / ((x * x + y * y + z * z) * (a * a + b * b + c * c)).sqrt();
        assert!(cosine > 1. - 1e-14);
        assert!(result.sin_angle_upper < 1e-8);
    }
}

#[test]
fn automatic_principal_axes_skip_repeated_moments_but_return_isolated_axis() {
    use nurbs_core::analysis::surface_measure::find_certified_principal_axis;
    let tensor = [[3., 0., 0.], [0., 3., 0.], [0., 0., 5.]]
        .map(|row| row.map(|value| [value; 2]));
    assert!(find_certified_principal_axis(tensor, 0, 1e-9).unwrap().is_none());
    assert!(find_certified_principal_axis(tensor, 1, 1e-9).unwrap().is_none());
    let axis = find_certified_principal_axis(tensor, 2, 1e-9).unwrap().unwrap();
    assert_eq!(axis.direction.map(f64::abs), [0., 0., 1.]);
    assert!(axis.sin_angle_upper < 1e-8);
}

#[test]
fn principal_axis_projector_encloses_rotated_direction_without_sign_convention() {
    use nurbs_core::analysis::surface_measure::{
        bound_principal_axis_projector, find_certified_principal_axis,
    };
    let tensor = [[3., 1., 0.], [1., 3., 0.], [0., 0., 5.]]
        .map(|row| row.map(|value| [value; 2]));
    let mut axis = find_certified_principal_axis(tensor, 0, 1e-9).unwrap().unwrap();
    let bounds = bound_principal_axis_projector(axis).unwrap();
    axis.direction = axis.direction.map(|component| -component);
    assert_eq!(bounds, bound_principal_axis_projector(axis).unwrap());
    for (row, expected) in bounds.into_iter().zip([[0.5, -0.5, 0.], [-0.5, 0.5, 0.], [0.; 3]]) {
        for ([lo, hi], value) in row.into_iter().zip(expected) {
            assert!(lo <= value && value <= hi);
            assert!(hi - lo < 1e-8);
        }
    }
}

#[test]
fn principal_axis_projector_accounts_for_certificate_uncertainty() {
    use nurbs_core::analysis::surface_measure::{
        PrincipalAxisCertificate, bound_principal_axis_projector,
    };
    let mut axis = PrincipalAxisCertificate {
        direction: [1., 0., 0.],
        sin_angle_upper: 0.5,
    };
    let bounds = bound_principal_axis_projector(axis).unwrap();
    let exact = [[0.75, 3_f64.sqrt() / 4., 0.], [3_f64.sqrt() / 4., 0.25, 0.], [0.; 3]];
    for (row, expected) in bounds.into_iter().zip(exact) {
        for ([lo, hi], value) in row.into_iter().zip(expected) {
            assert!(lo <= value && value <= hi);
        }
    }
    axis.sin_angle_upper = f64::NAN;
    assert!(bound_principal_axis_projector(axis).is_err());
}

#[test]
fn certified_principal_axis_distance_handles_uncertain_anchor_and_axis_sign() {
    use nurbs_core::analysis::surface_measure::{
        PrincipalAxisCertificate, bound_distance_to_principal_axis,
    };
    let mut axis = PrincipalAxisCertificate {
        direction: [0., 0., 3.],
        sin_angle_upper: 0.,
    };
    let anchor = [[1., 2.], [0.; 2], [-10., 10.]];
    let point = [5., 4., 0.];
    let bounds = bound_distance_to_principal_axis(axis, anchor, point).unwrap();
    assert!(bounds[0] <= 5. && 32_f64.sqrt() <= bounds[1]);
    assert!((bounds[0] - 5.).abs() < 1e-12);
    assert!((bounds[1] - 32_f64.sqrt()).abs() < 1e-12);
    axis.direction[2] = -3.;
    assert_eq!(bounds, bound_distance_to_principal_axis(axis, anchor, point).unwrap());
}

#[test]
fn certified_principal_axis_distance_accounts_for_angular_error() {
    use nurbs_core::analysis::surface_measure::{
        PrincipalAxisCertificate, bound_distance_to_principal_axis,
    };
    let mut axis = PrincipalAxisCertificate {
        direction: [1., 0., 0.],
        sin_angle_upper: 0.5,
    };
    let bounds = bound_distance_to_principal_axis(axis, [[0.; 2]; 3], [8., 0., 0.]).unwrap();
    assert_eq!(bounds[0], 0.);
    assert!(4. <= bounds[1] && bounds[1] < 4. + 1e-12);
    axis.sin_angle_upper = -0.5;
    assert!(bound_distance_to_principal_axis(axis, [[0.; 2]; 3], [8., 0., 0.]).is_err());
}

#[test]
fn principal_axis_angle_sine_is_invariant_under_reversal_and_direction_scale() {
    use nurbs_core::analysis::surface_measure::{
        PrincipalAxisCertificate, bound_principal_axis_angle_sine,
    };
    let axis = PrincipalAxisCertificate { direction: [1., 0., 0.], sin_angle_upper: 0. };
    let reference = 0.5_f64.sqrt();
    for scale in [f64::from_bits(1), -1., f64::MAX] {
        let [lo, hi] = bound_principal_axis_angle_sine(axis, [scale, scale, 0.]).unwrap();
        assert!(lo <= reference && reference <= hi && hi - lo < 1e-12);
    }
}

#[test]
fn principal_axis_angle_sine_respects_certified_angular_uncertainty() {
    use nurbs_core::analysis::surface_measure::{
        PrincipalAxisCertificate, bound_principal_axis_angle_sine,
    };
    let axis = PrincipalAxisCertificate { direction: [1., 0., 0.], sin_angle_upper: 0.25 };
    let [lo, hi] = bound_principal_axis_angle_sine(axis, [1., 0., 0.]).unwrap();
    assert_eq!(lo, 0.);
    assert!(0.25 <= hi && hi < 0.25 + 1e-12);
    let [lo, hi] = bound_principal_axis_angle_sine(axis, [0., 1., 0.]).unwrap();
    assert!(lo <= 0.75 && lo > 0.75 - 1e-12);
    assert_eq!(hi, 1.);
    assert!(bound_principal_axis_angle_sine(axis, [0.; 3]).is_err());
}

#[test]
fn assembly_balance_projects_before_combining_uncertain_masses() {
    use nurbs_core::analysis::surface_measure::{
        BalanceTolerance, SurfaceAssemblyMassBounds, bound_assembly_balance_about_line,
    };
    let pieces = [1., 100.].map(|x| SurfaceAssemblyMassBounds {
        mass_bounds: [1., 10.],
        centroid_bounds: [[x; 2], [x; 2], [3.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    for (tolerance, expected) in [
        (3.01, BalanceTolerance::Within),
        (2.99, BalanceTolerance::Outside),
        (3., BalanceTolerance::Undetermined),
    ] {
        let ([lo, hi], status) =
            bound_assembly_balance_about_line(&pieces, [0.; 3], [2., 2., 0.], tolerance)
                .unwrap();
        assert!(lo <= 3. && 3. <= hi && hi - lo < 1e-11);
        assert_eq!(status, expected);
    }
    assert!(bound_assembly_balance_about_line(&pieces, [0.; 3], [0.; 3], 1.).is_err());
}

#[test]
fn weighted_trim_balance_detects_offset_axis_and_incomplete_measurement() {
    use nurbs_core::analysis::surface_measure::{
        BalanceTolerance, periodic_trim_patch_set_mass_with_density,
    };
    let patches = periodic_trim_patches_with_nested_island();
    let densities = [[2.; 2], [6.; 2]];
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &densities, 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let point = [1., 0., 0.];
    let direction = [0., 0., 1.];
    let ([lo, hi], status) = report
        .bound_balance_about_line(point, direction, 0.5)
        .unwrap()
        .unwrap();
    let exact = (1. - 0.34375_f64 / 2.875).hypot(0.375 / 2.875);
    assert!(lo <= exact && exact <= hi);
    assert_eq!(status, BalanceTolerance::Outside);
    let incomplete =
        periodic_trim_patch_set_mass_with_density(&patches, &densities, 0.005, 1e-10, 2, 1_000_000)
            .unwrap();
    assert_eq!(incomplete.bound_balance_about_line(point, direction, 0.5).unwrap(), None);
}

#[test]
fn static_unbalance_sums_signed_component_vectors_before_taking_magnitude() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_static_unbalance,
    };
    let mut pieces = [(2., -3.), (3., 2.)].map(|(mass, x)| SurfaceAssemblyMassBounds {
        mass_bounds: [mass; 2],
        centroid_bounds: [[x; 2], [0.; 2], [4.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    let balanced = bound_assembly_static_unbalance(&pieces, [0.; 3], [0., 0., 1.]).unwrap();
    assert_eq!(balanced.magnitude_bounds[0], 0.);
    assert!(balanced.magnitude_bounds[1] < 1e-12);

    pieces[1].mass_bounds = [4., 5.];
    let unbalanced =
        bound_assembly_static_unbalance(&pieces, [0., 0., 10.], [0., 0., -2.]).unwrap();
    let [lo, hi] = unbalanced.magnitude_bounds;
    assert!(lo <= 2. && 4. <= hi && hi - lo < 2. + 1e-12);
    let [lo, hi] = unbalanced.vector_bounds[0];
    assert!(lo <= 2. && 4. <= hi);
}

#[test]
fn weighted_trim_static_unbalance_encloses_analytic_offset_vector() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;
    let patches = periodic_trim_patches_with_nested_island();
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &[[2.; 2], [6.; 2]], 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let result = report
        .bound_static_unbalance([1., 0., 0.], [0., 0., 1.])
        .unwrap()
        .unwrap();
    let exact = [0.34375_f64 - 2.875, 0.375, 0.];
    for ([lo, hi], expected) in result.vector_bounds.into_iter().zip(exact) {
        assert!(lo <= expected && expected <= hi);
    }
    let [lo, hi] = result.magnitude_bounds;
    assert!(lo <= exact[0].hypot(exact[1]) && exact[0].hypot(exact[1]) <= hi);
}

#[test]
fn couple_unbalance_detects_axial_mass_separation_in_a_statically_balanced_assembly() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_couple_unbalance, bound_assembly_static_unbalance,
    };
    let pieces = [-1., 1.].map(|coordinate| SurfaceAssemblyMassBounds {
        mass_bounds: [1.; 2],
        centroid_bounds: [[coordinate; 2], [0.; 2], [coordinate; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    });
    let static_unbalance =
        bound_assembly_static_unbalance(&pieces, [0.; 3], [0., 0., 1.]).unwrap();
    assert!(static_unbalance.magnitude_bounds[1] < 1e-12);

    for (point, direction) in [([0.; 3], [0., 0., 1.]), ([0., 0., 4.], [0., 0., -2.])] {
        let couple = bound_assembly_couple_unbalance(&pieces, point, direction).unwrap();
        for ([lo, hi], expected) in couple.vector_bounds.into_iter().zip([0., -2., 0.]) {
            assert!(lo <= expected && expected <= hi);
            assert!(hi - lo < 1e-12);
        }
        let [lo, hi] = couple.magnitude_bounds;
        assert!(lo <= 2. && 2. <= hi && lo > 1.99);
    }
}

#[test]
fn weighted_trim_couple_unbalance_encloses_analytic_products_of_inertia() {
    use nurbs_core::analysis::surface_measure::periodic_trim_patch_set_mass_with_density;
    let patches = periodic_trim_patches_with_nested_island();
    let report = periodic_trim_patch_set_mass_with_density(
        &patches, &[[2.; 2], [6.; 2]], 0.005, 1e-10, 20_000, 1_000_000,
    )
    .unwrap();
    let result = report.bound_couple_unbalance([0.; 3], [0., 0., 1.]).unwrap().unwrap();
    // The retained ranges are symmetric around world z = 1.
    let exact = [0.375_f64, -0.34375, 0.];
    for ([lo, hi], expected) in result.vector_bounds.into_iter().zip(exact) {
        assert!(lo <= expected && expected <= hi);
    }
    let [lo, hi] = result.magnitude_bounds;
    let expected = exact[0].hypot(exact[1]);
    assert!(lo <= expected && expected <= hi);
}

#[test]
fn two_plane_balance_recovers_separated_radial_unbalance_vectors() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_two_plane_unbalance,
    };
    let pieces = [([-1., 2., -2.], 2.), ([3., -1., 4.], 3.)].map(|(center, mass)| {
        SurfaceAssemblyMassBounds {
            mass_bounds: [mass; 2],
            centroid_bounds: center.map(|value| [value; 2]),
            central_inertia_bounds: [[[0.; 2]; 3]; 3],
        }
    });
    for (direction, offsets, exact) in [
        ([0., 0., 2.], [-2., 4.], [[-2., 4., 0.], [9., -3., 0.]]),
        ([0., 0., -2.], [-4., 2.], [[9., -3., 0.], [-2., 4., 0.]]),
    ] {
        let result = bound_assembly_two_plane_unbalance(&pieces, [0.; 3], direction, offsets)
            .unwrap();
        for (plane, expected) in result.into_iter().zip(exact) {
            for ([lo, hi], reference) in plane.into_iter().zip(expected) {
                assert!(lo <= reference && reference <= hi && hi - lo < 1e-12);
            }
        }
    }
}

#[test]
fn two_plane_balance_splits_midplane_mass_and_rejects_coincident_planes() {
    use nurbs_core::analysis::surface_measure::{
        SurfaceAssemblyMassBounds, bound_assembly_two_plane_unbalance,
    };
    let piece = SurfaceAssemblyMassBounds {
        mass_bounds: [1., 2.],
        centroid_bounds: [[2.; 2], [0.; 2], [1.; 2]],
        central_inertia_bounds: [[[0.; 2]; 3]; 3],
    };
    let direction = [0., 0., 1.];
    let bounds =
        bound_assembly_two_plane_unbalance(&[piece], [0., 0., 1.], direction, [-1., 1.]).unwrap();
    for plane in bounds {
        assert!(plane[0][0] <= 1. && plane[0][0] > 1. - 1e-12);
        assert!(plane[0][1] >= 2. && plane[0][1] < 2. + 1e-12);
    }
    assert!(
        bound_assembly_two_plane_unbalance(&[piece], [0.; 3], direction, [1.; 2]).is_err()
    );
}

#[test]
fn two_plane_tolerance_classifies_each_magnitude_and_the_assembly() {
    use nurbs_core::analysis::surface_measure::{BalanceTolerance::*, classify_two_plane_unbalance};
    let vectors = [[[3.; 2], [4.; 2], [0.; 2]], [[-2., 2.], [1.; 2], [0.; 2]]];
    for (limits, planes, combined) in [
        ([6., 3.], [Within, Within], Within),
        ([4., 1.5], [Outside, Undetermined], Outside),
        ([6., 1.5], [Within, Undetermined], Undetermined),
    ] {
        let report = classify_two_plane_unbalance(vectors, limits).unwrap();
        assert_eq!(report.plane_status, planes);
        assert_eq!(report.overall_status, combined);
        let [first, second] = report.magnitude_bounds;
        assert!(first[0] <= 5. && 5. <= first[1]);
        assert!(second[0] <= 1. && 5_f64.sqrt() <= second[1]);
    }
}

#[test]
fn two_plane_tolerance_rejects_malformed_inputs() {
    use nurbs_core::analysis::surface_measure::classify_two_plane_unbalance;
    let mut vectors = [[[0.; 2]; 3]; 2];
    assert!(classify_two_plane_unbalance(vectors, [-1., 2.]).is_err());
    assert!(classify_two_plane_unbalance(vectors, [1., f64::INFINITY]).is_err());
    vectors[1][0] = [3., 2.];
    assert!(classify_two_plane_unbalance(vectors, [1.; 2]).is_err());
}

#[test]
fn two_plane_counterweights_enclose_mass_and_opposing_radial_positions() {
    use nurbs_core::analysis::surface_measure::bound_two_plane_counterweights;
    let vectors = [[[3.; 2], [4.; 2], [0.; 2]], [[0.; 2], [-6.; 2], [0.; 2]]];
    let result = bound_two_plane_counterweights(vectors, [10., 2.]).unwrap();
    for (plane, (mass, offset)) in result.into_iter().zip([(0.5, [-6., -8., 0.]), (3., [0., 2., 0.])]) {
        assert!(plane.mass_bounds[0] <= mass && mass <= plane.mass_bounds[1]);
        for ([lo, hi], exact) in plane.radial_offset_bounds.unwrap().into_iter().zip(offset) {
            assert!(lo <= exact && exact <= hi && hi - lo < 1e-12);
        }
    }
}

#[test]
fn two_plane_counterweights_withhold_indeterminate_positions() {
    use nurbs_core::analysis::surface_measure::bound_two_plane_counterweights;
    let vectors = [[[-1., 1.], [0.; 2], [0.; 2]], [[2., 4.], [0.; 2], [0.; 2]]];
    let [first, second] = bound_two_plane_counterweights(vectors, [2., 4.]).unwrap();
    assert_eq!(first.mass_bounds[0], 0.);
    assert!(first.mass_bounds[1] >= 0.5);
    assert!(first.radial_offset_bounds.is_none());
    assert!(second.radial_offset_bounds.is_some());
    assert!(bound_two_plane_counterweights(vectors, [1., 0.]).is_err());
}

#[test]
fn two_plane_residual_accounts_for_installed_mass_and_position_uncertainty() {
    use nurbs_core::analysis::surface_measure::{
        BalanceTolerance, InstalledCounterweightBounds, bound_two_plane_residual_unbalance,
    };
    let vectors = [[[4.; 2], [0.; 2], [0.; 2]], [[0.; 2], [-6.; 2], [0.; 2]]];
    let weights = [
        InstalledCounterweightBounds {
            mass_bounds: [1.9, 2.1],
            radial_offset_bounds: [[-2.1, -1.9], [0.; 2], [0.; 2]],
        },
        InstalledCounterweightBounds {
            mass_bounds: [2.; 2],
            radial_offset_bounds: [[0.; 2], [3.; 2], [0.; 2]],
        },
    ];
    let (residual, report) = bound_two_plane_residual_unbalance(vectors, weights, [0.5; 2]).unwrap();
    assert_eq!(report.overall_status, BalanceTolerance::Within);
    assert!(residual[0][0][0] <= -0.41 && 0.39 <= residual[0][0][1]);
    assert!(report.magnitude_bounds[1][1] < 1e-12);
    let (_, report) = bound_two_plane_residual_unbalance(vectors, weights, [0.2, 0.5]).unwrap();
    assert_eq!(report.overall_status, BalanceTolerance::Undetermined);
}

#[test]
fn two_plane_residual_preserves_unmodified_planes_and_rejects_negative_mass() {
    use nurbs_core::analysis::surface_measure::{
        BalanceTolerance, InstalledCounterweightBounds, bound_two_plane_residual_unbalance,
    };
    let vectors = [[[4.; 2], [0.; 2], [0.; 2]], [[0.; 2], [-6.; 2], [0.; 2]]];
    let mut weights = [InstalledCounterweightBounds {
        mass_bounds: [0.; 2],
        radial_offset_bounds: [[-1., 1.]; 3],
    }; 2];
    let (residual, report) = bound_two_plane_residual_unbalance(vectors, weights, [1.; 2]).unwrap();
    assert_eq!(residual, vectors);
    assert_eq!(report.overall_status, BalanceTolerance::Outside);
    weights[0].mass_bounds = [-1., 1.];
    assert!(bound_two_plane_residual_unbalance(vectors, weights, [1.; 2]).is_err());
}

#[test]
fn two_plane_capacity_encloses_minimum_radii_under_mass_limits() {
    use nurbs_core::analysis::surface_measure::{
        BalanceTolerance::*, bound_two_plane_counterweight_capacity,
    };
    let vectors = [[[3.; 2], [4.; 2], [0.; 2]], [[2., 6.], [0.; 2], [0.; 2]]];
    for (radii, statuses, overall) in [
        ([3., 7.], [Within, Within], Within),
        ([2., 4.], [Outside, Undetermined], Outside),
        ([3., 4.], [Within, Undetermined], Undetermined),
    ] {
        let report = bound_two_plane_counterweight_capacity(vectors, radii, [2., 1.]).unwrap();
        assert_eq!(report.plane_status, statuses);
        assert_eq!(report.overall_status, overall);
        for ([lo, hi], [min, max]) in report
            .minimum_radius_bounds
            .into_iter()
            .zip([[2.5, 2.5], [2., 6.]])
        {
            assert!(lo <= min && max <= hi);
            assert!((lo - min).abs() < 1e-12 && (hi - max).abs() < 1e-12);
        }
    }
}

#[test]
fn two_plane_capacity_rejects_invalid_limits_and_mounting_radii() {
    use nurbs_core::analysis::surface_measure::bound_two_plane_counterweight_capacity;
    let vectors = [[[0.; 2]; 3]; 2];
    assert!(bound_two_plane_counterweight_capacity(vectors, [1.; 2], [0., 1.]).is_err());
    assert!(bound_two_plane_counterweight_capacity(vectors, [1., f64::INFINITY], [1.; 2]).is_err());
    assert!(bound_two_plane_counterweight_capacity(vectors, [1.; 2], [1., f64::NAN]).is_err());
}

#[test]
fn directional_curve_travel_counts_reversals_with_coincident_endpoints() {
    use nurbs_core::analysis::curve_measure::directional_travel;
    let curve = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![2., 2.], vec![0., 0.]],
        None,
    )
    .unwrap();
    for scale in [f64::from_bits(1), -1., f64::MAX] {
        let report = directional_travel(&curve, &[scale, scale], 1e-7, 4096).unwrap();
        let expected = 2. * 2_f64.sqrt();
        assert!(report.within_tolerance, "{report:?}");
        assert!(report.bounds[0] <= expected && expected <= report.bounds[1]);
    }
    let coarse = directional_travel(&curve, &[1., 1.], 1e-7, 1).unwrap();
    assert!(!coarse.within_tolerance);
    assert!(coarse.bounds[0] <= 2. * 2_f64.sqrt() && coarse.bounds[1] >= 2. * 2_f64.sqrt());
}

#[test]
fn directional_curve_travel_encloses_rotated_rational_circle() {
    use nurbs_core::analysis::curve_measure::directional_travel;
    let curve = nurbs_core::primitives::circle([1., 2., 3.], [0., 0., 1.], 2.).unwrap();
    let report = directional_travel(&curve, &[1., 2., 2.], 1e-6, 8192).unwrap();
    let expected = 8. * 5_f64.sqrt() / 3.;
    assert!(report.within_tolerance, "{report:?}");
    assert!(report.bounds[0] <= expected && expected <= report.bounds[1]);
    assert!(directional_travel(&curve, &[0.; 3], 1e-6, 8192).is_err());
}

#[test]
fn signed_directional_travel_separates_forward_and_reverse_motion() {
    use nurbs_core::analysis::curve_measure::directional_travel_by_sign;
    let curve = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![4., 0.], vec![2., 0.]],
        None,
    )
    .unwrap();
    let report = directional_travel_by_sign(&curve, &[1., 0.], 1e-7, 4096).unwrap();
    assert!(report.total.within_tolerance);
    for (bounds, expected) in [
        (report.forward_bounds, 8. / 3.),
        (report.reverse_bounds, 2. / 3.),
        (report.signed_travel_bounds, 2.),
    ] {
        assert!(bounds[0] <= expected && expected <= bounds[1]);
        assert!(bounds[1] - bounds[0] < 1e-6);
    }
    let reversed = directional_travel_by_sign(&curve, &[-1., 0.], 1e-7, 4096).unwrap();
    assert!(reversed.forward_bounds[0] <= 2. / 3. && 2. / 3. <= reversed.forward_bounds[1]);
    assert!(reversed.reverse_bounds[0] <= 8. / 3. && 8. / 3. <= reversed.reverse_bounds[1]);
}

#[test]
fn signed_directional_travel_gives_zero_reverse_travel_on_monotone_line() {
    use nurbs_core::analysis::curve_measure::directional_travel_by_sign;
    let curve = nurbs_core::paths::bezier(vec![vec![2., 2.], vec![5., 6.]], None).unwrap();
    let report = directional_travel_by_sign(&curve, &[0., 2.], 1e-8, 32).unwrap();
    assert!(report.total.within_tolerance);
    assert!(report.forward_bounds[0] <= 4. && 4. <= report.forward_bounds[1]);
    assert_eq!(report.reverse_bounds[0], 0.);
    assert!(report.reverse_bounds[1] < 1e-12);
}

#[test]
fn directional_travel_window_clips_both_sides_of_a_knot_reversal() {
    use nurbs_core::analysis::curve_measure::directional_travel_on_interval;
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 8., 8.],
        control_points: vec![vec![0., 0.], vec![4., 0.], vec![-4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report = directional_travel_on_interval(&curve, [3., 7.], &[1., 0.], 1e-8, 64).unwrap();
    assert!(report.total.within_tolerance);
    for (bounds, exact) in [
        (report.total.bounds, 8.),
        (report.forward_bounds, 2.),
        (report.reverse_bounds, 6.),
        (report.signed_travel_bounds, -4.),
    ] {
        assert!(bounds[0] <= exact && exact <= bounds[1]);
        assert!(bounds[1] - bounds[0] < 1e-10);
    }
    assert!(directional_travel_on_interval(&curve, [3., 7.], &[1., 0.], 1e-8, 1).is_err());
}

#[test]
fn directional_travel_window_contains_an_interior_rational_segment() {
    use nurbs_core::analysis::curve_measure::directional_travel_on_interval;
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![6., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let report = directional_travel_on_interval(&curve, [3., 4.], &[1., 0.], 1e-8, 1024).unwrap();
    let exact = 4. - 2.4;
    assert!(report.total.within_tolerance);
    assert!(report.forward_bounds[0] <= exact && exact <= report.forward_bounds[1]);
    assert!(report.reverse_bounds[1] < 1e-8);
    for invalid in [[1., 4.], [3., 3.], [5., 4.], [f64::NAN, 4.]] {
        assert!(directional_travel_on_interval(&curve, invalid, &[1., 0.], 1e-8, 32).is_err());
    }
}

#[test]
fn directional_backtracking_distinguishes_bounds_and_measurement_tolerance() {
    use nurbs_core::analysis::curve_measure::{ReverseTravelStatus, check_directional_backtracking};
    let curve = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![4., 0.], vec![2., 0.]],
        None,
    )
    .unwrap();
    let domain = curve.domain();
    for (limit, budget, expected) in [
        (0.6, 4096, ReverseTravelStatus::Exceeds),
        (0.7, 4096, ReverseTravelStatus::Within),
        (0.7, 1, ReverseTravelStatus::Undetermined),
        (100., 1, ReverseTravelStatus::Within),
    ] {
        let (report, status) =
            check_directional_backtracking(&curve, domain, &[1., 0.], limit, 1e-7, budget)
                .unwrap();
        assert_eq!(status, expected, "{report:?}");
        assert_eq!(report.total.within_tolerance, budget > 1);
        assert!(report.reverse_bounds[0] <= 2. / 3. && 2. / 3. <= report.reverse_bounds[1]);
    }
}

#[test]
fn directional_backtracking_rejects_invalid_limit_and_checks_selected_window() {
    use nurbs_core::analysis::curve_measure::{ReverseTravelStatus, check_directional_backtracking};
    let curve = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![4., 0.], vec![2., 0.]],
        None,
    )
    .unwrap();
    let (_, status) =
        check_directional_backtracking(&curve, [0., 0.5], &[1., 0.], 1e-6, 1e-7, 1024)
            .unwrap();
    assert_eq!(status, ReverseTravelStatus::Within);
    for invalid in [-1., f64::NAN, f64::INFINITY] {
        assert!(
            check_directional_backtracking(
                &curve, curve.domain(), &[1., 0.], invalid, 1e-7, 64,
            )
            .is_err()
        );
    }
}

#[test]
fn batched_directional_travel_keeps_input_order_and_counts_overlapping_windows() {
    use nurbs_core::analysis::curve_measure::directional_travel_on_intervals;
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 8., 8.],
        control_points: vec![vec![0., 0.], vec![4., 0.], vec![-4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let windows = [[4., 8.], [3., 7.], [2., 4.]];
    let reports = directional_travel_on_intervals(&curve, &windows, &[1., 0.], 1e-8, 4)
        .unwrap();
    assert_eq!(reports.iter().map(|report| report.total.cells).sum::<usize>(), 4);
    for (report, (forward, reverse)) in reports.iter().zip([(0., 8.), (2., 6.), (4., 0.)]) {
        assert!(report.total.within_tolerance);
        for ([lo, hi], exact) in [report.forward_bounds, report.reverse_bounds]
            .into_iter()
            .zip([forward, reverse])
        {
            assert!(lo <= exact && exact <= hi);
        }
    }
    assert!(directional_travel_on_intervals(&curve, &windows, &[1., 0.], 1e-8, 3).is_err());
}

#[test]
fn batched_directional_travel_reserves_coverage_from_early_refinement() {
    use nurbs_core::analysis::curve_measure::directional_travel_on_intervals;
    let curve = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![4., 0.], vec![2., 0.]],
        None,
    )
    .unwrap();
    let reports =
        directional_travel_on_intervals(&curve, &[[0., 1.], [0., 0.5]], &[1., 0.], 1e-7, 4)
            .unwrap();
    assert!(!reports[0].total.within_tolerance);
    assert!(reports[0].total.cells + reports[1].total.cells <= 4);
    assert!(reports[1].forward_bounds[0] <= 2.5 && 2.5 <= reports[1].forward_bounds[1]);
    assert!(directional_travel_on_intervals(&curve, &[[0., 1.], [0., 2.]], &[1., 0.], 1e-7, 4).is_err());
}

#[test]
fn directional_peak_speed_encloses_interior_maximum_in_original_parameter_units() {
    use nurbs_core::analysis::curve_measure::{StopReason, directional_peak_speed};
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.], vec![1., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = directional_peak_speed(&curve, [2., 6.], &[-2., 0.], 1e-5, 2048).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert!(report.bounds[0] <= 0.375 && 0.375 <= report.bounds[1]);
    let coarse = directional_peak_speed(&curve, [2., 6.], &[1., 0.], 1e-5, 1).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.bounds[0] <= 0.375 && 0.375 <= coarse.bounds[1]);
}

#[test]
fn directional_peak_speed_retains_both_one_sided_knot_derivatives() {
    use nurbs_core::analysis::curve_measure::directional_peak_speed;
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 8., 8.],
        control_points: vec![vec![0., 0.], vec![4., 0.], vec![-12., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    for (window, expected) in [([3., 4.], 2.), ([3., 5.], 4.), ([4., 5.], 4.)] {
        let [lo, hi] = directional_peak_speed(&curve, window, &[1., 0.], 1e-8, 32).unwrap().bounds;
        assert!(lo <= expected && expected <= hi && hi - lo < 1e-8);
    }
    assert!(directional_peak_speed(&curve, [3., 5.], &[1., 0.], 1e-8, 1).is_err());
}

#[test]
fn directional_peak_locations_narrow_around_an_interior_speed_maximum() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_speed;
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.], vec![1., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let result = locate_directional_peak_speed(&curve, [2., 6.], &[1., 0.], 1e-5, 0.02, 2048).unwrap();
    assert_eq!(result.parameter_intervals.len(), 1, "{result:?}");
    let [lo, hi] = result.parameter_intervals[0];
    assert!(lo <= 4. && 4. <= hi && hi - lo < 0.1, "{result:?}");
    let coarse = locate_directional_peak_speed(&curve, [2., 6.], &[1., 0.], 1e-5, 0.02, 1).unwrap();
    assert_eq!(coarse.parameter_intervals, vec![[2., 6.]]);
}

#[test]
fn directional_peak_locations_preserve_disjoint_tied_spans() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_speed;
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 3.],
        control_points: vec![vec![0., 0.], vec![3., 0.], vec![4., 0.], vec![1., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let result = locate_directional_peak_speed(&curve, [0., 3.], &[1., 0.], 1e-8, 0.02, 3).unwrap();
    assert_eq!(result.parameter_intervals, vec![[0., 1.], [2., 3.]]);
    assert!(result.peak.bounds[0] <= 3. && 3. <= result.peak.bounds[1]);
}

#[test]
fn directional_peak_acceleration_encloses_interior_maximum_with_source_clock_scaling() {
    use nurbs_core::analysis::curve_measure::{StopReason, directional_peak_acceleration};
    let curve = Curve {
        degree: 4,
        knots: vec![2., 2., 2., 2., 2., 4., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 1., 2.]
            .into_iter()
            .map(|x| vec![x, 0.])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let result = directional_peak_acceleration(&curve, [2., 4.], &[1., 0.], 1e-5, 2048).unwrap();
    assert_eq!(result.stop_reason, StopReason::Tolerance);
    assert!(result.bounds[0] <= 1.5 && 1.5 <= result.bounds[1], "{result:?}");
    let coarse = directional_peak_acceleration(&curve, [2., 4.], &[1., 0.], 1e-5, 1).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.bounds[0] <= 1.5 && 1.5 <= coarse.bounds[1]);
}

#[test]
fn directional_peak_acceleration_accounts_for_rational_parameter_speed() {
    use nurbs_core::analysis::curve_measure::{StopReason, directional_peak_acceleration};
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![6., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let result = directional_peak_acceleration(&curve, [3., 4.], &[1., 0.], 1e-5, 2048).unwrap();
    assert_eq!(result.stop_reason, StopReason::Tolerance);
    assert!(result.bounds[0] <= 0.768 && 0.768 <= result.bounds[1], "{result:?}");
}

#[test]
fn directional_peak_jerk_uses_cubed_source_parameter_width() {
    use nurbs_core::analysis::curve_measure::{StopReason, directional_peak_jerk};
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let result = directional_peak_jerk(&curve, [2.5, 3.5], &[-4., 0.], 1e-8, 32).unwrap();
    assert_eq!(result.stop_reason, StopReason::Tolerance);
    assert!(result.bounds[0] <= 6. && 6. <= result.bounds[1]);
    assert!(result.bounds[1] - result.bounds[0] < 1e-8);
}

#[test]
fn directional_peak_jerk_encloses_rational_linear_support() {
    use nurbs_core::analysis::curve_measure::{StopReason, directional_peak_jerk};
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![6., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let exact = 72. / (64. * 1.25_f64.powi(4));
    for (cells, status) in [(1, StopReason::WorkLimit), (2048, StopReason::Tolerance)] {
        let report = directional_peak_jerk(&curve, [3., 4.], &[1., 0.], 1e-5, cells).unwrap();
        assert_eq!(report.stop_reason, status, "{report:?}");
        assert!(report.bounds[0] <= exact && exact <= report.bounds[1]);
    }
    assert!(directional_peak_jerk(&curve, [3., 4.], &[0., 0.], 1e-5, 32).is_err());
}

#[test]
fn locate_directional_peak_acceleration_brackets_interior_maximum_under_shared_budget() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_acceleration;
    let curve = Curve {
        degree: 4,
        knots: vec![2., 2., 2., 2., 2., 4., 4., 4., 4., 4.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![0., 0.], vec![1., 0.], vec![2., 0.]],
        weights: vec![1.; 5],
        periodic: false,
    };
    let locate = |cells| {
        locate_directional_peak_acceleration(&curve, [2., 4.], &[1., 0.], 1e-5, 0.02, cells)
            .unwrap()
    };
    let fine = locate(2048);
    assert_eq!(fine.parameter_intervals.len(), 1, "{fine:?}");
    let [lo, hi] = fine.parameter_intervals[0];
    assert!(lo <= 3. && 3. <= hi && hi - lo < 0.1, "{fine:?}");
    assert!(fine.peak.bounds[0] <= 1.5 && 1.5 <= fine.peak.bounds[1]);
    let coarse = locate(1);
    assert_eq!(coarse.parameter_intervals, vec![[2., 4.]]);
    assert_eq!(coarse.peak.cells, 1);
}

#[test]
fn locate_directional_peak_acceleration_keeps_a_one_sided_knot_maximum() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_acceleration;
    let curve = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 2., 2., 2., 2.],
        control_points: [0., 0., 0., 1., 2., 3., 4.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 7],
        periodic: false,
    };
    let report =
        locate_directional_peak_acceleration(&curve, [0., 2.], &[1., 0.], 1e-5, 0.01, 128)
            .unwrap();
    assert_eq!(report.parameter_intervals.len(), 1, "{report:?}");
    let [lo, hi] = report.parameter_intervals[0];
    assert!(0.98 < lo && lo <= 1. && hi == 1., "{report:?}");
    assert!(report.peak.bounds[0] <= 6. && 6. <= report.peak.bounds[1]);
    assert!(
        locate_directional_peak_acceleration(&curve, [0., 2.], &[1., 0.], 1e-5, 0., 128)
            .is_err()
    );
}

#[test]
fn locate_directional_peak_jerk_brackets_an_interior_maximum() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_jerk;
    let curve = Curve {
        degree: 5,
        knots: vec![2., 2., 2., 2., 2., 2., 4., 4., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 0., 1., 3.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 6],
        periodic: false,
    };
    let locate = |cells| {
        locate_directional_peak_jerk(&curve, [2., 4.], &[-1., 0.], 1e-5, 0.02, cells).unwrap()
    };
    let fine = locate(2048);
    assert_eq!(fine.parameter_intervals.len(), 1, "{fine:?}");
    let [lo, hi] = fine.parameter_intervals[0];
    assert!(lo <= 3. && 3. <= hi && hi - lo < 0.1, "{fine:?}");
    assert!(fine.peak.bounds[0] <= 3.75 && 3.75 <= fine.peak.bounds[1]);
    let coarse = locate(1);
    assert_eq!(coarse.parameter_intervals, vec![[2., 4.]]);
    assert_eq!(coarse.peak.cells, 1);
}

#[test]
fn locate_directional_peak_jerk_keeps_rational_window_boundary() {
    use nurbs_core::analysis::curve_measure::locate_directional_peak_jerk;
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let report = locate_directional_peak_jerk(&curve, [3., 4.], &[1., 0.], 1e-5, 0.01, 256)
        .unwrap();
    assert_eq!(report.parameter_intervals.len(), 1, "{report:?}");
    let [lo, hi] = report.parameter_intervals[0];
    assert!(lo == 3. && hi < 3.03, "{report:?}");
    assert!(
        locate_directional_peak_jerk(&curve, [3., 4.], &[1., 0.], 1e-5, f64::NAN, 256).is_err()
    );
}

#[test]
fn axial_motion_limits_check_each_derivative_in_the_original_clock() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus, check_directional_motion_limits,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let check = |speed, acceleration, jerk| {
        check_directional_motion_limits(
            &curve, [2., 4.], &[-2., 0.],
            DirectionalMotionLimits { speed, acceleration, jerk }, [1e-6; 3], 32,
        )
    };
    let report = check(13., 13., 7.).unwrap();
    assert_eq!(report.status, DirectionalMotionStatus::Within);
    for ([lo, hi], exact) in [report.speed.bounds, report.acceleration.bounds, report.jerk.bounds]
        .into_iter()
        .zip([12., 12., 6.])
    {
        assert!(lo <= exact && exact <= hi);
    }
    for (s, a, j) in [(11., 13., 7.), (13., 11., 7.), (13., 13., 5.)] {
        assert_eq!(check(s, a, j).unwrap().status, DirectionalMotionStatus::Exceeds);
    }
    assert!(check(13., 13., f64::NAN).is_err());
}

#[test]
fn axial_motion_limits_preserve_undetermined_status_at_coarse_budget() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus, check_directional_motion_limits,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: [0., 0., 1., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let check = |cells| {
        check_directional_motion_limits(
            &curve, [2., 6.], &[1., 0.],
            DirectionalMotionLimits { speed: 0.5, acceleration: 1., jerk: 1. },
            [1e-5; 3], cells,
        )
        .unwrap()
    };
    let coarse = check(1);
    assert_eq!(coarse.status, DirectionalMotionStatus::Undetermined);
    assert_eq!([coarse.speed.cells, coarse.acceleration.cells, coarse.jerk.cells], [1; 3]);
    assert_eq!(check(128).status, DirectionalMotionStatus::Within);
}

#[test]
fn motion_slowdown_bounds_apply_each_derivative_power() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, directional_motion_slowdown_bounds,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let bounds = |speed, acceleration, jerk| {
        directional_motion_slowdown_bounds(
            &curve, [2., 4.], &[1., 0.],
            DirectionalMotionLimits { speed, acceleration, jerk }, [1e-6; 3], 32,
        )
        .unwrap()
    };
    for limits in [(6., 20., 20.), (20., 3., 20.), (20., 20., 0.75)] {
        let [lo, hi] = bounds(limits.0, limits.1, limits.2);
        assert!(lo <= 2. && 2. <= hi && hi - lo < 1e-6, "{lo}..{hi}");
    }
    assert_eq!(bounds(20., 20., 20.), [1.; 2]);
    assert_eq!(bounds(20., 20., 0.), [f64::MAX, f64::INFINITY]);
    assert_eq!(bounds(f64::from_bits(1), 20., 20.), [f64::MAX, f64::INFINITY]);
}

#[test]
fn motion_slowdown_bounds_cover_coarse_peak_uncertainty() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, directional_motion_slowdown_bounds,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: [0., 0., 1., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let measure = |cells| {
        directional_motion_slowdown_bounds(
            &curve, [2., 6.], &[1., 0.],
            DirectionalMotionLimits { speed: 0.25, acceleration: 1., jerk: 1. },
            [1e-5; 3], cells,
        )
        .unwrap()
    };
    let coarse = measure(1);
    let fine = measure(128);
    assert_eq!(coarse[0], 1.);
    assert!(coarse[1] >= 3.);
    assert!(fine[0] <= 1.5 && 1.5 <= fine[1] && fine[1] - fine[0] < 1e-4);
}

#[test]
fn multi_axis_slowdown_reports_the_limiting_axis_in_input_order() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, MotionAxis, multi_axis_motion_slowdown_bounds,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 8. * x]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [
        MotionAxis {
            direction: &[2., 0.],
            limits: DirectionalMotionLimits { speed: 6., acceleration: 12., jerk: 6. },
        },
        MotionAxis {
            direction: &[0., -1.],
            limits: DirectionalMotionLimits { speed: 96., acceleration: 96., jerk: 0.75 },
        },
    ];
    let report = multi_axis_motion_slowdown_bounds(&curve, [2., 4.], &axes, [1e-5; 3], 32)
        .unwrap();
    for ([lo, hi], target) in report.axis_bounds.iter().copied().zip([2., 4.]) {
        assert!(lo <= target && target <= hi && hi - lo < 1e-5, "{report:?}");
    }
    assert_eq!(report.axis_bounds.len(), 2);
    assert_eq!(report.bounds, report.axis_bounds[1]);
    assert!(report.cells <= 32);
    let reversed = multi_axis_motion_slowdown_bounds(
        &curve, [2., 4.], &[axes[1], axes[0]], [1e-5; 3], 32,
    )
    .unwrap();
    assert_eq!(reversed.bounds, report.bounds);
    assert_eq!(reversed.axis_bounds, [report.axis_bounds[1], report.axis_bounds[0]]);
}

#[test]
fn multi_axis_slowdown_reserves_initial_coverage_for_every_derivative() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, MotionAxis, multi_axis_motion_slowdown_bounds,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![3., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let axes = [MotionAxis {
        direction: &[1., 0.],
        limits: DirectionalMotionLimits { speed: 1., acceleration: 1., jerk: 1. },
    }; 2];
    let check = |axes: &[MotionAxis<'_>], cells| {
        multi_axis_motion_slowdown_bounds(&curve, [0., 2.], axes, [1e-5; 3], cells)
    };
    assert!(check(&[], 12).is_err());
    assert!(check(&axes, 11).is_err());
    let report = check(&axes, 12).unwrap();
    assert_eq!(report.cells, 12);
    assert!(report.bounds[0] <= 2. && 2. <= report.bounds[1]);
    let mut blocked = axes;
    blocked[1].limits.speed = 0.;
    assert_eq!(check(&blocked, 12).unwrap().bounds, [f64::MAX, f64::INFINITY]);
}

#[test]
fn multi_axis_motion_duration_uses_window_width_and_common_dilation() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, MotionAxis, multi_axis_motion_duration_bounds,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 8. * x]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [
        MotionAxis {
            direction: &[1., 0.],
            limits: DirectionalMotionLimits { speed: 6., acceleration: 12., jerk: 6. },
        },
        MotionAxis {
            direction: &[0., -1.],
            limits: DirectionalMotionLimits { speed: 96., acceleration: 96., jerk: 0.75 },
        },
    ];
    let report =
        multi_axis_motion_duration_bounds(&curve, [2.5, 4.], &axes, [1e-5; 3], 32).unwrap();
    let [lo, hi] = report.duration_bounds;
    assert!(lo <= 6. && 6. <= hi && hi - lo < 1e-5, "{report:?}");
    assert_eq!(report.slowdown.axis_bounds.len(), 2);
    assert!(report.slowdown.cells <= 32);
    let unconstrained = [MotionAxis {
        direction: &[1., 0.],
        limits: DirectionalMotionLimits { speed: 20., acceleration: 20., jerk: 20. },
    }];
    let report =
        multi_axis_motion_duration_bounds(&curve, [2.5, 4.], &unconstrained, [1e-5; 3], 32)
            .unwrap();
    assert_eq!(report.duration_bounds, [1.5; 2]);
}

#[test]
fn multi_axis_motion_duration_encloses_overflow_with_finite_dilation() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, MotionAxis, multi_axis_motion_duration_bounds,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [MotionAxis {
        direction: &[1., 0.],
        limits: DirectionalMotionLimits { speed: 1e-307, acceleration: 20., jerk: 20. },
    }];
    let report =
        multi_axis_motion_duration_bounds(&curve, [2., 4.], &axes, [1e-5; 3], 32).unwrap();
    assert!(report.slowdown.bounds[1].is_finite());
    assert_eq!(report.duration_bounds, [f64::MAX, f64::INFINITY]);
}

#[test]
fn motion_deadline_distinguishes_limiting_axis_and_slowdown_floor() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, MotionAxis,
        check_multi_axis_motion_deadline,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 8. * x]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [
        MotionAxis {
            direction: &[1., 0.],
            limits: DirectionalMotionLimits { speed: 100., acceleration: 100., jerk: 100. },
        },
        MotionAxis {
            direction: &[0., 1.],
            limits: DirectionalMotionLimits { speed: 96., acceleration: 96., jerk: 0.75 },
        },
    ];
    let check = |axes: &[MotionAxis<'_>], deadline| {
        check_multi_axis_motion_deadline(&curve, [2.5, 4.], axes, deadline, [1e-6; 3], 64)
    };
    let short = check(&axes, 5.).unwrap();
    assert_eq!(short.axis_statuses, [Status::Within, Status::Exceeds]);
    assert_eq!(short.status, Status::Exceeds);
    assert_eq!(check(&axes, 7.).unwrap().status, Status::Within);
    let floor = check(&axes[..1], 1.).unwrap();
    assert_eq!(floor.axis_statuses, [Status::Within]);
    assert_eq!(floor.status, Status::Exceeds);
    assert!(check(&axes, f64::INFINITY).is_err());
}

#[test]
fn motion_deadline_retains_unknown_status_until_the_peak_is_resolved() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, MotionAxis,
        check_multi_axis_motion_deadline,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: [0., 0., 1., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [MotionAxis {
        direction: &[1., 0.],
        limits: DirectionalMotionLimits { speed: 0.3, acceleration: 1., jerk: 1. },
    }];
    let check = |cells| {
        check_multi_axis_motion_deadline(&curve, [2., 6.], &axes, 5.5, [1e-5; 3], cells).unwrap()
    };
    assert_eq!(check(3).status, Status::Undetermined);
    assert_eq!(check(3).cells, 3);
    assert_eq!(check(96).status, Status::Within);
}

#[test]
fn speed_limit_partition_encloses_two_crossings_and_classifies_the_interior() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_directional_speed_limit,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: [0., 0., 1., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let limit = 0.25;
    let report = partition_directional_speed_limit(&curve, [2., 6.], &[-1., 0.], limit, 0.01, 256)
        .unwrap();
    assert_eq!(report.cells.first().unwrap().parameter_interval[0], 2.);
    assert_eq!(report.cells.last().unwrap().parameter_interval[1], 6.);
    assert!(report.work_cells <= 256);
    for pair in report.cells.windows(2) {
        assert_eq!(pair[0].parameter_interval[1], pair[1].parameter_interval[0]);
    }
    let crossing_offset = 2. / 3_f64.sqrt();
    for crossing in [4. - crossing_offset, 4. + crossing_offset] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            cell.status == Status::Undetermined && a <= crossing && crossing <= b && b - a <= 0.01
        }));
    }
    for (sample, status) in [(2., Status::Within), (4., Status::Exceeds), (6., Status::Within)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            cell.status == status && a <= sample && sample <= b
        }));
    }
}

#[test]
fn speed_limit_partition_keeps_both_one_sided_knot_values() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_directional_speed_limit,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: [0., 1., 4.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let check = |limit, cells| {
        partition_directional_speed_limit(&curve, [0.5, 1.5], &[1., 0.], limit, 0.01, cells)
    };
    assert!(check(2., 1).is_err());
    assert!(check(-1., 2).is_err());
    let report = check(2., 2).unwrap();
    assert_eq!(report.work_cells, 2);
    assert_eq!(report.cells.len(), 2);
    assert_eq!(report.cells[0].parameter_interval, [0.5, 1.]);
    assert_eq!(report.cells[0].status, Status::Within);
    assert_eq!(report.cells[1].parameter_interval, [1., 1.5]);
    assert_eq!(report.cells[1].status, Status::Exceeds);
    assert_ne!(report.cells[0].span, report.cells[1].span);
    let coarse = check(1., 2).unwrap();
    assert_ne!(coarse.cells[0].status, Status::Exceeds);
}

#[test]
fn motion_limit_partition_resolves_different_derivative_crossings() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status,
        partition_directional_motion_limits,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let limits = DirectionalMotionLimits { speed: 3., acceleration: 3., jerk: 5. };
    let measure = |budget| {
        partition_directional_motion_limits(&curve, [2., 4.], &[1., 0.], limits, 0.01, budget)
            .unwrap()
    };
    let coarse = measure(1);
    assert_eq!(coarse.work_cells, 1);
    assert_eq!(coarse.cells[0].statuses, [Status::Undetermined, Status::Undetermined, Status::Exceeds]);
    let report = measure(128);
    assert!(report.work_cells <= 128);
    for (index, crossing) in [3., 2.5].into_iter().enumerate() {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01
                && cell.statuses[index] == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| {
        cell.statuses == [Status::Within, Status::Exceeds, Status::Exceeds]
    }));
    assert!(report.cells.iter().all(|cell| cell.statuses[2] == Status::Exceeds));
}

#[test]
fn motion_limit_partition_measures_jerk_of_rational_linear_support() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status,
        partition_directional_motion_limits,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![6., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let limits = DirectionalMotionLimits { speed: 10., acceleration: 10., jerk: 0.35 };
    let report =
        partition_directional_motion_limits(&curve, [3., 4.], &[-1., 0.], limits, 0.01, 256)
            .unwrap();
    assert_eq!(report.cells.first().unwrap().statuses, [Status::Within, Status::Within, Status::Exceeds]);
    assert_eq!(report.cells.last().unwrap().statuses, [Status::Within; 3]);
}

#[test]
fn multi_axis_motion_partition_aligns_distinct_crossings_on_one_grid() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, MotionAxis,
        partition_multi_axis_motion_limits,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 2. * x]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let axes = [
        MotionAxis {
            direction: &[1., 0.],
            limits: DirectionalMotionLimits { speed: 3., acceleration: 20., jerk: 20. },
        },
        MotionAxis {
            direction: &[0., -2.],
            limits: DirectionalMotionLimits { speed: 30., acceleration: 6., jerk: 20. },
        },
    ];
    let report = partition_multi_axis_motion_limits(&curve, [2., 4.], &axes, 0.01, 256)
        .unwrap();
    assert!(report.work_axis_cells <= 256);
    for ([axis, order], crossing) in [([0, 0], 3.), ([1, 1], 2.5)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01
                && cell.axis_statuses[axis][order] == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| {
        cell.axis_statuses[0] == [Status::Within; 3]
            && cell.axis_statuses[1] == [Status::Within, Status::Exceeds, Status::Within]
    }));
}

#[test]
fn multi_axis_motion_partition_budget_covers_all_axes_and_spans() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, MotionAxis,
        partition_multi_axis_motion_limits,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![1., 3.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let limits = DirectionalMotionLimits { speed: 0.5, acceleration: 1., jerk: 1. };
    let axes = [
        MotionAxis { direction: &[1., 0.], limits },
        MotionAxis { direction: &[0., 1.], limits },
    ];
    let measure = |axes: &[MotionAxis<'_>], budget| {
        partition_multi_axis_motion_limits(&curve, [0.5, 1.5], axes, 0.01, budget)
    };
    assert!(measure(&[], 4).is_err());
    assert!(measure(&axes, 3).is_err());
    let report = measure(&axes, 4).unwrap();
    assert_eq!(report.work_axis_cells, 4);
    assert_eq!(report.cells.len(), 2);
    assert_eq!(report.cells[0].parameter_interval, [0.5, 1.]);
    assert_eq!(report.cells[1].parameter_interval, [1., 1.5]);
    assert_eq!(report.cells[0].axis_statuses[0][0], Status::Exceeds);
    assert_eq!(report.cells[0].axis_statuses[1][0], Status::Within);
    assert_eq!(report.cells[1].axis_statuses[0][0], Status::Within);
    assert_eq!(report.cells[1].axis_statuses[1][0], Status::Exceeds);
}

#[test]
fn motion_knot_jump_uses_original_span_widths_and_signed_axis() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status,
        classify_directional_motion_knot_jumps,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 3., 3.],
        control_points: [0., 1., 7.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let limits = DirectionalMotionLimits { speed: 1.5, acceleration: 1., jerk: 1. };
    let check = |domain, budget| {
        classify_directional_motion_knot_jumps(&curve, domain, &[-2., 0.], limits, budget)
    };
    assert!(check([0.75, 2.], 0).is_err());
    assert!(check([1., 2.], 0).unwrap().is_empty());
    let jumps = check([0.75, 2.], 1).unwrap();
    assert_eq!(jumps.len(), 1);
    assert_eq!(jumps[0].parameter, 1.);
    assert_eq!(jumps[0].statuses, [Status::Exceeds, Status::Within, Status::Within]);
    let [lo, hi] = jumps[0].jump_bounds[0];
    assert!(lo <= -2. && -2. <= hi && hi - lo < 1e-8);
}

#[test]
fn motion_knot_jump_proves_c2_continuity_and_detects_cubic_jerk_jump() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status,
        classify_directional_motion_knot_jumps,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 2., 2., 2., 2.],
        control_points: [0., 0., 0., 0., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let limits = DirectionalMotionLimits { speed: 0., acceleration: 0., jerk: 5. };
    let report = classify_directional_motion_knot_jumps(
        &curve, [0., 2.], &[1., 0.], limits, 1,
    )
    .unwrap();
    assert_eq!(report[0].jump_bounds[..2], [[0.; 2]; 2]);
    assert_eq!(report[0].statuses, [Status::Within, Status::Within, Status::Exceeds]);
    let [lo, hi] = report[0].jump_bounds[2];
    assert!(lo <= 6. && 6. <= hi && hi - lo < 1e-8);
}

#[test]
fn motion_join_uses_independent_clocks_and_reports_off_axis_gap() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionSegment,
        classify_directional_motion_join,
    };
    let curve = |knots, points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots,
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let left = curve(vec![0., 0., 4., 4.], [[0., 0.], [8., 0.]]);
    let right = curve(vec![10., 10., 20., 20.], [[6., 3.], [16., 3.]]);
    let segments = [
        TimedMotionSegment { curve: &left, interval: [1., 3.], duration: 2. },
        TimedMotionSegment { curve: &right, interval: [10., 14.], duration: 1. },
    ];
    let limits = DirectionalMotionLimits { speed: 1., acceleration: 1., jerk: 1. };
    let report = classify_directional_motion_join(segments, &[1., 0.], limits).unwrap();
    assert_eq!(report.statuses, [Status::Exceeds, Status::Within, Status::Within]);
    let [lo, hi] = report.jump_bounds[0];
    assert!(lo <= 2. && 2. <= hi && hi - lo < 1e-8);
    let [lo, hi] = report.position_gap_bounds;
    assert!(lo <= 3. && 3. <= hi && hi - lo < 1e-8);
    let mut invalid = segments;
    invalid[1].duration = 0.;
    assert!(classify_directional_motion_join(invalid, &[1., 0.], limits).is_err());
}

#[test]
fn motion_join_uses_duration_powers_for_higher_derivatives() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionSegment,
        classify_directional_motion_join,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 4., 4., 4., 4.],
        control_points: [0., 0., 0., 8.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let segments = [
        TimedMotionSegment { curve: &curve, interval: [2., 4.], duration: 4. },
        TimedMotionSegment { curve: &curve, interval: [2., 4.], duration: 2. },
    ];
    let report = classify_directional_motion_join(
        segments, &[1., 0.], DirectionalMotionLimits { speed: 7., acceleration: 2., jerk: 5. },
    )
    .unwrap();
    assert_eq!(report.statuses, [Status::Within, Status::Exceeds, Status::Exceeds]);
    for ([lo, hi], expected) in report.jump_bounds.into_iter().zip([-6., -3., 5.25]) {
        assert!(lo <= expected && expected <= hi && hi - lo < 1e-8);
    }
}

#[test]
fn motion_chain_checks_closing_gap_and_preserves_join_order() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionSegment,
        classify_directional_motion_chain,
    };
    let line = |a, b| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0.], vec![b, 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let curves = [line(0., 1.), line(1., 3.), line(3., 4.)];
    let segments = std::array::from_fn::<_, 3, _>(|index| TimedMotionSegment {
        curve: &curves[index],
        interval: [0., 1.],
        duration: [1., 2., 1.][index],
    });
    let check = |closed, budget| {
        classify_directional_motion_chain(
            &segments, closed, &[1., 0.], 0.01,
            DirectionalMotionLimits { speed: 0.01, acceleration: 0.01, jerk: 0.01 }, budget,
        )
    };
    let open = check(false, 2).unwrap();
    assert_eq!(open.status, Status::Within);
    assert!(check(true, 2).is_err());
    let closed = check(true, 3).unwrap();
    assert_eq!(closed.status, Status::Exceeds);
    assert_eq!(closed.joins.iter().map(|join| join.segment_indices).collect::<Vec<_>>(),
        [[0, 1], [1, 2], [2, 0]]);
    assert_eq!(closed.joins[2].position_status, Status::Exceeds);
    assert_eq!(closed.joins[2].join.statuses, [Status::Within; 3]);
}

#[test]
fn motion_chain_checks_a_single_closed_segment_for_velocity_discontinuity() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionSegment,
        classify_directional_motion_chain,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 1., 0.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let segment = TimedMotionSegment { curve: &curve, interval: [0., 1.], duration: 1. };
    let limits = DirectionalMotionLimits { speed: 3., acceleration: 1., jerk: 1. };
    let report = classify_directional_motion_chain(&[segment], true, &[1., 0.], 0.01, limits, 1)
        .unwrap();
    assert_eq!(report.status, Status::Exceeds);
    assert_eq!(report.joins[0].segment_indices, [0, 0]);
    assert_eq!(report.joins[0].position_status, Status::Within);
    assert_eq!(report.joins[0].join.statuses, [Status::Exceeds, Status::Within, Status::Within]);
    assert!(classify_directional_motion_chain(&[segment], false, &[1., 0.], 0.01, limits, 1).is_err());
}

#[test]
fn timed_motion_chain_distinguishes_segment_violations_from_join_violations() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionChainLimits,
        TimedMotionSegment, check_timed_motion_chain,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: [0., 4.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let segments = [
        TimedMotionSegment { curve: &curve, interval: [2., 4.], duration: 1. },
        TimedMotionSegment { curve: &curve, interval: [4., 6.], duration: 4. },
    ];
    let limits = TimedMotionChainLimits {
        segment: DirectionalMotionLimits { speed: 1., acceleration: 1., jerk: 1. },
        join: DirectionalMotionLimits { speed: 2., acceleration: 1., jerk: 1. },
        position_gap: 0.01,
    };
    let check = |segments: &[TimedMotionSegment<'_>], limits, cells| {
        check_timed_motion_chain(segments, false, &[1., 0.], limits, [1e-5; 3], cells, 1)
    };
    assert!(check(&segments, limits, 5).is_err());
    let report = check(&segments, limits, 6).unwrap();
    assert_eq!(report.cells, 6);
    assert_eq!(report.segment_statuses, [Status::Exceeds, Status::Within]);
    assert_eq!(report.joins.status, Status::Within);
    assert_eq!(report.status, Status::Exceeds);
    let mut limits = limits;
    limits.segment.speed = 3.;
    limits.join.speed = 1.;
    let report = check(&segments, limits, 6).unwrap();
    assert_eq!(report.segment_statuses, [Status::Within; 2]);
    assert_eq!(report.joins.status, Status::Exceeds);
    assert_eq!(report.status, Status::Exceeds);
    limits.join.speed = 2.;
    assert_eq!(check(&segments, limits, 6).unwrap().status, Status::Within);
}

#[test]
fn timed_motion_knot_jumps_scale_jerk_without_losing_proved_continuity() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, TimedMotionSegment,
        classify_timed_motion_knot_jumps,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 2., 2., 2., 2.],
        control_points: [0., 0., 0., 0., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let segment = TimedMotionSegment { curve: &curve, interval: [0.5, 1.5], duration: 2. };
    let limits = DirectionalMotionLimits { speed: 0., acceleration: 0., jerk: 1. };
    let report = classify_timed_motion_knot_jumps(segment, &[1., 0.], limits, 1).unwrap();
    assert_eq!(report[0].parameter, 1.);
    assert_eq!(report[0].jump_bounds[..2], [[0.; 2]; 2]);
    assert_eq!(report[0].statuses, [Status::Within; 3]);
    let [lo, hi] = report[0].jump_bounds[2];
    assert!(lo <= 0.75 && 0.75 <= hi && hi - lo < 1e-8);
    let faster = TimedMotionSegment { duration: 0.5, ..segment };
    let report = classify_timed_motion_knot_jumps(faster, &[1., 0.], limits, 1).unwrap();
    assert_eq!(report[0].statuses[2], Status::Exceeds);
    let [lo, hi] = report[0].jump_bounds[2];
    assert!(lo <= 48. && 48. <= hi && hi - lo < 1e-8);
    let invalid = TimedMotionSegment { duration: 0., ..segment };
    assert!(classify_timed_motion_knot_jumps(invalid, &[1., 0.], limits, 1).is_err());
}

#[test]
fn motion_knot_turn_measures_rational_right_angle_under_unequal_clocks() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_knot_turns,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 3., 3.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![1., 3.]],
        weights: vec![1., 4., 2.],
        periodic: false,
    };
    assert!(classify_motion_knot_turns(&curve, [0.5, 2.], 0.5, 0).is_err());
    let report = classify_motion_knot_turns(&curve, [0.5, 2.], 0.5, 1).unwrap();
    assert_eq!(report[0].parameter, 1.);
    assert_eq!(report[0].status, Status::Exceeds);
    let [lo, hi] = report[0].cosine_bounds.unwrap();
    assert!(lo <= 0. && hi >= 0. && hi - lo < 1e-8);
    assert_eq!(classify_motion_knot_turns(&curve, [0., 3.], -0.5, 1).unwrap()[0].status, Status::Within);
}

#[test]
fn motion_knot_turn_proves_c1_alignment_and_preserves_stationary_uncertainty() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_knot_turns,
    };
    let mut curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 2., 2., 2.],
        control_points: [0., 1., 2., 3.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = classify_motion_knot_turns(&curve, [0., 2.], 1., 1).unwrap();
    assert_eq!(report[0].cosine_bounds, Some([1.; 2]));
    assert_eq!(report[0].status, Status::Within);
    curve.control_points[2] = vec![1., 0.];
    let report = classify_motion_knot_turns(&curve, [0., 2.], 0., 1).unwrap();
    assert_eq!(report[0].cosine_bounds, None);
    assert_eq!(report[0].status, Status::Undetermined);
}

#[test]
fn motion_join_turn_distinguishes_direction_from_position_gap() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_join_turn,
    };
    let line = |points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 4., 4.],
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = line([[0., 0.], [4., 0.]]);
    let second = line([[2., 3.], [2., 7.]]);
    let report = classify_motion_join_turn((&first, [1., 2.]), (&second, [0., 3.]), 0.5)
        .unwrap();
    assert_eq!(report.turn_status, Status::Exceeds);
    let [lo, hi] = report.position_gap_bounds;
    assert!(lo <= 3. && 3. <= hi && hi - lo < 1e-8);
    let [lo, hi] = report.cosine_bounds.unwrap();
    assert!(lo <= 0. && 0. <= hi && hi - lo < 1e-8);
    let reversed = line([[2., 0.], [0., 0.]]);
    let report = classify_motion_join_turn((&first, [1., 2.]), (&reversed, [0., 4.]), -0.5)
        .unwrap();
    assert_eq!(report.turn_status, Status::Exceeds);
    assert_eq!(report.cosine_bounds.unwrap()[0], -1.);
}

#[test]
fn motion_join_turn_preserves_stationary_endpoint_uncertainty() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_join_turn,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 0., 1.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let window = (&curve, [0., 1.]);
    let report = classify_motion_join_turn(window, window, 0.).unwrap();
    assert_eq!(report.cosine_bounds, None);
    assert_eq!(report.turn_status, Status::Undetermined);
    assert!(classify_motion_join_turn(window, window, 1.1).is_err());
}

#[test]
fn motion_chain_turns_check_closure_with_independent_gap_and_turn_limits() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_chain_turns,
    };
    let line = |a, b| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = line(vec![0., 0.], vec![1., 0.]);
    let second = line(vec![1., 0.], vec![1., 1.]);
    let windows = [(&first, [0., 1.]), (&second, [0., 1.])];
    let open = classify_motion_chain_turns(&windows, false, -0.5, 1e-6, 1).unwrap();
    assert_eq!(open.status, Status::Within);
    let closed = classify_motion_chain_turns(&windows, true, -0.5, 1e-6, 2).unwrap();
    assert_eq!(closed.status, Status::Exceeds);
    assert_eq!(closed.joins[1].segment_indices, [1, 0]);
    assert_eq!(closed.joins[1].gap_status, Status::Exceeds);
    assert_eq!(closed.joins[1].join.turn_status, Status::Within);
    assert!(classify_motion_chain_turns(&windows, true, -0.5, 1e-6, 1).is_err());
    let sharp = classify_motion_chain_turns(&windows, false, 0.5, 1e-6, 1).unwrap();
    assert_eq!(sharp.joins[0].gap_status, Status::Within);
    assert_eq!(sharp.joins[0].join.turn_status, Status::Exceeds);
    assert_eq!(sharp.status, Status::Exceeds);
}

#[test]
fn motion_chain_turns_keep_a_stationary_closing_tangent_unresolved() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, classify_motion_chain_turns,
    };
    let curve = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: [0., 0., 1., 0.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = classify_motion_chain_turns(&[(&curve, [0., 1.])], true, 0., 1e-6, 1)
        .unwrap();
    assert_eq!(report.joins[0].segment_indices, [0, 0]);
    assert_eq!(report.joins[0].gap_status, Status::Within);
    assert_eq!(report.status, Status::Undetermined);
}

#[test]
fn directional_position_partition_locates_interior_overtravel_with_safe_endpoints() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_directional_position_range,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 2., 0.].map(|x| vec![x, 0.]).into(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let check = |cells| {
        partition_directional_position_range(&curve, [0., 1.], &[1., 0.], [-1., 0.75], 0.01, cells)
            .unwrap()
    };
    assert_eq!(check(1).cells[0].status, Status::Undetermined);
    let report = check(128);
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        cell.parameter_interval[0] <= 0.5 && 0.5 <= cell.parameter_interval[1]
            && cell.status == Status::Exceeds
    }));
    for crossing in [0.25, 0.75] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn directional_position_partition_handles_signed_oblique_projection() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_directional_position_range,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![3., 4.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let report = partition_directional_position_range(
        &curve, [0., 1.], &[-3., -4.], [-3., 0.5], 0.01, 128,
    )
    .unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    let last = report.cells.last().unwrap();
    assert_eq!(last.status, Status::Exceeds);
    assert!(last.position_bounds[0] <= -5. && last.position_bounds[1] >= -5.);
    assert!(partition_directional_position_range(
        &curve, [0., 1.], &[-3., -4.], [0.5, -3.], 0.01, 128,
    ).is_err());
}

#[test]
fn multi_axis_position_partition_aligns_distinct_limit_crossings() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, PositionAxis, partition_multi_axis_position_ranges,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 0.5], vec![0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let axes = [
        PositionAxis { direction: &[1., 0.], allowed: [-1., 0.75] },
        PositionAxis { direction: &[0., -1.], allowed: [-0.6, 1.] },
    ];
    assert!(partition_multi_axis_position_ranges(&curve, [0., 1.], &axes, 0.01, 1).is_err());
    let report =
        partition_multi_axis_position_ranges(&curve, [0., 1.], &axes, 0.01, 256).unwrap();
    assert!(report.work_axis_cells <= 256);
    for (axis, crossing) in [(0, 0.25), (1, 0.6), (0, 0.75)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01
                && cell.axes[axis].status == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| {
        cell.axes[0].status == Status::Exceeds && cell.axes[1].status == Status::Within
    }));
    assert_eq!(report.cells.last().unwrap().axes[1].status, Status::Exceeds);
    for adjacent in report.cells.windows(2) {
        assert_eq!(adjacent[0].parameter_interval[1], adjacent[1].parameter_interval[0]);
    }
}

#[test]
fn trajectory_limit_partition_preserves_independent_position_and_motion_results() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionLimits, DirectionalMotionStatus as Status, PositionAxis,
        TrajectoryAxis, partition_trajectory_limits,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 0.5], vec![0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut axes = [
        TrajectoryAxis {
            position: PositionAxis { direction: &[1., 0.], allowed: [-1., 0.75] },
            motion: DirectionalMotionLimits { speed: 3., acceleration: 9., jerk: 1. },
        },
        TrajectoryAxis {
            position: PositionAxis { direction: &[0., 1.], allowed: [-1., 2.] },
            motion: DirectionalMotionLimits { speed: 0.5, acceleration: 1., jerk: 1. },
        },
    ];
    let check = |axes: &[TrajectoryAxis<'_>], cells| {
        partition_trajectory_limits(&curve, [0., 1.], axes, 0.01, cells)
    };
    let coarse = check(&axes, 2).unwrap();
    assert_eq!(coarse.work_axis_cells, 2);
    assert_eq!(coarse.cells[0].axes[0].position.status, Status::Undetermined);
    assert_eq!(coarse.cells[0].axes[1].motion[0], Status::Exceeds);
    let report = check(&axes, 256).unwrap();
    assert!(report.work_axis_cells <= 256);
    let first = report.cells.first().unwrap();
    assert_eq!(first.axes[0].position.status, Status::Within);
    assert_eq!(first.axes[0].motion[0], Status::Exceeds);
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && cell.axes[0].position.status == Status::Exceeds
            && cell.axes[0].motion == [Status::Within; 3]
    }));
    axes[1].motion.jerk = -1.;
    assert!(check(&axes, 256).is_err());
}

#[test]
fn convex_workspace_partition_checks_shifted_oblique_planes_in_original_order() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_curve_convex_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![2., 1.], vec![4., 3.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let mut planes = [
        WorkspaceHalfSpace { normal: &[-2., -2.], offset: -8. },
        WorkspaceHalfSpace { normal: &[1., 1.], offset: 6. },
    ];
    let check = |planes: &[WorkspaceHalfSpace<'_>], budget| {
        partition_curve_convex_workspace(&curve, [0., 1.], planes, 0.01, budget)
    };
    assert!(check(&planes, 1).is_err());
    let report = check(&planes, 128).unwrap();
    assert!(report.work_plane_cells <= 128);
    assert_eq!(report.cells.first().unwrap().status, Status::Exceeds);
    assert!(report.cells.first().unwrap().plane_residuals[0][0] > 0.);
    assert_eq!(report.cells.last().unwrap().status, Status::Exceeds);
    assert!(report.cells.last().unwrap().plane_residuals[1][0] > 0.);
    assert!(report.cells.iter().any(|cell| cell.status == Status::Within));
    for crossing in [0.25, 0.75] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
    planes[1].normal = &[0., 0.];
    assert!(check(&planes, 128).is_err());
}

#[test]
fn obstacle_clearance_partition_distinguishes_two_separated_obstacles() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_curve_obstacle_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = [
        WorkspaceHalfSpace { normal: &[-1., 0.], offset: -1. },
        WorkspaceHalfSpace { normal: &[1., 0.], offset: 3. },
    ];
    let second = [
        WorkspaceHalfSpace { normal: &[-1., 0.], offset: -5. },
        WorkspaceHalfSpace { normal: &[1., 0.], offset: 7. },
    ];
    let obstacles = [&first[..], &second[..]];
    let check = |obstacles: &[&[WorkspaceHalfSpace<'_>]], budget| {
        partition_curve_obstacle_clearance(&curve, [0., 1.], obstacles, 0.01, budget)
    };
    assert!(check(&obstacles, 3).is_err());
    assert!(check(&[&[]], 256).is_err());
    let report = check(&obstacles, 512).unwrap();
    assert!(report.work_plane_cells <= 512);
    for (t, expected) in [
        (0.25, [Status::Exceeds, Status::Within]),
        (0.5, [Status::Within; 2]),
        (0.75, [Status::Within, Status::Exceeds]),
    ] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= t && t <= b && cell.obstacles == expected
                && cell.status == if t == 0.5 { Status::Within } else { Status::Exceeds }
        }));
    }
    for t in [0.125, 0.375, 0.625, 0.875] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= t && t <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn ball_obstacle_clearance_respects_tool_radius_and_normal_scale() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_curve_ball_obstacle_clearance,
    };
    let line = |height| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., height], vec![2., height]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacle = [WorkspaceHalfSpace { normal: &[0., 4.], offset: 0. }];
    let check = |curve: &Curve, radius| {
        partition_curve_ball_obstacle_clearance(&curve, [0., 1.], &[&obstacle], radius, 0.01, 1)
    };
    assert_eq!(check(&line(0.25), 0.125).unwrap().cells[0].status, Status::Within);
    assert_eq!(check(&line(0.25), 0.25).unwrap().cells[0].status, Status::Undetermined);
    assert_eq!(check(&line(0.25), 0.5).unwrap().cells[0].status, Status::Undetermined);
    assert_eq!(check(&line(-0.25), 0.5).unwrap().cells[0].status, Status::Exceeds);
    assert!(check(&line(0.25), -1.).is_err());
}

#[test]
fn ball_workspace_partition_finds_overtravel_before_the_center_crosses() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_curve_ball_convex_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let plane = [WorkspaceHalfSpace { normal: &[3., 4.], offset: 6. }];
    let check = |radius, budget| {
        partition_curve_ball_convex_workspace(&curve, [0., 1.], &plane, radius, 0.01, budget)
    };
    let report = check(0.6, 128).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Exceeds);
    for (parameter, status) in [(0.5, Status::Undetermined), (0.75, Status::Exceeds)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= parameter && parameter <= b && cell.status == status
        }));
    }
    let [lo, hi] = report.cells.last().unwrap().plane_residuals[0];
    assert!(lo > 0. && hi >= 3.);
    assert_eq!(check(0., 128).unwrap().cells.first().unwrap().status, Status::Within);
    assert_eq!(check(0.6, 1).unwrap().cells[0].status, Status::Undetermined);
    assert!(check(f64::NAN, 128).is_err());
}

#[test]
fn capsule_workspace_partition_accounts_for_offset_and_radius() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceCapsule, WorkspaceHalfSpace,
        partition_curve_capsule_convex_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let planes = [WorkspaceHalfSpace { normal: &[2., 0.], offset: 4. }];
    let check = |offsets, radius| {
        partition_curve_capsule_convex_workspace(
            &curve, [0., 1.], &planes,
            WorkspaceCapsule { endpoint_offsets: offsets, radius }, 0.01, 128,
        )
    };
    let offsets = [&[0., 0.][..], &[0.5, 1.][..]];
    for (radius, crossing) in [(0., 0.75), (0.5, 0.5)] {
        let report = check(offsets, radius).unwrap();
        assert_eq!(report.cells.first().unwrap().status, Status::Within);
        assert_eq!(report.cells.last().unwrap().status, Status::Exceeds);
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01
                && cell.status == Status::Undetermined
        }));
    }
    assert!(check([&[0.], &[0., 1.]], 0.).is_err());
}

#[test]
fn rounded_hull_workspace_selects_distinct_support_vertices_per_plane() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_curve_rounded_hull_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![2., 2.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let planes = [
        WorkspaceHalfSpace { normal: &[2., 0.], offset: 5. },
        WorkspaceHalfSpace { normal: &[0., 1.], offset: 2.75 },
    ];
    let vertices = [&[0., 1.5][..], &[1., 0.][..], &[-1., -1.][..]];
    let verify = |vertices: &[&[f64]], radius| {
        partition_curve_rounded_hull_workspace(
            &curve, [0., 1.], &planes, vertices, radius, 0.01, 256,
        )
    };
    let report = verify(&vertices, 0.25).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    let last = report.cells.last().unwrap();
    assert_eq!(last.status, Status::Exceeds);
    for ([lo, hi], expected) in last.plane_residuals.iter().copied().zip([1.5, 1.]) {
        assert!(lo <= expected && expected <= hi);
    }
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && b - a <= 0.01 && cell.status == Status::Undetermined
    }));
    assert!(verify(&[], 0.).is_err());
    assert!(verify(&[&[1.]], 0.).is_err());
}

#[test]
fn rounded_hull_obstacle_clearance_detects_contact_away_from_curve_origin() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_curve_rounded_hull_obstacle_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 2.], vec![1., 2.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacle = [
        WorkspaceHalfSpace { normal: &[0., 2.], offset: 0. },
        WorkspaceHalfSpace { normal: &[-1., 0.], offset: 1. },
    ];
    let check = |vertices: &[&[f64]], radius, budget| {
        partition_curve_rounded_hull_obstacle_clearance(
            &curve, [0., 1.], &[&obstacle], vertices, radius, 0.01, budget,
        )
    };
    let colliding = [&[0., 0.][..], &[0., -3.][..], &[1., 0.][..]];
    assert!(check(&colliding, 0., 5).is_err());
    let report = check(&colliding, 0., 6).unwrap();
    assert_eq!(report.work_plane_cells, 6);
    assert_eq!(report.cells[0].status, Status::Exceeds);
    let clear = [&[0., -1.][..], &[1., 0.][..]];
    assert_eq!(check(&clear, 0.5, 4).unwrap().cells[0].status, Status::Within);
    assert_eq!(check(&clear, 1.5, 4).unwrap().cells[0].status, Status::Undetermined);
}

#[test]
fn spherical_clearance_certifies_collision_with_center_outside_obstacle() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, SphericalObstacle, partition_curve_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 0.75], vec![2., 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        SphericalObstacle { center: &[0., 0.], radius: 0.5 },
        SphericalObstacle { center: &[0., 5.], radius: 1. },
    ];
    let check = |radius, budget| {
        partition_curve_spherical_clearance(&curve, [0., 1.], &obstacles, radius, 0.01, budget)
    };
    assert!(check(0.5, 1).is_err());
    assert_eq!(check(0., 2).unwrap().cells[0].status, Status::Within);
    let report = check(0.5, 256).unwrap();
    assert!(report.work_obstacle_cells <= 256);
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && cell.status == Status::Exceeds
            && cell.gap_bounds[0][0] <= -0.25 && cell.gap_bounds[0][1] >= -0.25
            && cell.gap_bounds[1][0] > 0.
    }));
    for parameter in [0.5 - 7_f64.sqrt() / 16., 0.5 + 7_f64.sqrt() / 16.] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= parameter && parameter <= b && b - a <= 0.01
                && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn capsule_clearance_checks_body_caps_and_degenerate_axis() {
    use nurbs_core::analysis::curve_measure::{
        CapsuleObstacle, DirectionalMotionStatus as Status, partition_curve_capsule_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 0.75], vec![4., 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        CapsuleObstacle { endpoints: [&[0., 0.], &[2., 0.]], radius: 0.5 },
        CapsuleObstacle { endpoints: [&[1., 5.]; 2], radius: 1. },
    ];
    let report = partition_curve_capsule_clearance(
        &curve, [0., 1.], &obstacles, 0.5, 0.01, 512,
    ).unwrap();
    assert!(report.work_obstacle_cells <= 512);
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && cell.status == Status::Exceeds
            && cell.gap_bounds[0][0] <= -0.25 && cell.gap_bounds[1][0] > 0.
    }));
    let cap = 7_f64.sqrt() / 4.;
    for t in [(2. - cap) / 6., (4. + cap) / 6.] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= t && t <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn box_clearance_checks_euclidean_corner_and_solid_interior() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, DirectionalMotionStatus as Status, partition_curve_box_clearance,
    };
    let line = |points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacle = BoxObstacle { minimum: &[0., 0.], maximum: &[1., 1.] };
    let measure = |curve: &Curve, radius| {
        partition_curve_box_clearance(curve, [0., 1.], &[obstacle], radius, 0.01, 128)
            .unwrap()
    };
    let outside_corner = line([[1.6, 1.8], [2., 2.]]);
    let report = measure(&outside_corner, 0.9);
    assert_eq!(report.cells[0].status, Status::Within);
    assert!(report.cells[0].gap_bounds[0][0] > 0.);
    let report = measure(&outside_corner, 1.1);
    assert_eq!(report.cells.first().unwrap().status, Status::Exceeds);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    let inside = line([[0.2, 0.2], [0.8, 0.8]]);
    let report = measure(&inside, 0.);
    assert_eq!(report.cells[0].status, Status::Exceeds);
    assert_eq!(report.cells[0].gap_bounds[0], [0.; 2]);
    let invalid = BoxObstacle { minimum: &[2., 0.], ..obstacle };
    assert!(partition_curve_box_clearance(&inside, [0., 1.], &[invalid], 0., 0.01, 1).is_err());
}

#[test]
fn oriented_box_clearance_uses_local_axis_extents() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, OrientedBoxObstacle, partition_curve_oriented_box_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![10., 20.], vec![13., 24.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let box_shape = OrientedBoxObstacle {
        center: &[10., 20.],
        axes: &[&[3., 4.], &[-8., 6.]],
        half_extents: &[2., 0.25],
    };
    let measure = |obstacle, radius| {
        partition_curve_oriented_box_clearance(&curve, [0., 1.], &[obstacle], radius, 0.01, 128)
    };
    let report = measure(box_shape, 0.5).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Exceeds);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && b - a <= 0.01 && cell.status == Status::Undetermined
    }));
    let [lo, hi] = report.cells.last().unwrap().gap_bounds[0];
    assert!(lo <= 2.5 && hi >= 2.5);
    let skew = OrientedBoxObstacle { axes: &[&[3., 4.], &[-8., 7.]], ..box_shape };
    assert!(measure(skew, 0.5).is_err());
}

#[test]
fn first_toolpath_contact_bounds_earliest_contact_across_multiple_obstacles() {
    use nurbs_core::analysis::curve_measure::{
        FirstToolpathContact, SphericalObstacle, partition_curve_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 10., 10.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        SphericalObstacle { center: &[6., 0.], radius: 0.5 },
        SphericalObstacle { center: &[3., 0.], radius: 0.5 },
    ];
    let check = |window, budget| {
        partition_curve_spherical_clearance(&curve, window, &obstacles, 0., 0.01, budget)
            .unwrap()
            .first_contact()
    };
    assert_eq!(check([2., 4.], 2), FirstToolpathContact::Clear);
    assert_eq!(check([2., 10.], 2), FirstToolpathContact::Possible { earliest_parameter: 2. });
    let FirstToolpathContact::Confirmed { parameter_bounds: [lo, hi], obstacle_index } =
        check([2., 10.], 512)
    else {
        panic!("expected confirmed contact");
    };
    assert_eq!(obstacle_index, 1);
    assert!(lo <= 4.5 && hi >= 4.5 && hi - lo < 0.025, "{lo}..{hi}");
    assert_eq!(check([4.75, 5.25], 2), FirstToolpathContact::Confirmed {
        parameter_bounds: [4.75; 2],
        obstacle_index: 1,
    });
}

#[test]
fn first_toolpath_contact_preserves_unresolved_tangency_before_later_collision() {
    use nurbs_core::analysis::curve_measure::{
        FirstToolpathContact, SphericalObstacle, partition_curve_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        SphericalObstacle { center: &[2., 1.], radius: 1. },
        SphericalObstacle { center: &[6., 0.], radius: 0.5 },
    ];
    let report = partition_curve_spherical_clearance(&curve, [0., 1.], &obstacles, 0., 0.002, 512)
        .unwrap();
    let FirstToolpathContact::Confirmed { parameter_bounds: [lo, hi], obstacle_index } =
        report.first_contact()
    else {
        panic!("expected a later confirmed contact");
    };
    assert_eq!(obstacle_index, 1);
    assert!(0.24 < lo && lo <= 0.25 && hi >= 5.5 / 8.);
}

#[test]
fn minimum_toolpath_clearance_encloses_minimum_on_clear_and_colliding_paths() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, partition_curve_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 0.75], vec![2., 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        SphericalObstacle { center: &[0., 5.], radius: 1. },
        SphericalObstacle { center: &[0., 0.], radius: 0.5 },
    ];
    for (radius, exact) in [(0., 0.25), (0.5, -0.25)] {
        let report = partition_curve_spherical_clearance(
            &curve, [0., 1.], &obstacles, radius, 0.01, 256,
        ).unwrap();
        let minimum = report.minimum_clearance().unwrap();
        let [lo, hi] = minimum.gap_bounds;
        assert!(lo <= exact && exact <= hi);
        let [cell, obstacle] = minimum.upper_witness;
        assert_eq!(obstacle, 1);
        assert_eq!(report.cells[cell].gap_bounds[obstacle][1], hi);
        assert!(minimum.parameter_intervals.iter().any(|[lo, hi]| *lo <= 0.5 && 0.5 <= *hi));
    }
}

#[test]
fn minimum_toolpath_clearance_preserves_disjoint_tied_intervals() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, SphericalClearanceCell, SphericalClearancePartition,
    };
    let make_cell = |parameter_interval, bounds| SphericalClearanceCell {
        parameter_interval,
        span: 1,
        gap_bounds: vec![bounds],
        status: Status::Within,
    };
    let report = SphericalClearancePartition {
        cells: vec![
            make_cell([0., 1.], [1., 2.]),
            make_cell([1., 2.], [3., 4.]),
            make_cell([2., 3.], [1., 2.]),
            make_cell([3., 4.], [1.5, 3.]),
        ],
        work_obstacle_cells: 4,
    };
    let minimum = report.minimum_clearance().unwrap();
    assert_eq!(minimum.gap_bounds, [1., 2.]);
    assert_eq!(minimum.parameter_intervals, [[0., 1.], [2., 4.]]);
    assert_eq!(minimum.upper_witness, [0, 0]);
}

#[test]
fn minimum_spherical_clearance_refines_an_oblique_clear_path() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, StopReason, measure_minimum_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![4., 4.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        SphericalObstacle { center: &[10., 10.], radius: 1. },
        SphericalObstacle { center: &[2., 0.], radius: 0.25 },
    ];
    let exact = 2_f64.sqrt() - 0.5;
    let check = |budget| {
        measure_minimum_spherical_clearance(&curve, [2., 6.], &obstacles, 0.25, 1e-4, budget)
    };
    assert!(check(1).is_err());
    let coarse = check(2).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(4096).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo <= 1e-4);
    assert_eq!(fine.witness_obstacle, 1);
    assert!((fine.witness_parameter - 3.).abs() < 0.01);
    assert!(fine.work_obstacle_cells <= 4096);
}

#[test]
fn minimum_box_clearance_refines_a_corner_approach_and_reports_the_obstacle() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, StopReason, measure_minimum_box_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![4., 4.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        BoxObstacle { minimum: &[10., 10.], maximum: &[11., 11.] },
        BoxObstacle { minimum: &[2., -1.], maximum: &[3., 0.] },
    ];
    let exact = 2_f64.sqrt() - 0.5;
    let check = |budget| {
        measure_minimum_box_clearance(&curve, [2., 6.], &obstacles, 0.5, 1e-4, budget)
    };
    assert!(check(1).is_err());
    let coarse = check(2).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(4096).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo <= 1e-4);
    assert_eq!(fine.witness_obstacle, 1);
    assert!((fine.witness_parameter - 3.).abs() < 0.01);
}

#[test]
fn minimum_oriented_box_clearance_refines_rotated_corner_distance() {
    use nurbs_core::analysis::curve_measure::{
        OrientedBoxObstacle, StopReason, measure_minimum_oriented_box_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![-1., 7.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacle = OrientedBoxObstacle {
        center: &[3., 4.],
        axes: &[&[3., 4.], &[-4., 3.]],
        half_extents: &[1., 1.],
    };
    let check = |obstacle, budget| {
        measure_minimum_oriented_box_clearance(&curve, [2., 6.], &[obstacle], 0.5, 1e-4, budget)
    };
    let exact = 3. / 2_f64.sqrt() - 0.5;
    let coarse = check(obstacle, 1).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(obstacle, 4096).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo <= 1e-4);
    assert!((fine.witness_parameter - 4.).abs() < 0.01);
    let skew = OrientedBoxObstacle { axes: &[&[3., 4.], &[-4., 2.]], ..obstacle };
    assert!(check(skew, 4096).is_err());
}

#[test]
fn minimum_capsule_clearance_refines_to_a_rounded_tip() {
    use nurbs_core::analysis::curve_measure::{
        CapsuleObstacle, StopReason, measure_minimum_capsule_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![4., 4.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        CapsuleObstacle { endpoints: [&[10., 10.]; 2], radius: 1. },
        CapsuleObstacle { endpoints: [&[2., 0.], &[3., -1.]], radius: 0.25 },
    ];
    let exact = 2_f64.sqrt() - 0.5;
    let check = |budget| {
        measure_minimum_capsule_clearance(&curve, [2., 6.], &obstacles, 0.25, 1e-4, budget)
    };
    assert!(check(1).is_err());
    let coarse = check(2).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(4096).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo <= 1e-4);
    assert_eq!(fine.witness_obstacle, 1);
    assert!((fine.witness_parameter - 3.).abs() < 0.01);
}

#[test]
fn minimum_toolpath_clearance_measures_mixed_obstacles_in_input_order() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, CapsuleObstacle, OrientedBoxObstacle, SphericalObstacle, StopReason,
        ToolpathObstacle, measure_minimum_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![4., 4.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let mut obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[9., 9.], radius: 1. }),
        ToolpathObstacle::Box(BoxObstacle { minimum: &[2., -1.], maximum: &[3., 0.] }),
        ToolpathObstacle::Capsule(CapsuleObstacle { endpoints: [&[1., 4.], &[0., 4.]], radius: 0.1 }),
        ToolpathObstacle::OrientedBox(OrientedBoxObstacle {
            center: &[8., 0.], axes: &[&[3., 4.], &[-4., 3.]], half_extents: &[1., 2.],
        }),
    ];
    let check = |obstacles: &[ToolpathObstacle<'_>], budget| {
        measure_minimum_toolpath_clearance(&curve, [2., 6.], obstacles, 0.5, 1e-4, budget)
    };
    assert!(check(&obstacles, 3).is_err());
    let exact = 2_f64.sqrt() - 0.5;
    let report = check(&obstacles, 8192).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    let [lo, hi] = report.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo <= 1e-4);
    assert_eq!(report.witness_obstacle, 1);
    assert!(report.work_obstacle_cells <= 8192);
    obstacles.swap(1, 3);
    assert_eq!(check(&obstacles, 8192).unwrap().witness_obstacle, 3);
}

#[test]
fn mixed_toolpath_partition_reports_separated_contacts_in_one_grid() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, DirectionalMotionStatus as Status, FirstToolpathContact,
        SphericalObstacle, ToolpathObstacle, partition_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Box(BoxObstacle { minimum: &[5., -1.], maximum: &[7., 1.] }),
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[2., 0.], radius: 0.5 }),
    ];
    let check = |budget| partition_toolpath_clearance(&curve, [0., 1.], &obstacles, 0.25, 0.01, budget);
    assert!(check(1).is_err());
    let report = check(512).unwrap();
    assert!(report.work_obstacle_cells <= 512);
    let FirstToolpathContact::Confirmed { parameter_bounds: [lo, hi], obstacle_index } =
        report.first_contact()
    else {
        panic!("expected a confirmed contact");
    };
    assert_eq!(obstacle_index, 1);
    assert!(lo <= 1.25 / 8. && hi >= 1.25 / 8. && hi - lo < 0.02);
    for (parameter, status) in [(0.5, Status::Within), (0.75, Status::Exceeds)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= parameter && parameter <= b && cell.status == status
                && cell.gap_bounds[1][0] > 0.
        }));
    }
    for pair in report.cells.windows(2) {
        assert_eq!(pair[0].parameter_interval[1], pair[1].parameter_interval[0]);
    }
}

#[test]
fn toolpath_clearance_margin_detects_near_miss_before_contact() {
    use nurbs_core::analysis::curve_measure::{
        FirstToolpathContact, SphericalObstacle, ToolpathObstacle,
        partition_toolpath_clearance_with_margin,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 0.75], vec![2., 0.75]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle {
        center: &[0., 0.], radius: 0.25,
    })];
    let check = |margin| {
        partition_toolpath_clearance_with_margin(&curve, [0., 1.], &obstacles, 0.25, margin, 0.01, 128)
    };
    assert_eq!(check(0.).unwrap().first_contact(), FirstToolpathContact::Clear);
    assert!(check(-0.1).is_err());
    let report = check(0.5).unwrap();
    let FirstToolpathContact::Confirmed { parameter_bounds: [a, b], obstacle_index: 0 } =
        report.first_contact()
    else {
        panic!("expected confirmed margin violation");
    };
    let crossing = 0.5 - 7_f64.sqrt() / 16.;
    assert!(a <= crossing && crossing <= b && b - a < 0.02);
    let bounds = report.minimum_clearance().unwrap().gap_bounds;
    assert!(bounds[0] <= -0.25 && -0.25 <= bounds[1]);
}

#[test]
fn toolpath_violation_length_measures_union_in_model_units() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, ToolpathObstacle, measure_toolpath_clearance_violation_length,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 3., 3.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[3., 0.], radius: 1. }),
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[4., 0.], radius: 1. }),
    ];
    let check = |window, obstacle_budget, length_budget| {
        measure_toolpath_clearance_violation_length(
            &curve, window, &obstacles, 0., 0.25, 0.001, 1e-6, obstacle_budget, length_budget,
        )
    };
    let fine = check([2., 3.], 512, 256).unwrap();
    let [lo, hi] = fine.bounds;
    assert!(lo <= 3.5 && 3.5 <= hi && hi - lo < 0.04, "{fine:?}");
    assert!(fine.work_obstacle_cells <= 512 && fine.work_length_cells <= 256);
    let coarse = check([2., 3.], 2, 1).unwrap();
    assert_eq!(coarse.bounds[0], 0.);
    assert!(coarse.bounds[1] >= 8.);
    assert_eq!(check([2., 2.125], 2, 1).unwrap().bounds, [0.; 2]);
    assert!(check([2., 3.], 512, 1).is_err());
}

#[test]
fn toolpath_violation_lengths_attribute_shared_cells_to_each_obstacle() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, SphericalObstacle, ToolpathObstacle,
        measure_toolpath_violation_lengths_by_obstacle,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 3., 3.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[3., 0.], radius: 1. }),
        ToolpathObstacle::Box(BoxObstacle { minimum: &[3., -1.], maximum: &[6., 1.] }),
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[0., 9.], radius: 1. }),
    ];
    let check = |obstacle_budget, length_budget| {
        measure_toolpath_violation_lengths_by_obstacle(
            &curve, [2., 3.], &obstacles, 0., 0.25, 0.001, 1e-6, obstacle_budget, length_budget,
        )
    };
    let report = check(768, 128).unwrap();
    for (bounds, exact) in report.by_obstacle.iter().zip([2.5, 3.5, 0.]) {
        assert!(bounds[0] <= exact && exact <= bounds[1] && bounds[1] - bounds[0] < 0.04);
    }
    let [lo, hi] = report.union.bounds;
    assert!(lo <= 4.5 && 4.5 <= hi && hi - lo < 0.04);
    let coarse = check(3, 1).unwrap();
    assert_eq!(coarse.union.work_length_cells, 1);
    assert_eq!(coarse.by_obstacle[0][0], 0.);
    assert!(coarse.by_obstacle[0][1] >= 8.);
    assert_eq!(coarse.by_obstacle[2], [0.; 2]);
}

#[test]
fn toolpath_violation_duration_uses_time_window_and_preserves_overlap() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, SphericalObstacle, TimedMotionSegment, ToolpathObstacle,
        measure_toolpath_clearance_violation_duration,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[3., 0.], radius: 1. }),
        ToolpathObstacle::Box(BoxObstacle { minimum: &[3., -1.], maximum: &[6., 1.] }),
    ];
    let segment = TimedMotionSegment { curve: &curve, interval: [2.5, 5.5], duration: 9. };
    let check = |segment, budget| {
        measure_toolpath_clearance_violation_duration(segment, &obstacles, 0., 0., 0.001, budget)
    };
    let coarse = check(segment, 2).unwrap();
    assert_eq!(coarse.bounds, [0., 9.]);
    let fine = check(segment, 512).unwrap();
    for ([lo, hi], expected) in fine.by_obstacle.into_iter().zip([3., 4.5]) {
        assert!(lo <= expected && expected <= hi && hi - lo < 0.02);
    }
    let [lo, hi] = fine.bounds;
    assert!(lo <= 6. && 6. <= hi && hi - lo < 0.02);
    assert!(check(TimedMotionSegment { duration: 0., ..segment }, 512).is_err());
}

#[test]
fn toolpath_chain_violation_duration_uses_each_clock_and_one_work_budget() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, TimedMotionSegment, ToolpathObstacle, measure_toolpath_chain_violation_duration,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let segments = [
        TimedMotionSegment { curve: &curve, interval: [0., 2.], duration: 8. },
        TimedMotionSegment { curve: &curve, interval: [1., 2.], duration: 3. },
    ];
    let obstacles = [ToolpathObstacle::Box(BoxObstacle {
        minimum: &[1., -1.], maximum: &[3., 1.],
    })];
    let check = |budget| {
        measure_toolpath_chain_violation_duration(&segments, &obstacles, 0., 0., 0.001, budget)
    };
    assert!(check(2).is_err());
    assert_eq!(check(3).unwrap().total.bounds, [0., 11.]);
    let report = check(256).unwrap();
    for (segment, exact) in report.segments.iter().zip([4., 1.5]) {
        assert!(segment.bounds[0] <= exact && segment.bounds[1] >= exact);
    }
    let [lo, hi] = report.total.bounds;
    assert!(lo <= 5.5 && 5.5 <= hi && hi - lo < 0.03);
    assert_eq!(report.total.by_obstacle, [report.total.bounds]);
    assert!(report.total.work_obstacle_cells <= 256);
}

#[test]
fn first_toolpath_chain_violation_maps_witness_and_earlier_uncertainty_to_time() {
    use nurbs_core::analysis::curve_measure::{
        FirstToolpathChainViolation, SphericalObstacle, TimedMotionSegment, ToolpathObstacle,
        first_toolpath_chain_violation,
    };
    let line = |y| Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., y], vec![4., y]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let tangent = line(1.);
    let crossing = line(0.);
    let mut segments = [
        TimedMotionSegment { curve: &tangent, interval: [2., 6.], duration: 8. },
        TimedMotionSegment { curve: &crossing, interval: [2., 6.], duration: 4. },
    ];
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle { center: &[2., 0.], radius: 1. })];
    let search = |segments: &[TimedMotionSegment<'_>], budget| {
        first_toolpath_chain_violation(segments, &obstacles, 0., 0., 0.01, budget)
    };
    assert!(search(&segments, 1).is_err());
    let FirstToolpathChainViolation::Confirmed {
        time_bounds: [lo, hi], earliest_segment: 0, witness_segment: 1, obstacle_index: 0,
    } = search(&segments, 512).unwrap()
    else { panic!("expected the second segment to supply a contact witness"); };
    assert!(3.97 < lo && lo <= 4. && 9. <= hi && hi < 9.02);
    segments[0].interval = [2., 3.];
    let FirstToolpathChainViolation::Confirmed {
        time_bounds: [lo, hi], earliest_segment: 1, witness_segment: 1, ..
    } = search(&segments, 512).unwrap()
    else { panic!("expected first possible contact on the second segment"); };
    assert!(lo <= 9. && 9. <= hi && hi - lo < 0.03);
}

#[test]
fn minimum_toolpath_chain_clearance_reports_elapsed_witness_time() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, TimedMotionSegment, ToolpathObstacle,
        measure_minimum_toolpath_chain_clearance,
    };
    let line = |points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = line([[0., 5.], [0., 0.]]);
    let second = line([[0., 0.], [4., 4.]]);
    let segments = [
        TimedMotionSegment { curve: &first, interval: [2., 6.], duration: 7. },
        TimedMotionSegment { curve: &second, interval: [2., 6.], duration: 3. },
    ];
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle {
        center: &[2., 0.], radius: 0.25,
    })];
    let check = |budget| {
        measure_minimum_toolpath_chain_clearance(&segments, &obstacles, 0.25, 1e-4, budget)
    };
    assert!(check(1).is_err());
    let exact = 2_f64.sqrt() - 0.5;
    let coarse = check(2).unwrap();
    assert!(!coarse.within_tolerance);
    assert!(coarse.gap_bounds[0] <= exact && coarse.gap_bounds[1] >= exact);
    let fine = check(4096).unwrap();
    assert!(fine.within_tolerance);
    assert!(fine.gap_bounds[0] <= exact && fine.gap_bounds[1] >= exact);
    assert_eq!(fine.witness_segment, 1);
    assert_eq!(fine.witness_obstacle, 0);
    assert_eq!(fine.witness_parameter, 3.);
    assert_eq!(fine.witness_time_bounds, [7.75; 2]);
    assert!(fine.work_obstacle_cells <= 4096);
}

#[test]
fn last_toolpath_chain_violation_retains_later_unresolved_tangency() {
    use nurbs_core::analysis::curve_measure::{
        LastToolpathChainViolation, SphericalObstacle, TimedMotionSegment, ToolpathObstacle,
        last_toolpath_chain_violation,
    };
    let line = |y| Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![0., y], vec![4., y]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let crossing = line(0.);
    let tangent = line(1.);
    let mut segments = [
        TimedMotionSegment { curve: &crossing, interval: [2., 6.], duration: 4. },
        TimedMotionSegment { curve: &tangent, interval: [2., 6.], duration: 8. },
    ];
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle { center: &[2., 0.], radius: 1. })];
    let search = |segments: &[TimedMotionSegment<'_>], budget| {
        last_toolpath_chain_violation(segments, &obstacles, 0., 0., 0.01, budget)
    };
    assert!(search(&segments, 1).is_err());
    let LastToolpathChainViolation::Confirmed {
        time_bounds: [lo, hi], latest_segment: 1, witness_segment: 0, obstacle_index: 0,
    } = search(&segments, 512).unwrap()
    else { panic!("expected a confirmed earlier contact"); };
    assert!(2.98 < lo && lo <= 3. && 8. <= hi && hi < 8.03);
    segments[1].interval = [5., 6.];
    let LastToolpathChainViolation::Confirmed {
        time_bounds: [lo, hi], latest_segment: 0, witness_segment: 0, ..
    } = search(&segments, 512).unwrap()
    else { panic!("expected last possible contact on the first segment"); };
    assert!(lo <= 3. && 3. <= hi && hi - lo < 0.03);
}

#[test]
fn toolpath_chain_timeline_keeps_clear_gaps_and_shared_time_boundaries() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, SphericalObstacle, TimedMotionSegment, ToolpathObstacle,
        toolpath_chain_clearance_timeline,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 8., 8.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let segments = [
        TimedMotionSegment { curve: &curve, interval: [0., 1.], duration: 3. },
        TimedMotionSegment { curve: &curve, interval: [1., 8.], duration: 2. },
    ];
    let obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[2., 0.], radius: 0.5 }),
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[6., 0.], radius: 0.5 }),
    ];
    let report = toolpath_chain_clearance_timeline(&segments, &obstacles, 0., 0., 0.01, 512)
        .unwrap();
    assert!(report.work_obstacle_cells <= 512);
    let cells = &report.intervals;
    assert_eq!(cells[0].status, Status::Within);
    assert_eq!(cells[0].time_endpoint_bounds, [[0.; 2], [3.; 2]]);
    assert_eq!(cells.last().unwrap().time_endpoint_bounds[1], [5.; 2]);
    assert_eq!(cells.iter().filter(|cell| cell.status == Status::Exceeds).count(), 2);
    assert!(cells.iter().any(|cell| {
        cell.segment_index == 1 && cell.status == Status::Within
            && cell.parameter_interval[0] < 4. && 4. < cell.parameter_interval[1]
    }));
    for pair in cells.windows(2) {
        assert_eq!(pair[0].time_endpoint_bounds[1], pair[1].time_endpoint_bounds[0]);
        assert!(pair[0].segment_index != pair[1].segment_index || pair[0].status != pair[1].status);
    }
}

#[test]
fn longest_toolpath_violation_brackets_a_run_across_segment_boundaries() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, TimedMotionSegment, ToolpathObstacle, toolpath_chain_clearance_timeline,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 8., 8.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let segments = [
        TimedMotionSegment { curve: &curve, interval: [0., 2.], duration: 4. },
        TimedMotionSegment { curve: &curve, interval: [2., 8.], duration: 6. },
    ];
    let obstacles = [
        ToolpathObstacle::Box(BoxObstacle { minimum: &[1., -1.], maximum: &[3., 1.] }),
        ToolpathObstacle::Box(BoxObstacle { minimum: &[5., -1.], maximum: &[7., 1.] }),
    ];
    let measure = |budget| {
        toolpath_chain_clearance_timeline(&segments, &obstacles, 0., 0., 0.001, budget)
            .unwrap().longest_violation_duration()
    };
    let [lo, hi] = measure(1024);
    assert!(lo <= 3. && 3. <= hi && hi - lo < 0.02, "{lo}..{hi}");
    let [lo, hi] = measure(4);
    assert_eq!(lo, 0.);
    assert!(hi >= 10.);
    let free = [ToolpathObstacle::Box(BoxObstacle { minimum: &[10., -1.], maximum: &[12., 1.] })];
    let report = toolpath_chain_clearance_timeline(&segments, &free, 0., 0., 0.001, 2).unwrap();
    assert_eq!(report.longest_violation_duration(), [0.; 2]);
}

#[test]
fn synchronized_curve_clearance_measures_simultaneous_positions_with_distinct_knots() {
    use nurbs_core::analysis::curve_measure::{StopReason, measure_synchronized_curve_clearance};
    let first = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut second = first.clone();
    second.knots[2] = 0.75;
    second.control_points = vec![vec![4., 1.], vec![1., 1.], vec![0., 1.]];
    let verify = |budget| {
        measure_synchronized_curve_clearance([&first, &second], [0., 1.], [0.2, 0.3], 1e-6, budget)
    };
    assert!(verify(2).is_err());
    let report = verify(64).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert!(report.gap_bounds[0] <= 0.5 && 0.5 <= report.gap_bounds[1]);
    assert_eq!(report.witness_parameter, 0.5);
    assert!(report.work_cells <= 64);
}

#[test]
fn synchronized_clearance_partition_brackets_contact_and_preserves_both_knot_grids() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_synchronized_curve_clearance,
    };
    let first = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut second = first.clone();
    second.knots[2] = 0.75;
    second.control_points = vec![vec![4., 0.], vec![1., 0.], vec![0., 0.]];
    let check = |budget| {
        partition_synchronized_curve_clearance([&first, &second], [0., 1.], [0.5, 0.5], 0.01, budget)
    };
    assert!(check(2).is_err());
    let report = check(128).unwrap();
    assert!(report.work_cells <= 128);
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    for crossing in [0.375, 0.625] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| {
        let [a, b] = cell.parameter_interval;
        a <= 0.5 && 0.5 <= b && cell.spans == [2, 1] && cell.status == Status::Exceeds
    }));
}

#[test]
fn affine_synchronized_clearance_handles_different_clocks_and_internal_knots() {
    use nurbs_core::analysis::curve_measure::{StopReason, measure_affine_synchronized_curve_clearance};
    let first = Curve {
        degree: 1,
        knots: vec![-4., -4., -2., 4., 4.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut second = first.clone();
    second.knots = vec![10., 10., 19., 22., 22.];
    second.control_points = vec![vec![4., 1.], vec![1., 1.], vec![0., 1.]];
    let report = measure_affine_synchronized_curve_clearance(
        [&first, &second], [[-4., 4.], [10., 22.]], [0.25, 0.25], 1e-6, 64,
    ).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert!(report.gap_bounds[0] <= 0.5 && report.gap_bounds[1] >= 0.5);
    assert_eq!(report.witness_parameter, 0.5);
}

#[test]
fn affine_synchronized_partition_brackets_contact_on_different_source_windows() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, partition_affine_synchronized_curve_clearance,
    };
    let mut first = Curve {
        degree: 1,
        knots: vec![-4., -4., 4., 4.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let second = Curve {
        degree: 1,
        knots: vec![10., 10., 19., 22., 22.],
        control_points: vec![vec![4., 0.], vec![1., 0.], vec![0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let check = |first: &Curve, budget| {
        partition_affine_synchronized_curve_clearance(
            [first, &second], [[-4., 4.], [10., 22.]], [0.5, 0.5], 0.01, budget,
        )
    };
    let coarse = check(&first, 1).unwrap();
    assert_eq!(coarse.cells[0].status, Status::Undetermined);
    let report = check(&first, 128).unwrap();
    assert!(report.work_cells <= 128);
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    for boundary in [0.375, 0.625] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.progress_interval;
            a <= boundary && boundary <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| cell.status == Status::Exceeds));
    first.control_points[0].push(0.);
    first.control_points[1].push(0.);
    assert!(check(&first, 128).is_err());
}

#[test]
fn scheduled_curve_clearance_handles_partial_overlap_and_separate_durations() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, TimedMotionSegment, measure_scheduled_curve_clearance,
    };
    let make = |knots, points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots,
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = make(vec![0., 0., 1., 1.], [[0., 0.], [8., 0.]]);
    let second = make(vec![10., 10., 16., 16.], [[6., 1.], [2., 1.]]);
    let segments = [
        TimedMotionSegment { curve: &first, interval: [0., 1.], duration: 8. },
        TimedMotionSegment { curve: &second, interval: [10., 16.], duration: 4. },
    ];
    let check = |starts| measure_scheduled_curve_clearance(segments, starts, [0.25; 2], 1e-5, 128);
    assert!(check([0., 9.]).unwrap().is_none());
    let report = check([0., 2.]).unwrap().unwrap();
    assert_eq!(report.minimum.stop_reason, StopReason::Tolerance);
    assert_eq!(report.witness_time_bounds, [4.; 2]);
    let [lo, hi] = report.minimum.gap_bounds;
    assert!(lo <= 0.5 && 0.5 <= hi && hi - lo <= 1e-5);
    let instant = check([0., 8.]).unwrap().unwrap();
    assert_eq!(instant.witness_time_bounds, [8.; 2]);
}

#[test]
fn scheduled_clearance_partition_maps_contact_to_absolute_time() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, TimedMotionSegment, partition_scheduled_curve_clearance,
    };
    let mut first = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let mut second = first.clone();
    second.knots = vec![10., 10., 16., 16.];
    second.control_points = vec![vec![6., 0.], vec![2., 0.]];
    let check = |first: &Curve, starts| {
        let segments = [
            TimedMotionSegment { curve: first, interval: [0., 1.], duration: 8. },
            TimedMotionSegment { curve: &second, interval: [10., 16.], duration: 4. },
        ];
        partition_scheduled_curve_clearance(segments, starts, [0.5; 2], 0.01, 128)
    };
    assert!(check(&first, [0., 9.]).unwrap().is_none());
    let report = check(&first, [0., 2.]).unwrap().unwrap();
    assert_eq!(report.cells.first().unwrap().time_endpoint_bounds[0], [2.; 2]);
    assert_eq!(report.cells.last().unwrap().time_endpoint_bounds[1], [6.; 2]);
    for crossing in [3.5, 4.5] {
        assert!(report.cells.iter().any(|cell| {
            let [[lo, _], [_, hi]] = cell.time_endpoint_bounds;
            lo <= crossing && crossing <= hi && hi - lo < 0.04 && cell.status == Status::Undetermined
        }));
    }
    assert!(report.cells.iter().any(|cell| cell.status == Status::Exceeds));
    first.control_points[1] = vec![6., 0.];
    let instant = check(&first, [0., 8.]).unwrap().unwrap();
    assert_eq!(instant.work_cells, 1);
    assert_eq!(instant.cells[0].time_endpoint_bounds, [[8.; 2]; 2]);
    assert_eq!(instant.cells[0].status, Status::Exceeds);
}

#[test]
fn scheduled_clearance_with_holds_finds_collision_after_first_tool_stops() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, TimedMotionSegment,
        partition_scheduled_clearance_with_holds, partition_scheduled_curve_clearance,
    };
    let line = |a, b| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0.], vec![b, 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = line(0., 2.);
    let second = line(4., 0.);
    let segments = [
        TimedMotionSegment { curve: &first, interval: [0., 1.], duration: 1. },
        TimedMotionSegment { curve: &second, interval: [0., 1.], duration: 4. },
    ];
    assert!(partition_scheduled_curve_clearance(segments, [0., 2.], [0.25; 2], 0.01, 128)
        .unwrap().is_none());
    assert!(partition_scheduled_clearance_with_holds(segments, [0., 2.], [0.25; 2], 0.01, 2)
        .is_err());
    let report = partition_scheduled_clearance_with_holds(
        segments, [0., 2.], [0.25; 2], 0.01, 128,
    ).unwrap();
    assert_eq!(report.cells.first().unwrap().time_endpoint_bounds[0], [0.; 2]);
    assert_eq!(report.cells.last().unwrap().time_endpoint_bounds[1], [6.; 2]);
    assert!(report.cells.iter().any(|cell| {
        let [[a, _], [_, b]] = cell.time_endpoint_bounds;
        a <= 4. && 4. <= b && cell.status == Status::Exceeds
    }));
    for pair in report.cells.windows(2) {
        assert_eq!(pair[0].time_endpoint_bounds[1], pair[1].time_endpoint_bounds[0]);
    }
}

#[test]
fn minimum_scheduled_clearance_with_holds_finds_witness_during_a_hold() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, TimedMotionSegment, measure_scheduled_clearance_with_holds,
    };
    let line = |points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let first = line([[0., 0.], [2., 0.]]);
    let second = line([[4., 0.25], [0., 0.25]]);
    let segments = [
        TimedMotionSegment { curve: &first, interval: [0., 1.], duration: 1. },
        TimedMotionSegment { curve: &second, interval: [0., 1.], duration: 4. },
    ];
    let check = |budget| measure_scheduled_clearance_with_holds(
        segments, [0., 2.], [0.25; 2], 1e-6, budget,
    );
    assert!(check(2).is_err());
    let coarse = check(3).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= -0.25 && coarse.gap_bounds[1] >= -0.25);
    let fine = check(128).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    assert_eq!(fine.witness_time_bounds, [4.; 2]);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= -0.25 && -0.25 <= hi && hi - lo < 1e-6);
}

#[test]
fn scheduled_tools_clearance_reports_closest_pair_under_one_budget() {
    use nurbs_core::analysis::curve_measure::{
        ScheduledTool, StopReason, TimedMotionSegment, measure_scheduled_tools_clearance,
    };
    let line = |points: [[f64; 2]; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: points.map(Vec::from).into(),
        weights: vec![1.; 2],
        periodic: false,
    };
    let distant = line([[0., 10.], [2., 10.]]);
    let first = line([[0., 0.], [2., 0.]]);
    let second = line([[4., 0.25], [0., 0.25]]);
    let tool = |curve, start_time, duration| ScheduledTool {
        segment: TimedMotionSegment { curve, interval: [0., 1.], duration },
        start_time,
        radius: 0.25,
    };
    let tools = [tool(&distant, 0., 1.), tool(&first, 0., 1.), tool(&second, 2., 4.)];
    assert!(measure_scheduled_tools_clearance(&tools, 1e-6, 6).is_err());
    let report = measure_scheduled_tools_clearance(&tools, 1e-6, 128).unwrap();
    assert_eq!(report.witness_tools, [1, 2]);
    assert_eq!(report.minimum.witness_time_bounds, [4.; 2]);
    assert_eq!(report.minimum.stop_reason, StopReason::Tolerance);
    let [lo, hi] = report.minimum.gap_bounds;
    assert!(lo <= -0.25 && -0.25 <= hi && hi - lo < 1e-6);
    assert!(report.minimum.work_cells <= 128);
}

#[test]
fn scheduled_tools_partition_retains_pair_identity_and_contact_windows() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledTool, TimedMotionSegment,
        partition_scheduled_tools_clearance,
    };
    let line = |a, b, height| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, height], vec![b, height]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let far = line(0., 2., 10.);
    let first = line(0., 2., 0.);
    let second = line(4., 0., 0.);
    let tool = |curve, start_time, duration| ScheduledTool {
        segment: TimedMotionSegment { curve, interval: [0., 1.], duration },
        start_time,
        radius: 0.25,
    };
    let tools = [tool(&far, 0., 1.), tool(&first, 0., 1.), tool(&second, 2., 4.)];
    assert!(partition_scheduled_tools_clearance(&tools, 0.01, 6).is_err());
    let result = partition_scheduled_tools_clearance(&tools, 0.01, 128).unwrap();
    assert!(result.work_cells <= 128);
    assert!(result.cells.iter().filter(|cell| cell.tools.contains(&0))
        .all(|cell| cell.clearance.status == Status::Within));
    assert!(result.cells.iter().any(|cell| {
        let [[a, _], [_, b]] = cell.clearance.time_endpoint_bounds;
        cell.tools == [1, 2] && a <= 4. && 4. <= b && cell.clearance.status == Status::Exceeds
    }));
    for pair in result.cells.windows(2) {
        if pair[0].tools == pair[1].tools {
            assert_eq!(pair[0].clearance.time_endpoint_bounds[1], pair[1].clearance.time_endpoint_bounds[0]);
        }
    }
}

#[test]
fn first_scheduled_tool_contact_compares_times_across_pair_groups() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, FirstScheduledToolContact, ScheduledClearanceCell,
        ScheduledToolPairCell, ScheduledToolsClearancePartition,
    };
    let cell = |tools, start, end, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: [start, end],
            gap_bounds: match status {
                Status::Within => [1., 2.],
                Status::Exceeds => [-2., -1.],
                Status::Undetermined => [-1., 1.],
            },
            status,
        },
    };
    let mut report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [3.; 2], [4.; 2], Status::Exceeds),
            cell([0, 2], [0.9, 1.], [1.1; 2], Status::Undetermined),
            cell([1, 2], [1.9, 2.], [2.5; 2], Status::Exceeds),
        ],
        work_cells: 3,
    };
    assert_eq!(report.first_contact(), FirstScheduledToolContact::Confirmed {
        time_bounds: [0.9, 2.],
        witness_tools: [1, 2],
    });
    report.cells.retain(|cell| cell.clearance.status != Status::Exceeds);
    assert_eq!(report.first_contact(), FirstScheduledToolContact::Possible { earliest_time: 0.9 });
    report.cells[0].clearance.status = Status::Within;
    assert_eq!(report.first_contact(), FirstScheduledToolContact::Clear);
}

#[test]
fn last_scheduled_tool_contact_compares_times_across_pair_groups() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, LastScheduledToolContact, ScheduledClearanceCell,
        ScheduledToolPairCell, ScheduledToolsClearancePartition,
    };
    let cell = |tools, start, end, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: [start, end],
            gap_bounds: match status {
                Status::Within => [1., 2.],
                Status::Exceeds => [-2., -1.],
                Status::Undetermined => [-1., 1.],
            },
            status,
        },
    };
    let mut report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [3.; 2], [4.; 2], Status::Exceeds),
            cell([0, 2], [0.9, 1.], [5.9, 6.], Status::Undetermined),
            cell([1, 2], [1.9, 2.], [2.5; 2], Status::Exceeds),
        ],
        work_cells: 3,
    };
    assert_eq!(report.last_contact(), LastScheduledToolContact::Confirmed {
        time_bounds: [4., 6.],
        witness_tools: [0, 1],
    });
    report.cells.retain(|cell| cell.clearance.status != Status::Exceeds);
    assert_eq!(report.last_contact(), LastScheduledToolContact::Possible { latest_time: 6. });
    report.cells[0].clearance.status = Status::Within;
    assert_eq!(report.last_contact(), LastScheduledToolContact::Clear);
}

#[test]
fn scheduled_contact_duration_counts_overlapping_pairs_once() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledToolPairCell,
        ScheduledToolsClearancePartition,
    };
    let cell = |tools, start, end, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: [start, end],
            gap_bounds: match status {
                Status::Within => [1., 2.],
                Status::Exceeds => [-2., -1.],
                Status::Undetermined => [-1., 1.],
            },
            status,
        },
    };
    let mut report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [3., 3.25], [5., 5.25], Status::Exceeds),
            cell([0, 2], [1., 1.25], [4., 4.25], Status::Exceeds),
            cell([1, 2], [7., 7.25], [8., 8.25], Status::Undetermined),
        ],
        work_cells: 3,
    };
    assert_eq!(report.contact_duration_bounds().unwrap(), [3.75, 5.5]);
    report.cells[2].clearance.time_endpoint_bounds[1] = [f64::MAX, f64::INFINITY];
    assert_eq!(report.contact_duration_bounds().unwrap(), [3.75, f64::INFINITY]);
    report.cells.clear();
    assert_eq!(report.contact_duration_bounds().unwrap(), [0.; 2]);
}

#[test]
fn longest_scheduled_contact_duration_handles_pair_handoffs_and_uncertain_bridges() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledToolPairCell,
        ScheduledToolsClearancePartition,
    };
    let cell = |tools, start, end, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: [start, end],
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let mut report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [0., 0.25], [2., 2.25], Status::Exceeds),
            cell([0, 2], [1., 1.25], [3., 3.25], Status::Exceeds),
            cell([1, 2], [5.; 2], [7.; 2], Status::Exceeds),
            cell([0, 1], [3.25; 2], [5.; 2], Status::Undetermined),
        ],
        work_cells: 4,
    };
    assert_eq!(report.longest_contact_duration_bounds().unwrap(), [2.75, 7.]);
    report.cells.pop();
    assert_eq!(report.longest_contact_duration_bounds().unwrap(), [2.75, 3.25]);
    assert_eq!(report.contact_duration_bounds().unwrap(), [4.75, 5.25]);
}

#[test]
fn peak_scheduled_contact_pairs_counts_distinct_pairs_at_shared_endpoints() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledToolPairCell,
        ScheduledToolsClearancePartition,
    };
    let cell = |tools, start, end, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: [start, end],
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [-1.; 2], [-0.; 2], Status::Exceeds),
            cell([0, 1], [0.; 2], [1.; 2], Status::Exceeds),
            cell([0, 2], [0.; 2], [0.; 2], Status::Exceeds),
            cell([1, 2], [-0.25, 0.25], [0.5; 2], Status::Undetermined),
        ],
        work_cells: 4,
    };
    assert_eq!(report.peak_contact_pair_count_bounds(), [2, 3]);
}

#[test]
fn simultaneous_contact_duration_measures_distinct_pairs_and_excludes_isolated_instants() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledToolPairCell,
        ScheduledToolsClearancePartition,
    };
    let cell = |tools, time, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: time,
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [[0.; 2], [3.; 2]], Status::Exceeds),
            cell([0, 1], [[2.; 2], [4.; 2]], Status::Exceeds),
            cell([0, 2], [[2., 2.25], [5., 5.25]], Status::Exceeds),
            cell([1, 2], [[3.; 2]; 2], Status::Exceeds),
            cell([1, 2], [[4.; 2], [5.; 2]], Status::Undetermined),
        ],
        work_cells: 5,
    };
    assert_eq!(report.peak_contact_pair_count_bounds(), [3, 3]);
    assert_eq!(report.simultaneous_contact_duration_bounds(2).unwrap(), [1.75, 3.]);
    assert_eq!(report.simultaneous_contact_duration_bounds(3).unwrap(), [0.; 2]);
    assert!(report.simultaneous_contact_duration_bounds(0).is_err());
}

#[test]
fn simultaneous_contact_windows_include_isolated_instants_and_uncertain_extensions() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledToolPairCell,
        ScheduledToolsClearancePartition,
    };
    let cell = |tools, time, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: time,
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([0, 1], [[0.; 2], [3.; 2]], Status::Exceeds),
            cell([0, 1], [[2.; 2], [4.; 2]], Status::Exceeds),
            cell([0, 2], [[2., 2.25], [5., 5.25]], Status::Exceeds),
            cell([1, 2], [[3.; 2]; 2], Status::Exceeds),
            cell([1, 2], [[4.; 2], [5.; 2]], Status::Undetermined),
        ],
        work_cells: 5,
    };
    let two = report.simultaneous_contact_windows(2).unwrap();
    assert_eq!(two.confirmed, [[2.25, 4.]]);
    assert_eq!(two.possible, [[2., 5.]]);
    let three = report.simultaneous_contact_windows(3).unwrap();
    assert_eq!(three.confirmed, [[3.; 2]]);
    assert_eq!(three.possible, [[3.; 2], [4.; 2]]);
    assert!(report.simultaneous_contact_windows(0).is_err());
}

#[test]
fn scheduled_peak_contact_report_returns_sorted_confirmed_pairs_and_time() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, PeakScheduledContactWitness, ScheduledClearanceCell,
        ScheduledToolPairCell, ScheduledToolsClearancePartition,
    };
    let cell = |tools, time, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: time,
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let mut report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([1, 2], [[2., 2.25], [4.; 2]], Status::Exceeds),
            cell([0, 1], [[-1.; 2], [3.; 2]], Status::Exceeds),
            cell([0, 1], [[0.; 2], [3.; 2]], Status::Exceeds),
            cell([0, 2], [[2.5; 2], [3.; 2]], Status::Undetermined),
        ],
        work_cells: 4,
    };
    let peak = report.peak_contact_report();
    assert_eq!(peak.pair_count_bounds, [2, 3]);
    assert_eq!(peak.witness, Some(PeakScheduledContactWitness {
        time: 2.25,
        tools: vec![[0, 1], [1, 2]],
    }));
    report.cells.retain(|cell| cell.clearance.status == Status::Undetermined);
    let uncertain = report.peak_contact_report();
    assert_eq!(uncertain.pair_count_bounds, [0, 1]);
    assert_eq!(uncertain.witness, None);
}

#[test]
fn scheduled_contact_query_accounts_for_rounding_and_shared_endpoints() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, ScheduledClearanceCell, ScheduledContactPairs,
        ScheduledToolPairCell, ScheduledToolsClearancePartition,
    };
    let cell = |tools, time, status| ScheduledToolPairCell {
        tools,
        clearance: ScheduledClearanceCell {
            time_endpoint_bounds: time,
            gap_bounds: if status == Status::Exceeds { [-2., -1.] } else { [-1., 1.] },
            status,
        },
    };
    let report = ScheduledToolsClearancePartition {
        cells: vec![
            cell([1, 2], [[2., 2.25], [4.; 2]], Status::Exceeds),
            cell([0, 1], [[-1.; 2], [2.; 2]], Status::Exceeds),
            cell([0, 1], [[2.; 2], [3.; 2]], Status::Exceeds),
            cell([0, 2], [[2.; 2]; 2], Status::Undetermined),
        ],
        work_cells: 4,
    };
    assert_eq!(report.contact_pairs_at_time(2.).unwrap(), ScheduledContactPairs {
        confirmed: vec![[0, 1]],
        possible: vec![[0, 1], [0, 2], [1, 2]],
    });
    assert_eq!(report.contact_pairs_at_time(2.25).unwrap(), ScheduledContactPairs {
        confirmed: vec![[0, 1], [1, 2]],
        possible: vec![[0, 1], [1, 2]],
    });
    assert!(report.contact_pairs_at_time(f64::NAN).is_err());
}

#[test]
fn variable_radius_clearance_brackets_growth_and_shrinkage_contacts() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, SphericalObstacle, partition_variable_radius_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![vec![2., 0.], vec![3., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [SphericalObstacle { center: &[0., 0.], radius: 0.5 }];
    let check = |radii| {
        partition_variable_radius_spherical_clearance(&curve, [2., 6.], &obstacles, radii, 0.01, 128)
    };
    for (radii, first, last) in [
        ([0., 4.], Status::Within, Status::Exceeds),
        ([4., 0.], Status::Exceeds, Status::Within),
    ] {
        let report = check(radii).unwrap();
        assert_eq!(report.cells.first().unwrap().status, first);
        assert_eq!(report.cells.last().unwrap().status, last);
        assert!(report.cells.iter().any(|cell| {
            let [lo, hi] = cell.parameter_interval;
            lo <= 4. && 4. <= hi && hi - lo <= 0.01 && cell.status == Status::Undetermined
        }));
        assert!(report.work_obstacle_cells <= 128);
    }
    assert!(check([-1., 0.]).is_err());
}

#[test]
fn minimum_variable_radius_clearance_couples_the_witness_position_and_radius() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, StopReason, measure_minimum_variable_radius_spherical_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 1.], vec![2., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [SphericalObstacle { center: &[0., 0.], radius: 0.25 }];
    let check = |budget| measure_minimum_variable_radius_spherical_clearance(
        &curve, [0., 1.], &obstacles, [0.25, 2.25], 1e-4, budget,
    ).unwrap();
    let exact = 3_f64.sqrt() / 2. - 1.5;
    let coarse = check(1);
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(4096);
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo < 1e-4);
    assert!((fine.witness_parameter - (0.5 + 1. / (4. * 3_f64.sqrt()))).abs() < 0.01);
}

#[test]
fn variable_radius_mixed_clearance_tracks_sphere_and_box_contacts() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, DirectionalMotionStatus as Status, SphericalObstacle, ToolpathObstacle,
        partition_variable_radius_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 8., 8.],
        control_points: vec![vec![0., 0.], vec![8., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Box(BoxObstacle { minimum: &[5., -1.], maximum: &[6., 1.] }),
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[2., 0.], radius: 0.25 }),
    ];
    let check = |budget| partition_variable_radius_toolpath_clearance(
        &curve, [0., 8.], &obstacles, [0., 1.], 0.01, budget,
    );
    assert!(check(1).is_err());
    let report = check(512).unwrap();
    assert!(report.work_obstacle_cells <= 512);
    for (parameter, obstacle) in [(2., 1), (5.5, 0)] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= parameter && parameter <= b && cell.status == Status::Exceeds
                && cell.gap_bounds[obstacle][1] < 0.
                && cell.gap_bounds[1 - obstacle][0] > 0.
        }));
    }
    for crossing in [14. / 9., 40. / 9.] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn minimum_variable_radius_mixed_clearance_finds_a_box_corner_witness() {
    use nurbs_core::analysis::curve_measure::{
        BoxObstacle, SphericalObstacle, StopReason, ToolpathObstacle,
        measure_minimum_variable_radius_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![-2., 1.], vec![2., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let obstacles = [
        ToolpathObstacle::Sphere(SphericalObstacle { center: &[0., 8.], radius: 0.25 }),
        ToolpathObstacle::Box(BoxObstacle { minimum: &[-3., -2.], maximum: &[0., 0.] }),
    ];
    let check = |budget| measure_minimum_variable_radius_toolpath_clearance(
        &curve, [0., 1.], &obstacles, [0.25, 2.25], 1e-4, budget,
    );
    assert!(check(1).is_err());
    let exact = 3_f64.sqrt() / 2. - 1.25;
    let coarse = check(2).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= exact && exact <= coarse.gap_bounds[1]);
    let fine = check(8192).unwrap();
    assert_eq!(fine.stop_reason, StopReason::Tolerance);
    let [lo, hi] = fine.gap_bounds;
    assert!(lo <= exact && exact <= hi && hi - lo < 1e-4);
    assert_eq!(fine.witness_obstacle, 1);
    assert!((fine.witness_parameter - (0.5 + 1. / (4. * 3_f64.sqrt()))).abs() < 0.01);
    assert!(fine.work_obstacle_cells <= 8192);
}

#[test]
fn profiled_radius_clearance_preserves_radius_breaks_and_curve_knots() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, SphericalObstacle, ToolpathObstacle,
        partition_profiled_radius_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 2.], vec![1., 2.], vec![4., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle {
        center: &[2., 0.], radius: 0.5,
    })];
    let check = |profile: &[(f64, f64)], budget| {
        partition_profiled_radius_toolpath_clearance(&curve, profile, &obstacles, 0.01, budget)
    };
    let profile = [(0., 0.), (0.5, 3.), (1., 0.)];
    assert!(check(&profile, 2).is_err());
    assert!(check(&[(0., 0.), (0., 1.), (1., 0.)], 128).is_err());
    let report = check(&profile, 128).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    for boundary in [0.25, 0.5] {
        assert!(report.cells.iter().any(|cell| cell.parameter_interval[1] == boundary));
        assert!(report.cells.iter().any(|cell| cell.parameter_interval[0] == boundary));
    }
    assert!(report.cells.iter().any(|cell| {
        cell.parameter_interval[0] == 0.5 && cell.status == Status::Exceeds
    }));
    assert!(report.work_obstacle_cells <= 128);
}

#[test]
fn minimum_profiled_radius_clearance_samples_a_radius_break_directly() {
    use nurbs_core::analysis::curve_measure::{
        SphericalObstacle, StopReason, ToolpathObstacle,
        measure_minimum_profiled_radius_toolpath_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 2.], vec![1., 2.], vec![4., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let obstacles = [ToolpathObstacle::Sphere(SphericalObstacle {
        center: &[1.5, 0.], radius: 0.5,
    })];
    let profile = [(0., 0.), (0.375, 3.), (1., 0.)];
    let check = |budget| measure_minimum_profiled_radius_toolpath_clearance(
        &curve, &profile, &obstacles, 1e-8, budget,
    );
    assert!(check(2).is_err());
    let report = check(3).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert_eq!(report.witness_parameter, 0.375);
    assert_eq!(report.work_obstacle_cells, 3);
    let [lo, hi] = report.gap_bounds;
    assert!(lo <= -1.5 && -1.5 <= hi && hi - lo <= 1e-8);
}

#[test]
fn profiled_radius_workspace_detects_overtravel_between_clear_endpoints() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_profiled_radius_convex_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let profile = [(0., 0.), (0.5, 3.), (1., 0.)];
    let planes = [WorkspaceHalfSpace { normal: &[2., 0.], offset: 9. }];
    let check = |budget| partition_profiled_radius_convex_workspace(
        &curve, &profile, &planes, 0.01, budget,
    );
    assert!(check(2).is_err());
    let report = check(128).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        let [lo, hi] = cell.plane_residuals[0];
        cell.parameter_interval[0] == 0.5 && cell.status == Status::Exceeds && lo <= 1. && hi >= 1.
    }));
    for crossing in [0.45, 0.75] {
        assert!(report.cells.iter().any(|cell| {
            let [a, b] = cell.parameter_interval;
            a <= crossing && crossing <= b && b - a <= 0.01 && cell.status == Status::Undetermined
        }));
    }
}

#[test]
fn minimum_profiled_workspace_clearance_normalizes_the_limiting_plane() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, WorkspaceHalfSpace, measure_minimum_profiled_radius_workspace_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![4., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let profile = [(0., 0.), (0.5, 3.), (1., 0.)];
    let planes = [
        WorkspaceHalfSpace { normal: &[0., -1.], offset: 4. },
        WorkspaceHalfSpace { normal: &[10., 0.], offset: 45. },
    ];
    let check = |budget| measure_minimum_profiled_radius_workspace_clearance(
        &curve, &profile, &planes, 1e-5, budget,
    );
    assert!(check(5).is_err());
    let coarse = check(6).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= -0.5 && coarse.gap_bounds[1] >= -0.5);
    let report = check(4096).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert_eq!(report.witness_parameter, 0.5);
    assert_eq!(report.witness_plane, 1);
    let [lo, hi] = report.gap_bounds;
    assert!(lo <= -0.5 && -0.5 <= hi && hi - lo <= 1e-5);
}

#[test]
fn profiled_rounded_hull_workspace_combines_vertex_support_and_radius() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_profiled_radius_convex_workspace,
        partition_profiled_rounded_hull_workspace,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let profile = [(0., 0.), (0.5, 3.), (1., 0.)];
    let planes = [WorkspaceHalfSpace { normal: &[2., 0.], offset: 11. }];
    let offsets = [&[0., 0.][..], &[1., 2.][..], &[-1., 0.][..]];
    let ball = partition_profiled_radius_convex_workspace(&curve, &profile, &planes, 0.01, 128).unwrap();
    assert!(ball.cells.iter().all(|cell| cell.status == Status::Within));
    let report = partition_profiled_rounded_hull_workspace(
        &curve, &profile, &planes, &offsets, 0.01, 128,
    ).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        let [lo, hi] = cell.plane_residuals[0];
        cell.parameter_interval[0] == 0.5 && cell.status == Status::Exceeds && lo <= 1. && hi >= 1.
    }));
    assert!(partition_profiled_rounded_hull_workspace(
        &curve, &profile, &planes, &[], 0.01, 128,
    ).is_err());
}

#[test]
fn minimum_profiled_hull_workspace_clearance_reports_limiting_plane_and_parameter() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, WorkspaceHalfSpace, measure_minimum_profiled_rounded_hull_workspace_clearance,
    };
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let profile = [(0., 0.), (0.5, 3.), (1., 0.)];
    let planes = [
        WorkspaceHalfSpace { normal: &[0., 3.], offset: 30. },
        WorkspaceHalfSpace { normal: &[2., 0.], offset: 11. },
    ];
    let offsets = [&[0., 0.][..], &[1., 2.][..], &[-1., 0.][..]];
    let check = |budget| measure_minimum_profiled_rounded_hull_workspace_clearance(
        &curve, &profile, &planes, &offsets, 1e-5, budget,
    );
    assert!(check(3).is_err());
    let coarse = check(4).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= -0.5 && coarse.gap_bounds[1] >= -0.5);
    let report = check(4096).unwrap();
    let [lo, hi] = report.gap_bounds;
    assert!(lo <= -0.5 && -0.5 <= hi && hi - lo < 1e-5);
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert_eq!(report.witness_plane, 1);
    assert_eq!(report.witness_parameter, 0.5);
}

#[test]
fn rotating_hull_workspace_tracks_rotation_and_orientation_knots() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_planar_rotating_hull_workspace,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., 2., 1.],
        ..position.clone()
    };
    let planes = [WorkspaceHalfSpace { normal: &[1., 0.], offset: 1.5 }];
    let hull = [&[2., 0.][..], &[0., 0.][..]];
    let report = partition_planar_rotating_hull_workspace(
        &position, &direction, [0., 1.], &planes, &hull, 0.005, 255,
    ).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Exceeds);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    for cell in report.cells {
        assert!(!(cell.parameter_interval[0] < 0.5 && cell.parameter_interval[1] > 0.5));
        let [lo, hi] = cell.plane_residuals[0];
        for t in cell.parameter_interval {
            let (x, y): (f64, f64) = if t <= 0.5 {
                (1. + 2. * t, 4. * t)
            } else {
                (4. * (1. - t), 3. - 2. * t)
            };
            let residual = 2. * x / x.hypot(y) - 1.5;
            assert!(lo <= residual && residual <= hi);
        }
    }
}

#[test]
fn rotating_hull_workspace_retains_undefined_orientation() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_planar_rotating_hull_workspace,
    };
    let direction = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1., 0.], vec![-1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let planes = [WorkspaceHalfSpace { normal: &[1., 0.], offset: 10. }];
    let report = partition_planar_rotating_hull_workspace(
        &direction, &direction, [0., 1.], &planes, &[&[2., 0.]], 0.01, 100,
    ).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().filter(|cell| {
        cell.parameter_interval[0] <= 0.5 && cell.parameter_interval[1] >= 0.5
    }).all(|cell| cell.status == Status::Undetermined));
}

#[test]
fn minimum_rotating_hull_clearance_refines_an_interior_rotation_witness() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, WorkspaceHalfSpace, measure_minimum_planar_rotating_hull_workspace_clearance,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., -1.], vec![1., 1.]],
        ..position.clone()
    };
    let planes = [
        WorkspaceHalfSpace { normal: &[0., 3.], offset: 30. },
        WorkspaceHalfSpace { normal: &[2., 0.], offset: 3. },
    ];
    let hull = [&[0., 0.][..], &[2., 0.][..]];
    let measure = |orientation, budget| measure_minimum_planar_rotating_hull_workspace_clearance(
        &position, orientation, [0., 1.], &planes, &hull, 1e-6, budget,
    );
    let coarse = measure(&direction, 4).unwrap();
    assert_eq!(coarse.stop_reason, StopReason::WorkLimit);
    assert!(coarse.gap_bounds[0] <= -0.5 && coarse.gap_bounds[1] >= -0.5);
    let refined = measure(&direction, 256).unwrap();
    assert_eq!(refined.stop_reason, StopReason::Tolerance);
    assert!(refined.gap_bounds[0] <= -0.5 && refined.gap_bounds[1] >= -0.5);
    assert!(refined.gap_bounds[1] - refined.gap_bounds[0] <= 1e-6);
    assert_eq!(refined.witness_plane, 1);
    assert_eq!(refined.witness_parameter, 0.5);
    assert!(measure(&direction, 3).is_err());

    let singular = Curve {
        control_points: vec![vec![1., 0.], vec![-1., 0.]],
        ..direction.clone()
    };
    assert!(measure(&singular, 256).is_err());
}

#[test]
fn rotating_hull_obstacle_clearance_tracks_vertex_contact_and_separation() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace, partition_planar_rotating_hull_obstacle_clearance,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., 0.], vec![0., 1.]],
        weights: vec![1., 2.],
        ..position.clone()
    };
    let right = [WorkspaceHalfSpace { normal: &[-1., 0.], offset: -1.6 }];
    let above = [WorkspaceHalfSpace { normal: &[0., -2.], offset: -3.2 }];
    let obstacles = [&right[..], &above[..]];
    let hull = [&[0., 0.][..], &[2., 0.][..]];
    let partition = |orientation, budget| partition_planar_rotating_hull_obstacle_clearance(
        &position, orientation, [0., 1.], &obstacles, &hull, 0.005, budget,
    );
    assert!(partition(&direction, 3).is_err());
    let report = partition(&direction, 512).unwrap();
    assert!(report.work_plane_cells <= 512);
    assert_eq!(report.cells.first().unwrap().obstacles, [Status::Exceeds, Status::Within]);
    assert_eq!(report.cells.last().unwrap().obstacles, [Status::Within, Status::Exceeds]);
    assert!(report.cells.iter().any(|cell| cell.status == Status::Within));
    let singular = Curve {
        control_points: vec![vec![1., 0.], vec![-1., 0.]],
        weights: vec![1.; 2],
        ..position.clone()
    };
    let report = partition(&singular, 128).unwrap();
    for cell in report.cells.iter().filter(|cell| {
        cell.parameter_interval[0] <= 0.5 && cell.parameter_interval[1] >= 0.5
    }) {
        assert_eq!(cell.obstacles, [Status::Undetermined; 2]);
    }
}

#[test]
fn rotating_rounded_hull_reserves_the_radius_against_nonunit_obstacle_planes() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_planar_rotating_rounded_hull_obstacle_clearance,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., 0.], vec![0., 1.]],
        ..position.clone()
    };
    let right = [WorkspaceHalfSpace { normal: &[-4., 0.], offset: -9. }];
    let partition = |radius| partition_planar_rotating_rounded_hull_obstacle_clearance(
        &position, &direction, [0., 1.], &[&right], &[&[2., 0.]], radius, 0.01, 255,
    );
    let sharp = partition(0.).unwrap();
    assert!(sharp.cells.iter().all(|cell| cell.status == Status::Within));
    let rounded = partition(0.3).unwrap();
    assert_eq!(rounded.cells.first().unwrap().status, Status::Undetermined);
    assert_eq!(rounded.cells.last().unwrap().status, Status::Within);
    assert!(rounded.cells.iter().all(|cell| cell.status != Status::Exceeds));
    assert!(partition(-0.1).is_err());
    assert!(partition(f64::NAN).is_err());
}

#[test]
fn rotating_profiled_hull_obstacle_clearance_splits_at_radius_peaks() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_planar_rotating_profiled_hull_obstacle_clearance,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        knots: vec![0., 0., 0.75, 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1.; 3],
        ..position.clone()
    };
    let obstacle = [WorkspaceHalfSpace { normal: &[-2., 0.], offset: -4.5 }];
    let partition = |profile, budget| partition_planar_rotating_profiled_hull_obstacle_clearance(
        &position, &direction, profile, &[&obstacle], &[&[2., 0.]], 0.005, budget,
    );
    let profile = [(0., 0.), (0.5, 1.), (1., 0.)];
    assert!(partition(&profile, 2).is_err());
    let report = partition(&profile, 255).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        cell.parameter_interval[0] == 0.5 && cell.status == Status::Undetermined
    }));
    assert!(report.cells.iter().all(|cell| {
        let [a, b] = cell.parameter_interval;
        [0.5, 0.75].into_iter().all(|knot| !(a < knot && knot < b))
    }));
    assert!(partition(&[(0., 0.), (0., 1.)], 255).is_err());
}

#[test]
fn rotating_profiled_hull_workspace_certifies_overtravel_at_a_radius_peak() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_planar_rotating_profiled_hull_workspace,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., -1.], vec![1., 1.]],
        ..position.clone()
    };
    let planes = [WorkspaceHalfSpace { normal: &[2., 0.], offset: 4.5 }];
    let profile = [(0., 0.), (0.5, 1.), (1., 0.)];
    let partition = |budget| partition_planar_rotating_profiled_hull_workspace(
        &position, &direction, &profile, &planes, &[&[2., 0.]], 0.005, budget,
    );
    assert!(partition(1).is_err());
    let report = partition(255).unwrap();
    assert_eq!(report.cells.first().unwrap().status, Status::Within);
    assert_eq!(report.cells.last().unwrap().status, Status::Within);
    assert!(report.cells.iter().any(|cell| {
        let [lo, hi] = cell.plane_residuals[0];
        cell.parameter_interval[0] == 0.5 && cell.status == Status::Exceeds && lo <= 1.5 && hi >= 1.5
    }));
}

#[test]
fn minimum_rotating_profiled_hull_workspace_clearance_witnesses_a_radius_break() {
    use nurbs_core::analysis::curve_measure::{
        StopReason, WorkspaceHalfSpace, measure_minimum_planar_rotating_profiled_hull_workspace_clearance,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 1.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., -1.], vec![1., 1.]],
        ..position.clone()
    };
    let planes = [
        WorkspaceHalfSpace { normal: &[0., 3.], offset: 30. },
        WorkspaceHalfSpace { normal: &[2., 0.], offset: 4.5 },
    ];
    let profile = [(0.25, 0.), (0.5, 1.), (0.75, 0.)];
    let measure = |budget| measure_minimum_planar_rotating_profiled_hull_workspace_clearance(
        &position, &direction, &profile, &planes, &[&[2., 0.]], 1e-6, budget,
    );
    assert!(measure(7).is_err());
    let report = measure(8).unwrap();
    assert_eq!(report.stop_reason, StopReason::Tolerance);
    assert!(report.gap_bounds[0] <= -0.75 && report.gap_bounds[1] >= -0.75);
    assert!(report.gap_bounds[1] - report.gap_bounds[0] <= 1e-6);
    assert_eq!(report.witness_plane, 1);
    assert_eq!(report.witness_parameter, 0.5);
}

#[test]
fn rotating_rounded_contact_checks_displaced_vertex_against_every_plane() {
    use nurbs_core::analysis::curve_measure::{
        DirectionalMotionStatus as Status, WorkspaceHalfSpace,
        partition_planar_rotating_profiled_hull_obstacle_contact,
    };
    let position = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let direction = Curve {
        control_points: vec![vec![1., 0.], vec![1., 0.1]],
        ..position.clone()
    };
    let face = [WorkspaceHalfSpace { normal: &[-4., 0.], offset: -9. }];
    let corner = [
        face[0],
        WorkspaceHalfSpace { normal: &[0., -1.], offset: -0.6 },
    ];
    let partition = |budget| partition_planar_rotating_profiled_hull_obstacle_contact(
        &position, &direction, &[(0., 0.4), (1., 0.4)],
        &[&face, &corner], &[&[2., 0.]], 0.005, budget,
    );
    assert!(partition(5).is_err());
    let report = partition(384).unwrap();
    assert!(report.work_plane_cells <= 384);
    assert!(report.cells.iter().all(|cell| cell.obstacles[0] == Status::Exceeds));
    assert!(report.cells.iter().all(|cell| cell.obstacles[1] != Status::Exceeds));
}
