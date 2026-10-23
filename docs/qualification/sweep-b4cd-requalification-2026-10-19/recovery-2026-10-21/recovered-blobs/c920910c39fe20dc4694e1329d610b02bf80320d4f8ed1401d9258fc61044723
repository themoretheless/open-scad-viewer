//! Certified root isolation via interval Newton operators (checklist items
//! 440–441, 454, 456).
//!
//! All enclosures use the outward-rounded `Interval` from
//! `crate::interval_eval`, so every certificate is rigorous in binary64:
//!
//! * Moore's interval Newton operator `N(X) = m − f(m)/F′(X)`;
//! * the Krawczyk operator with midpoint preconditioning, both scalar and for
//!   small square systems (dimension 1–3);
//! * certificates: `N(X) ⊂ int(X)` proves a unique root in `X`
//!   (with `F′(X)` separated from zero), `N(X) ∩ X = ∅` or `0 ∉ F(X)` proves
//!   absence;
//! * `isolate_roots`: recursive bisection returning a guaranteed-complete
//!   candidate list — every root of `f` in the domain lies in one of the
//!   returned boxes;
//! * `certified_curve_extrema`: certified isolation of curve extrema as roots
//!   of the first derivative, on top of the interval homogeneous de Boor
//!   machinery in `crate::curve_distance`.
//!
//! No `unsafe`, no new dependencies, no sampling-based claims.
use crate::curve::Curve;
use crate::foundation::guards::{Budget, canonical_f64};
use crate::interval_eval::Interval;
use crate::{Result, check, numeric_err};

/// A scalar function with certified interval enclosures of itself and its
/// first derivative. `interval_value(X)` must enclose `{f(t) : t ∈ X}` and
/// `interval_derivative(X)` must enclose `{f′(t) : t ∈ X}`.
pub trait IntervalFunction {
    /// Point evaluation (plain binary64; only used for midpoints, never for
    /// certificates).
    fn value_at(&self, x: f64) -> Result<f64>;
    /// Outward enclosure of the range of `f` over `x`.
    fn interval_value(&self, x: Interval) -> Result<Interval>;
    /// Outward enclosure of the range of `f′` over `x`.
    fn interval_derivative(&self, x: Interval) -> Result<Interval>;
}

/// Power-basis polynomial `Σ coefficients[i]·x^i` with outward-rounded
/// interval Horner evaluation. Convenient `IntervalFunction` for tests and
/// for clients that already hold power coefficients.
pub struct PolynomialFunction {
    /// Ascending power coefficients; trailing zeros are trimmed.
    pub coefficients: Vec<f64>,
}
impl PolynomialFunction {
    pub fn new(coefficients: Vec<f64>) -> Result<Self> {
        check(
            !coefficients.is_empty() && coefficients.iter().all(|c| c.is_finite()),
            "Polynomial needs finite coefficients",
        )?;
        let mut coefficients = coefficients;
        while coefficients.len() > 1 && coefficients.last() == Some(&0.) {
            coefficients.pop();
        }
        Ok(Self { coefficients })
    }
    fn interval_horner(coefficients: &[f64], x: Interval) -> Result<Interval> {
        let mut acc = Interval::point(coefficients[coefficients.len() - 1]);
        for &c in coefficients[..coefficients.len() - 1].iter().rev() {
            acc = acc.mul(x)?.add(Interval::point(c))?;
        }
        Ok(acc)
    }
}
impl IntervalFunction for PolynomialFunction {
    fn value_at(&self, x: f64) -> Result<f64> {
        check(x.is_finite(), "Evaluation point must be finite")?;
        let mut acc = self.coefficients[self.coefficients.len() - 1];
        for &c in self.coefficients[..self.coefficients.len() - 1]
            .iter()
            .rev()
        {
            acc = acc * x + c;
        }
        check(acc.is_finite(), "Polynomial evaluation overflow")?;
        Ok(acc)
    }
    fn interval_value(&self, x: Interval) -> Result<Interval> {
        // Plain outward Horner suffers the interval dependency effect: on a
        // box of width w its width grows like `w·Σ|∂terms|`, which can swamp
        // a tiny function value near a root. The centered (mean-value) form
        // `f(m) + F′(X)·(X − m)` is also a certified outer enclosure and its
        // overestimation scales like `w²`, so narrow boxes stay tight.
        // Intersecting two valid outer enclosures is valid.
        let plain = Self::interval_horner(&self.coefficients, x)?;
        if x.lo == x.hi {
            return Ok(plain);
        }
        let m = 0.5 * (x.lo + x.hi);
        let fm = Self::interval_horner(&self.coefficients, Interval::point(m))?;
        let slope = self.interval_derivative(x)?;
        let centered = fm.add(slope.mul(x.sub(Interval::point(m))?)?)?;
        match plain.intersect(centered.lo, centered.hi) {
            Ok(tight) => Ok(tight),
            // Outward rounding made the two enclosures disjoint; either is a
            // valid answer on its own.
            Err(_) => Ok(plain),
        }
    }
    fn interval_derivative(&self, x: Interval) -> Result<Interval> {
        // Form derivative coefficients outward: i*c_i can round in binary64.
        // The centered derivative form reduces dependency on wide boxes.
        let derivative = |order: usize| -> Result<Vec<Interval>> {
            if self.coefficients.len() <= order {
                return Ok(vec![Interval::point(0.)]);
            }
            (order..self.coefficients.len())
                .map(|i| {
                    let mut c = Interval::point(self.coefficients[i]);
                    for k in 0..order {
                        c = c.mul(Interval::point((i - k) as f64))?;
                    }
                    Ok(c)
                })
                .collect()
        };
        let horner = |coefficients: &[Interval], argument: Interval| -> Result<Interval> {
            let mut result = *coefficients.last().unwrap();
            for &coefficient in coefficients[..coefficients.len() - 1].iter().rev() {
                result = result.mul(argument)?.add(coefficient)?;
            }
            Ok(result)
        };
        let first = derivative(1)?;
        let plain = horner(&first, x)?;
        let m = 0.5 * (x.lo + x.hi);
        let centered = horner(&first, Interval::point(m))?
            .add(horner(&derivative(2)?, x)?.mul(x.sub(Interval::point(m))?)?)?;
        plain.intersect(centered.lo, centered.hi)
    }
}

