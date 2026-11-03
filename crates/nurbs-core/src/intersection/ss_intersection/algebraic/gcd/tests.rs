use super::*;
fn p(terms: &[([usize; 4], i64)]) -> ExactSsPolynomial {
    ExactSsPolynomial {
        terms: terms
            .iter()
            .map(|(e, c)| (*e, Q::from_integer((*c).into())))
            .collect(),
    }
}
#[test]
fn recursive_gauss_gcd_preserves_content_and_genuine_multivariate_factors() {
    let mut w = Work {
        used: 0,
        limits: Default::default(),
    };
    let a = p(&[([1, 0, 0, 0], 1), ([0, 1, 0, 0], 1), ([0, 0, 0, 0], 1)]);
    let b = p(&[([0, 0, 1, 0], 1), ([0, 0, 0, 1], -1)]);
    let c = p(&[([0, 1, 0, 0], 1), ([0, 0, 1, 0], 1), ([0, 0, 0, 0], -2)]);
    let ab = a.product(&b, &mut w).unwrap();
    let ac = a.product(&c, &mut w).unwrap();
    let g = compute(&ab, &ac, &[0, 1, 2, 3], &mut w).unwrap();
    assert_eq!(g, monic(&a, &mut w).unwrap());
    let repeated = ab
        .product(&ab, &mut w)
        .unwrap()
        .product(&a, &mut w)
        .unwrap();
    let mut factor = repeated.clone();
    for k in 0..4 {
        factor = compute(
            &factor,
            &repeated.derivative(k).unwrap(),
            &[0, 1, 2, 3],
            &mut w,
        )
        .unwrap();
    }
    let reduced = exact(&repeated, &factor, &mut w).unwrap();
    assert_eq!(
        monic(&reduced, &mut w).unwrap(),
        monic(&ab, &mut w).unwrap()
    );
}
#[test]
fn squarefree_node_is_not_erased_by_differentiation() {
    let mut w = Work {
        used: 0,
        limits: Default::default(),
    };
    let f = p(&[([1, 1, 0, 0], 1)]);
    let mut gcd = f.clone();
    for axis in 0..2 {
        gcd = compute(&gcd, &f.derivative(axis).unwrap(), &[0, 1], &mut w).unwrap();
    }
    assert!(is_one(&gcd));
}
