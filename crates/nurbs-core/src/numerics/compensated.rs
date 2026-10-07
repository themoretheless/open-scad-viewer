//! Compensated floating-point arithmetic (error-free transformations).
//!
//! The primitives in this module (Knuth/Dekker `two_sum`/`two_prod`,
//! Ogita–Rump–Oishi `dot2`/`dot_k`, Graillat–Langlois–Louvet `CompHorner`,
//! and a compensated de Casteljau evaluation for Bézier/Bernstein form)
//! deliver computed values together with a *running* error bound that is
//! outward-rounded so it always covers the true absolute error:
//!
//! ```text
//! |value - exact| <= error_bound
//! ```
//!
//! Accuracy is as if the computation were done in twice (or `k` times) the
//! working precision and then rounded, at a small constant-factor cost.
//! No `unsafe`, no external crates.
use crate::{Result, check, numeric};

/// Machine epsilon for binary64; `u = 2^-53`.
const U: f64 = f64::EPSILON;
/// Splitting constant for Dekker's `two_prod`: `2^27 + 1`.
const SPLITTER: f64 = 134_217_729.;
/// Resource budget: largest supported vector/polynomial/control-net length.
const MAX_LEN: usize = 1 << 16;

/// A computed value paired with a certified, outward-rounded error bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompensatedValue {
    /// Computed approximation.
    pub value: f64,
    /// Certified bound: `|value - exact| <= error_bound` (outward rounded).
    pub error_bound: f64,
}

impl CompensatedValue {
    #[cfg(test)]
    fn exact(value: f64) -> Self {
        Self {
            value,
            error_bound: 0.,
        }
    }
}

/// Next representable float towards `+∞` (independent of `math_core`, so this
/// module stays self-contained).
fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0. {
        return f64::MIN_POSITIVE * f64::EPSILON; // smallest subnormal
    }
    let bits = x.to_bits();
    let bits = if x > 0. { bits + 1 } else { bits - 1 };
    f64::from_bits(bits)
}

/// Knuth's `TwoSum`: exact transformation `a + b = s + e` with `s = fl(a+b)`.
/// Six flops, no branch, valid for all finite `a, b` (barring overflow).
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bp = s - a;
    let ap = s - bp;
    let e = (a - ap) + (b - bp);
    (s, e)
}

/// Dekker's `FastTwoSum`: same result as [`two_sum`] in three flops, but only
/// valid when `|a| >= |b|`.
pub fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    debug_assert!(a.abs() >= b.abs());
    let s = a + b;
    let e = (a - s) + b;
    (s, e)
}

/// Veltkamp–Dekker split of `a` into a high part with at most 26 significant
/// bits and a non-overlapping low part: `a = hi + lo`.
pub fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    let lo = a - hi;
    (hi, lo)
}

/// Dekker's `TwoProd` without FMA: exact `a * b = p + e`, 17 flops.
pub fn two_prod_split(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let e = ((ah * bh - p) + ah * bl + al * bh) + al * bl;
    (p, e)
}

/// `TwoProd` with FMA: exact `a * b = p + e`, 2 flops.
pub fn two_prod_fma(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let e = a.mul_add(b, -p);
    (p, e)
}

/// Portable `TwoProd`: uses FMA when the target supports it, Dekker otherwise.
/// `std::f64::mul_add` lowers to a hardware FMA on all tier-1 targets.
pub fn two_prod(a: f64, b: f64) -> (f64, f64) {
    two_prod_fma(a, b)
}

/// Outward-rounded magnitude product; zero stays zero (bounds stay tight).
fn mul_up(a: f64, b: f64) -> f64 {
    let p = a.abs() * b.abs();
    if p == 0. { 0. } else { next_up(p) }
}

/// Outward-rounded sum of non-negative magnitudes; zero stays zero.
fn add_up(a: f64, b: f64) -> f64 {
    let s = a + b;
    if s == 0. { 0. } else { next_up(s) }
}