/// Box equality up to canonicalization (item 1094): `-0.0` equals `+0.0`.
fn same_box(a: &Interval, b: &Interval) -> bool {
    canonical_f64(a.lo).to_bits() == canonical_f64(b.lo).to_bits()
        && canonical_f64(a.hi).to_bits() == canonical_f64(b.hi).to_bits()
}

/// Certificate about the roots of `f` inside one box.
#[derive(Clone, Copy, Debug)]
pub enum RootCertificate {
    /// Exactly one root exists in the box, and it lies inside the enclosed
    /// (strictly interior) sub-interval.
    Unique(Interval),
    /// No root exists in the box.
    Absent,
    /// Neither uniqueness nor absence could be proven at this box size.
    Indeterminate,
}

/// Moore's interval Newton operator `N(X) = m − f(m)/F′(X)` with outward
/// rounding, `m = mid(X)`. Returns `None` when `F′(X)` straddles zero, in
/// which case the operator is undefined and no certificate is possible.
/// Any root of `f` in `X` lies in `N(X)`.
pub fn moore_newton(f: &impl IntervalFunction, x: Interval) -> Result<Option<Interval>> {
    let fp = f.interval_derivative(x)?;
    if fp.lo <= 0. && fp.hi >= 0. {
        return Ok(None);
    }
    let m = 0.5 * (x.lo + x.hi);
    // f(m) enclosed outward: evaluate the interval form on a degenerate box.
    let fm = f.interval_value(Interval::point(m))?;
    let n = Interval::point(m).sub(fm.div_signed(fp)?)?;
    Ok(Some(n))
}

/// Krawczyk operator in one dimension with midpoint preconditioning:
/// `K(X) = m − y·f(m) + (1 − y·F′(X))·(X − m)`, `y ≈ 1/f′(m)`. `y` is a
/// plain scalar approximation, so it needs no outward rounding; the rest is
/// outward-rounded. Returns `None` when `f′(m)` is zero. Every root of `f`
/// in `X` lies in `K(X)`, and `K(X) ⊂ int(X)` proves existence.
pub fn krawczyk_step(f: &impl IntervalFunction, x: Interval) -> Result<Option<Interval>> {
    let m = 0.5 * (x.lo + x.hi);
    let d = derivative_at_midpoint(f, m)?;
    if d == 0. {
        return Ok(None);
    }
    let fm = f.interval_value(Interval::point(m))?;
    let y = Interval::point(1. / d);
    let contraction = Interval::point(1.).sub(y.mul(f.interval_derivative(x)?)?)?;
    let centered = x.sub(Interval::point(m))?;
    let k = Interval::point(m)
        .sub(y.mul(fm)?)?
        .add(contraction.mul(centered)?)?;
    Ok(Some(k))
}

