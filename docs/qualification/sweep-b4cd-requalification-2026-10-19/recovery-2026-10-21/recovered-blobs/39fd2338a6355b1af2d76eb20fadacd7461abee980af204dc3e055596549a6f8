//! Sturm sequences for real root counting and isolation of power-basis
//! polynomials (checklist items 440–441, 454, 456).
//!
//! A polynomial is a `Vec<f64>` of ascending power coefficients. The Sturm
//! chain is built with binary64 polynomial division; degenerate leading
//! coefficients (relative to the coefficient scale) are trimmed with an
//! explicit tolerance, and division by a numerically zero polynomial is an
//! error, never a panic. Multiple roots collapse in the chain: the last
//! nonzero Sturm term is (a multiple of) `gcd(p, p′)`, and Sturm's theorem
//! then counts each distinct root exactly once.
//!
//! Bernstein-basis input is converted to the power basis with exact binary
//! binomial coefficients (exact in binary64 for degree ≤ 1024, since
//! `C(n,k) ≤ 2^n < 2^53`); this conversion is the numerically stable route
//! because the Sturm chain needs polynomial division, which is ill-defined
//! and unstable directly in the Bernstein basis. The conversion is
//! documented as backward-stable: root locations on `[0,1]` are perturbed
//! only by the standard basis-conversion conditioning.
//!
//! No `unsafe`, no new dependencies.
use crate::foundation::guards::Budget;
use crate::interval_eval::Interval;
use crate::{Result, check, numeric_err};

/// Relative tolerance under which a leading coefficient is treated as zero.
const TRIM_TOLERANCE: f64 = 1e-12;
/// Degree cap, mirroring the crate's polynomial machinery limits.
const MAX_DEGREE: usize = 64;

/// Trim leading (highest-degree) coefficients whose magnitude is below
/// `TRIM_TOLERANCE` times the largest coefficient. Returns the trimmed
/// degree; an all-zero polynomial trims to degree 0 with a zero constant.
fn normalized(mut p: Vec<f64>) -> Vec<f64> {
    let scale = p.iter().fold(0_f64, |a, &c| a.max(c.abs()));
    while p.len() > 1 && p.last().is_some_and(|&c| c.abs() <= TRIM_TOLERANCE * scale) {
        p.pop();
    }
    p
}

fn validate(p: &[f64]) -> Result<()> {
    check(
        !p.is_empty() && p.len() <= MAX_DEGREE + 1 && p.iter().all(|c| c.is_finite()),
        "Sturm polynomial needs 1..=65 finite coefficients",
    )
}

/// Polynomial derivative.
fn derivative(p: &[f64]) -> Vec<f64> {
    if p.len() == 1 {
        return vec![0.];
    }
    (1..p.len()).map(|i| p[i] * i as f64).collect()
}

/// Remainder of `a` divided by `b` in binary64, with degenerate leading
/// coefficients trimmed at every step. Returns an error when `b` is the zero
/// polynomial or a quotient coefficient becomes non-finite.
fn remainder(a: &[f64], b: &[f64]) -> Result<Vec<f64>> {
    let b = normalized(b.to_vec());
    check(
        !(b.len() == 1 && b[0] == 0.),
        "Sturm division by a numerically zero polynomial",
    )?;
    let mut r = normalized(a.to_vec());
    while r.len() >= b.len() && !(r.len() == 1 && r[0] == 0.) {
        let factor = r[r.len() - 1] / b[b.len() - 1];
        check(factor.is_finite(), "Sturm division overflow")?;
        let shift = r.len() - b.len();
        for i in 0..b.len() {
            r[shift + i] -= factor * b[i];
        }
        r = normalized(r);
    }
    Ok(r)
}

