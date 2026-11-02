
    use super::*;
    #[test]
    fn separable_extraction_matches_dense_rationals_and_binary_enclosures() {
        let q = |a: i64, b: i64| Q::new(a.into(), b.into());
        let u = vec![
            vec![q(1, 1), q(0, 1), q(0, 1)],
            vec![q(1, 3), q(2, 3), q(0, 1)],
            vec![q(1, 9), q(4, 9), q(4, 9)],
        ];
        let v = vec![
            vec![q(4, 9), q(4, 9), q(1, 9)],
            vec![q(0, 1), q(2, 3), q(1, 3)],
            vec![q(0, 1), q(0, 1), q(1, 1)],
        ];
        let poles = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        std::array::from_fn(|k| {
                            q(i as i64 * 7 - j as i64 * 11 + k as i64 * 3, (k + 1) as i64)
                        })
                    })
                    .collect()
            })
            .collect();
        let jet = Jet {
            degrees: [2, 2],
            knots: [vec![], vec![]],
            poles,
        };
        let mut fast = Work {
            used: 0,
            limit: 1_000_000,
        };
        let net = extract_tensor_net(&jet, [2, 2], &u, &v, &mut fast).unwrap();
        let mut dense = Work {
            used: 0,
            limit: 1_000_000,
        };
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..4 {
                    let mut sum = Q::zero();
                    for r in 0..3 {
                        for t in 0..3 {
                            sum += &u[i][r] * &v[j][t] * &jet.poles[r][t][k];
                            dense.charge(&sum).unwrap();
                        }
                    }
                    let expected = enclosure(&sum).unwrap();
                    assert_eq!(
                        (net[i][j][k].lo.to_bits(), net[i][j][k].hi.to_bits()),
                        (expected.lo.to_bits(), expected.hi.to_bits())
                    );
                }
            }
        }
        assert!(fast.used < dense.used, "{} vs {}", fast.used, dense.used);
        let mut exhausted = Work { used: 0, limit: 0 };
        assert!(extract_tensor_net(&jet, [2, 2], &u, &v, &mut exhausted).is_err());
        assert_eq!(exhausted.used, 0);
    }
