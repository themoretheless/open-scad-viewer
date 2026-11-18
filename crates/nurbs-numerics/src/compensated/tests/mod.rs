use super::*;

/// Deterministic xorshift/LCG-style generator (no external crates).
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in (-scale, scale) with full mantissa variety.
    fn f64(&mut self, scale: f64) -> f64 {
        let u = self.next_u64();
        let unit = (u >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        (unit * 2. - 1.) * scale
    }
}

/// Minimal double-double reference (exact enough to act as "truth").
#[derive(Clone, Copy)]
struct Dd(f64, f64);
impl Dd {
    fn of(x: f64) -> Self {
        Self(x, 0.)
    }
    fn add(self, o: Self) -> Self {
        let (s, e) = two_sum(self.0, o.0);
        let lo = self.1 + o.1 + e;
        let (s2, e2) = fast_two_sum(s, lo);
        Self(s2, e2)
    }
    fn mul(self, o: Self) -> Self {
        let (p, e) = two_prod(self.0, o.0);
        let lo = e + (self.0 * o.1 + self.1 * o.0);
        let (s, e2) = fast_two_sum(p, lo);
        Self(s, e2)
    }
    fn value(self) -> f64 {
        self.0 + self.1
    }
}

// -- EFT identities -----------------------------------------------------

#[test]
fn two_sum_is_exact_on_random_pairs() {
    let mut rng = Lcg::new(42);
    for _ in 0..10_000 {
        let a = rng.f64(1e6);
        let b = rng.f64(1e6);
        let (s, e) = two_sum(a, b);
        assert_eq!(s, a + b);
        // With |a| >= |b|, a − s is exact, so the error term satisfies the
        // checkable identity e == (a − s) + b, and fast_two_sum agrees.
        let (big, small) = if a.abs() >= b.abs() { (a, b) } else { (b, a) };
        let (fs, fe) = fast_two_sum(big, small);
        assert_eq!(fs, s);
        assert_eq!(fe, e);
        assert_eq!((big - s) + small, e, "a={a:e} b={b:e}");
        assert_eq!(s + e, s);
    }
}

#[test]
fn two_sum_handles_mixed_signs_and_zero() {
    let (s, e) = two_sum(1., 1.);
    assert_eq!((s, e), (2., 0.));
    let (s, e) = two_sum(1e300, -1e300);
    assert_eq!((s, e), (0., 0.));
    let (s, e) = two_sum(0., -0.);
    assert_eq!(s, 0.);
    assert_eq!(e, 0.);
    // Classic cancellation: exact when Sterbenz applies.
    let (s, e) = two_sum(1., -0.5);
    assert_eq!((s, e), (0.5, 0.));
}

#[test]
fn fast_two_sum_matches_two_sum_when_ordered() {
    let mut rng = Lcg::new(7);
    for _ in 0..10_000 {
        let a = rng.f64(1e8);
        let b = rng.f64(1e8);
        let (a, b) = if a.abs() >= b.abs() { (a, b) } else { (b, a) };
        assert_eq!(fast_two_sum(a, b), two_sum(a, b));
    }
}

#[test]
fn split_is_exact_and_non_overlapping() {
    let mut rng = Lcg::new(99);
    for _ in 0..10_000 {
        let a = rng.f64(1e12);
        let (hi, lo) = split(a);
        assert_eq!(hi + lo, a);
        // hi keeps at most 53−27 = 26 significant bits: re-splitting hi
        // must return (hi, 0).
        assert_eq!(split(hi), (hi, 0.));
    }
}

#[test]
fn two_prod_fma_matches_split_variant() {
    let mut rng = Lcg::new(1234);
    for _ in 0..10_000 {
        let a = rng.f64(1e150);
        let b = rng.f64(1e-100);
        let (p1, e1) = two_prod_fma(a, b);
        let (p2, e2) = two_prod_split(a, b);
        assert_eq!(p1, a * b);
        assert_eq!(p1, p2);
        assert_eq!(e1, e2, "a={a:e} b={b:e}");
        // Exactness: p + e rounds back to p.
        assert_eq!(p1 + e1, p1);
    }
}

// -- Dot2 on ill-conditioned data ----------------------------------------

#[test]
fn dot2_recovers_catastrophic_cancellation() {
    // True dot = 1; naive f64 dot = 0.
    let x = [1e16, 1., -1e16];
    let y = [1e16, 1., 1e16];
    assert_eq!(naive_dot(&x, &y), 0.);
    let r = dot2(&x, &y).unwrap();
    assert_eq!(r.value, 1.);
    assert!(r.error_bound >= 0.);
    assert!((r.value - 1.).abs() <= r.error_bound + f64::EPSILON);
}

