use super::*;
#[test]
fn literal_unit_product_requires_exact_original_structure() {
    let mut s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..5)
            .map(|i| vec![vec![i as f64, 0., 0.], vec![i as f64, 0., 1.]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    assert_eq!(
        polynomial::Prepared::new(&s, 0.5, [1.; 2])
            .unwrap()
            .unit_product_axis,
        Some(2)
    );
    s.control_points[1][1][1] = f64::from_bits(1);
    assert_eq!(
        polynomial::Prepared::new(&s, 0.5, [1.; 2])
            .unwrap()
            .unit_product_axis,
        None
    );
    s.control_points[1][1][1] = 0.;
    s.weights[1][1] = 1f64.next_up();
    assert_eq!(
        polynomial::Prepared::new(&s, 0.5, [1.; 2])
            .unwrap()
            .unit_product_axis,
        None
    );
}
fn graph(kink: f64) -> Surface {
    let x = [0., 0.25, 0.5, 0.75, 1.];
    let z = [0., 0., 0.25, 0.5, 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            x[i],
                            j as f64 / 2.,
                            z[i] + if j == 2 { 1. } else { 0. }
                                + if i > 2 { kink * (x[i] - 0.5) } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn correlated_original_jets_prove_a_complete_curved_c0_storage_join() {
    for scale in [1e-12, 1., 1e12] {
        let mut s = graph(0.);
        for row in &mut s.weights {
            row.fill(scale);
        }
        let before = s.clone();
        let r = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 128).unwrap();
        assert_eq!(r.decision, Decision::WithinTolerance);
        assert!(r.regularity_proven && r.pending == 0 && r.cells <= 128);
        assert!(r.normal[1] <= 1e-5 && r.tensor[1] <= 1e-4);
        assert_eq!(s, before);
    }
}
#[test]
fn actual_one_sided_kink_is_not_erased_by_common_position_or_covariance() {
    let s = graph(0.25);
    let r = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 128).unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(r.normal[0] > 1e-5 || r.tensor[0] > 1e-4);
}
#[test]
fn no_work_or_no_regular_parameterization_cannot_prove_a_join() {
    let mut s = graph(0.);
    let empty = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 0).unwrap();
    assert_eq!(empty.decision, Decision::Unresolved);
    assert!(!empty.regularity_proven);
    let single = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 1).unwrap();
    assert_eq!(single.decision, Decision::Unresolved);
    assert_eq!(single.cells, 0);
    for row in &mut s.control_points {
        for p in row {
            p[1] = 0.;
            p[2] = 0.;
        }
    }
    let r = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 32).unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
    assert!(inspect_original_span_join(&s, 0.25, 1e-5, 1e-4, 32).is_err());
}

#[test]
fn rational_shared_weights_and_translated_originals_preserve_the_complete_join() {
    let mut s = graph(0.);
    for (row, weights) in s.control_points.iter_mut().zip(&mut s.weights) {
        *weights = vec![1., 2., 1.];
        for p in row {
            p[0] += 1234.;
            p[1] -= 50.;
            p[2] += 20.;
        }
    }
    let before = s.clone();
    let r = inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 128).unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert!(r.regularity_proven && r.pending == 0 && r.cells <= 128);
    assert_eq!(s, before);
}

#[test]
fn positive_clock_conditioning_preserves_g2_after_piecewise_reclocking() {
    for gauge in [1e-12, 1., 1e12] {
        let mut s = graph(0.);
        s.knots_u = vec![0., 0., 0., 1., 1., 4., 4., 4.];
        for row in &mut s.weights {
            row.fill(gauge);
        }
        let before = s.clone();
        let raw = crate::analysis::surface_continuity::qualify(&s, 2, 100000, 1000000).unwrap();
        assert_eq!(raw.outcome, cad_predicates::BezierIdentity::Different);
        let r = inspect_original_geometric_span_join(&s, 1., 1e-5, 1e-4, 1000).unwrap();
        assert_eq!(
            r.decision,
            Decision::WithinTolerance,
            "normal={:?} tensor={:?}",
            r.normal,
            r.tensor
        );
        assert!(r.regularity_proven && r.pending == 0 && r.cells <= 1000);
        assert!(r.cross_scales.iter().all(|s| s.is_finite() && *s > 0.));
        assert_eq!(s, before);
        let zero = inspect_original_geometric_span_join(&s, 1., 1e-5, 1e-4, 2).unwrap();
        assert_eq!(zero.decision, Decision::Unresolved);
        assert!(zero.cells <= 2);
    }
    let mut bad = graph(0.1);
    bad.knots_u = vec![0., 0., 0., 1., 1., 4., 4., 4.];
    let r = inspect_original_geometric_span_join(&bad, 1., 1e-5, 1e-4, 1000).unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
}
#[test]
fn balanced_clocks_cover_large_ratios_without_hiding_a_real_kink() {
    for kink in [0., 0.1] {
        let mut s = graph(kink);
        s.knots_u = vec![0., 0., 0., 1., 1., 1e8 + 1., 1e8 + 1., 1e8 + 1.];
        let before = s.clone();
        let r = inspect_original_geometric_span_join(&s, 1., 1e-5, 1e-4, 1000).unwrap();
        assert_eq!(
            r.decision,
            if kink == 0. {
                Decision::WithinTolerance
            } else {
                Decision::CorrespondenceExceedsTolerance
            },
            "normal={:?} tensor={:?}",
            r.normal,
            r.tensor
        );
        assert!(r.cross_scales[0] != 1. && r.cross_scales.iter().all(|x| (1e-6..=1e6).contains(x)));
        assert_eq!(s, before);
        let zero = inspect_original_geometric_span_join(&s, 1., 1e-5, 1e-4, 2).unwrap();
        assert_eq!(zero.decision, Decision::Unresolved);
        assert!(zero.cells <= 2);
    }
}
