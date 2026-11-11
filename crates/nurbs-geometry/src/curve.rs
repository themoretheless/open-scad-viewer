mod basis;
mod decompose;
mod evaluator;
mod knot_ops;
#[path = "curve/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
use crate::{Result, check, numeric, numeric_err, resource};
pub use basis::{continuity_meta, stack_basis_validated};
pub use basis::{Basis, LocalBasis, basis, basis_funs_ders, find_span, find_span_hinted};
pub use decompose::Segment;
pub use evaluator::{CurveEvaluator, CurveSampler};
pub use knot_ops::{clamped, merge_near_knots, multiplicity_eps, normalize_knots};

/// Dimension-flexible NURBS control data.
///
/// # Storage contract (AoS, not SoA)
/// `control_points[i]` owns all coordinates of one control point (array of
/// structures); `weights[i]` is its positive rational weight. This keeps every
/// control net coherent during knot insertion, decomposition and editing and
/// avoids a second parallel API for 2D/3D data. Evaluation traverses only the
/// active `degree + 1` points and accumulates coordinates in separate scalar
/// lanes, so a persistent SoA copy would add allocation and synchronization
/// costs without improving the general kernel path. SIMD basis batching is a
/// separate numerics concern and does not change this storage contract.
///
/// Native fixed-size [`Point3`](crate::geometry::Point3) and
/// [`Vector3`](crate::geometry::Vector3) adapters make affine semantics
/// explicit without breaking these established public fields. Rational control
/// point `i` represents `(weights[i]·control_points[i], weights[i])` in
/// homogeneous space, never an unweighted Cartesian point with an unrelated
/// weight.
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
pub struct CompensatedSum {
    sum: f64,
    compensation: f64,
    magnitude: f64,
}
impl CompensatedSum {
    pub fn new() -> Self {
        Self {
            sum: 0.,
            compensation: 0.,
            magnitude: 0.,
        }
    }
    pub fn add(&mut self, term: f64) {
        let t = self.sum + term;
        self.compensation += if self.sum.abs() >= term.abs() {
            (self.sum - t) + term
        } else {
            (term - t) + self.sum
        };
        self.sum = t;
        self.magnitude += term.abs();
    }
    pub fn value(&self) -> f64 {
        self.sum + self.compensation
    }
}
/// Accuracy evidence shared by both evaluation paths: `c·u·Σ|terms|` with
/// `c = 2` (Neumaier) on the largest post-division coordinate magnitude, plus
/// one `u` on the quotient itself, outward-rounded twice.
pub fn rounding_bound(axes: &[CompensatedSum], weight: f64) -> f64 {
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
        crate::numerics::guards::require_finite_at(k[i], "knots", i)?;
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
            validate_periodic_knots(&self.knots, period_controls, domain)?;
        }
        Ok(())
    }
    pub fn evaluate(&self, u: f64) -> Result<Evaluation> {
        self.validate()?;
        self.evaluate_validated(u)
    }
    pub fn evaluate_points(&self, parameters: &[f64]) -> Result<Vec<Vec<f64>>> {
        check(
            parameters.len() <= 4096,
            "Batch curve evaluation exceeds 4096 parameters",
        )?;
        let sampler = CurveSampler::new(self)?;
        let dim = sampler.dimension();
        parameters
            .iter()
            .map(|&u| {
                let eval = self.evaluate_stack_validated(sampler.domain, sampler.scale, u)?;
                Ok(eval.point[..dim].to_vec())
            })
            .collect()
    }
    pub fn sample_uniform(&self, segments: usize, include_end: bool) -> Result<Vec<Vec<f64>>> {
        check(
            (1..=4096).contains(&segments),
            "Curve uniform sampling segments must be in [1, 4096]",
        )?;
        let sampler = CurveSampler::new(self)?;
        let [start, end] = sampler.domain;
        let dim = sampler.dimension();
        let count = if include_end { segments + 1 } else { segments };
        (0..count)
            .map(|i| {
                let u = start + (end - start) * (i as f64) / (segments as f64);
                let eval = self.evaluate_stack_validated(sampler.domain, sampler.scale, u)?;
                Ok(eval.point[..dim].to_vec())
            })
            .collect()
    }
    /// Evaluation without re-validation; callers must have validated `self`.
    pub fn evaluate_validated(&self, u: f64) -> Result<Evaluation> {
        let scale = self.weights.iter().copied().fold(0., f64::max);
        self.evaluate_validated_with(self.domain(), scale, u)
    }
    pub fn evaluate_validated_with(
        &self,
        domain: [f64; 2],
        scale: f64,
        u: f64,
    ) -> Result<Evaluation> {
        let dim = self.control_points[0].len();
        let eval = self.evaluate_stack_validated(domain, scale, u)?;
        let c = eval.continuity.unwrap_or(2);
        Ok(Evaluation {
            point: eval.point[..dim].to_vec(),
            d1: (c >= 1).then(|| eval.d1[..dim].to_vec()),
            d2: (c >= 2).then(|| eval.d2[..dim].to_vec()),
            domain: eval.domain,
            continuity: eval.continuity,
            derivative_status: eval.derivative_status,
            derivative_side: eval.derivative_side,
            rounding_bound: eval.rounding_bound,
        })
    }
    pub(super) fn evaluate_stack_validated(
        &self,
        domain: [f64; 2],
        scale: f64,
        u: f64,
    ) -> Result<StackCurveEval> {
        let b = stack_basis_validated(
            self.degree,
            &self.knots,
            self.control_points.len(),
            domain,
            u,
            self.periodic,
        )?;
        let dim = self.control_points[0].len();
        let first_off = b.basis[..b.len]
            .iter()
            .position(|v| *v > 0.)
            .ok_or_else(|| numeric_err("Empty basis support"))?;
        let origin = &self.control_points[b.base + first_off];
        let mut weight = CompensatedSum::new();
        let mut w1 = CompensatedSum::new();
        let mut w2 = CompensatedSum::new();
        let mut point_acc: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        let mut d1_acc: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        let mut d2_acc: [CompensatedSum; 3] = std::array::from_fn(|_| CompensatedSum::new());
        for j in 0..b.len {
            let i = b.base + j;
            let w = self.weights[i] / scale;
            weight.add(b.basis[j] * w);
            w1.add(b.d1[j] * w);
            w2.add(b.d2[j] * w);
            let cp = &self.control_points[i];
            for axis in 0..dim {
                let coordinate = cp[axis] - origin[axis];
                point_acc[axis].add(b.basis[j] * w * coordinate);
                d1_acc[axis].add(b.d1[j] * w * coordinate);
                d2_acc[axis].add(b.d2[j] * w * coordinate);
            }
        }
        let weight = weight.value();
        let w1 = w1.value();
        let w2 = w2.value();
        let rounding = rounding_bound(&point_acc[..dim], weight);
        numeric(
            weight > 0. && weight.is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        let mut point = [0.; 3];
        let mut d1 = [0.; 3];
        let mut d2 = [0.; 3];
        for axis in 0..dim {
            let rel = point_acc[axis].value() / weight;
            d1[axis] = (d1_acc[axis].value() - w1 * rel) / weight;
            d2[axis] = (d2_acc[axis].value() - 2. * w1 * d1[axis] - w2 * rel) / weight;
            point[axis] = rel + origin[axis];
        }
        numeric(
            point[..dim]
                .iter()
                .chain(&d1[..dim])
                .chain(&d2[..dim])
                .all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        Ok(StackCurveEval {
            point,
            d1,
            d2,
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

pub(super) struct StackCurveEval {
    pub(super) point: [f64; 3],
    pub(super) d1: [f64; 3],
    pub(super) d2: [f64; 3],
    pub(super) domain: [f64; 2],
    pub(super) continuity: Option<i32>,
    pub(super) derivative_status: &'static str,
    pub(super) derivative_side: &'static str,
    pub(super) rounding_bound: f64,
}

pub fn validate_periodic_knots(
    knots: &[f64],
    period_controls: usize,
    domain: [f64; 2],
) -> Result<()> {
    let period = domain[1] - domain[0];
    let scale = knots.iter().map(|k| k.abs()).fold(0., f64::max);
    for i in 0..knots.len() - period_controls {
        let difference = knots[i + period_controls] - knots[i];
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
    Ok(())
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

    /// Independent rational Bézier evaluation using de Casteljau in `(w·P,w)`.
    /// The curve must be one nonperiodic clamped 3D Bézier span. General
    /// B-spline curves should use `evaluate` or `evaluate_de_boor`; callers can
    /// obtain a Bézier span with `decompose` first.
    pub fn evaluate_bezier_de_casteljau(&self, u: f64) -> Result<crate::geometry::Point3> {
        use crate::geometry::{HomogeneousPoint3, de_casteljau_homogeneous};
        self.validate()?;
        check(
            self.control_points[0].len() == 3
                && !self.periodic
                && self.control_points.len() == self.degree + 1
                && self.knots[..=self.degree]
                    .iter()
                    .all(|k| *k == self.knots[0])
                && self.knots[self.degree + 1..]
                    .iter()
                    .all(|k| *k == self.knots[self.knots.len() - 1]),
            "de Casteljau requires one nonperiodic clamped 3D Bezier span",
        )?;
        let [start, end] = self.domain();
        check(
            u.is_finite() && u >= start && u <= end,
            "Parameter is outside the active knot domain",
        )?;
        let t = (u - start) / (end - start);
        let controls = self
            .control_points
            .iter()
            .zip(&self.weights)
            .map(|(point, &weight)| {
                HomogeneousPoint3::from_weighted(
                    crate::geometry::Point3::try_from_slice(point)?,
                    weight,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        de_casteljau_homogeneous(&controls, t)
    }
}

#[cfg(test)]
#[path = "tests/curve.rs"]
mod tests;

/// Exact degree-one line definition shared by analytic recognition clients.
pub fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve { degree: 1, knots: vec![0., 0., 1., 1.], control_points: vec![a, b], weights: vec![1., 1.], periodic: false }
}