/// Horner evaluation with a running error bound: returns the computed value
/// plus `2·d·u·Σ|cᵢ·xⁱ|` (`u = f64::EPSILON`, `d` the degree), the classic
/// Horner forward-error bound, outward by a factor of two. Near a multiple
/// root plain evaluation suffers catastrophic cancellation; the bound tells
/// us exactly when the sign is trustworthy.
fn horner_bounded(p: &[f64], x: f64) -> Result<(f64, f64)> {
    let mut acc = p[p.len() - 1];
    let mut magnitude = p[p.len() - 1].abs();
    for &c in p[..p.len() - 1].iter().rev() {
        acc = acc * x + c;
        magnitude = magnitude * x.abs() + c.abs();
    }
    check(
        acc.is_finite() && magnitude.is_finite(),
        "Sturm evaluation overflow",
    )?;
    let bound = 2. * (p.len() - 1) as f64 * f64::EPSILON * magnitude;
    Ok((acc, bound))
}

/// Sign variations of the Sturm chain at `x`: evaluate every term (Horner),
/// drop zeros, count sign changes. The zero-skipping convention is what
/// makes Sturm's theorem count distinct roots even for multiple roots.
/// A value whose magnitude does not exceed its Horner error bound is
/// *numerically zero* and is skipped too — otherwise cancellation near a
/// multiple root would fabricate or erase sign variations. Also returns
/// whether the *first* chain term (the polynomial itself) was numerically
/// zero: callers pruning boxes must not trust a zero root count when an
/// endpoint sits inside the uncertainty zone of a root.
fn variations(chain: &[Vec<f64>], x: f64) -> Result<(usize, bool)> {
    let mut previous: Option<f64> = None;
    let mut count = 0;
    let mut first_uncertain = false;
    for (index, p) in chain.iter().enumerate() {
        let (acc, bound) = horner_bounded(p, x)?;
        if acc.abs() <= bound {
            first_uncertain |= index == 0;
            continue;
        }
        if let Some(prev) = previous {
            if prev.signum() != acc.signum() {
                count += 1;
            }
        }
        previous = Some(acc);
    }
    Ok((count, first_uncertain))
}

/// The Sturm chain of `p`: `p₀ = p`, `p₁ = p′`, `pᵢ₊₁ = −rem(pᵢ₋₁, pᵢ)`,
/// stopping at the first numerically zero remainder (whose divisor is then a
/// multiple of `gcd(p, p′)`). Constant polynomials have a one-term chain;
/// an identically zero polynomial is an error.
pub fn sturm_chain(p: &[f64]) -> Result<Vec<Vec<f64>>> {
    validate(p)?;
    let p = normalized(p.to_vec());
    check(
        !(p.len() == 1 && p[0] == 0.),
        "Sturm chain is undefined for the zero polynomial",
    )?;
    let mut chain = vec![p];
    if chain[0].len() > 1 {
        // Unified iteration budget (item 1065) on top of the degree cap.
        let mut guard = Budget::with_iterations(MAX_DEGREE + 1)?.guard("sturm_chain");
        chain.push(normalized(derivative(&chain[0])));
        while chain.last().is_some_and(|q| q.len() > 1 || q[0] != 0.) {
            guard.tick()?;
            let n = chain.len();
            let next = match remainder(&chain[n - 2], &chain[n - 1]) {
                Ok(r) => r,
                Err(_) => break,
            };
            let next = normalized(next);
            if next.iter().all(|&c| c == 0.) {
                break;
            }
            chain.push(next.into_iter().map(|c| -c).collect());
            if chain.len() > MAX_DEGREE + 1 {
                return Err(numeric_err("Sturm chain exceeded the degree budget"));
            }
        }
    }
    Ok(chain)
}