#[test]
fn dot2_beats_naive_on_random_ill_conditioned_data() {
    let mut rng = Lcg::new(2024);
    for case in 0..200 {
        let n = 4 + (rng.next_u64() % 60) as usize;
        let mut x = Vec::with_capacity(n);
        let mut y = Vec::with_capacity(n);
        // Heavy cancellation: pair ±mag with tiny residuals.
        for i in 0..n {
            let mag = rng.f64(1e8);
            let sign = if i % 2 == 0 { 1. } else { -1. };
            x.push(sign * mag);
            y.push(mag.abs() + rng.f64(1.));
        }
        let truth = {
            let mut acc = Dd::of(0.);
            for i in 0..n {
                acc = acc.add(Dd::of(x[i]).mul(Dd::of(y[i])));
            }
            acc.value()
        };
        let r = dot2(&x, &y).unwrap();
        let err = (r.value - truth).abs();
        assert!(
            err <= r.error_bound,
            "case {case}: |err|={err:e} bound={:e}",
            r.error_bound
        );
        let naive_err = (naive_dot(&x, &y) - truth).abs();
        assert!(
            r.error_bound <= naive_err.max(U * truth.abs()) * 4. + 1e-12,
            "case {case}: bound {:e} not tighter than naive err {naive_err:e}",
            r.error_bound
        );
    }
}

#[test]
fn dot_k_refines_monotonically() {
    let mut rng = Lcg::new(555);
    let n = 32;
    let x: Vec<f64> = (0..n).map(|_| rng.f64(1e6)).collect();
    let y: Vec<f64> = (0..n).map(|_| rng.f64(1e6)).collect();
    let truth = {
        let mut acc = Dd::of(0.);
        for i in 0..n {
            acc = acc.add(Dd::of(x[i]).mul(Dd::of(y[i])));
        }
        acc.value()
    };
    let e2 = (dot_k(&x, &y, 2).unwrap().value - truth).abs();
    let e4 = (dot_k(&x, &y, 4).unwrap().value - truth).abs();
    assert!(e4 <= e2 * 4. + U * truth.abs());
    assert!(dot_k(&x, &y, 1).is_err());
    assert!(dot_k(&x, &y, 9).is_err());
}

#[test]
fn dot_validates_inputs() {
    assert!(dot2(&[1.], &[1., 2.]).is_err());
    assert!(dot2(&[], &[]).is_err());
    assert!(dot2(&[f64::NAN], &[1.]).is_err());
}

// -- CompHorner -----------------------------------------------------------

/// Wilkinson polynomial W_10(x) = Π_{i=1..10} (x − i), ascending coeffs.
fn wilkinson_10() -> Vec<f64> {
    // Roots 1..=10, expanded (exact integers, exactly representable).
    let mut coeffs = vec![1.0f64]; // running product, ascending
    for r in 1..=10u32 {
        let mut next = vec![0.; coeffs.len() + 1];
        for (i, &c) in coeffs.iter().enumerate() {
            next[i] -= c * r as f64; // multiply by (x - r): -r·c
            next[i + 1] += c; //            + x·c
        }
        coeffs = next;
    }
    coeffs
}

#[test]
fn wilkinson_coefficients_are_exact_integers() {
    let c = wilkinson_10();
    assert_eq!(c.len(), 11);
    assert_eq!(c[10], 1.);
    assert_eq!(c[0], 3_628_800.); // 10!
    assert!(c.iter().all(|v| v.fract() == 0.));
}

#[test]
fn comp_horner_near_wilkinson_root() {
    let c = wilkinson_10();
    // Near root 9: naive Horner loses ~8 digits; the exact value is the
    // product Π(x − i), cheap and accurate in f64 away from the root.
    let x = 9.000_001;
    let exact: f64 = (1..=10).map(|i| x - i as f64).product();
    let naive = {
        let mut s = c[10];
        for i in (0..10).rev() {
            s = s * x + c[i];
        }
        s
    };
    let r = comp_horner(&c, x).unwrap();
    let err = (r.value - exact).abs();
    assert!(
        err <= r.error_bound,
        "|err|={err:e} bound={:e}",
        r.error_bound
    );
    assert!(
        err <= (naive - exact).abs(),
        "compensated {err:e} should beat naive {:e}",
        (naive - exact).abs()
    );
    // Relative accuracy near double-double territory.
    assert!(err / exact.abs().max(1e-300) < 1e-13);
}

