use nurbs_core::{
    curve::Curve,
    surface::{self, Surface},
    surface_injectivity::{certify, certify_on},
};
fn tube() -> Surface {
    let c = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    surface::extrude(&c, [0., 0., 2.]).unwrap()
}
#[test]
fn original_periodic_support_has_local_proofs_without_a_global_injectivity_claim() {
    let s = tube();
    let original = s.clone();
    assert!(!certify(&s, 100).unwrap().proven);
    for u in [[0., 0.5], [3.5, 4.]] {
        let r = certify_on(&s, [u, [0.25, 0.75]], 100).unwrap();
        assert!(r.proven, "{r:?}");
        assert_eq!(r.spans, 1);
        assert!(r.contraction_upper.unwrap() < 1.);
    }
    assert_eq!(s, original);
    assert_eq!(
        s.evaluate(0., 0.5).unwrap().point,
        s.evaluate(4., 0.5).unwrap().point
    );
}
#[test]
fn full_requested_rectangle_and_its_actual_span_census_are_mandatory() {
    let s = tube();
    let r = certify_on(&s, [[0.5, 1.5], [0.25, 0.75]], 1).unwrap();
    assert!(!r.proven);
    assert_eq!(r.reason, "work-limit");
    assert_eq!(r.spans, 1);
    for d in [
        [[0., 0.], [0., 1.]],
        [[-0.1, 0.5], [0., 1.]],
        [[0., f64::NAN], [0., 1.]],
        [[0., 0.5], [0., 1.1]],
    ] {
        assert!(certify_on(&s, d, 100).is_err());
    }
    assert!(certify_on(&s, [[0., 0.5], [0., 1.]], 0).is_err());
}
#[test]
fn a_valid_local_patch_never_certifies_the_folded_remainder() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            vec![vec![-1., 0., 0.], vec![-1., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 1.]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(!certify(&s, 100).unwrap().proven);
    assert!(certify_on(&s, [[0., 0.2], [0., 1.]], 100).unwrap().proven);
    let a = s.evaluate(0.1, 0.3).unwrap().point;
    let b = s.evaluate(0.9, 0.3).unwrap().point;
    assert!((a[0] - b[0]).abs() < 1e-14);
}

#[test]
fn refinement_proves_one_common_original_domain_and_never_combines_independent_local_successes() {
    use nurbs_core::surface_injectivity::certify_on_refined;
    let s = tube();
    let all = [[0., 4.], [0., 1.]];
    for u in [[0., 1.], [1., 2.], [2., 3.], [3., 4.]] {
        assert!(certify_on(&s, [u, [0., 1.]], 100).unwrap().proven);
    }
    assert!(!certify_on_refined(&s, all, 1000, 3).unwrap().proven);
    let limited = certify_on_refined(&s, all, 5, 3).unwrap();
    assert!(!limited.proven);
    assert_eq!(limited.reason, "work-limit");
    assert_eq!(limited.spans, 4);
    assert!(certify_on_refined(&s, all, 100, 7).is_err());
    let torus = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let r = certify_on_refined(&torus, [[0.95, 1.], [3.95, 4.]], 10000, 4).unwrap();
    assert!(r.proven, "{r:?}");
    assert!(r.spans > 1);
    assert!(r.contraction_upper.unwrap() < 1.);
}