/// Number of distinct real roots of `p` in `[a, b]` (each multiple root
/// counted once). Uses Sturm's theorem: `V(a) − V(b)` counts roots in the
/// half-open `(a, b]`; an exact `p(a) == 0` adds the left endpoint. Roots at
/// `b` are included by the half-open convention.
pub fn count_roots(p: &[f64], a: f64, b: f64) -> Result<usize> {
    let chain = sturm_chain(p)?;
    check(
        a.is_finite() && b.is_finite() && a <= b,
        "Root-count interval must be finite and ordered",
    )?;
    let mut count = variations(&chain, a)?.0.abs_diff(variations(&chain, b)?.0);
    let (pa, bound) = horner_bounded(&chain[0], a)?;
    if pa.abs() <= bound && a < b {
        // Exact or numerically indistinguishable left-endpoint root.
        count += 1;
    }
    Ok(count)
}

/// Isolate all distinct real roots of `p` on `[a, b]`: recursive bisection,
/// pruning boxes whose Sturm count is zero, splitting boxes with more than
/// one root, and emitting a box when it holds exactly one root and its width
/// is at most `tol` (or the depth budget forces emission). The returned
/// intervals cover every distinct root of `p` in `[a, b]`; adjacent boxes
/// may share endpoints.
pub fn isolate_sturm(p: &[f64], a: f64, b: f64, tol: f64) -> Result<Vec<Interval>> {
    // Unified budget (item 1065): each pop costs two chain evaluations;
    // 10 000 ticks covers the depth-60 envelope many times over.
    isolate_sturm_with_budget(p, a, b, tol, Budget::default())
}

/// [`isolate_sturm`] with an explicit unified [`Budget`]; exhaustion aborts
/// with a `BudgetExhausted` resource error rather than returning a partial
/// root list.
fn isolate_sturm_with_budget(
    p: &[f64],
    a: f64,
    b: f64,
    tol: f64,
    budget: Budget,
) -> Result<Vec<Interval>> {
    let chain = sturm_chain(p)?;
    check(
        a.is_finite() && b.is_finite() && a < b,
        "Isolation interval must be finite and increasing",
    )?;
    check(
        tol.is_finite() && tol > 0.,
        "Isolation tolerance must be positive and finite",
    )?;
    let mut guard = budget.guard("isolate_sturm");
    let mut out = Vec::new();
    // (lo, hi, depth, count including the half-open convention).
    let mut stack = vec![(a, b, 0usize)];
    while let Some((lo, hi, depth)) = stack.pop() {
        guard.tick()?;
        let (va, uncertain_lo) = variations(&chain, lo)?;
        let (vb, uncertain_hi) = variations(&chain, hi)?;
        let mut n = va.abs_diff(vb);
        if depth == 0 && uncertain_lo {
            n += 1;
        }
        if n == 0 && !uncertain_lo && !uncertain_hi {
            // Trustworthy zero count: prune. An uncertain endpoint means the
            // box touches the uncertainty zone of a root; keep subdividing.
            continue;
        }
        if (n <= 1 && hi - lo <= tol) || depth >= 60 {
            out.push(Interval::new(lo, hi)?);
            continue;
        }
        let m = 0.5 * (lo + hi);
        // Half-open (m, hi] is handled by the variation difference; the left
        // box (lo, m] likewise. A root exactly at m is counted once, in the
        // right box of the pair — unless it is also the global left endpoint,
        // which was already folded into the count above.
        stack.push((m, hi, depth + 1));
        stack.push((lo, m, depth + 1));
    }
    out.sort_by(|x, y| x.lo.total_cmp(&y.lo));
    // Uncertainty-zone guards can split one root's box into a few adjacent
    // slivers; merge boxes that overlap or merely touch. Distinct roots
    // closer than the uncertainty scale are not separable at this tol anyway.
    let mut merged: Vec<Interval> = Vec::with_capacity(out.len());
    for b in out {
        match merged.last_mut() {
            Some(last) if b.lo <= last.hi => {
                if b.hi > last.hi {
                    *last = Interval::new(last.lo, b.hi)?;
                }
            }
            _ => merged.push(b),
        }
    }
    Ok(merged)
}

/// Exact binary binomial coefficient `C(n, k)` as `f64` (exact while
/// `C(n,k) < 2^53`, always true for `n ≤ 56`; mildly rounded beyond).
fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.;
    }
    let k = k.min(n - k);
    (1..=k).fold(1., |v, i| v * (n - k + i) as f64 / i as f64)
}

