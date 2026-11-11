use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Reason, Sign, SourceArena, ToleranceContext,
    rational_bspline_quadric_projected_edge_sign,
};
fn query(
    frame: [[f64; 3]; 3],
    axes: [usize; 2],
    edge: usize,
    interval: [f64; 2],
    weights: [f64; 2],
    budget: u64,
) -> Result<cad_predicates::OriginalQuadricRootSignDecision, cad_predicates::InputError> {
    let mut v = vec![
        -2., 0., 0., weights[0], 2., 0., 0., weights[1], -2., -2., 5., 5.,
    ];
    v.extend(
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., -1.],
        ]
        .into_iter()
        .flatten(),
    );
    v.extend(frame.into_iter().flatten());
    let arena = SourceArena::authored(
        "original-quadric-edge-sign-test",
        1,
        v.into_iter()
            .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls: Vec<_> = (0..2)
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let knots: Vec<_> = (0..4).map(|i| arena.leaf(8 + i).unwrap()).collect();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(12 + i * 4 + j).unwrap()));
    let frame =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(28 + i * 3 + j).unwrap()));
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: budget,
            ..Limits::default()
        },
        None,
    );
    let r = rational_bspline_quadric_projected_edge_sign(
        &mut ctx,
        cad_predicates::RationalSpanRefs { controls: &controls, knots: &knots, degree: 1, span: 1 },
        cad_predicates::QuadricRefs { matrix, products: &[], frame, axes },
        edge,
        interval,
    )?;
    assert!(r.work_used <= budget);
    Ok(r)
}
fn frame() -> [[f64; 3]; 3] {
    [[-1., -1., 0.], [1., -1., 0.], [-1., 1., 0.]]
}
#[test]
fn sphere_roots_have_exact_projected_coordinate_signs() {
    for axes in [[0, 1], [1, 0]] {
        for (edge, sign) in [Sign::Zero, Sign::Negative, Sign::Positive, Sign::Negative]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                query(frame(), axes, edge, [0.2, 0.3], [1., 1.], 100000)
                    .unwrap()
                    .sign
                    .unwrap(),
                sign
            );
        }
    }
}
#[test]
fn negative_projected_determinant_is_included_in_the_sign() {
    let frame = [[1., -1., 0.], [-1., -1., 0.], [1., 1., 0.]];
    assert_eq!(
        query(frame, [0, 1], 0, [0.2, 0.3], [1., 1.], 100000)
            .unwrap()
            .sign
            .unwrap(),
        Sign::Positive
    );
    assert_eq!(
        query(frame, [0, 1], 1, [0.2, 0.3], [1., 1.], 100000)
            .unwrap()
            .sign
            .unwrap(),
        Sign::Zero
    );
}
#[test]
fn rational_clock_nonbinary_root_keeps_exact_edge_membership() {
    assert_eq!(
        query(frame(), [0, 1], 0, [0.09, 0.11], [1., 3.], 100000)
            .unwrap()
            .sign
            .unwrap(),
        Sign::Zero
    );
}
#[test]
fn nonunique_absent_degenerate_and_exhausted_queries_refuse_proof() {
    for interval in [[0., 1.], [0.3, 0.4]] {
        assert!(
            query(frame(), [0, 1], 0, interval, [1., 1.], 100000)
                .unwrap()
                .sign
                .is_err()
        );
    }
    assert!(matches!(
        query(frame(), [0, 1], 0, [0.2, 0.3], [1., 1.], 0)
            .unwrap()
            .sign,
        Err(Reason::ResourceLimit)
    ));
    assert!(
        query([[0.; 3]; 3], [0, 1], 0, [0.2, 0.3], [1., 1.], 100000)
            .unwrap()
            .sign
            .is_err()
    );
    assert!(query(frame(), [0, 0], 0, [0.2, 0.3], [1., 1.], 100000).is_err());
    assert!(query(frame(), [0, 1], 4, [0.2, 0.3], [1., 1.], 100000).is_err());
}