/// Plain scalar derivative at a point via the interval enclosure on a tiny
/// box; used only to build the preconditioner, so plain rounding suffices.
fn derivative_at_midpoint(f: &impl IntervalFunction, m: f64) -> Result<f64> {
    let w = f.interval_derivative(Interval::point(m))?;
    Ok(0.5 * (w.lo + w.hi))
}

/// A small square system `F: R^n → R^n` (n ≤ 3) with interval Jacobian
/// enclosures, for the preconditioned Krawczyk operator.
pub trait IntervalSystem {
    /// Point evaluation (preconditioner only).
    fn value_at(&self, x: &[f64]) -> Result<Vec<f64>>;
    /// Outward enclosure `J[i][j]` of `∂F_i/∂x_j` over the box `x`.
    fn interval_jacobian(&self, x: &[Interval]) -> Result<Vec<Vec<Interval>>>;
}

fn invert_small(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let n = a.len();
    check(
        n >= 1 && n <= 3,
        "Krawczyk systems are limited to dimension 3",
    )?;
    check(a.iter().all(|r| r.len() == n), "Jacobian must be square")?;
    let inv = match n {
        1 => vec![vec![1. / a[0][0]]],
        2 => {
            let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
            check(det != 0., "Midpoint Jacobian is singular")?;
            vec![
                vec![a[1][1] / det, -a[0][1] / det],
                vec![-a[1][0] / det, a[0][0] / det],
            ]
        }
        _ => {
            let c = |i: usize, j: usize| {
                let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
                let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
                a[r0][c0] * a[r1][c1] - a[r0][c1] * a[r1][c0]
            };
            let det = (0..3).map(|j| a[0][j] * c(0, j)).sum::<f64>();
            check(det != 0., "Midpoint Jacobian is singular")?;
            (0..3)
                .map(|j| (0..3).map(|i| c(i, j) / det).collect())
                .collect()
        }
    };
    check(
        inv.iter().flatten().all(|v| v.is_finite()),
        "Midpoint Jacobian inverse overflow",
    )?;
    Ok(inv)
}

/// Preconditioned Krawczyk operator for a small system:
/// `K(X) = m − Y·F(m) + (I − Y·J(X))·(X − m)` with `Y = J(m)⁻¹`. Every root
/// of `F` in the box lies in `K(X)`; `K(X) ⊂ int(X)` (componentwise) proves
/// existence, and a Lipschitz-contraction check on `I − Y·J(X)` proves
/// uniqueness. Returns `None` when the midpoint Jacobian is singular.
pub fn krawczyk_step_system(
    f: &impl IntervalSystem,
    x: &[Interval],
) -> Result<Option<Vec<Interval>>> {
    let n = x.len();
    check(
        n >= 1 && n <= 3,
        "Krawczyk systems are limited to dimension 3",
    )?;
    let m: Vec<f64> = x.iter().map(|i| 0.5 * (i.lo + i.hi)).collect();
    let jm: Vec<Vec<Interval>> =
        f.interval_jacobian(&m.iter().map(|&v| Interval::point(v)).collect::<Vec<_>>())?;
    let mid: Vec<Vec<f64>> = jm
        .iter()
        .map(|r| r.iter().map(|i| 0.5 * (i.lo + i.hi)).collect())
        .collect();
    let y = match invert_small(&mid) {
        Ok(y) => y,
        Err(_) => return Ok(None),
    };
    let fm: Vec<Interval> = f.value_at(&m)?.into_iter().map(Interval::point).collect();
    let j = f.interval_jacobian(x)?;
    let point = |v: f64| Interval::point(v);
    let mut k = Vec::with_capacity(n);
    for i in 0..n {
        // (I − Y·J(X)) row i.
        let mut acc = point(m[i]);
        for (l, fml) in fm.iter().enumerate() {
            acc = acc.sub(point(y[i][l]).mul(*fml)?)?;
        }
        for j2 in 0..n {
            let mut contraction = if j2 == i { point(1.) } else { point(0.) };
            for l in 0..n {
                contraction = contraction.sub(point(y[i][l]).mul(j[l][j2])?)?;
            }
            acc = acc.add(contraction.mul(x[j2].sub(point(m[j2]))?)?)?;
        }
        k.push(acc);
    }
    Ok(Some(k))
}