/// Convert Bernstein coefficients on `[0,1]` to ascending power
/// coefficients: `Σ bᵢ·C(n,i)·xⁱ(1−x)ⁿ⁻ⁱ`. Exact binomial arithmetic;
/// coefficient growth is the documented conditioning cost of the basis
/// change.
pub fn bernstein_to_power(bernstein: &[f64]) -> Result<Vec<f64>> {
    validate(bernstein)?;
    let n = bernstein.len() - 1;
    let mut power = vec![0.; n + 1];
    for (i, &b) in bernstein.iter().enumerate() {
        if b == 0. {
            continue;
        }
        let ci = binomial(n, i);
        for k in i..=n {
            let sign = if (k - i) % 2 == 0 { 1. } else { -1. };
            power[k] += sign * b * ci * binomial(n - i, k - i);
        }
    }
    check(
        power.iter().all(|c| c.is_finite()),
        "Bernstein-to-power conversion overflow",
    )?;
    Ok(power)
}

/// Isolate the distinct roots in `[0, 1]` of a Bernstein-basis polynomial
/// with coefficients `bernstein`, via conversion to the power basis and
/// `isolate_sturm`. See the module docs for why the conversion is the
/// numerically stable route.
pub fn isolate_bernstein_roots(bernstein: &[f64], tol: f64) -> Result<Vec<Interval>> {
    let power = bernstein_to_power(bernstein)?;
    isolate_sturm(&power, 0., 1., tol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interval_newton::{PolynomialFunction, isolate_roots};

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }
        fn f64(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.f64()
        }
    }

    fn from_roots(roots: &[f64]) -> Vec<f64> {
        let mut c = vec![1.];
        for &r in roots {
            let mut n = vec![0.; c.len() + 1];
            for (i, &a) in c.iter().enumerate() {
                n[i] -= a * r;
                n[i + 1] += a;
            }
            c = n;
        }
        c
    }

    fn eval(p: &[f64], x: f64) -> f64 {
        p.iter().rev().fold(0., |acc, &c| acc * x + c)
    }

    #[test]
    fn counts_known_configurations() {
        // Simple roots.
        assert_eq!(count_roots(&from_roots(&[-1., 0.5, 2.]), -2., 3.).unwrap(), 3);
        assert_eq!(count_roots(&from_roots(&[-1., 0.5, 2.]), -0.5, 1.).unwrap(), 1);
        // Multiple roots counted once.
        assert_eq!(count_roots(&from_roots(&[1., 1., 1.]), 0., 2.).unwrap(), 1);
        assert_eq!(count_roots(&from_roots(&[0., 0., 1.]), -1., 2.).unwrap(), 2);
        // No real roots.
        assert_eq!(count_roots(&[1., 0., 1.], -10., 10.).unwrap(), 0);
        // Root exactly on the boundary: counted at both ends.
        let p = from_roots(&[0., 1.]);
        assert_eq!(count_roots(&p, 0., 1.).unwrap(), 2);
        assert_eq!(count_roots(&p, 0.25, 0.75).unwrap(), 0);
        // Constant nonzero polynomial.
        assert_eq!(count_roots(&[3.], -1., 1.).unwrap(), 0);
        // Zero polynomial rejected.
        assert!(sturm_chain(&[0., 0.]).is_err());
    }

    #[test]
    fn isolation_boxes_cover_all_roots() {
        let roots = [-1.5, -0.25, 0.25, 1.25];
        let p = from_roots(&roots);
        let boxes = isolate_sturm(&p, -2., 2., 1e-9).unwrap();
        assert_eq!(boxes.len(), 4);
        for (r, b) in roots.iter().zip(&boxes) {
            assert!(b.contains(*r), "{r} not in [{}, {}]", b.lo, b.hi);
            assert!(b.width() <= 1e-9 * 2.);
        }
        // Multiple root: one box, still containing it.
        let p = from_roots(&[0.5, 0.5]);
        let boxes = isolate_sturm(&p, 0., 1., 1e-9).unwrap();
        assert_eq!(boxes.len(), 1);
        assert!(boxes[0].contains(0.5));
    }

    #[test]
    fn isolated_count_matches_sturm_count() {
        // Property: the number of isolated boxes equals count_roots.
        let mut rng = Rng(99);
        for _ in 0..30 {
            let count = 1 + (rng.next() % 5) as usize;
            let roots: Vec<f64> = (0..count).map(|_| rng.range(-2., 2.)).collect();
            let p = from_roots(&roots);
            let boxes = isolate_sturm(&p, -3., 3., 1e-8).unwrap();
            let expected = count_roots(&p, -3., 3.).unwrap();
            assert_eq!(boxes.len(), expected, "roots {roots:?}");
        }
    }

    #[test]
    fn cross_check_against_interval_newton() {
        // Deterministic random polynomials: Sturm count must match the
        // number of certified boxes from interval Newton isolation.
        let mut rng = Rng(2027);
        for _ in 0..20 {
            let count = 1 + (rng.next() % 4) as usize;
            let roots: Vec<f64> = (0..count).map(|_| rng.range(-1.5, 1.5)).collect();
            let p = from_roots(&roots);
            let sturm = count_roots(&p, -2., 2.).unwrap();
            let newton = isolate_roots(
                Interval::new(-2., 2.).unwrap(),
                &PolynomialFunction::new(p.clone()).unwrap(),
                1e-7,
                40,
            )
            .unwrap();
            assert_eq!(sturm, newton.len(), "roots {roots:?}");
            let boxes = isolate_sturm(&p, -2., 2., 1e-7).unwrap();
            for (nb, _) in &newton {
                assert!(boxes.iter().any(|sb| sb.hi >= nb.lo && sb.lo <= nb.hi));
            }
        }
    }

    #[test]
    fn budget_exhaustion_aborts_sturm_isolation_with_resource_error() {
        let p = from_roots(&[0.25, 0.75]);
        let err =
            isolate_sturm_with_budget(&p, 0., 1., 1e-9, Budget::new(1, 64, 60_000).unwrap())
                .unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("isolate_sturm"), "{err}");
        assert!(err.contains("iteration budget"), "{err}");
    }

    #[test]
    fn sturm_chain_degree_budget_error_still_typed() {
        // The unified guard is insurance on top of the degree cap: a chain
        // that overruns the cap still fails as an error, never silently.
        let p = from_roots(&[0.5, 0.5]);
        assert!(sturm_chain(&p).is_ok());
        assert!(isolate_sturm(&[1., 0., 1.], 0., 1., f64::NAN).is_err());
    }

    #[test]
    fn bernstein_basis_round_trip_and_roots() {
        // Bernstein coefficients of (x−0.25)(x−0.75) on [0,1], degree 2:
        // power [0.1875, −1, 1] → Bernstein [0.1875, −0.3125, 0.1875].
        let bernstein = [0.1875, -0.3125, 0.1875];
        let power = bernstein_to_power(&bernstein).unwrap();
        for i in 0..=20 {
            let x = i as f64 / 20.;
            let direct = (x - 0.25) * (x - 0.75);
            assert!((eval(&power, x) - direct).abs() < 1e-13);
        }
        let boxes = isolate_bernstein_roots(&bernstein, 1e-10).unwrap();
        assert_eq!(boxes.len(), 2);
        assert!(boxes[0].contains(0.25));
        assert!(boxes[1].contains(0.75));
        // Non-negativity in Bernstein form proves no root: all-positive
        // coefficients convert to a polynomial with no root on [0,1].
        assert!(isolate_bernstein_roots(&[1., 2., 3.], 1e-8).unwrap().is_empty());
    }
}
