mod basis;
mod decompose;
mod knot_ops;
#[path = "curve/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
use crate::{Result, check, numeric, numeric_err, resource};
use basis::continuity_meta;
pub use basis::{Basis, LocalBasis, basis, basis_funs_ders, find_span, find_span_hinted};
pub use decompose::Segment;
pub use knot_ops::{clamped, merge_near_knots, multiplicity_eps, normalize_knots};

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub control_points: Vec<Vec<f64>>,
    pub weights: Vec<f64>,
    pub periodic: bool,
}

#[derive(Debug)]
pub struct Evaluation {
    pub point: Vec<f64>,
    pub d1: Option<Vec<f64>>,
    pub d2: Option<Vec<f64>>,
    pub domain: [f64; 2],
    pub continuity: Option<i32>,
    pub derivative_status: &'static str,
    pub derivative_side: &'static str,
    /// Outward-rounded accuracy evidence for `point`: an upper bound
    /// `c·u·Σ|terms|` on the absolute error of the largest coordinate, where
    /// `u = f64::EPSILON`, `Σ|terms|` is the sum of the absolute weighted
    /// homogeneous accumulation terms (post-division), and `c = 2` is the
    /// classic Neumaier compensated-summation constant plus one `u` for the
    /// final rational division. This is local runtime evidence and is not
    /// part of the wire encoding.
    pub rounding_bound: f64,
}

/// Neumaier (improved Kahan) compensated accumulator for the weighted
/// control-point sums. `value()` is identical to naive summation whenever
/// every partial sum is exact and is never less accurate; the classic error
/// bound is `2·u·Σ|terms|` with `u = f64::EPSILON`.
struct CompensatedSum {
    sum: f64,
    compensation: f64,
    magnitude: f64,
}
impl CompensatedSum {
    fn new() -> Self {
        Self {
            sum: 0.,
            compensation: 0.,
            magnitude: 0.,
        }
    }
    fn add(&mut self, term: f64) {
        let t = self.sum + term;
        self.compensation += if self.sum.abs() >= term.abs() {
            (self.sum - t) + term
        } else {
            (term - t) + self.sum
        };
        self.sum = t;
        self.magnitude += term.abs();
    }
    fn value(&self) -> f64 {
        self.sum + self.compensation
    }
}
/// Accuracy evidence shared by both evaluation paths: `c·u·Σ|terms|` with
/// `c = 2` (Neumaier) on the largest post-division coordinate magnitude, plus
/// one `u` on the quotient itself, outward-rounded twice.
fn rounding_bound(axes: &[CompensatedSum], weight: f64) -> f64 {
    let magnitude = axes.iter().map(|a| a.magnitude / weight).fold(0., f64::max);
    (2. * f64::EPSILON * magnitude + f64::EPSILON * magnitude)
        .next_up()
        .next_up()
}

