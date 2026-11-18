use super::*;
fn basis(p: usize, i: usize, t: &Q) -> Q {
    (0..i).fold(choose(p, i), |a, _| a * t) * (0..p - i).fold(q(1.), |a, _| a * (q(1.) - t))
}
fn at(a: &Poly, u: &Q, v: &Q) -> Q {
    let [p, r] = a.degrees();
    (0..=p)
        .map(|i| {
            (0..=r)
                .map(|j| &a.0[i][j] * basis(p, i, u) * basis(r, j, v))
                .sum::<Q>()
        })
        .sum()
}
#[test]
fn tensor_product_and_derivatives_match_independent_basis_sums_for_every_degree_pair() {
    for p in 1..=8 {
        for r in 1..=8 {
            let a = Poly(
                (0..=p)
                    .map(|i| (0..=r).map(|j| q((1 + i * i + 2 * j) as f64)).collect())
                    .collect(),
            );
            let b = Poly(vec![vec![q(1.), q(3.)], vec![q(2.), q(7.)]]);
            let mut w = Work::new(LIMIT);
            let (u, v) = (q(0.25), q(0.75));
            assert_eq!(
                at(&a.mul(&b, &mut w).unwrap(), &u, &v),
                at(&a, &u, &v) * at(&b, &u, &v)
            );
            for axis in 0..2 {
                let mut derivative = q(0.);
                for i in 0..=p {
                    for j in 0..=r {
                        let (d, k, t) = if axis == 0 { (p, i, &u) } else { (r, j, &v) };
                        let db = q(d as f64)
                            * ((if k > 0 { basis(d - 1, k - 1, t) } else { q(0.) })
                                - (if k < d { basis(d - 1, k, t) } else { q(0.) }));
                        derivative += &a.0[i][j]
                            * db
                            * if axis == 0 {
                                basis(r, j, &v)
                            } else {
                                basis(p, i, &u)
                            };
                    }
                }
                assert_eq!(at(&a.derivative(axis, &mut w).unwrap(), &u, &v), derivative);
            }
        }
    }
}
#[test]
fn positivity_subdivision_distinguishes_strict_minimum_zero_and_negative_pockets() {
    for (middle, positive) in [(-0.24, true), (-0.25, false), (-0.26, false)] {
        let p = Poly(vec![
            vec![q(0.25) + (q(middle) + q(0.25))],
            vec![q(middle)],
            vec![q(0.25) + (q(middle) + q(0.25))],
        ]);
        // Coefficients [1/4+epsilon,-1/4+epsilon,1/4+epsilon].
        assert_eq!(p.positive(&mut Work::new(LIMIT), 0).unwrap(), positive);
    }
}
#[test]
fn exact_work_and_bit_limits_fail_closed() {
    let p = Poly(vec![vec![q(1.), q(2.)], vec![q(3.), q(4.)]]);
    assert!(p.mul(&p, &mut Work::new(0)).is_none());
    assert!(p.positive(&mut Work::new(0), 0).is_none());
    let huge = Q::from_integer(num_bigint::BigInt::from(1u8) << 32769usize);
    assert!(Work::new(LIMIT).keep(huge).is_none());
}
