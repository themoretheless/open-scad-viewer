use super::*;
fn sources() -> [Surface; 2] {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-3., -3., 5., 5.],
        knots_v: vec![7., 7., 11., 11.],
        control_points: [-0.5, 0.5]
            .iter()
            .map(|&x| {
                [-0.5, 0.5]
                    .iter()
                    .map(|&y| vec![x, y, x * y / 8.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![17., 17., 17., 29., 29., 29.],
        knots_v: vec![-7., -7., -7., 13., 13., 13.],
        control_points: [-0.5, -0.25, 0.5]
            .iter()
            .map(|&x| {
                [-0.5, -0.25, 0.5]
                    .iter()
                    .map(|&y| vec![x, y, x * y / 8.])
                    .collect()
            })
            .collect(),
        weights: [1., 2., 3.]
            .iter()
            .map(|a| [1., 2., 3.].iter().map(|b| 8. * a * b).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    [a, b]
}
#[test]
fn complete_original_normalized_identity_is_opaque_borrowed_and_budgeted_across_representations() {
    let [a, b] = sources();
    let before = [a.clone(), b.clone()];
    let run = |l| certify_normalized_carrier_identity([&a, &b], l).unwrap();
    let l = NormalizedCarrierIdentityLimits {
        cells: 100000,
        exact_work: 1000000,
    };
    let NormalizedCarrierIdentityAttempt::Certified(p) = run(l) else {
        panic!("independent exact saddle representations")
    };
    assert!(p.report().certified && p.matches([&a, &b]));
    assert!(std::ptr::eq(p.surfaces()[1], &b));
    let clone = b.clone();
    assert!(!p.matches([&a, &clone]));
    let r = p.report();
    assert!(r.cells > 0 && r.exact_work > 0);
    let exact = NormalizedCarrierIdentityLimits {
        cells: r.cells,
        exact_work: r.exact_work,
    };
    assert!(matches!(
        run(exact),
        NormalizedCarrierIdentityAttempt::Certified(_)
    ));
    for l in [
        NormalizedCarrierIdentityLimits { cells: 0, ..exact },
        NormalizedCarrierIdentityLimits {
            exact_work: 0,
            ..exact
        },
        NormalizedCarrierIdentityLimits {
            cells: r.cells - 1,
            ..exact
        },
        NormalizedCarrierIdentityLimits {
            exact_work: r.exact_work - 1,
            ..exact
        },
    ] {
        let NormalizedCarrierIdentityAttempt::Refused(q) = run(l) else {
            panic!("incomplete carrier proof admitted")
        };
        assert!(!q.certified && q.cells <= l.cells && q.exact_work <= l.exact_work);
    }
    let mut refined = a.clone();
    refined
        .control_points
        .insert(1, [-0.5, 0.5].iter().map(|&y| vec![0., y, 0.]).collect());
    refined.weights = vec![vec![1.; 2]; 3];
    refined.knots_u = vec![-3., -3., 1., 5., 5.];
    let NormalizedCarrierIdentityAttempt::Certified(refined_proof) =
        certify_normalized_carrier_identity([&refined, &b], l).unwrap()
    else {
        panic!("complete common partition of original knots")
    };
    assert!(refined_proof.matches([&refined, &b]));
    assert_eq!([a.clone(), b.clone()], before);
}
#[test]
fn equal_boundaries_do_not_replace_full_carrier_identity_and_literal_queries_keep_scope() {
    let [a, mut b] = sources();
    b.control_points[1][1][2] += 1. / 1024.;
    let l = NormalizedCarrierIdentityLimits {
        cells: 100000,
        exact_work: 1000000,
    };
    assert!(matches!(
        certify_normalized_carrier_identity([&a, &b], l).unwrap(),
        NormalizedCarrierIdentityAttempt::Refused(_)
    ));
    let mut reversed = a.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    // The same surface image with a reversed UV map is not normalized identity.
    assert!(matches!(
        certify_normalized_carrier_identity([&a, &reversed], l).unwrap(),
        NormalizedCarrierIdentityAttempt::Refused(_)
    ));
    let clone = a.clone();
    let another = a.clone();
    let NormalizedCarrierIdentityAttempt::Certified(p) = certify_normalized_carrier_identity(
        [&a, &clone],
        NormalizedCarrierIdentityLimits {
            cells: 0,
            exact_work: 0,
        },
    )
    .unwrap() else {
        panic!("literal whole-map identity")
    };
    assert!(p.matches([&a, &clone]) && !p.matches([&a, &another]));
    assert_eq!((p.report().cells, p.report().exact_work), (0, 0));
}