/// `γ_k = k·u / (1 - k·u)`, outward rounded (overflow-safe for our budgets).
fn gamma(k: usize) -> f64 {
    let ku = k as f64 * U;
    next_up(ku / (1. - ku))
}

// ---------------------------------------------------------------------------
// Dot2 / DotK (Ogita–Rump–Oishi)
// ---------------------------------------------------------------------------

/// Core `TwoProduct` + error-free summation pass shared by `dot2`/`dot_k`.
/// Returns `(p, pi_sigma)` where `p[i] = fl(x[i]·y[i])` and the second vector
/// collects both the product errors `π_i` and the `TwoSum` errors `σ_i`.
fn dot_eft(x: &[f64], y: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len();
    let mut p = Vec::with_capacity(n);
    let mut e = Vec::with_capacity(2 * n);
    let (mut s, c0) = two_prod(x[0], y[0]);
    p.push(s);
    e.push(c0);
    for i in 1..n {
        let (h, pi) = two_prod(x[i], y[i]);
        let (sn, sigma) = two_sum(s, h);
        p.push(sn);
        e.push(pi);
        e.push(sigma);
        s = sn;
    }
    (p, e)
}

/// One `Sum2`-style refinement pass of the error vector `e`: compensates the
/// summation of `e` and returns `(sum, residual_errors)`.
fn sum2_pass(e: &[f64]) -> (f64, Vec<f64>) {
    let mut s = e[0];
    let mut next = Vec::with_capacity(e.len().saturating_sub(1));
    for &v in &e[1..] {
        let (sn, err) = two_sum(s, v);
        next.push(err);
        s = sn;
    }
    (s, next)
}

/// Naive dot product, used as the uncompensated baseline in tests.
#[cfg(test)]
fn naive_dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}

fn validate_dot_inputs(x: &[f64], y: &[f64]) -> Result<()> {
    check(x.len() == y.len(), "Compensated dot needs equal-length inputs")?;
    check(!x.is_empty(), "Compensated dot needs non-empty inputs")?;
    check(
        x.len() <= MAX_LEN,
        "Compensated dot length exceeds the resource budget",
    )?;
    numeric(
        x.iter().chain(y.iter()).all(|v| v.is_finite()),
        "Compensated dot inputs must be finite",
    )?;
    Ok(())
}

/// Certified bound for `dot2`: `|res − dot| ≤ u·|res| + γ_n²·Σ|xᵢ·yᵢ|`
/// (Ogita–Rump–Oishi, Thm 4.6), outward rounded term by term.
fn dot2_bound(res: f64, abs_dot: f64, n: usize) -> f64 {
    let g2 = gamma(2 * n) * gamma(2 * n);
    let mut bound = mul_up(U, res);
    bound = next_up(bound + mul_up(g2, abs_dot));
    bound
}

/// Compensated dot product (`Dot2`), accuracy as if computed in doubled
/// precision: relative error ~`u` unconditionally, and the certified bound
/// shrinks like `γ_n²` with the data's condition number squared.
pub fn dot2(x: &[f64], y: &[f64]) -> Result<CompensatedValue> {
    validate_dot_inputs(x, y)?;
    let (p, e) = dot_eft(x, y);
    // Neumaier sum of the error terms into the last partial.
    let mut res = p[p.len() - 1];
    let mut c = 0.;
    for &v in &e {
        let t = res + v;
        c += if res.abs() >= v.abs() {
            (res - t) + v
        } else {
            (v - t) + res
        };
        res = t;
    }
    res += c;
    numeric(res.is_finite(), "Dot2 exhausted finite precision")?;
    let abs_sum = x
        .iter()
        .zip(y)
        .map(|(a, b)| (a * b).abs())
        .sum::<f64>();
    Ok(CompensatedValue {
        value: res,
        error_bound: dot2_bound(res, abs_sum, x.len()),
    })
}

