use super::*;
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
