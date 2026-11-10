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
    /// Construct a certified value with its outward-rounded error bound.
    #[inline]
    pub const fn new(value: f64, error_bound: f64) -> Self {
        Self { value, error_bound }
    }

    /// Construct an exact value (`error_bound = 0.0`).
    #[inline]
    pub const fn exact(value: f64) -> Self {
        Self {
            value,
            error_bound: 0.,
        }
    }
}

/// Next representable float towards `+∞` (independent of `math_core`, so this
/// module stays self-contained).
#[inline]
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
#[inline]
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bp = s - a;
    let ap = s - bp;
    let e = (a - ap) + (b - bp);
    (s, e)
}

/// Dekker's `FastTwoSum`: same result as [`two_sum`] in three flops, but only
/// valid when `|a| >= |b|`.
#[inline]
pub fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    debug_assert!(a.abs() >= b.abs());
    let s = a + b;
    let e = (a - s) + b;
    (s, e)
}

/// Veltkamp–Dekker split of `a` into a high part with at most 26 significant
/// bits and a non-overlapping low part: `a = hi + lo`.
#[inline]
pub fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let hi = c - (c - a);
    let lo = a - hi;
    (hi, lo)
}

/// Dekker's `TwoProd` without FMA: exact `a * b = p + e`, 17 flops.
#[inline]
pub fn two_prod_split(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let e = ((ah * bh - p) + ah * bl + al * bh) + al * bl;
    (p, e)
}

/// `TwoProd` with FMA: exact `a * b = p + e`, 2 flops.
#[inline]
pub fn two_prod_fma(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let e = a.mul_add(b, -p);
    (p, e)
}

/// Portable `TwoProd`: uses FMA when the target supports it, Dekker otherwise.
/// `std::f64::mul_add` lowers to a hardware FMA on all tier-1 targets.
#[inline]
pub fn two_prod(a: f64, b: f64) -> (f64, f64) {
    two_prod_fma(a, b)
}

/// Outward-rounded magnitude product; zero stays zero (bounds stay tight).
#[inline]
fn mul_up(a: f64, b: f64) -> f64 {
    let p = a.abs() * b.abs();
    if p == 0. { 0. } else { next_up(p) }
}

/// Outward-rounded sum of non-negative magnitudes; zero stays zero.
#[inline]
fn add_up(a: f64, b: f64) -> f64 {
    let s = a + b;
    if s == 0. { 0. } else { next_up(s) }
}

/// `γ_k = k·u / (1 - k·u)`, outward rounded (overflow-safe for our budgets).
#[inline]
fn gamma(k: usize) -> f64 {
    let ku = k as f64 * U;
    next_up(ku / (1. - ku))
}

/// Compensated 3D dot product `a · b` in stack-only Ogita–Rump–Oishi EFT
/// (`TwoProd` + `TwoSum`), delivering doubled-precision accuracy without
/// heap allocation.
#[inline]
pub fn compensated_dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (p0, e0) = two_prod(a[0], b[0]);
    let (p1, e1) = two_prod(a[1], b[1]);
    let (s1, sigma1) = two_sum(p0, p1);
    let (p2, e2) = two_prod(a[2], b[2]);
    let (s2, sigma2) = two_sum(s1, p2);
    s2 + ((e0 + e1) + (sigma1 + e2) + sigma2)
}

/// Compensated 2x2 determinant `u0 * v1 - u1 * v0` via `TwoProd` and `TwoSum`.
#[inline]
fn compensated_det2(u0: f64, u1: f64, v0: f64, v1: f64) -> f64 {
    let (p0, e0) = two_prod(u0, v1);
    let (p1, e1) = two_prod(-u1, v0);
    let (s, sigma) = two_sum(p0, p1);
    s + (e0 + e1 + sigma)
}

/// Compensated 3D cross product `a × b` using Ogita–Rump–Oishi EFT
/// (`TwoProd` + `TwoSum`) on each 2×2 minor, immune to catastrophic
/// cancellation on nearly parallel vectors.
#[inline]
pub fn compensated_cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        compensated_det2(a[1], a[2], b[1], b[2]),
        compensated_det2(a[2], a[0], b[2], b[0]),
        compensated_det2(a[0], a[1], b[0], b[1]),
    ]
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
#[path = "compensated/tests/mod.rs"]
mod tests;