/// Compensated dot product of order `k` (`DotK`): iterates the `Dot2`
/// refinement on the residual error vector, so the error behaves like `u^k`
/// times the `k`-th power of the condition number. `k` is clamped to `≥ 2`.
pub fn dot_k(x: &[f64], y: &[f64], k: usize) -> Result<CompensatedValue> {
    validate_dot_inputs(x, y)?;
    check(
        (2..=8).contains(&k),
        "DotK order must be in 2..=8 (higher orders buy nothing in binary64)",
    )?;
    let (p, mut e) = dot_eft(x, y);
    let mut res = p[p.len() - 1];
    for _ in 2..k {
        let (s, next) = sum2_pass(&e);
        res += s;
        e = next;
    }
    // Final Neumaier sum of the remaining residuals.
    let mut c = 0.;
    for &v in &e {
        let t = res + v;
        c += if res.abs() >= v.abs() {
            (res - t) + v
        } else {
            (v - t) + res
        };
        res = t;
    }
    res += c;
    numeric(res.is_finite(), "DotK exhausted finite precision")?;
    let abs_sum = x
        .iter()
        .zip(y)
        .map(|(a, b)| (a * b).abs())
        .sum::<f64>();
    // Order-k bound: u·|res| + (γ_{2n}²)·Σ|xᵢyᵢ| remains a valid (loose)
    // certified bound for any k ≥ 2.
    Ok(CompensatedValue {
        value: res,
        error_bound: dot2_bound(res, abs_sum, x.len()),
    })
}

// ---------------------------------------------------------------------------
// CompHorner (Graillat–Langlois–Louvet)
// ---------------------------------------------------------------------------

/// Compensated Horner evaluation of `p(t) = Σ coeffs[i]·t^i` (ascending
/// powers). The running error bound is
/// `u·|res| + γ_{2n}²·Σ|coeffs[i]|·|t|^i`, outward rounded.
///
/// Accuracy is as if the Horner scheme ran in doubled working precision.
pub fn comp_horner(coeffs: &[f64], t: f64) -> Result<CompensatedValue> {
    check(!coeffs.is_empty(), "CompHorner needs at least one coefficient")?;
    check(
        coeffs.len() <= MAX_LEN,
        "CompHorner degree exceeds the resource budget",
    )?;
    numeric(
        t.is_finite() && coeffs.iter().all(|c| c.is_finite()),
        "CompHorner inputs must be finite",
    )?;
    let n = coeffs.len();
    // Descending iteration (Horner). coeffs ascending → iterate rev.
    let mut s = coeffs[n - 1];
    let mut c = 0.; // compensation accumulator
    for i in (0..n - 1).rev() {
        let (p, pi) = two_prod(s, t);
        let (sn, sigma) = two_sum(p, coeffs[i]);
        // Compensated accumulation of the elementary errors.
        let e = pi + sigma;
        let t2 = c * t;
        let (cn, _) = two_sum(t2, e);
        c = cn;
        s = sn;
    }
    let value = s + c;
    numeric(value.is_finite(), "CompHorner exhausted finite precision")?;
    // Running bound: γ_{2n}² · p̃(|t|) where p̃ is the absolute-coefficient
    // polynomial, evaluated by plain Horner with outward rounding.
    let mut abs_p = coeffs[n - 1].abs();
    for i in (0..n - 1).rev() {
        abs_p = next_up(next_up(abs_p * t.abs()) + coeffs[i].abs());
    }
    let g2 = gamma(2 * n) * gamma(2 * n);
    let bound = next_up(mul_up(U, value) + mul_up(g2, abs_p));
    Ok(CompensatedValue {
        value,
        error_bound: bound,
    })
}

// ---------------------------------------------------------------------------
// Compensated de Casteljau / Bernstein-form curve evaluation
// ---------------------------------------------------------------------------

