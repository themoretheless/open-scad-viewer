use cad_predicates::{
    AuthoredScalar, Limits, PlanarPairOutcome, PredicateContext, SourceArena, ToleranceContext,
    planar_bezier_pair_roots,
};
fn pair(a: &[[f64; 3]], b: &[[f64; 3]]) -> PlanarPairOutcome {
    let source = SourceArena::authored(
        "degree-nine-singularities",
        1,
        a.iter()
            .chain(b)
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let refs = |start: usize, n: usize| {
        (start..start + n)
            .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
            .collect::<Vec<_>>()
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    planar_bezier_pair_roots(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()), 1e-12)
        .unwrap()
        .outcome
}
#[test]
fn degree_nine_census_retains_a_sixfold_cusp_and_an_independent_simple_contact() {
    // u=3t-1, v=3s-1. A=(3u²,3u³), B=(3v²+3v³,6v³).
    // Its eliminant is 27v^6(4-(1+v)^3); the three remaining roots
    // consist of one real contact and a conjugate pair.
    let a = [[3., -3., 1.], [-3., 6., 1.], [0., -12., 1.], [12., 24., 1.]];
    let b = [
        [0., -6., 1.],
        [3., 12., 1.],
        [-12., -24., 1.],
        [36., 48., 1.],
    ];
    for rotated in [false, true] {
        let frame = |p: [f64; 3]| {
            if rotated {
                [17. + p[0] - p[1], -9. + p[0] + p[1], p[2]]
            } else {
                p
            }
        };
        let outcome = pair(&a.map(frame), &b.map(frame));
        let PlanarPairOutcome::Roots(roots) = outcome else {
            panic!("incomplete degree-nine census: {outcome:?}")
        };
        assert_eq!(roots.len(), 2);
        let c = 4_f64.cbrt();
        for (r, (t, s, m)) in roots
            .iter()
            .zip([(1. / 3., 1. / 3., 6), (1. - 2. / (3. * c), c / 3., 1)])
        {
            assert!(r.first[0] <= t && t <= r.first[1]);
            assert!(r.second[0] <= s && s <= r.second[1]);
            assert_eq!(r.multiplicity, m);
            assert!(r.first[1] - r.first[0] <= 1e-12);
            assert!(r.second[1] - r.second[0] <= 1e-12);
        }
    }
}
#[test]
fn degree_nine_seventh_order_cusp_does_not_create_a_second_branch() {
    // A=(3t²,3t³), B=(3s²+3s³,3s³); the eliminant is
    // -27s^7(3+3s+s²), with no other real zero in the unit domain.
    let a = [[0., 0., 1.], [0., 0., 1.], [1., 0., 1.], [3., 3., 1.]];
    let b = [[0., 0., 1.], [0., 0., 1.], [1., 0., 1.], [6., 3., 1.]];
    let outcome = pair(&a, &b);
    let PlanarPairOutcome::Roots(roots) = outcome else {
        panic!("missing seventh-order contact: {outcome:?}")
    };
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].first, [0., 0.]);
    assert_eq!(roots[0].second, [0., 0.]);
    assert_eq!(roots[0].multiplicity, 7);
}

#[test]
fn nonprimitive_rational_cubic_cancels_only_an_exact_homogeneous_factor() {
    // H=3(1+t)*(10t,5t²,1), with all Euclidean poles/weights exactly binary64.
    let a=[[0.,0.,3.],[2.5,0.,4.],[6.,1.,5.],[10.,5.,6.]];
    let b=[[5.,0.,1.],[5.,5.,1.]];
    let outcome=pair(&a,&b);
    let PlanarPairOutcome::Roots(roots)=outcome else{panic!("Nonprimitive carrier refused: {outcome:?}");};
    assert_eq!(roots.len(),1);
    assert!(roots[0].first[0]<=0.5&&0.5<=roots[0].first[1]);
    assert!(roots[0].second[0]<=0.25&&0.25<=roots[0].second[1]);
    let mut perturbed=a;perturbed[2][1]=f64::from_bits(1f64.to_bits()+1);
    // Perturbing a single source bit must never be treated as exact equality
    // with the reduced polynomial carrier.
    let q=pair(&perturbed,&b);
    if let PlanarPairOutcome::Roots(points)=q {
        assert_eq!(points.len(),1);
    }
}
