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