pub(super) fn budget(count: usize) -> Result<()> {
    if count > 256 {
        return Err(resource("The result exceeds 256 control points"));
    }
    Ok(())
}
pub fn validate_basis(p: usize, k: &[f64], n: usize) -> Result<[f64; 2]> {
    check(
        (1..=25).contains(&p),
        "Degree must be an integer in [1, 25]",
    )?;
    check(
        n > p && n <= 256,
        "Control point count must be between degree+1 and 256",
    )?;
    check(
        k.len() == n + p + 1,
        "Expanded knot count must equal control point count + degree + 1",
    )?;
    let mut multiplicity = 0;
    for i in 0..k.len() {
        crate::foundation::guards::require_finite_at(k[i], "knots", i)?;
        check(k[i].abs() <= 1e9, "Knots must be finite and bounded by 1e9")?;
        check(i == 0 || k[i] >= k[i - 1], "Knots must be nondecreasing")?;
        multiplicity = if i > 0 && k[i] == k[i - 1] {
            multiplicity + 1
        } else {
            1
        };
        check(
            multiplicity <= p + 1,
            "Knot multiplicity must not exceed degree+1",
        )?;
    }
    let domain = [k[p], k[n]];
    check(
        domain[0] < domain[1],
        "The active knot domain must have positive length",
    )?;
    let mut i = 0;
    while i < k.len() {
        let mut end = i + 1;
        while end < k.len() && k[end] == k[i] {
            end += 1;
        }
        check(
            k[i] <= domain[0] || k[i] >= domain[1] || end - i <= p,
            "Interior multiplicity must not exceed degree; disconnected curves need separate nodes",
        )?;
        i = end;
    }
    Ok(domain)
}
impl Curve {
    /// Exact degree-one representation of a 2D or 3D polyline. Repeating the
    /// first point closes it; this does not create periodic spline storage.
    pub fn from_polyline(points: Vec<Vec<f64>>) -> Result<Self> {
        check(points.len() >= 2, "A polyline needs at least two points")?;
        budget(points.len())?;
        let count = points.len();
        let mut knots = vec![0.];
        knots.extend((0..count).map(|i| i as f64));
        knots.push((count - 1) as f64);
        let curve = Self {
            degree: 1,
            knots,
            control_points: points,
            weights: vec![1.; count],
            periodic: false,
        };
        curve.validate()?;
        Ok(curve)
    }
    pub fn domain(&self) -> [f64; 2] {
        [
            self.knots[self.degree],
            self.knots[self.control_points.len()],
        ]
    }
    pub fn validate(&self) -> Result<()> {
        let n = self.control_points.len();
        let domain = validate_basis(self.degree, &self.knots, n)?;
        let dim = self.control_points[0].len();
        check(
            dim == 2 || dim == 3,
            "Control points must have two or three coordinates",
        )?;
        for (i, p) in self.control_points.iter().enumerate() {
            check(
                p.len() == dim,
                "Control points must have consistent dimensions and finite coordinates bounded by 1e9",
            )?;
            if p.iter().any(|x| !x.is_finite()) {
                return Err(crate::input(format!(
                    "control_points[{i}] must have finite coordinates (no NaN or Inf)"
                )));
            }
        }
        check(
            self.control_points
                .iter()
                .all(|p| p.iter().all(|x| x.abs() <= 1e9)),
            "Control points must have consistent dimensions and finite coordinates bounded by 1e9",
        )?;
        check(
            self.weights.len() == n,
            "Weights must match the control point count",
        )?;
        for (i, &w) in self.weights.iter().enumerate() {
            if !w.is_finite() {
                return Err(crate::input(format!(
                    "weights[{i}] must be positive, finite (no NaN or Inf)"
                )));
            }
        }
        check(
            self.weights.iter().all(|w| *w >= 1e-12 && *w <= 1e12),
            "Weights must be positive, finite, and in [1e-12, 1e12]",
        )?;
        let max = self.weights.iter().copied().fold(0., f64::max);
        let min = self.weights.iter().copied().fold(f64::INFINITY, f64::min);
        check(
            max / min <= 1e12,
            "Weight conditioning must not exceed 1e12",
        )?;
        if self.periodic {
            let period_controls = n - self.degree;
            check(
                period_controls > self.degree,
                "A periodic curve needs at least degree+1 unwrapped control points",
            )?;
            for i in 0..self.degree {
                check(
                    self.weights[i] == self.weights[period_controls + i]
                        && self.control_points[i] == self.control_points[period_controls + i],
                    "Periodic curves must explicitly repeat the first degree control points and weights at the end",
                )?;
            }
            let period = domain[1] - domain[0];
            let scale = self.knots.iter().map(|k| k.abs()).fold(0., f64::max);
            for i in 0..self.knots.len() - period_controls {
                let difference = self.knots[i + period_controls] - self.knots[i];
                check(
                    (difference - period).abs()
                        <= 32.
                            * f64::EPSILON
                            * scale
                                .max(difference.abs())
                                .max(period.abs())
                                .max(f64::from_bits(1)),
                    "Periodic exterior knots must repeat with the active period",
                )?;
            }
        }
        Ok(())
    }
    pub fn evaluate(&self, u: f64) -> Result<Evaluation> {
        self.validate()?;
        self.evaluate_validated(u)
    }
    /// Evaluation without re-validation; callers must have validated `self`.
    pub(crate) fn evaluate_validated(&self, u: f64) -> Result<Evaluation> {
        let b = basis(
            self.degree,
            &self.knots,
            self.control_points.len(),
            u,
            self.periodic,
        )?;
        let dim = self.control_points[0].len();
        let scale = self.weights.iter().copied().fold(0., f64::max);
        let index = b
            .basis
            .iter()
            .position(|v| *v > 0.)
            .ok_or_else(|| numeric_err("Empty basis support"))?;
        let origin = &self.control_points[index];
        let mut weight = CompensatedSum::new();
        let mut w1 = CompensatedSum::new();
        let mut w2 = CompensatedSum::new();
        let mut point: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        let mut d1: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        let mut d2: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        for i in 0..self.control_points.len() {
            let w = self.weights[i] / scale;
            weight.add(b.basis[i] * w);
            w1.add(b.d1[i] * w);
            w2.add(b.d2[i] * w);
            for axis in 0..dim {
                let coordinate = self.control_points[i][axis] - origin[axis];
                point[axis].add(b.basis[i] * w * coordinate);
                d1[axis].add(b.d1[i] * w * coordinate);
                d2[axis].add(b.d2[i] * w * coordinate);
            }
        }
        let weight = weight.value();
        let w1 = w1.value();
        let w2 = w2.value();
        let rounding = rounding_bound(&point, weight);
        let mut point: Vec<f64> = point.iter().map(|a| a.value()).collect();
        let mut d1: Vec<f64> = d1.iter().map(|a| a.value()).collect();
        let mut d2: Vec<f64> = d2.iter().map(|a| a.value()).collect();
        numeric(
            weight > 0. && weight.is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        for axis in 0..dim {
            point[axis] /= weight;
            d1[axis] = (d1[axis] - w1 * point[axis]) / weight;
            d2[axis] = (d2[axis] - 2. * w1 * d1[axis] - w2 * point[axis]) / weight;
            point[axis] += origin[axis];
        }
        numeric(
            point.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        Ok(Evaluation {
            point,
            d1: (b.continuity.unwrap_or(2) >= 1).then_some(d1),
            d2: (b.continuity.unwrap_or(2) >= 2).then_some(d2),
            domain: b.domain,
            continuity: b.continuity,
            derivative_status: b.derivative_status,
            derivative_side: b.derivative_side,
            rounding_bound: rounding,
        })
    }
    pub fn bounds(&self) -> Result<Bounds> {
        self.validate()?;
        bounds(&self.control_points)
    }
}
pub use crate::bounds::{Bounds, from_points as bounds};

pub(super) fn multiplicity(knots: &[f64], u: f64) -> usize {
    knots.iter().filter(|k| **k == u).count()
}
pub(super) struct Homogeneous {
    degree: usize,
    knots: Vec<f64>,
    controls: Vec<Vec<f64>>,
}
impl From<&Curve> for Homogeneous {
    fn from(c: &Curve) -> Self {
        Self {
            degree: c.degree,
            knots: c.knots.clone(),
            controls: c
                .control_points
                .iter()
                .zip(&c.weights)
                .map(|(p, w)| p.iter().map(|x| x * w).chain(std::iter::once(*w)).collect())
                .collect(),
        }
    }
}
impl Homogeneous {
    pub(super) fn to_curve(&self) -> Result<Curve> {
        budget(self.controls.len())?;
        let weights: Vec<f64> = self.controls.iter().map(|p| *p.last().unwrap()).collect();
        let c = Curve {
            degree: self.degree,
            knots: self.knots.clone(),
            control_points: self
                .controls
                .iter()
                .zip(&weights)
                .map(|(p, w)| p[..p.len() - 1].iter().map(|x| x / w).collect())
                .collect(),
            weights,
            periodic: false,
        };
        c.validate()?;
        Ok(c)
    }
    pub(super) fn insert(&self, u: f64) -> Result<Self> {
        let p = self.degree;
        let n = self.controls.len() - 1;
        let s = multiplicity(&self.knots, u);
        check(s <= p, "Requested knot already has degree+1 multiplicity")?;
        let mut k = p;
        while k + 1 < self.knots.len() && self.knots[k + 1] <= u {
            k += 1;
        }
        check(
            k >= p && k - s <= n,
            "Knot insertion has an empty local span",
        )?;
        let mut controls = vec![Vec::new(); n + 2];
        controls[..=k - p].clone_from_slice(&self.controls[..=k - p]);
        controls[k - s + 1..n + 2].clone_from_slice(&self.controls[k - s..n + 1]);
        for (i, control) in controls
            .iter_mut()
            .enumerate()
            .take(k - s + 1)
            .skip(k - p + 1)
        {
            let denominator = self.knots[i + p] - self.knots[i];
            check(denominator > 0., "Knot insertion has an empty local span")?;
            let alpha = (u - self.knots[i]) / denominator;
            *control = self.controls[i]
                .iter()
                .enumerate()
                .map(|(axis, x)| alpha * x + (1. - alpha) * self.controls[i - 1][axis])
                .collect();
        }
        let mut knots = self.knots.clone();
        knots.insert(k + 1, u);
        Ok(Self {
            degree: p,
            knots,
            controls,
        })
    }
}

/// Validate-once curve evaluator: `new()` runs `Curve::validate()` a single
/// time, then `evaluate()` reuses a cached span hint (monotone sampling fast
/// path) and the O(p²) local basis with no full-width allocations.
pub struct CurveEvaluator<'a> {
    curve: &'a Curve,
    domain: [f64; 2],
    scale: f64,
    span: usize,
}
impl<'a> CurveEvaluator<'a> {
    pub fn new(curve: &'a Curve) -> Result<Self> {
        curve.validate()?;
        Ok(Self {
            curve,
            domain: curve.domain(),
            scale: curve.weights.iter().copied().fold(0., f64::max),
            span: curve.degree,
        })
    }
    fn locate(&mut self, u: f64) -> Result<usize> {
        let p = self.curve.degree;
        let n = self.curve.control_points.len();
        if u == self.domain[1] {
            self.span = n - 1;
            return Ok(n - 1);
        }
        let k = &self.curve.knots;
        let s = self.span;
        if s >= p && s + 1 < k.len() && u >= k[s] && u < k[s + 1] {
            return Ok(s);
        }
        let s = find_span(p, k, n, u)?;
        self.span = s;
        Ok(s)
    }
    pub fn evaluate(&mut self, u: f64) -> Result<Evaluation> {
        check(
            u.is_finite() && u >= self.domain[0] && u <= self.domain[1],
            "Parameter is outside the active knot domain",
        )?;
        let span = self.locate(u)?;
        let p = self.curve.degree;
        let local = basis_funs_ders(p, &self.curve.knots, span, u, 2)?;
        let dim = self.curve.control_points[0].len();
        let first = local
            .values
            .iter()
            .position(|v| *v > 0.)
            .ok_or_else(|| numeric_err("Empty basis support"))?;
        let origin = &self.curve.control_points[span - p + first];
        let mut weight = CompensatedSum::new();
        let mut w1 = CompensatedSum::new();
        let mut w2 = CompensatedSum::new();
        let mut point: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        let mut d1: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        let mut d2: Vec<CompensatedSum> = (0..dim).map(|_| CompensatedSum::new()).collect();
        for j in 0..=p {
            let i = span - p + j;
            let w = self.curve.weights[i] / self.scale;
            weight.add(local.values[j] * w);
            w1.add(local.d1[j] * w);
            w2.add(local.d2[j] * w);
            for axis in 0..dim {
                let coordinate = self.curve.control_points[i][axis] - origin[axis];
                point[axis].add(local.values[j] * w * coordinate);
                d1[axis].add(local.d1[j] * w * coordinate);
                d2[axis].add(local.d2[j] * w * coordinate);
            }
        }
        let weight = weight.value();
        let w1 = w1.value();
        let w2 = w2.value();
        let rounding = rounding_bound(&point, weight);
        let mut point: Vec<f64> = point.iter().map(|a| a.value()).collect();
        let mut d1: Vec<f64> = d1.iter().map(|a| a.value()).collect();
        let mut d2: Vec<f64> = d2.iter().map(|a| a.value()).collect();
        numeric(
            weight > 0. && weight.is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        for axis in 0..dim {
            point[axis] /= weight;
            d1[axis] = (d1[axis] - w1 * point[axis]) / weight;
            d2[axis] = (d2[axis] - 2. * w1 * d1[axis] - w2 * point[axis]) / weight;
            point[axis] += origin[axis];
        }
        numeric(
            point.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        let (continuity, status, side) =
            continuity_meta(p, &self.curve.knots, self.domain, u, self.curve.periodic);
        Ok(Evaluation {
            point,
            d1: (continuity.unwrap_or(2) >= 1).then_some(d1),
            d2: (continuity.unwrap_or(2) >= 2).then_some(d2),
            domain: self.domain,
            continuity,
            derivative_status: status,
            derivative_side: side,
            rounding_bound: rounding,
        })
    }
}
impl Curve {
    /// Classic de Boor evaluation in homogeneous space, provided as an
    /// evaluator independent of the basis-function path for cross-checking.
    /// The triangle is a sequence of convex interpolations rather than an
    /// accumulation, so Neumaier compensation is not applicable here; use
    /// `evaluate`/`CurveEvaluator` when compensated accuracy evidence
    /// (`Evaluation::rounding_bound`) is needed.
    pub fn evaluate_de_boor(&self, u: f64) -> Result<Vec<f64>> {
        self.validate()?;
        let p = self.degree;
        let n = self.control_points.len();
        let domain = self.domain();
        check(
            u.is_finite() && u >= domain[0] && u <= domain[1],
            "Parameter is outside the active knot domain",
        )?;
        let span = find_span(p, &self.knots, n, u)?;
        let work = Homogeneous::from(self);
        // Knots equal to `u` ending at `span`; at the clamped right end the
        // span lies below the end knot, so s = 0 and the full triangle runs.
        let mut s = 0;
        while s < p && span > s && self.knots[span - s] == u {
            s += 1;
        }
        let mut d: Vec<Vec<f64>> = (0..=p - s)
            .map(|j| work.controls[span - p + j].clone())
            .collect();
        for r in 1..=p - s {
            for j in (r..=p - s).rev() {
                let idx = span - p + j;
                let denominator = self.knots[idx + p - r + 1] - self.knots[idx];
                let alpha = if denominator == 0. {
                    0.
                } else {
                    (u - self.knots[idx]) / denominator
                };
                let previous = d[j - 1].clone();
                for axis in 0..d[j].len() {
                    d[j][axis] = alpha * d[j][axis] + (1. - alpha) * previous[axis];
                }
            }
        }
        let h = &d[p - s];
        let weight = *h
            .last()
            .ok_or_else(|| numeric_err("Empty homogeneous point"))?;
        numeric(
            weight.is_finite() && weight > 0.,
            "Rational denominator lost its positive finite value",
        )?;
        let point: Vec<f64> = h[..h.len() - 1].iter().map(|x| x / weight).collect();
        numeric(
            point.iter().all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        Ok(point)
    }
}

#[cfg(test)]
mod hinted_span_tests {
    use super::{find_span, find_span_hinted};

    fn lcg(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 11) as f64 / (1u64 << 53) as f64
    }

    #[test]
    fn hinted_matches_binary_search_on_random_inputs() {
        // Non-uniform knots with a repeated interior knot.
        let degree = 3;
        let knots = [0., 0., 0., 0., 1., 2., 2., 3., 4., 4., 4., 4.];
        let n = 8;
        let mut state = 0xDEADBEEFCAFEF00D;
        for case in 0..2000 {
            let u = 4. * lcg(&mut state);
            let u = match case % 5 {
                0 => 0.,
                1 => 4.,
                2 => 2., // exactly on the repeated knot
                _ => u,
            };
            // Hints cover valid spans, stale spans and out-of-range garbage.
            let mut hint = (lcg(&mut state) * 1000.) as usize;
            let expected = find_span(degree, &knots, n, u).unwrap();
            let got = find_span_hinted(degree, &knots, n, u, &mut hint).unwrap();
            assert_eq!(got, expected, "u={u} initial hint mismatch");
            assert_eq!(hint, expected, "hint must be updated to the span");
        }
    }

    #[test]
    fn hinted_monotone_sequence_is_stable() {
        let degree = 2;
        let knots = [0., 0., 0., 1., 2., 3., 3., 3.];
        let n = 5;
        let mut hint = usize::MAX;
        for i in 0..=64 {
            let u = 3. * i as f64 / 64.;
            let got = find_span_hinted(degree, &knots, n, u, &mut hint).unwrap();
            assert_eq!(got, find_span(degree, &knots, n, u).unwrap());
        }
    }

    #[test]
    fn hinted_rejects_invalid_arguments() {
        let knots = [0., 0., 0., 1., 1., 1.];
        let mut hint = 0;
        assert!(find_span_hinted(0, &knots, 3, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 2, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots[..5], 3, 0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 3, -0.5, &mut hint).is_err());
        assert!(find_span_hinted(2, &knots, 3, f64::NAN, &mut hint).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Random clamped curve on [0, 1] with well-separated interior knots.
    fn random_curve(rng: &mut Rng) -> Curve {
        let p = 1 + (rng.next() % 4) as usize;
        let interior = (rng.next() % 6) as usize;
        let mut knots = vec![0.; p + 1];
        for i in 0..interior {
            knots.push((i + 1) as f64 / (interior + 1) as f64);
        }
        knots.extend(vec![1.; p + 1]);
        let n = knots.len() - p - 1;
        let control_points = (0..n)
            .map(|_| (0..3).map(|_| rng.range(-5., 5.)).collect::<Vec<f64>>())
            .collect();
        let weights = (0..n).map(|_| rng.range(0.5, 2.)).collect();
        let curve = Curve {
            degree: p,
            knots,
            control_points,
            weights,
            periodic: false,
        };
        curve.validate().unwrap();
        curve
    }

    fn close(a: &[f64], b: &[f64], tol: f64) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| (x - y).abs() <= tol * (1. + x.abs().max(y.abs())))
    }

    #[test]
    fn find_span_matches_linear_scan() {
        let mut rng = Rng(42);
        for _ in 0..30 {
            let c = random_curve(&mut rng);
            let (p, n) = (c.degree, c.control_points.len());
            for _ in 0..20 {
                let u = rng.range(0., 1.);
                let span = find_span(p, &c.knots, n, u).unwrap();
                let mut reference = p;
                while reference + 1 <= n - 1 && c.knots[reference + 1] <= u {
                    reference += 1;
                }
                assert_eq!(span, reference);
                assert!(c.knots[span] <= u && u < c.knots[span + 1]);
            }
            assert_eq!(find_span(p, &c.knots, n, 1.).unwrap(), n - 1);
            assert_eq!(find_span(p, &c.knots, n, 0.).unwrap(), p);
            assert!(find_span(p, &c.knots, n, 1.5).is_err());
            assert!(find_span(p, &c.knots, n, -0.5).is_err());
        }
    }

    #[test]
    fn basis_funs_ders_matches_full_basis() {
        let mut rng = Rng(7);
        for _ in 0..30 {
            let c = random_curve(&mut rng);
            let (p, n) = (c.degree, c.control_points.len());
            for _ in 0..25 {
                let u = rng.range(0., 1.);
                let span = find_span(p, &c.knots, n, u).unwrap();
                let local = basis_funs_ders(p, &c.knots, span, u, 2).unwrap();
                let scattered = local.scatter(p, &c.knots, n, u, false).unwrap();
                let full = basis(p, &c.knots, n, u, false).unwrap();
                assert!(
                    close(&scattered.basis, &full.basis, 1e-9),
                    "basis mismatch at {u}"
                );
                assert!(close(&scattered.d1, &full.d1, 1e-7), "d1 mismatch at {u}");
                assert!(close(&scattered.d2, &full.d2, 1e-5), "d2 mismatch at {u}");
                assert_eq!(scattered.continuity, full.continuity);
                assert_eq!(scattered.derivative_status, full.derivative_status);
                assert_eq!(scattered.derivative_side, full.derivative_side);
                assert!(local.values.iter().all(|v| *v >= 0.));
                assert!((local.values.iter().sum::<f64>() - 1.).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn evaluator_matches_evaluate() {
        let mut rng = Rng(11);
        for _ in 0..25 {
            let c = random_curve(&mut rng);
            let mut evaluator = CurveEvaluator::new(&c).unwrap();
            // Monotone sweep exercises the cached-span fast path.
            let samples: Vec<f64> = (0..=50).map(|i| i as f64 / 50.).collect();
            // Reverse sweep defeats the hint and exercises the fallback.
            let all: Vec<f64> = samples
                .iter()
                .chain(samples.iter().rev())
                .copied()
                .collect();
            for u in all {
                let fast = evaluator.evaluate(u).unwrap();
                let reference = c.evaluate(u).unwrap();
                assert!(
                    close(&fast.point, &reference.point, 1e-9),
                    "point mismatch at {u}"
                );
                match (&fast.d1, &reference.d1) {
                    (Some(a), Some(b)) => assert!(close(a, b, 1e-6), "d1 mismatch at {u}"),
                    (None, None) => {}
                    _ => panic!("d1 availability mismatch at {u}"),
                }
                match (&fast.d2, &reference.d2) {
                    (Some(a), Some(b)) => assert!(close(a, b, 1e-4), "d2 mismatch at {u}"),
                    (None, None) => {}
                    _ => panic!("d2 availability mismatch at {u}"),
                }
                assert_eq!(fast.continuity, reference.continuity);
                assert_eq!(fast.derivative_status, reference.derivative_status);
                assert_eq!(fast.derivative_side, reference.derivative_side);
                assert_eq!(fast.domain, reference.domain);
            }
        }
    }

    #[test]
    fn de_boor_matches_evaluate() {
        let mut rng = Rng(23);
        for _ in 0..25 {
            let c = random_curve(&mut rng);
            for i in 0..=40 {
                let u = i as f64 / 40.;
                let a = c.evaluate_de_boor(u).unwrap();
                let b = c.evaluate(u).unwrap().point;
                assert!(close(&a, &b, 1e-9), "de Boor mismatch at {u}");
            }
        }
    }

    #[test]
    fn refine_matches_repeated_insert() {
        let mut rng = Rng(31);
        for _ in 0..20 {
            let c = random_curve(&mut rng);
            if c.degree < 2 {
                continue;
            }
            let xs: Vec<f64> = (0..3)
                .map(|_| {
                    loop {
                        let x = rng.range(0.05, 0.95);
                        if c.knots.iter().all(|k| *k != x) {
                            break x;
                        }
                    }
                })
                .collect();
            let mut xs = xs;
            xs.sort_by(f64::total_cmp);
            let refined = c.refine(&xs).unwrap();
            let mut inserted = c.clone();
            for &x in &xs {
                inserted = inserted.insert(x, 1).unwrap();
            }
            assert_eq!(refined.knots, inserted.knots);
            assert_eq!(refined.degree, c.degree);
            assert_eq!(
                refined.control_points.len(),
                c.control_points.len() + xs.len()
            );
            for i in 0..=20 {
                let u = i as f64 / 20.;
                assert!(close(
                    &refined.evaluate(u).unwrap().point,
                    &c.evaluate(u).unwrap().point,
                    1e-8,
                ));
            }
        }
    }

    #[test]
    fn refine_validates_inputs() {
        let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]]).unwrap();
        assert!(c.refine(&[0.25, 0.75]).is_ok());
        assert!(c.refine(&[]).unwrap() == c);
        assert!(c.refine(&[0.75, 0.25]).is_err());
        assert!(c.refine(&[-0.1]).is_err());
        assert!(c.refine(&[2.1]).is_err());
        assert!(c.refine(&[0.5, 0.5, 0.5]).is_err());
    }

    #[test]
    fn merge_near_knots_collapses_noise() {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 0.5 + 1e-12, 1., 1., 1., 1.],
            control_points: (0..6).map(|i| vec![i as f64, (i % 3) as f64, 0.]).collect(),
            weights: vec![1.; 6],
            periodic: false,
        };
        c.validate().unwrap();
        let merged = merge_near_knots(&c, 1e-9).unwrap();
        assert_eq!(merged.knots.len(), c.knots.len());
        assert_eq!(merged.knots[4], merged.knots[5]);
        assert_eq!(multiplicity_eps(&merged.knots, merged.knots[4], 0.), 2);
        for i in 0..=20 {
            let u = i as f64 / 20.;
            assert!(close(
                &merged.evaluate(u).unwrap().point,
                &c.evaluate(u).unwrap().point,
                1e-6,
            ));
        }
        assert!(merge_near_knots(&c, 0.).is_err());
    }

    #[test]
    fn multiplicity_eps_counts_with_tolerance() {
        let knots = [0., 0., 0.25, 0.25 + 1e-13, 0.5, 1.];
        assert_eq!(multiplicity_eps(&knots, 0.25, 0.), 1);
        assert_eq!(multiplicity_eps(&knots, 0.25, 1e-12), 2);
        assert_eq!(multiplicity_eps(&knots, 0., 1e-12), 2);
    }

    #[test]
    fn clamped_and_normalize_preserve_geometry() {
        let mut rng = Rng(37);
        for _ in 0..15 {
            let c = random_curve(&mut rng);
            let clamped = clamped(&c).unwrap();
            assert!(!clamped.periodic);
            assert_eq!(clamped.domain(), c.domain());
            assert_eq!(multiplicity(&clamped.knots, 0.), c.degree + 1);
            assert_eq!(multiplicity(&clamped.knots, 1.), c.degree + 1);
            let scaled = Curve {
                knots: c.knots.iter().map(|k| 2. + 3. * k).collect(),
                ..c.clone()
            };
            scaled.validate().unwrap();
            let normalized = normalize_knots(&scaled).unwrap();
            assert_eq!(normalized.domain(), [0., 1.]);
            for i in 0..=10 {
                let u = i as f64 / 10.;
                assert!(close(
                    &clamped.evaluate(u).unwrap().point,
                    &c.evaluate(u).unwrap().point,
                    1e-9,
                ));
                assert!(close(
                    &normalized.evaluate(u).unwrap().point,
                    &scaled.evaluate(2. + 3. * u).unwrap().point,
                    1e-9,
                ));
            }
        }
    }

    #[test]
    fn evaluate_validated_skips_revalidation() {
        let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]]).unwrap();
        let a = c.evaluate(0.4).unwrap();
        let b = c.evaluate_validated(0.4).unwrap();
        assert!(close(&a.point, &b.point, 0.));
    }

    /// Double-double (two-float) accumulator used as a near-exact reference
    /// for the compensated-summation tests.
    #[derive(Clone, Copy)]
    struct DoubleDouble {
        hi: f64,
        lo: f64,
    }
    impl DoubleDouble {
        fn add(&mut self, x: f64) {
            let s = self.hi + x;
            let z = s - self.hi;
            self.lo += (self.hi - (s - z)) + (x - z);
            self.hi = s;
        }
        fn normalized(self) -> (f64, f64) {
            let s = self.hi + self.lo;
            (s, self.lo + (self.hi - s))
        }
    }

    /// Naive (uncompensated) replica of the evaluation weighted sums, used to
    /// compare against the compensated production path.
    fn naive_weighted_point(c: &Curve, u: f64) -> Vec<f64> {
        let b = basis(c.degree, &c.knots, c.control_points.len(), u, c.periodic).unwrap();
        let dim = c.control_points[0].len();
        let scale = c.weights.iter().copied().fold(0., f64::max);
        let index = b.basis.iter().position(|v| *v > 0.).unwrap();
        let origin = &c.control_points[index];
        let mut weight = 0.;
        let mut point = vec![0.; dim];
        for i in 0..c.control_points.len() {
            let w = c.weights[i] / scale;
            weight += b.basis[i] * w;
            for axis in 0..dim {
                let coordinate = c.control_points[i][axis] - origin[axis];
                point[axis] += b.basis[i] * w * coordinate;
            }
        }
        point.iter_mut().for_each(|x| *x /= weight);
        point
    }

    /// Near-exact double-double replica of the same sums.
    fn reference_weighted_point(c: &Curve, u: f64) -> Vec<f64> {
        let b = basis(c.degree, &c.knots, c.control_points.len(), u, c.periodic).unwrap();
        let dim = c.control_points[0].len();
        let scale = c.weights.iter().copied().fold(0., f64::max);
        let index = b.basis.iter().position(|v| *v > 0.).unwrap();
        let origin = &c.control_points[index];
        let mut weight = DoubleDouble { hi: 0., lo: 0. };
        let mut point = vec![DoubleDouble { hi: 0., lo: 0. }; dim];
        for i in 0..c.control_points.len() {
            let w = c.weights[i] / scale;
            weight.add(b.basis[i] * w);
            for axis in 0..dim {
                let coordinate = c.control_points[i][axis] - origin[axis];
                point[axis].add(b.basis[i] * w * coordinate);
            }
        }
        let (wh, wl) = weight.normalized();
        point
            .iter()
            .map(|a| {
                let (h, l) = a.normalized();
                // One Newton refinement of the double-double quotient.
                let q = h / wh;
                q + (l - q * wl) / wh
            })
            .collect()
    }

    #[test]
    fn compensation_beats_naive_on_adversarial_weights() {
        // Absorption case: t1 ≈ +2e8, t2 ≈ 1e-8 (below half an ulp of t1, so
        // naive summation drops it entirely), t3 ≈ -2e8. The true numerator is
        // dominated by the tiny middle term; weights span 1..1e-12 (the
        // validation ceiling ratio 1e12).
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.5, 0.25],
                vec![0.5 + 2e8 / 0.288, 1.],
                vec![0.5 + 1e-8 / (0.432e-12), 2.],
                vec![0.5 - 2e8 / 0.216, 3.],
            ],
            weights: vec![1., 1., 1e-12, 1.],
            periodic: false,
        };
        c.validate().unwrap();
        let u = 0.6;
        let reference = reference_weighted_point(&c, u);
        let naive = naive_weighted_point(&c, u);
        let e = c.evaluate(u).unwrap();
        let compensated: Vec<f64> = e
            .point
            .iter()
            .zip(&c.control_points[0])
            .map(|(x, o)| x - o)
            .collect();
        let naive_err = (naive[0] - reference[0]).abs();
        let compensated_err = (compensated[0] - reference[0]).abs();
        assert!(
            naive_err > 1e-10,
            "case not adversarial: naive error {naive_err:e}"
        );
        assert!(
            compensated_err <= naive_err,
            "compensated {compensated_err:e} must not exceed naive {naive_err:e}"
        );
        assert!(
            compensated_err < naive_err * 0.01,
            "expected real improvement: compensated {compensated_err:e} vs naive {naive_err:e}"
        );
        assert!(e.rounding_bound.is_finite() && e.rounding_bound >= 0.);
        // CurveEvaluator agrees with evaluate and carries the same evidence.
        let mut evaluator = CurveEvaluator::new(&c).unwrap();
        let fast = evaluator.evaluate(u).unwrap();
        assert!(fast.rounding_bound.is_finite() && fast.rounding_bound >= 0.);
        assert!(close(&fast.point, &e.point, 1e-15));
    }

    #[test]
    fn repeated_evaluation_is_stable_and_finite() {
        let mut rng = Rng(101);
        let c = random_curve(&mut rng);
        let mut evaluator = CurveEvaluator::new(&c).unwrap();
        let mut checksum = 0.;
        for i in 0..100_000 {
            let u = (i % 1001) as f64 / 1000.;
            let e = evaluator.evaluate(u).unwrap();
            assert!(e.point.iter().all(|v| v.is_finite()));
            assert!(e.rounding_bound.is_finite() && e.rounding_bound >= 0.);
            checksum += e.point[0];
        }
        assert!(checksum.is_finite());
        // Cross-check a sample against the independent de Boor path.
        for i in 0..=20 {
            let u = i as f64 / 20.;
            assert!(close(
                &c.evaluate(u).unwrap().point,
                &c.evaluate_de_boor(u).unwrap(),
                1e-9,
            ));
        }
    }
}