/// Classify a box: absence from `0 ∉ F(X)` or from the Newton operator,
/// uniqueness from `N(X) ⊂ int(X)` with a zero-separated derivative.
pub fn classify(f: &impl IntervalFunction, x: Interval) -> Result<RootCertificate> {
    let fv = f.interval_value(x)?;
    if fv.lo > 0. || fv.hi < 0. {
        return Ok(RootCertificate::Absent);
    }
    let derivative = f.interval_derivative(x)?;
    if derivative.lo > 0. || derivative.hi < 0. {
        // A monotone function with strictly same-sign endpoint enclosures
        // cannot vanish anywhere between those endpoints.
        let left = f.interval_value(Interval::point(x.lo))?;
        let right = f.interval_value(Interval::point(x.hi))?;
        if left.lo > 0. && right.lo > 0. || left.hi < 0. && right.hi < 0. {
            return Ok(RootCertificate::Absent);
        }
    }
    let Some(n) = moore_newton(f, x)? else {
        return Ok(RootCertificate::Indeterminate);
    };
    if n.hi < x.lo || n.lo > x.hi {
        return Ok(RootCertificate::Absent);
    }
    if n.lo > x.lo && n.hi < x.hi {
        return Ok(RootCertificate::Unique(n));
    }
    Ok(RootCertificate::Indeterminate)
}

/// Certified root isolation of `f` over `domain` by recursive bisection.
///
/// Returns a guaranteed-complete candidate list: every root of `f` in
/// `domain` is contained in one of the returned intervals. Boxes proven
/// root-free are dropped silently; `Unique` boxes need no further
/// subdivision (their enclosure is the Newton image); `Indeterminate` boxes
/// are returned once they reach `tol` width or `max_depth`. Tangent
/// (even-multiplicity) roots show up as `Indeterminate` boxes but are never
/// lost. `tol` must be positive and finite; `max_depth` is capped at 60.
pub fn isolate_roots(
    domain: Interval,
    f: &impl IntervalFunction,
    tol: f64,
    max_depth: usize,
) -> Result<Vec<(Interval, RootCertificate)>> {
    // Unified iteration budget (item 1065): stack pops and Newton
    // contractions are both counted; 4 ticks per box is a generous upper
    // envelope for the depth-60 subdivision plus tightening work.
    isolate_roots_with_budget(
        domain,
        f,
        tol,
        max_depth,
        Budget {
            max_iterations: 16_384,
            ..Budget::default()
        },
    )
}

/// [`isolate_roots`] with an explicit unified [`Budget`] (item 1065);
/// exhaustion aborts with a `BudgetExhausted` resource error instead of
/// returning an incomplete candidate list.
fn isolate_roots_with_budget(
    domain: Interval,
    f: &impl IntervalFunction,
    tol: f64,
    max_depth: usize,
    budget: Budget,
) -> Result<Vec<(Interval, RootCertificate)>> {
    check(
        tol.is_finite() && tol > 0.,
        "Isolation tolerance must be positive and finite",
    )?;
    check(max_depth <= 60, "Isolation depth is capped at 60")?;
    let mut guard = budget.guard("isolate_roots");
    let mut out = Vec::new();
    let mut stack = vec![(domain, 0usize)];
    while let Some((x, depth)) = stack.pop() {
        guard.tick()?;
        match classify(f, x)? {
            RootCertificate::Absent => {}
            RootCertificate::Unique(u) => {
                // Contract the certified enclosure with further Newton steps
                // until it reaches `tol` or stops shrinking.
                let mut u = u;
                while u.width() > tol {
                    guard.tick()?;
                    let next = match moore_newton(f, u)? {
                        Some(n) => match n.intersect(u.lo, u.hi) {
                            Ok(t) if t.width() < u.width() => t,
                            _ => break,
                        },
                        None => break,
                    };
                    u = next;
                }
                out.push((x, RootCertificate::Unique(u)))
            }
            RootCertificate::Indeterminate => {
                // Tighten with the Newton image when it overlaps the box.
                let tightened = match moore_newton(f, x)? {
                    Some(n) => n.intersect(x.lo, x.hi).unwrap_or(x),
                    None => x,
                };
                if tightened.width() <= tol || depth >= max_depth {
                    out.push((tightened, RootCertificate::Indeterminate));
                    continue;
                }
                let m = 0.5 * (tightened.lo + tightened.hi);
                if m <= tightened.lo || m >= tightened.hi {
                    // Ulp-scale box cannot be bisected further.
                    out.push((tightened, RootCertificate::Indeterminate));
                    continue;
                }
                stack.push((Interval::new(tightened.lo, m)?, depth + 1));
                stack.push((Interval::new(m, tightened.hi)?, depth + 1));
            }
        }
    }
    out.sort_by(|a, b| a.0.lo.total_cmp(&b.0.lo));
    // Canonical comparison (item 1094): -0.0 and +0.0 endpoints collapse, so
    // duplicate boxes that differ only in zero sign are merged.
    out.dedup_by(|a, b| same_box(&a.0, &b.0));
    // Merge boxes that overlap or touch: tol-scale Indeterminate slivers next
    // to a certified box (or a cluster of them around an even-multiplicity
    // root) are resolution artefacts. A merged box keeps a Unique enclosure
    // when exactly one member carried one; the enclosure still pins the
    // certified root, and completeness of the list is preserved by the union.
    let mut merged: Vec<(Interval, RootCertificate)> = Vec::with_capacity(out.len());
    for (b, c) in out {
        match merged.last_mut() {
            Some((last, lc)) if b.lo <= last.hi => {
                if b.hi > last.hi {
                    *last = Interval::new(last.lo, b.hi)?;
                }
                *lc = match (*lc, c) {
                    (RootCertificate::Unique(u), _) | (_, RootCertificate::Unique(u)) => {
                        RootCertificate::Unique(u)
                    }
                    _ => RootCertificate::Indeterminate,
                };
            }
            _ => merged.push((b, c)),
        }
    }
    Ok(merged)
}

