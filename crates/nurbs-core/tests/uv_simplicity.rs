use nurbs_core::{
    curve::Curve,
    trim_simplicity::{self, Outcome},
};
fn polygon(points: &[[f64; 2]]) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            nurbs_core::paths::bezier(
                vec![points[i].to_vec(), points[(i + 1) % points.len()].to_vec()],
                Some(vec![1., 3.]),
            )
            .unwrap()
        })
        .collect()
}
#[test]
fn weighted_square_is_proven_simple_without_sampling() {
    let c = polygon(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]]);
    let r = trim_simplicity::inspect(&c, 1e-6, 6, 4096).unwrap();
    assert!(r.proven_simple);
    assert_eq!(r.outcome(), Outcome::ProvenSimple);
    assert_eq!(r.total_pairs, 6);
    assert!(r.injective.iter().all(|v| *v));
    assert!(r.pairs.iter().all(|p| p.proven));
    let limited = trim_simplicity::inspect(&c, 1e-6, 1, 4096).unwrap();
    assert!(!limited.proven_simple);
    assert_eq!(limited.outcome(), Outcome::PairBudgetExhausted);
}
#[test]
fn crossing_does_not_become_a_false_simplicity_proof() {
    let c = polygon(&[[0., 0.], [4., 4.], [0., 4.], [4., 0.]]);
    let r = trim_simplicity::inspect(&c, 1e-6, 6, 4096).unwrap();
    assert!(!r.proven_simple);
    assert_eq!(r.outcome(), Outcome::SeparationUnproven);
    assert!(r.pairs.iter().any(|p| !p.proven));
}
#[test]
fn closure_refinement_and_noninjective_segment_proofs_have_distinct_outcomes() {
    let mut c = polygon(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]]);
    c[0].control_points[0][0] = 1.;
    assert_eq!(
        trim_simplicity::inspect(&c, 1e-6, 6, 4096)
            .unwrap()
            .outcome(),
        Outcome::ClosureMismatch
    );
    let mut c = polygon(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]]);
    c[0] = c[0].insert(0.5, 1).unwrap();
    assert_eq!(
        trim_simplicity::inspect(&c, 1e-6, 6, 4096)
            .unwrap()
            .outcome(),
        Outcome::ProvenSimple
    );
    // A genuinely folded edge cannot receive the new multispan monotonicity proof.
    c[0] = nurbs_core::paths::bezier(
        vec![vec![0., 0.], vec![6., 0.], vec![-2., 0.], vec![4., 0.]],
        None,
    )
    .unwrap();
    assert_eq!(
        trim_simplicity::inspect(&c, 1e-6, 6, 4096)
            .unwrap()
            .outcome(),
        Outcome::InjectivityUnproven
    );
    assert!(trim_simplicity::inspect(&c, 0., 6, 4096).is_err());
    assert!(trim_simplicity::inspect(&c, 1e-6, 0, 4096).is_err());
}
