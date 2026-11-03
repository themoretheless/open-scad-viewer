use super::*;
#[test]
fn stationary_points_and_unordered_rational_poles_are_certified_without_sampling() {
    for gauge in [1., 7.] {
        let scaled = |n: &[f64], d: &[f64]| {
            n.iter()
                .zip(d)
                .map(|(n, d)| [q(n * gauge), q(d * gauge)])
                .collect::<Vec<_>>()
        };
        let mut w = Work::new();
        assert!(w.prove(&scaled(&[0., 1., 0., 1.], &[1.; 4])));
        assert!(Work::new().prove(&scaled(
            &[0., 0.75, 1., 0.25, 2.],
            &[1., 1.25, 1.5, 1.75, 2.]
        )));
        let rational = vec![
            [q(0.), q(1.)],
            [q(1.) / q(3.), q(1.)],
            [-q(1.) / q(3.), q(1.)],
            [q(1.), q(1.)],
        ];
        assert!(Work::new().prove(&rational));
        assert!(!Work::new().prove(&scaled(&[0., 2., -1., 1.], &[1.; 4])));
    }
}
#[test]
fn square_free_odd_part_sturm_proof_handles_multiplicities_endpoints_and_near_complex_roots() {
    for m in 1..=8 {
        let mut w = Work::new();
        let f = vec![-q(1.) / q(3.), q(1.)];
        let mut p = vec![q(1.)];
        for _ in 0..m {
            p = multiply(&p, &f, &mut w).unwrap();
        }
        assert_eq!(
            nonnegative(&p, &mut w),
            Some(m % 2 == 0),
            "multiplicity {m}"
        );
    }
    for a in 1..=4 {
        for b in 1..=4 {
            let mut w = Work::new();
            let mut p = vec![q(1.)];
            for _ in 0..a {
                p = multiply(&p, &vec![-q(1.) / q(3.), q(1.)], &mut w).unwrap();
            }
            for _ in 0..b {
                p = multiply(&p, &vec![-q(2.) / q(3.), q(1.)], &mut w).unwrap();
            }
            assert_eq!(
                nonnegative(&p, &mut w),
                Some(a % 2 == 0 && b % 2 == 0),
                "multiplicities {a},{b}"
            );
        }
    }
    for (p, expected) in [
        (vec![q(0.), q(1.), q(-1.)], true),
        (vec![q(0.), q(-1.), q(1.)], false),
        (vec![q(1.), q(1.)], true),
        (vec![q(2.), q(-1.)], true),
        (vec![q(-2.), q(1.)], false),
    ] {
        assert_eq!(nonnegative(&p, &mut Work::new()), Some(expected));
    }
    let quadratic = vec![-q(0.5), q(0.), q(1.)];
    let mut exact = Work::new();
    let algebraic_square = multiply(&quadratic, &quadratic, &mut exact).unwrap();
    assert_eq!(nonnegative(&algebraic_square, &mut exact), Some(true));
    let tiny = (0..200).fold(q(1.), |a, _| a / q(2.));
    let base = vec![q(1.) / q(9.), -q(2.) / q(3.), q(1.)];
    let mut positive = base.clone();
    positive[0] += &tiny;
    assert_eq!(nonnegative(&positive, &mut Work::new()), Some(true));
    let mut crossing = base;
    crossing[0] -= tiny;
    assert_eq!(nonnegative(&crossing, &mut Work::new()), Some(false));
}
#[test]
fn checked_step_and_bit_exhaustion_never_prove_a_clock() {
    let h = vec![
        [q(0.), q(1.)],
        [q(1.), q(1.)],
        [q(0.), q(1.)],
        [q(1.), q(1.)],
    ];
    assert!(!Work { left: 0 }.prove(&h));
    let big = (0..16).fold(q(2.), |a, _| &a * &a);
    assert!(!Work::new().prove(&[[q(0.), big.clone()], [big.clone(), big]]));
}

#[test]
fn degree_elevation_keeps_stationary_and_fold_certificates_through_degree_eight() {
    let bases = [
        (
            vec![
                [q(0.), q(1.)],
                [q(1.), q(1.)],
                [q(0.), q(1.)],
                [q(1.), q(1.)],
            ],
            true,
        ),
        (
            vec![
                [q(0.), q(1.)],
                [q(2.), q(1.)],
                [q(-1.), q(1.)],
                [q(1.), q(1.)],
            ],
            false,
        ),
        (
            vec![
                [q(0.), q(1.)],
                [q(0.75), q(1.25)],
                [q(1.), q(1.5)],
                [q(0.25), q(1.75)],
                [q(2.), q(2.)],
            ],
            true,
        ),
    ];
    for (mut h, expected) in bases {
        loop {
            assert_eq!(Work::new().prove(&h), expected, "degree {}", h.len() - 1);
            if h.len() == 9 {
                break;
            }
            let n = h.len();
            let mut elevated = Vec::new();
            elevated.push(h[0].clone());
            for i in 1..n {
                let a = q(i as f64) / q(n as f64);
                elevated.push(std::array::from_fn(|k| {
                    &a * &h[i - 1][k] + (q(1.) - &a) * &h[i][k]
                }));
            }
            elevated.push(h[n - 1].clone());
            h = elevated;
        }
    }
}