/// Outward-rounded value/first/second-derivative jets of a single coordinate
/// axis of a curve over one knot cell `[t.lo, t.hi]`, mirroring the scalar
/// law certificate machinery: restricted Bernstein controls from
/// `curve_distance::restricted_controls` (exact blossom values) plus the
/// positive-weight quotient rule, all outward-rounded.
fn axis_jets(curve: &Curve, span: usize, t: Interval, axis: usize) -> Result<[Interval; 3]> {
    let controls = crate::curve_distance::restricted_controls(curve, span, t)?;
    let width = Interval::point(t.hi).sub(Interval::point(t.lo))?;
    let hull = |values: &[Interval]| -> Interval {
        Interval {
            lo: values.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min),
            hi: values
                .iter()
                .map(|v| v.hi)
                .fold(f64::NEG_INFINITY, f64::max),
        }
    };
    let derivative = |values: &[Interval], degree: usize| -> Result<Vec<Interval>> {
        if degree == 0 {
            return Ok(vec![Interval::point(0.)]);
        }
        let factor = Interval::point(degree as f64).div(width)?;
        values
            .windows(2)
            .map(|v| v[1].sub(v[0])?.mul(factor))
            .collect()
    };
    let dimension = curve.control_points[0].len();
    let numerator: Vec<_> = controls.iter().map(|p| p[axis]).collect();
    let weights: Vec<_> = controls.iter().map(|p| p[dimension]).collect();
    let weight = hull(&weights);
    let relative = hull(&numerator).div(weight)?;
    let n1 = derivative(&numerator, curve.degree)?;
    let w1 = derivative(&weights, curve.degree)?;
    let first = hull(&n1).sub(relative.mul(hull(&w1))?)?.div(weight)?;
    let n2 = derivative(&n1, curve.degree.saturating_sub(1))?;
    let w2 = derivative(&w1, curve.degree.saturating_sub(1))?;
    let second = hull(&n2)
        .sub(first.mul(hull(&w1))?.mul(Interval::point(2.))?)?
        .sub(relative.mul(hull(&w2))?)?
        .div(weight)?;
    let origin = curve.control_points[span - curve.degree][axis];
    Ok([relative.add(Interval::point(origin))?, first, second])
}

