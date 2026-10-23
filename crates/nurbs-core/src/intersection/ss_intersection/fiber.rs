//! Uniform three-variable Krawczyk fibers of the ORIGINAL intersection system.
//! A qualified local fiber is not complete surface/surface branch coverage.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiberAxis {
    FirstU,
    FirstV,
    SecondU,
    SecondV,
}
impl FiberAxis {
    pub(super) fn index(self) -> usize {
        match self {
            Self::FirstU => 0,
            Self::FirstV => 1,
            Self::SecondU => 2,
            Self::SecondV => 3,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FiberLimits {
    /// Shared original rectangles for full boxes and center/free-parameter boxes.
    pub max_rectangles: usize,
    /// Proposal only; the actual outward operator and contraction must pass.
    pub preconditioner: Option<[[f64; 3]; 3]>,
}
impl Default for FiberLimits {
    fn default() -> Self {
        Self {
            max_rectangles: 10000,
            preconditioner: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiberStatus {
    Excluded,
    UniformUniqueRoot,
    Unresolved,
}
/// Actual outward operator context retained only after uniform root admission.
#[derive(Clone, Debug, PartialEq)]
pub struct FiberOperator {
    pub(super) preconditioner: [[f64; 3]; 3],
    pub(super) scales: [f64; 3],
    pub(super) first_jacobian: [[[f64; 2]; 2]; 3],
}
impl FiberOperator {
    /// Verified proposal R, not a claim of exact Jacobian inversion.
    pub fn preconditioner(&self) -> [[f64; 3]; 3] {
        self.preconditioner
    }
    pub fn scales(&self) -> [f64; 3] {
        self.scales
    }
    pub fn first_jacobian(&self) -> [[[f64; 2]; 2]; 3] {
        self.first_jacobian
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FiberReport {
    pub status: FiberStatus,
    pub reason: &'static str,
    pub rectangles: usize,
    /// Contains F=S1-S2 on the complete supplied four-parameter product.
    pub residual_box: Option<[[f64; 2]; 3]>,
    /// Actual roots lie in this image, ordered by the three nonfree axes.
    pub root_image: Option<[[f64; 2]; 3]>,
    pub contraction_upper: Option<f64>,
    pub operator: Option<FiberOperator>,
    /// Separate original-span rank evidence. False means unproven, not
    /// singular. A parameter fiber alone need not be a physical curve.
    pub support_regularity_proven: [bool; 2],
}
#[derive(Clone, Debug)]
struct Pair {
    first: Surface,
    second: Surface,
}
#[derive(Clone, Debug)]
pub struct FiberSources {
    pair: Arc<Pair>,
}
impl FiberSources {
    pub fn new(first: &Surface, second: &Surface) -> Result<Self> {
        first.validate()?;
        second.validate()?;
        Ok(Self {
            pair: Arc::new(Pair {
                first: first.clone(),
                second: second.clone(),
            }),
        })
    }
    pub fn first(&self) -> &Surface {
        &self.pair.first
    }
    pub fn second(&self) -> &Surface {
        &self.pair.second
    }
    /// Refine only the unknown coordinates using fresh original Krawczyk root
    /// images. Every original root remains in the restricted box; the complete
    /// free interval is unchanged. No numerical seed or sampled range excludes
    /// a root. A successful fiber names its ACTUAL contracted domain.
    pub fn certify_contracted(
        &self,
        mut domain: [[f64; 2]; 4],
        axis: FiberAxis,
        max_rectangles: usize,
    ) -> Result<FiberContraction> {
        check(
            (1..=100000).contains(&max_rectangles)
                && [self.first(), self.second()].iter().all(|s| {
                    s.control_points
                        .len()
                        .checked_mul(s.control_points[0].len())
                        .is_some_and(|n| n <= 4096)
                }),
            "Need bounded original contraction work",
        )?;
        let mut out = FiberContraction {
            fiber: None,
            rectangles: 0,
            steps: 0,
            domain,
            root_image: None,
            reason: "root-contraction-unproven",
        };
        let free = axis.index();
        for _ in 0..16 {
            if out.rectangles == max_rectangles {
                out.reason = "rectangle-budget";
                return Ok(out);
            }
            let fresh = self.certify(
                domain,
                axis,
                FiberLimits {
                    max_rectangles: max_rectangles - out.rectangles,
                    preconditioner: None,
                },
            )?;
            out.rectangles += fresh.report.rectangles;
            out.steps += 1;
            out.domain = domain;
            out.reason = fresh.report.reason;
            out.root_image = fresh.report.root_image;
            if fresh.fiber.is_some() {
                out.fiber = fresh.fiber;
                return Ok(out);
            }
            if fresh.report.status == FiberStatus::Excluded {
                return Ok(out);
            }
            let Some(image) = fresh.report.root_image else {
                return Ok(out);
            };
            let mut next = domain;
            let mut changed = false;
            let mut unknown = 0;
            for a in 0..4 {
                if a == free {
                    continue;
                }
                // Padding is a proposal for a useful strict inclusion box.
                // Outward image bounds remain enclosed after clipping to the
                // current original box, so no actual root is discarded.
                let padding = 0.1 * (domain[a][1] - domain[a][0]);
                next[a] = [
                    domain[a][0].max(crate::intersection::next_down(image[unknown][0] - padding)),
                    domain[a][1].min(crate::intersection::next_up(image[unknown][1] + padding)),
                ];
                if next[a][0] >= next[a][1] {
                    return Ok(out);
                }
                changed |= next[a] != domain[a];
                unknown += 1;
            }
            if !changed {
                return Ok(out);
            }
            domain = next;
        }
        Ok(out)
    }
    /// A uniform local proof for every value of the chosen free parameter.
    /// No tolerance, sampled seed or continuation flag authorizes a root.
    pub fn certify(
        &self,
        domain: [[f64; 2]; 4],
        axis: FiberAxis,
        limits: FiberLimits,
    ) -> Result<FiberResult> {
        let report = certify(self, domain, axis, limits)?;
        let fiber = (report.status == FiberStatus::UniformUniqueRoot).then(|| QualifiedFiber {
            source: self.clone(),
            domain,
            axis,
            limits,
            report: report.clone(),
        });
        Ok(FiberResult { report, fiber })
    }
}
pub struct FiberContraction {
    fiber: Option<QualifiedFiber>,
    rectangles: usize,
    steps: usize,
    domain: [[f64; 2]; 4],
    root_image: Option<[[f64; 2]; 3]>,
    reason: &'static str,
}
impl FiberContraction {
    /// Last fresh original root enclosure, ordered by nonfree axes. This is
    /// proposal data when no qualified fiber exists, not an admission receipt.
    pub fn root_image(&self) -> Option<[[f64; 2]; 3]> {
        self.root_image
    }
    pub fn fiber(&self) -> Option<&QualifiedFiber> {
        self.fiber.as_ref()
    }
    pub fn into_fiber(self) -> Option<QualifiedFiber> {
        self.fiber
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}
pub struct FiberResult {
    report: FiberReport,
    fiber: Option<QualifiedFiber>,
}
impl FiberResult {
    pub fn report(&self) -> &FiberReport {
        &self.report
    }
    pub fn fiber(&self) -> Option<&QualifiedFiber> {
        self.fiber.as_ref()
    }
    pub fn into_fiber(self) -> Option<QualifiedFiber> {
        self.fiber
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedFiber {
    source: FiberSources,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: FiberLimits,
    report: FiberReport,
}
impl QualifiedFiber {
    pub(super) fn limits(&self) -> FiberLimits {
        self.limits
    }
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn domain(&self) -> [[f64; 2]; 4] {
        self.domain
    }
    pub fn free_axis(&self) -> FiberAxis {
        self.axis
    }
    pub fn report(&self) -> &FiberReport {
        &self.report
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = certify(&self.source, self.domain, self.axis, self.limits)?;
        check(
            fresh.status == FiberStatus::UniformUniqueRoot && fresh == self.report,
            "Original intersection fiber or its operator receipt changed",
        )
    }
    /// Fresh specialization inside the SAME original parameter box. This
    /// does not extrapolate beyond the complete qualified free interval.
    pub fn at(&self, parameter: f64, limits: FiberLimits) -> Result<FiberResult> {
        let free = self.axis.index();
        check(
            parameter.is_finite()
                && self.domain[free][0] <= parameter
                && parameter <= self.domain[free][1],
            "Fiber query is outside its original free-parameter interval",
        )?;
        let mut domain = self.domain;
        domain[free] = [parameter; 2];
        self.source.certify(domain, self.axis, limits)
    }
}
struct Budget {
    max: usize,
    used: usize,
}
type Evaluation = ([I; 3], [[I; 2]; 3]);
fn evaluate(
    s: &Surface,
    domain: [[f64; 2]; 2],
    jacobian: bool,
    budget: &mut Budget,
) -> Result<Option<Evaluation>> {
    let mut values = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    let mut j = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    for u in s.degree_u..s.control_points.len() {
        if s.knots_u[u] == s.knots_u[u + 1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] == s.knots_v[v + 1] {
                continue;
            }
            let section = [
                [
                    domain[0][0].max(s.knots_u[u]),
                    domain[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    domain[1][0].max(s.knots_v[v]),
                    domain[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if (0..2).any(|a| {
                section[a][0] > section[a][1]
                    || (section[a][0] == section[a][1] && domain[a][0] < domain[a][1])
            }) {
                continue;
            }
            if budget.used == budget.max {
                return Ok(None);
            }
            budget.used += 1;
            let next = crate::surface_distance::enclosure(s, [u, v], section)?;
            for k in 0..3 {
                values[k].lo = values[k].lo.min(next[k].lo);
                values[k].hi = values[k].hi.max(next[k].hi);
            }
            if jacobian {
                let next = crate::surface_injectivity::section_jacobian(s, [u, v], section)?;
                for k in 0..3 {
                    for a in 0..2 {
                        j[k][a].lo = j[k][a].lo.min(next[k][a].lo);
                        j[k][a].hi = j[k][a].hi.max(next[k][a].hi);
                    }
                }
            }
        }
    }
    Ok(Some((values, j)))
}
// Apply R to original homogeneous interval coefficients BEFORE hulls lose
// coordinate correlation. No rounded transformed surface is materialized.
fn evaluate_projected(
    s: &Surface,
    domain: [[f64; 2]; 2],
    r: [[f64; 3]; 3],
    origin: &[f64],
    jacobian: bool,
    budget: &mut Budget,
) -> Result<Option<Evaluation>> {
    let empty = I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    };
    let mut values = [empty; 3];
    let mut derivatives = [[empty; 2]; 3];
    for u in s.degree_u..s.control_points.len() {
        if s.knots_u[u] == s.knots_u[u + 1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] == s.knots_v[v + 1] {
                continue;
            }
            let section = [
                [
                    domain[0][0].max(s.knots_u[u]),
                    domain[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    domain[1][0].max(s.knots_v[v]),
                    domain[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if (0..2).any(|a| {
                section[a][0] > section[a][1]
                    || (section[a][0] == section[a][1] && domain[a][0] < domain[a][1])
            }) {
                continue;
            }
            if budget.used == budget.max {
                return Ok(None);
            }
            budget.used += 1;
            let mut net = match crate::curve_surface_composition::surface_net_on_bernstein(
                s,
                [u, v],
                section,
            )? {
                Some(net) => net,
                None => crate::curve_surface_composition::surface_net_on(s, [u, v], section)?,
            };
            for h in net.iter_mut().flatten() {
                let centered = [
                    h[0].sub(I::point(origin[0]).mul(h[3])?)?,
                    h[1].sub(I::point(origin[1]).mul(h[3])?)?,
                    h[2].sub(I::point(origin[2]).mul(h[3])?)?,
                ];
                for k in 0..3 {
                    h[k] = I::point(0.);
                    for a in 0..3 {
                        h[k] = h[k].add(I::point(r[k][a]).mul(centered[a])?)?;
                    }
                }
                for k in 0..3 {
                    let q = h[k].div(h[3])?;
                    values[k].lo = values[k].lo.min(q.lo);
                    values[k].hi = values[k].hi.max(q.hi);
                }
            }
            if jacobian {
                let next = crate::surface_injectivity::jacobian_from_net(
                    &net,
                    [s.degree_u, s.degree_v],
                    section,
                )?;
                for k in 0..3 {
                    for a in 0..2 {
                        derivatives[k][a].lo = derivatives[k][a].lo.min(next[k][a].lo);
                        derivatives[k][a].hi = derivatives[k][a].hi.max(next[k][a].hi);
                    }
                }
            }
        }
    }
    Ok(Some((values, derivatives)))
}
fn inverse(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let mut a = [[0.; 6]; 3];
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] = m[i][j];
        }
        a[i][3 + i] = 1.;
    }
    for col in 0..3 {
        let pivot = (col..3).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col] == 0. || !a[pivot][col].is_finite() {
            return None;
        }
        a.swap(col, pivot);
        let d = a[col][col];
        for j in 0..6 {
            a[col][j] /= d;
        }
        for i in 0..3 {
            if i != col {
                let factor = a[i][col];
                for j in 0..6 {
                    a[i][j] -= factor * a[col][j];
                }
            }
        }
    }
    let out = std::array::from_fn(|i| std::array::from_fn(|j| a[i][3 + j]));
    out.iter().flatten().all(|v| v.is_finite()).then_some(out)
}
fn pair(v: I) -> [f64; 2] {
    [v.lo, v.hi]
}
fn regular(j: [[I; 2]; 3]) -> bool {
    (0..3).any(|k| {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        let n = (|| -> Result<I> { j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?) })();
        n.is_ok_and(|n| n.lo > 0. || n.hi < 0.)
    })
}


/// Positive weights are proposals only. Every accepted row sum is recomputed
/// outward from the original operator bounds; no spectral-radius estimate or
/// sampled Jacobian grants contraction. The actual Krawczyk image is unchanged.
fn rebalance_norm(m: [[f64; 3]; 3], initial: [f64; 3], initial_q: f64) -> Result<([f64; 3], f64)> {
    let mut weights = [1.; 3];
    let mut best = (initial, initial_q);
    for _ in 0..16 {
        let next = [0, 1, 2].map(|i| {
            1. + (0..3)
                .map(|j| m[i][j] * (initial[j] / initial[i]) * weights[j])
                .sum::<f64>()
        });
        let scales = [0, 1, 2].map(|i| initial[i] * next[i]);
        if scales.iter().any(|s| !s.is_finite() || *s <= 0.) {
            break;
        }
        let q = norm_upper(m, scales)?;
        if q < best.1 {
            best = (scales, q);
        }
        weights = next;
    }
    Ok(best)
}

fn norm_upper(m: [[f64; 3]; 3], scales: [f64; 3]) -> Result<f64> {
    let mut q = 0_f64;
    for i in 0..3 {
        let mut row = I::point(0.);
        for j in 0..3 {
            row = row.add(I::point(m[i][j]).mul(I::point(scales[j]).div(I::point(scales[i]))?)?)?;
        }
        q = q.max(row.hi);
    }
    Ok(q)
}
fn world_gain(
    r: [[f64; 3]; 3],
    ja: [[I; 2]; 3],
    unknown: &[usize],
    free: usize,
    scales: [f64; 3],
    q: f64,
) -> Result<f64> {
    let mut gain = 0_f64;
    for i in 0..3 {
        let mut row = I::point(0.);
        for value in r[i] {
            row = row.add(I::point(value.abs()))?;
        }
        gain = gain.max(row.div(I::point(scales[i]))?.hi);
    }
    let mut world = I::point(0.);
    for row in ja {
        for a in 0..2 {
            if a == free {
                continue;
            }
            let index = unknown.iter().position(|&n| n == a).unwrap();
            world = world.add(
                I::point(row[a].lo.abs().max(row[a].hi.abs())).mul(I::point(scales[index]))?,
            )?;
        }
    }
    Ok(world
        .mul(I::point(gain))?
        .div(I::point(1.).sub(I::point(q))?)?
        .hi)
}

#[cfg(test)]
#[path = "fiber/tests/operator_receipt_tests_integration.rs"]
mod operator_receipt_tests;

#[path = "fiber/fiber_certificate.rs"]
mod fiber_certificate;
use fiber_certificate::certify;
