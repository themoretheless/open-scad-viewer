use nurbs_core::{
    curve::Curve,
    periodic_trim::{AdmissionStatus, split_candidates, verify_partition},
};
fn source() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![3.5, 0.1], vec![4., 0.5], vec![4.5, 0.9]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    }
}
#[test]
fn original_rational_functions_and_translated_actual_pieces_keep_one_parameter() {
    let s = source();
    let original = s.clone();
    let pieces = split_candidates(&s, [[0., 4.], [0., 1.]], &[0.5], &[[0, 0], [1, 0]], 2).unwrap();
    let proof = verify_partition(&s, &pieces, 1e-8, 2).unwrap();
    assert_eq!(proof.status, AdmissionStatus::WithinTolerance);
    assert_eq!(proof.work, 2);
    assert!(proof.error_upper < 1e-10);
    assert_eq!(s, original);
    for piece in &pieces.pieces {
        for i in 0..=16 {
            let t = piece.source_domain[0]
                + (piece.source_domain[1] - piece.source_domain[0]) * i as f64 / 16.;
            let a = s.evaluate_de_boor(t).unwrap();
            let b = piece.curve.evaluate_de_boor(t).unwrap();
            let error = (0..2)
                .map(|k| (a[k] - b[k] - piece.translation[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= proof.error_upper + 1e-14);
        }
    }
    let mut changed = pieces.clone();
    changed.pieces[1].translation[0] += 0.001;
    assert_ne!(
        verify_partition(&s, &changed, 1e-8, 64).unwrap().status,
        AdmissionStatus::WithinTolerance
    );
    let mut changed = pieces.clone();
    changed.pieces[0].curve.control_points[1][1] += 0.01;
    assert_ne!(
        verify_partition(&s, &changed, 1e-8, 64).unwrap().status,
        AdmissionStatus::WithinTolerance
    );
    let mut changed = pieces;
    changed.pieces[1].curve.weights[1] *= 1.1;
    assert_ne!(
        verify_partition(&s, &changed, 1e-8, 64).unwrap().status,
        AdmissionStatus::WithinTolerance
    );
}
#[test]
fn a_partial_cell_budget_never_becomes_a_complete_partition_proof() {
    let s = source();
    let p = split_candidates(&s, [[0., 4.], [0., 1.]], &[0.5], &[[0, 0], [1, 0]], 2).unwrap();
    let r = verify_partition(&s, &p, 1e-8, 1).unwrap();
    assert_eq!(r.status, AdmissionStatus::Unresolved);
    assert!(r.error_upper.is_infinite());
}