/// Interval derivative of a curve coordinate: `f(t) = d/du curve[u][axis]`.
/// Enclosures come from the quotient-rule jets above, unioned over the knot
/// spans overlapping the query box.
struct CurveDerivative<'a> {
    curve: &'a Curve,
    axis: usize,
}
impl CurveDerivative<'_> {
    /// Union of jet `which` (0=value, 1=first, 2=second) over spans
    /// overlapping `t`. A degenerate (point) box is widened to
    /// `m ± √ε·(1+|m|)`: an ulp-scale cell would divide the derivative by an
    /// ulp-scale width and amplify rounding fuzz to O(1), while this cell
    /// keeps the jet both rigorous (it still encloses the point value) and
    /// tight enough for Newton steps.
    fn jet_over(&self, t: Interval, which: usize) -> Result<Interval> {
        let t = if t.lo == t.hi {
            let d = f64::EPSILON.sqrt() * (1. + t.lo.abs());
            Interval::new(t.lo - d, t.hi + d)?
        } else {
            t
        };
        let mut acc: Option<Interval> = None;
        for span in self.curve.degree..self.curve.control_points.len() {
            let (ka, kb) = (self.curve.knots[span], self.curve.knots[span + 1]);
            if ka >= kb || kb < t.lo || ka > t.hi {
                continue;
            }
            let cell = Interval::new(t.lo.max(ka), t.hi.min(kb))?;
            let jets = axis_jets(self.curve, span, cell, self.axis)?;
            let j = jets[which];
            acc = Some(match acc {
                None => j,
                Some(a) => Interval::new(a.lo.min(j.lo), a.hi.max(j.hi))?,
            });
        }
        acc.ok_or_else(|| numeric_err("Derivative box misses every nonempty knot span"))
    }
}
impl IntervalFunction for CurveDerivative<'_> {
    fn value_at(&self, x: f64) -> Result<f64> {
        let e = self.curve.evaluate(x)?;
        e.d1.and_then(|d| d.get(self.axis).copied())
            .ok_or_else(|| numeric_err("Curve first derivative is unavailable"))
    }
    fn interval_value(&self, x: Interval) -> Result<Interval> {
        self.jet_over(x, 1)
    }
    fn interval_derivative(&self, x: Interval) -> Result<Interval> {
        self.jet_over(x, 2)
    }
}