#[test]
fn comp_horner_bound_covers_random_polynomials() {
    let mut rng = Lcg::new(31337);
    for case in 0..300 {
        let deg = 1 + (rng.next_u64() % 20) as usize;
        let c: Vec<f64> = (0..=deg).map(|_| rng.f64(1e4)).collect();
        let x = rng.f64(2.);
        let truth = {
            let mut acc = Dd::of(c[deg]);
            for i in (0..deg).rev() {
                acc = acc.mul(Dd::of(x)).add(Dd::of(c[i]));
            }
            acc.value()
        };
        let r = comp_horner(&c, x).unwrap();
        let err = (r.value - truth).abs();
        assert!(
            err <= r.error_bound,
            "case {case}: |err|={err:e} bound={:e}",
            r.error_bound
        );
    }
    assert!(comp_horner(&[], 0.5).is_err());
    assert!(comp_horner(&[1., f64::NAN], 0.5).is_err());
}

// -- Compensated de Casteljau --------------------------------------------

#[test]
fn bezier_point_endpoints_and_line() {
    let line = vec![vec![0., 0., 0.], vec![2., 4., 6.]];
    let p0 = bezier_point(&line, 0.).unwrap();
    let p1 = bezier_point(&line, 1.).unwrap();
    assert_eq!(p0[0], CompensatedValue::exact(0.));
    assert_eq!(p1[2].value, 6.);
    // t=1: s=0, δ=0 — only the final u·|value| rounding term remains.
    assert!(p1[2].error_bound <= 2. * U * 6.);
    let mid = bezier_point(&line, 0.5).unwrap();
    assert_eq!(mid.iter().map(|c| c.value).collect::<Vec<_>>(), [1., 2., 3.]);
    assert!(mid.iter().all(|c| c.error_bound >= 0.));
}

#[test]
fn bezier_point_matches_bernstein_sum() {
    // Quadratic Bézier; compare against an exact Bernstein evaluation in
    // double-double arithmetic at several parameters.
    let ctrl = vec![vec![1e8, -3.], vec![-1e8, 1.], vec![1e8, 7.]];
    let mut rng = Lcg::new(777);
    for _ in 0..200 {
        let t = rng.next_u64() % 1_000_001;
        let t = t as f64 / 1_000_000.;
        let got = bezier_point(&ctrl, t).unwrap();
        for (d, comp) in got.iter().enumerate() {
            let mut acc = Dd::of(0.);
            for (i, p) in ctrl.iter().enumerate() {
                // Bernstein weight B_i^2(t) via exact dd products.
                let b = match i {
                    0 => Dd::of(1. - t).mul(Dd::of(1. - t)),
                    1 => Dd::of(2.).mul(Dd::of(t)).mul(Dd::of(1. - t)),
                    _ => Dd::of(t).mul(Dd::of(t)),
                };
                acc = acc.add(b.mul(Dd::of(p[d])));
            }
            let err = (comp.value - acc.value()).abs();
            assert!(
                err <= comp.error_bound,
                "d={d} t={t}: |err|={err:e} bound={:e}",
                comp.error_bound
            );
        }
    }
}

#[test]
fn bezier_point_cancellation_beats_naive_de_casteljau() {
    // Heavy cancellation along coordinate 0.
    let ctrl = vec![vec![1e16, 1.], vec![-1e16, 2.], vec![1e16, 3.]];
    let t = 0.5;
    let got = bezier_point(&ctrl, t).unwrap();
    // Exact: B·P with B = (1/4, 1/2, 1/4) → coord0 = 1e16/4 − 1e16/2 + 1e16/4 = 0.
    assert_eq!(got[0].value, 0.);
    assert!(got[0].error_bound <= 1e16 * 1e-10);
    assert!((got[1].value - 2.).abs() <= got[1].error_bound + 1e-15);
}

#[test]
fn bezier_point_validates_inputs() {
    let ctrl = vec![vec![0.], vec![1.]];
    assert!(bezier_point(&ctrl, -0.1).is_err());
    assert!(bezier_point(&ctrl, 1.1).is_err());
    assert!(bezier_point(&ctrl, f64::NAN).is_err());
    assert!(bezier_point(&[], 0.5).is_err());
    assert!(bezier_point(&[vec![0.], vec![1., 2.]], 0.5).is_err());
    assert!(bezier_point(&[vec![f64::INFINITY], vec![1.]], 0.5).is_err());
}

#[test]
fn compensated_dot3_and_cross3_recover_cancellation() {
    // True dot3 = 1; naive f64 dot = 0.
    let a = [1e16, 1., -1e16];
    let b = [1e16, 1., 1e16];
    assert_eq!(compensated_dot3(a, b), 1.);

    // Nearly parallel vectors: u = (1, 1 + 2^-52, 0), v = (1 - 2^-52, 1, 0)
    // z-component of u × v is 1*1 - (1 + eps)*(1 - eps) = eps^2 = 2^-104.
    let eps = f64::EPSILON;
    let u = [1., 1. + eps, 0.];
    let v = [1. - eps, 1., 0.];
    let c = compensated_cross3(u, v);
    assert_eq!(c[0], 0.);
    assert_eq!(c[1], 0.);
    assert_eq!(c[2], eps * eps);
}
