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
fn certify(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    axis: FiberAxis,
    limits: FiberLimits,
) -> Result<FiberReport> {
    check(
        (1..=100000).contains(&limits.max_rectangles)
            && limits
                .preconditioner
                .as_ref()
                .is_none_or(|r| r.iter().flatten().all(|v| v.is_finite())),
        "Fiber proof requires finite preconditioning and bounded positive rectangle work",
    )?;
    let free = axis.index();
    let surfaces = [source.first(), source.second()];
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (knots, d, n) = if k % 2 == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        check(
            domain[k].iter().all(|v| v.is_finite())
                && domain[k][0] >= knots[d]
                && domain[k][1] <= knots[n]
                && (if k == free {
                    domain[k][0] <= domain[k][1]
                } else {
                    domain[k][0] < domain[k][1]
                }),
            "Fiber box must cover noncollapsed unknowns inside the original surface domains",
        )?;
    }
    let mut out = FiberReport {
        status: FiberStatus::Unresolved,
        reason: "source-bounds-unproven",
        rectangles: 0,
        residual_box: None,
        root_image: None,
        contraction_upper: None,
        operator: None,
        support_regularity_proven: [false; 2],
    };
    // Closed original-domain claims require the actual evaluator's natural
    // endpoint span to be nonempty; an interior one-sided limit is insufficient.
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (knots, p, n) = if k % 2 == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        if (domain[k][0] == knots[p] && knots[p] == knots[p + 1])
            || (domain[k][1] == knots[n] && knots[n - 1] == knots[n])
        {
            out.reason = "original-endpoint-basis-unproven";
            return Ok(out);
        }
    }
    for k in 0..4 {
        let s = surfaces[k / 2];
        let (periodic, upper) = if k % 2 == 0 {
            (s.periodic_u, s.knots_u[s.control_points.len()])
        } else {
            (s.periodic_v, s.knots_v[s.control_points[0].len()])
        };
        if periodic
            && domain[k][1] == upper
            && crate::periodic_trim::supporting_seam_identity(s, k % 2).is_none()
        {
            out.reason = "periodic-upper-seam-unproven";
            return Ok(out);
        }
    }
    let mut budget = Budget {
        max: limits.max_rectangles,
        used: 0,
    };
    let run = (|| -> Result<()> {
        let Some((a, ja)) = evaluate(surfaces[0], [domain[0], domain[1]], true, &mut budget)?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let Some((b, jb)) = evaluate(surfaces[1], [domain[2], domain[3]], true, &mut budget)?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let residual = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        out.residual_box = Some(residual.map(pair));
        if residual.iter().any(|v| v.lo > 0. || v.hi < 0.) {
            out.status = FiberStatus::Excluded;
            out.reason = "original-box-separation";
            return Ok(());
        }
        let mut unknown = [0_usize; 3];
        let mut u_idx = 0;
        for k in 0..4 {
            if k != free {
                unknown[u_idx] = k;
                u_idx += 1;
            }
        }
        out.support_regularity_proven = [regular(ja), regular(jb)];
        let mut centers = [0.; 3];
        let mut scales = [0.; 3];
        let mut center_domain = domain;
        for i in 0..3 {
            let d = domain[unknown[i]];
            let c = 0.5 * d[0] + 0.5 * d[1];
            if c <= d[0] || c >= d[1] {
                out.reason = "unknown-center-unrepresentable";
                return Ok(());
            }
            centers[i] = c;
            scales[i] = (c - d[0]).max(d[1] - c);
            center_domain[unknown[i]] = [c; 2];
        }
        let mut j = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            for i in 0..3 {
                let a = unknown[i];
                j[k][i] = if a < 2 {
                    ja[k][a]
                } else {
                    I::point(0.).sub(jb[k][a - 2])?
                };
            }
        }
        let midpoint = j.map(|row| row.map(|v| 0.5 * v.lo + 0.5 * v.hi));
        let Some(r) = limits.preconditioner.or_else(|| inverse(midpoint)) else {
            out.reason = "preconditioner-unavailable";
            return Ok(());
        };
        let Some((a, _)) = evaluate(
            surfaces[0],
            [center_domain[0], center_domain[1]],
            false,
            &mut budget,
        )?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let Some((b, _)) = evaluate(
            surfaces[1],
            [center_domain[2], center_domain[3]],
            false,
            &mut budget,
        )?
        else {
            out.reason = "rectangle-budget";
            return Ok(());
        };
        let f = [a[0].sub(b[0])?, a[1].sub(b[1])?, a[2].sub(b[2])?];
        let mut image = [I::point(0.); 3];
        let mut magnitudes = [[0.; 3]; 3];
        let mut q = 0f64;
        for i in 0..3 {
            let mut base = I::point(centers[i]);
            for k in 0..3 {
                base = base.sub(I::point(r[i][k]).mul(f[k])?)?;
            }
            let mut row = I::point(0.);
            for c in 0..3 {
                let mut v = I::point(if i == c { 1. } else { 0. });
                for k in 0..3 {
                    v = v.sub(I::point(r[i][k]).mul(j[k][c])?)?;
                }
                let magnitude = v.lo.abs().max(v.hi.abs());
                magnitudes[i][c] = magnitude;
                row = row
                    .add(I::point(magnitude).mul(I::point(scales[c]).div(I::point(scales[i]))?)?)?;
                let delta = I::new(domain[unknown[c]][0], domain[unknown[c]][1])?
                    .sub(I::point(centers[c]))?;
                base = base.add(v.mul(delta)?)?;
            }
            q = q.max(row.hi);
            image[i] = base;
        }
        if q >= 1. {
            let (new_scales, new_q) = rebalance_norm(magnitudes, scales, q)?;
            scales = new_scales;
            q = new_q;
        }
        // Keep the existing inexpensive enclosure when it qualifies. For
        // rotated correlated coordinates, a fresh original-net R*F enclosure
        // can avoid artificial cancellation loss in R*[DF]. Both constructions
        // implement the SAME fixed preconditioner; no geometry/tolerance edit.
        let included = |image: &[I; 3]| {
            (0..3)
                .all(|i| domain[unknown[i]][0] < image[i].lo && image[i].hi < domain[unknown[i]][1])
        };
        if (q >= 1. || !included(&image)) && budget.used < budget.max {
            let origin = &surfaces[0].control_points[0][0];
            let Some((_, pa)) = evaluate_projected(
                surfaces[0],
                [domain[0], domain[1]],
                r,
                origin,
                true,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((_, pb)) = evaluate_projected(
                surfaces[1],
                [domain[2], domain[3]],
                r,
                origin,
                true,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((ca, _)) = evaluate_projected(
                surfaces[0],
                [center_domain[0], center_domain[1]],
                r,
                origin,
                false,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let Some((cb, _)) = evaluate_projected(
                surfaces[1],
                [center_domain[2], center_domain[3]],
                r,
                origin,
                false,
                &mut budget,
            )?
            else {
                out.reason = "rectangle-budget";
                return Ok(());
            };
            let mut candidate = [I::point(0.); 3];
            let mut contraction = 0_f64;
            let mut candidate_magnitudes = [[0.; 3]; 3];
            for i in 0..3 {
                let mut base = I::point(centers[i]).sub(ca[i].sub(cb[i])?)?;
                let mut row = I::point(0.);
                for c in 0..3 {
                    let a = unknown[c];
                    let j = if a < 2 {
                        pa[i][a]
                    } else {
                        I::point(0.).sub(pb[i][a - 2])?
                    };
                    let v = I::point(if i == c { 1. } else { 0. }).sub(j)?;
                    let magnitude = v.lo.abs().max(v.hi.abs());
                    candidate_magnitudes[i][c] = magnitude;
                    row = row.add(
                        I::point(magnitude).mul(I::point(scales[c]).div(I::point(scales[i]))?)?,
                    )?;
                    base = base.add(
                        v.mul(I::new(domain[a][0], domain[a][1])?.sub(I::point(centers[c]))?)?,
                    )?;
                }
                candidate[i] = base;
                contraction = contraction.max(row.hi);
            }
            let (candidate_scales, contraction) = if contraction >= 1. {
                rebalance_norm(candidate_magnitudes, scales, contraction)?
            } else {
                (scales, contraction)
            };
            // Both images enclose the SAME fixed-R Krawczyk map on the
            // original box. Their intersection is a valid map enclosure even
            // before strict inclusion succeeds; retaining it lets contraction
            // refine an inconclusive box without inventing a sampled seed.
            let combined = [0, 1, 2].map(|i| {
                (
                    image[i].lo.max(candidate[i].lo),
                    image[i].hi.min(candidate[i].hi),
                )
            });
            if combined.iter().all(|&(lo, hi)| lo <= hi) {
                for i in 0..3 {
                    image[i] = I::new(combined[i].0, combined[i].1)?;
                }
            }
            if contraction < q {
                q = contraction;
                scales = candidate_scales;
                magnitudes = candidate_magnitudes;
            }
        }
        if q < 1. {
            // A nearly constant unknown has a tiny box radius. Reprove a norm
            // that avoids artificial physical residual amplification; keeping
            // a better verified world-error bound does not change inclusion.
            let size = scales.iter().copied().fold(0_f64, f64::max);
            let equal = [size; 3];
            let equal_q = norm_upper(magnitudes, equal)?;
            let (balanced, balanced_q) = rebalance_norm(magnitudes, equal, equal_q)?;
            if balanced_q < 1.
                && world_gain(r, ja, &unknown, free, balanced, balanced_q)?
                    < world_gain(r, ja, &unknown, free, scales, q)?
            {
                scales = balanced;
                q = balanced_q;
            }
        }
        out.root_image = Some(image.map(pair));
        out.contraction_upper = Some(q);
        if (0..3)
            .any(|i| image[i].hi < domain[unknown[i]][0] || image[i].lo > domain[unknown[i]][1])
        {
            out.status = FiberStatus::Excluded;
            out.reason = "original-krawczyk-exclusion";
            return Ok(());
        }
        if q >= 1. {
            out.reason = "contraction-unproven";
            return Ok(());
        }
        if !(0..3)
            .all(|i| domain[unknown[i]][0] < image[i].lo && image[i].hi < domain[unknown[i]][1])
        {
            out.reason = "uniform-inclusion-unproven";
            return Ok(());
        }
        out.operator = Some(FiberOperator {
            preconditioner: r,
            scales,
            first_jacobian: ja.map(|row| row.map(pair)),
        });
        out.status = FiberStatus::UniformUniqueRoot;
        out.reason = "uniform-original-krawczyk-fiber";
        Ok(())
    })();
    out.rectangles = budget.used;
    if run.is_err() {
        out.status = FiberStatus::Unresolved;
        out.reason = "source-operator-unavailable";
    }
    Ok(out)
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
mod operator_receipt_tests {
    use super::*;
    #[test]
    fn weighted_norm_qualifies_triangular_coupling_but_not_expansion() {
        let triangular = [[0.1, 3., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
        let (scales, q) = rebalance_norm(triangular, [1.; 3], 3.1).unwrap();
        assert!(q < 1. && scales[0] > scales[1]);
        for i in 0..3 {
            let actual: f64 = (0..3)
                .map(|j| triangular[i][j] * scales[j] / scales[i])
                .sum();
            assert!(actual <= q);
        }
        let expanding = [[1.01, 3., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
        let (_, q) = rebalance_norm(expanding, [1.; 3], 4.01).unwrap();
        assert!(q >= 1.01);
    }
    #[test]
    fn retained_original_operator_scales_and_jacobian_are_fresh_authority() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            control_points: vec![
                vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
                vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
            ],
            ..a.clone()
        };
        let source = FiberSources::new(&a, &b).unwrap();
        let f = source
            .certify(
                [[0., 1.], [0.25, 0.75], [0.125, 0.625], [0.25, 0.75]],
                FiberAxis::FirstU,
                FiberLimits::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        f.reverify().unwrap();
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().preconditioner[0][1] += 1.;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().scales[0] *= 2.;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().first_jacobian[1][1][1] += 0.1;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.contraction_upper = Some(0.75);
        assert!(bad.reverify().is_err());
        let mut bad = f;
        bad.report.operator = None;
        assert!(bad.reverify().is_err());
    }
}