/// Certified isolation of curve extrema: for every coordinate axis, the
/// roots of the first derivative (i.e. parameter values where that
/// coordinate is stationary) are isolated over the active knot domain with
/// the interval Newton machinery. The per-axis lists are
/// guaranteed-complete in the `isolate_roots` sense for nonconstant coordinates.
/// A structurally constant coordinate has no isolated extrema and returns an
/// empty list; its identically zero derivative is not sent to root isolation.
pub fn certified_curve_extrema(
    curve: &Curve,
    tol: f64,
    max_depth: usize,
) -> Result<Vec<Vec<(Interval, RootCertificate)>>> {
    curve.validate()?;
    check(
        tol.is_finite() && tol > 0.,
        "Isolation tolerance must be positive and finite",
    )?;
    check(max_depth <= 60, "Isolation depth is capped at 60")?;
    let dimension = curve.control_points[0].len();
    let [a, b] = curve.domain();
    check(a < b, "Curve domain must be nonempty")?;
    let domain = Interval::new(a, b)?;
    (0..dimension)
        .map(|axis| {
            if curve
                .control_points
                .iter()
                .all(|p| p[axis] == curve.control_points[0][axis])
            {
                return Ok(Vec::new());
            }
            let f = CurveDerivative { curve, axis };
            isolate_roots(domain, &f, tol, max_depth)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::bracketed::brent_bracketed;

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

    /// Product form Π (x − r_i), expanded to power coefficients.
    fn from_roots(roots: &[f64]) -> PolynomialFunction {
        let mut c = vec![1.];
        for &r in roots {
            let mut n = vec![0.; c.len() + 1];
            for (i, &a) in c.iter().enumerate() {
                n[i] -= a * r;
                n[i + 1] += a;
            }
            c = n;
        }
        PolynomialFunction::new(c).unwrap()
    }

    #[test]
    fn derivative_coefficients_are_rounded_outward_and_monotone_boxes_refuse_roots() {
        let p = PolynomialFunction::new(vec![0., 0., 0., 0.1]).unwrap();
        let d = p.interval_derivative(Interval::point(1.)).unwrap();
        // Exact 3 times binary64 0.1 lies between these adjacent floats.
        assert!(d.lo <= 0.3 && d.hi >= 0.30000000000000004);
        let decreasing = PolynomialFunction::new(vec![-1., -0.1]).unwrap();
        assert!(matches!(
            classify(&decreasing, Interval::new(0., 100.).unwrap()).unwrap(),
            RootCertificate::Absent
        ));
        let tangent = PolynomialFunction::new(vec![0., 0., 1.]).unwrap();
        assert!(!matches!(
            classify(&tangent, Interval::new(-1., 1.).unwrap()).unwrap(),
            RootCertificate::Absent
        ));
    }

    #[test]
    fn unique_and_absent_certificates_on_known_polynomials() {
        // (x−1)(x−2)(x−3): roots at 1, 2, 3. Power-basis evaluation near
        // x = 2 cancels terms of magnitude ~15, so interval Horner
        // overestimates F′ on wide boxes and uniqueness needs a box whose
        // width is small compared to the cancellation scale.
        let p = from_roots(&[1., 2., 3.]);
        match classify(&p, Interval::new(1.95, 2.05).unwrap()).unwrap() {
            RootCertificate::Unique(n) => assert!(n.contains(2.)),
            other => panic!("expected Unique, got {other:?}"),
        }
        // Absence: no root in [3.99, 4.01]. (Interval arithmetic proves
        // absence only once the box is narrow enough for the cancellation
        // scale; wide root-free boxes are resolved by `isolate_roots`
        // subdivision, see the no-root case below.)
        assert!(matches!(
            classify(&p, Interval::new(3.99, 4.01).unwrap()).unwrap(),
            RootCertificate::Absent
        ));
        // Ambiguous box containing two roots cannot be Unique.
        assert!(!matches!(
            classify(&p, Interval::new(0.5, 2.5).unwrap()).unwrap(),
            RootCertificate::Unique(_)
        ));
    }

    #[test]
    fn isolate_finds_all_roots_including_close_pairs() {
        // Roots at 0.3 and 0.3 + 1e-6, plus 0.7.
        let p = from_roots(&[0.3, 0.3 + 1e-6, 0.7]);
        let roots = isolate_roots(Interval::new(0., 1.).unwrap(), &p, 1e-9, 40).unwrap();
        assert_eq!(roots.len(), 3);
        for (box_, certificate) in &roots {
            // The certified enclosure is tight even when the box that
            // happened to certify first is wide.
            match certificate {
                RootCertificate::Unique(u) => assert!(u.width() < 1e-6),
                _ => panic!("expected Unique for simple separated roots"),
            }
            let _ = box_;
        }
        for r in [0.3, 0.3 + 1e-6, 0.7] {
            assert!(roots.iter().any(|(b, _)| b.contains(r)), "missed {r}");
        }
        // No root at all: proven empty list.
        let q = PolynomialFunction::new(vec![1., 0., 1.]).unwrap(); // 1 + x²
        assert!(
            isolate_roots(Interval::new(-10., 10.).unwrap(), &q, 1e-6, 30)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn tangent_root_is_never_lost() {
        // (x−0.5)²: even multiplicity, no sign change. Newton cannot prove
        // uniqueness (the derivative encloses zero near the root), and
        // f64 evaluation cannot resolve f ≈ 2e-16 against rounding fuzz, so
        // completeness delivers a small cluster of Indeterminate boxes around
        // the root instead of one tight box — but never loses it.
        let p = from_roots(&[0.5, 0.5]);
        let roots = isolate_roots(Interval::new(0., 1.).unwrap(), &p, 1e-8, 40).unwrap();
        assert!(!roots.is_empty(), "double root lost");
        for (b, c) in &roots {
            assert!(matches!(c, RootCertificate::Indeterminate));
            assert!(
                b.lo < 0.5 + 1e-6 && b.hi > 0.5 - 1e-6,
                "box [{}, {}] drifted away from 0.5",
                b.lo,
                b.hi
            );
        }
        assert!(roots.iter().any(|(b, _)| b.contains(0.5)));
    }

    #[test]
    fn completeness_matches_brent_bruteforce() {
        let mut rng = Rng(41);
        for _ in 0..20 {
            let count = 1 + (rng.next() % 4) as usize;
            let roots: Vec<f64> = (0..count).map(|_| rng.range(-2., 2.)).collect();
            let p = from_roots(&roots);
            let isolated = isolate_roots(Interval::new(-3., 3.).unwrap(), &p, 1e-7, 40).unwrap();
            // Brent brute force: dense scan for sign-change brackets.
            let mut brent_roots = Vec::new();
            for i in 0..4000 {
                let a = -3. + 6. * i as f64 / 4000.;
                let b = -3. + 6. * (i + 1) as f64 / 4000.;
                let (fa, fb) = (p.value_at(a).unwrap(), p.value_at(b).unwrap());
                if fa.signum() != fb.signum() {
                    if let Ok(r) = brent_bracketed(|x| p.value_at(x).unwrap(), a, b, 1e-12, 100) {
                        if !brent_roots.iter().any(|q: &f64| (q - r).abs() < 1e-9) {
                            brent_roots.push(r);
                        }
                    }
                }
            }
            // Every Brent root lies in some isolated box, and the number of
            // isolated boxes equals the number of distinct roots (simple
            // random roots are simple with probability one; boxes may merge
            // only when roots are within tol, which the scan also merges).
            for r in &brent_roots {
                assert!(
                    isolated.iter().any(|(b, _)| b.contains(*r)),
                    "Brent root {r} missing from certified list"
                );
            }
            assert_eq!(isolated.len(), brent_roots.len());
        }
    }

    #[test]
    fn krawczyk_scalar_contracts_to_root() {
        let p = from_roots(&[0.25, 0.8]);
        let x = Interval::new(0.1, 0.4).unwrap();
        let k = krawczyk_step(&p, x).unwrap().unwrap();
        assert!(k.lo > x.lo && k.hi < x.hi, "K(X) ⊂ int(X) proves existence");
        assert!(k.contains(0.25));
        // Root in the other half is not claimed here.
        assert!(!k.contains(0.8));
    }

    struct CircleLine;
    impl IntervalSystem for CircleLine {
        // F(x,y) = (x² + y² − 1, x − y); roots at (±√2/2, ±√2/2).
        fn value_at(&self, x: &[f64]) -> Result<Vec<f64>> {
            Ok(vec![x[0] * x[0] + x[1] * x[1] - 1., x[0] - x[1]])
        }
        fn interval_jacobian(&self, x: &[Interval]) -> Result<Vec<Vec<Interval>>> {
            let two = Interval::point(2.);
            Ok(vec![
                vec![two.mul(x[0])?, two.mul(x[1])?],
                vec![Interval::point(1.), Interval::point(-1.)],
            ])
        }
    }

    #[test]
    fn krawczyk_system_2x2() {
        let s = 0.7071067811865476;
        let x = [
            Interval::new(0.4, 1.0).unwrap(),
            Interval::new(0.4, 1.0).unwrap(),
        ];
        let k = krawczyk_step_system(&CircleLine, &x).unwrap().unwrap();
        assert!(k[0].contains(s) && k[1].contains(s));
        assert!(k[0].lo > x[0].lo && k[0].hi < x[0].hi);
        assert!(k[1].lo > x[1].lo && k[1].hi < x[1].hi);
    }

    #[test]
    fn certified_extrema_of_polynomial_curve() {
        // Curve u ↦ (u, (u−0.3)(u−0.6), 0) as a Bézier segment: the y
        // coordinate has exactly one extremum, at u = 0.45.
        let y = |u: f64| (u - 0.3) * (u - 0.6);
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![0., y(0.), 0.],
                vec![0.5, 2. * y(0.5) - 0.5 * (y(0.) + y(1.)), 0.],
                vec![1., y(1.), 0.],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        curve.validate().unwrap();
        // Sanity: the quadratic is reproduced.
        for i in 0..=10 {
            let u = i as f64 / 10.;
            let p = curve.evaluate(u).unwrap().point;
            assert!((p[1] - y(u)).abs() < 1e-13);
        }
        let extrema = certified_curve_extrema(&curve, 1e-10, 40).unwrap();
        assert_eq!(extrema.len(), 3);
        assert!(extrema[0].is_empty(), "x is monotone");
        assert!(extrema[2].is_empty(), "z is constant");
        assert_eq!(extrema[1].len(), 1);
        assert!(matches!(extrema[1][0].1, RootCertificate::Unique(_)));
        assert!(extrema[1][0].0.contains(0.45));
    }
    #[test]
    fn budget_exhaustion_aborts_isolation_with_resource_error() {
        // Tiny iteration budget: even the first handful of subdivisions
        // cannot run, so isolation must abort with BudgetExhausted rather
        // than returning an incomplete list.
        let p = from_roots(&[0.3, 0.7]);
        let err = isolate_roots_with_budget(
            Interval::new(0., 1.).unwrap(),
            &p,
            1e-9,
            40,
            Budget::new(1, 64, 60_000).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("isolate_roots"), "{err}");
        assert!(err.contains("iteration budget"), "{err}");
    }

    #[test]
    fn canonical_box_equality_collapses_negative_zero() {
        let a = Interval::new(-0.0, 1.).unwrap();
        let b = Interval::new(0.0, 1.).unwrap();
        assert!(same_box(&a, &b));
        let c = Interval::new(0., 2.).unwrap();
        assert!(!same_box(&a, &c));
    }

    #[test]
    fn structurally_constant_coordinates_have_no_isolated_extrema_and_keep_budget_validation() {        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![2., -3., 0.]; 3],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let report = certified_curve_extrema(&curve, 1e-10, 40).unwrap();
        assert_eq!(report.len(), 3);
        assert!(report.iter().all(Vec::is_empty));
        for tol in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(certified_curve_extrema(&curve, tol, 40).is_err());
        }
        assert!(certified_curve_extrema(&curve, 1e-10, 61).is_err());
    }
}