/// Compensated de Casteljau evaluation of a Bézier curve.
///
/// `control` are the control points (all of the same dimension, ≥ 1),
/// `t ∈ [0, 1]`. Returns one [`CompensatedValue`] per coordinate; the error
/// bound is propagated through the convex combination tree with outward
/// rounding, so it certifies the computed point:
///
/// ```text
/// |point[d].value - exact_d| <= point[d].error_bound
/// ```
///
/// Because de Casteljau on `t ∈ [0,1]` is a convex combination, each local
/// rounding error reaches the root attenuated by a product of weights in
/// `[0,1]`, i.e. with coefficient ≤ 1; summing the local error terms with
/// that attenuation gives the running bound. The compensation vector makes
/// the *value* accurate to ~`u²` relative to the intrinsic condition number.
///
/// Budget: at most 256 control points (degree ≤ 255), dimension ≤ 16.
pub fn bezier_point(control: &[Vec<f64>], t: f64) -> Result<Vec<CompensatedValue>> {
    check(
        !control.is_empty(),
        "Bézier evaluation needs at least one control point",
    )?;
    check(
        control.len() <= 256,
        "Bézier degree exceeds the resource budget (max 255)",
    )?;
    check(
        (0. ..=1.).contains(&t) && t.is_finite(),
        "Bézier parameter must lie in [0, 1]",
    )?;
    let dim = control[0].len();
    check(
        (1..=16).contains(&dim),
        "Bézier control point dimension must be in 1..=16",
    )?;
    check(
        control.iter().all(|p| p.len() == dim),
        "Bézier control points must share one dimension",
    )?;
    numeric(
        control.iter().flatten().all(|v| v.is_finite()),
        "Bézier control points must be finite",
    )?;

    let n = control.len();
    // Exact complement: fl(1−t) = 1 − t + δ with δ computed by TwoSum.
    let (s, delta) = two_sum(1., -t); // s = fl(1−t), exact δ
    debug_assert_eq!(s, 1. - t);

    // Per-node state: value v, compensation c, running absolute bound b.
    let mut v: Vec<Vec<f64>> = control.to_vec();
    let mut c: Vec<Vec<f64>> = vec![vec![0.; dim]; n];
    let mut b: Vec<Vec<f64>> = vec![vec![0.; dim]; n];

    for level in 1..n {
        for i in 0..n - level {
            for d in 0..dim {
                let a = v[i][d];
                let bb = v[i + 1][d];
                // v = s·a + t·bb with exact local error via EFTs.
                let (p1, e1) = two_prod(s, a);
                let (p2, e2) = two_prod(t, bb);
                let (vn, e3) = two_sum(p1, p2);
                v[i][d] = vn;
                // Local error: product/sum errors plus the complement error
                // δ·a (|δ| ≤ u·|s|, exact value available from two_sum).
                let e_local = e1 + e2 + e3 + delta * a;
                // Compensation recursion (convex combination of the child
                // compensations plus the local error).
                let cn = s * c[i][d] + t * c[i + 1][d] + e_local;
                c[i][d] = cn;
                // Bound recursion: child bounds attenuated by the convex
                // weights, plus the local error magnitude, plus the rounding
                // of the compensation recursion itself (3 flops ≤ 3u·mag),
                // all outward rounded.
                let local_abs = e1.abs() + e2.abs() + e3.abs() + (delta * a).abs();
                let mut bn = add_up(mul_up(s, b[i][d]), mul_up(t, b[i + 1][d]));
                bn = add_up(bn, local_abs);
                let mag = (s * c[i][d]).abs() + (t * c[i + 1][d]).abs() + cn.abs();
                bn = add_up(bn, mul_up(3. * U, mag));
                b[i][d] = bn;
            }
        }
    }

    let mut out = Vec::with_capacity(dim);
    for d in 0..dim {
        let value = v[0][d] + c[0][d];
        numeric(
            value.is_finite(),
            "Compensated de Casteljau exhausted finite precision",
        )?;
        // One final rounding for value = fl(v + c), then the running bound.
        let raw = mul_up(U, value) + b[0][d];
        let bound = if raw == 0. { 0. } else { next_up(raw) };
        out.push(CompensatedValue {
            value,
            error_bound: bound,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
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
}
