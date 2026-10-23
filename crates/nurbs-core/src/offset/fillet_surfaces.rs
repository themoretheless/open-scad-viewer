//! Surface-pair fillet surfaces (no B-rep): rolling-ball, variable-radius,
//! chordal, G2 and hold-line fillets between two NURBS support surfaces.
//!
//! The ball-center spine is traced as the intersection of the two offset
//! surfaces: seed pairs come from sampling [`super::surface_offset::offset`]
//! results, then a least-norm Newton predictor-corrector marches the exact
//! implicit offset map `A(u,v) + r·sa·nA − B(s,t) − r·sb·nB = 0` with a
//! curvature-adaptive step. Cross sections are rational circular proposals with original coordinate-plane
//! contact constraints (rolling-ball / variable / chordal), G2 quintic Bézier sections, or
//! tangent cubic hold-line sections, skinned by
//! [`crate::natural_loft::interpolate`]. Variable G2 proposals on curved supports
//! use bounded quintic station interpolation with matched UV proposals. Neither
//! interpolation method certifies contact, radius or whole-seam G2.
//!
//! Graceful failure: geometric breakdowns never panic and never abort with a
//! bare error; they return a [`FilletReport`] whose `failure` field carries
//! the spine parameter `t` and the reason. Only invalid *inputs* (non-finite
//! radii, bad options, invalid surfaces) produce `Err` via `check`.
use crate::{
    Result, check,
    curve::Curve,
    foundation::guards::{Budget, require_finite_at, require_finite_f64, require_finite_point},
    natural_loft,
    offset::surface_offset::{offset, offset_validity},
    surface::Surface,
};
use math_core::{dot, norm};
mod station_hermite;
mod station_radius;
pub use station_radius::{StationRadiusReport, linear_at_stations};

/// Seed sampling grid side per offset surface.
const SEED_GRID: usize = 9;
/// Newton corrector iteration budget.
const NEWTON_ITERS: usize = 32;
/// Skin section count cap (natural loft admits 2..=11 sections).
const MAX_SECTIONS: usize = 11;
/// Law sampling budget for range/reachability estimates.
const LAW_SAMPLES: usize = 64;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    (n.is_finite() && n > 1e-300).then(|| scale(a, 1. / n))
}
fn dot4(a: [f64; 4], b: [f64; 4]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}
fn norm4(a: [f64; 4]) -> f64 {
    dot4(a, a).sqrt()
}
/// Robust angle between two nonzero vectors, in `[0, π]`.
fn angle_between(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}

fn domain(surface: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
        ],
    )
}

/// Evaluate point + reliable unit normal; None on poles/singular charts and
/// outside the active domain (domain exits are soft signals for marching,
/// never propagated errors).
fn point_normal(surface: &Surface, u: f64, v: f64) -> Result<Option<([f64; 3], [f64; 3])>> {
    let e = match surface.evaluate(u, v) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    let Some(n) = e.unit_normal() else { return Ok(None) };
    let Some((du, dv)) = e.first_derivatives() else {
        return Ok(None);
    };
    let (su, sv) = (norm(du), norm(dv));
    if su <= 0. || sv <= 0. || su.min(sv) <= 1e-9 * su.max(sv) {
        return Ok(None);
    }
    if norm(cross(du, dv)) <= 1e-6 * su * sv {
        return Ok(None);
    }
    Ok(Some((e.point, n)))
}

/// Solve a symmetric-in-practice 3x3 system; None when singular.
fn solve3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let mut a = matrix;
    let mut b = rhs;
    for column in 0..3 {
        let pivot = (column..3).max_by(|&r, &s| a[r][column].abs().total_cmp(&a[s][column].abs()))?;
        if a[pivot][column].abs() <= 1e-14 {
            return None;
        }
        a.swap(column, pivot);
        b.swap(column, pivot);
        let divisor = a[column][column];
        for c in column..3 {
            a[column][c] /= divisor;
        }
        b[column] /= divisor;
        for r in 0..3 {
            if r == column {
                continue;
            }
            let factor = a[r][column];
            for c in column..3 {
                a[r][c] -= factor * a[column][c];
            }
            b[r] -= factor * b[column];
        }
    }
    Some(b)
}

mod radius_law;
pub use radius_law::{RadiusLaw};



/// Natural cubic spline scalar interpolation (Thomas solve, budgeted by the
/// 64-point validation cap).
fn natural_cubic(points: &[(f64, f64)], t: f64) -> f64 {
    let n = points.len();
    let clamped = t.clamp(points[0].0, points[n - 1].0);
    // Second derivatives m[i]; natural boundary m[0] = m[n-1] = 0. Thomas
    // forward sweep over the interior rows of the tridiagonal system.
    let mut m_stack = [0.; 64];
    let mut cp_stack = [0.; 64];
    let mut dp_stack = [0.; 64];
    let mut m_heap;
    let mut cp_heap;
    let mut dp_heap;
    let interior = n.saturating_sub(2);
    let (m, cp, dp): (&mut [f64], &mut [f64], &mut [f64]) = if n <= 64 {
        (&mut m_stack[..n], &mut cp_stack[..interior], &mut dp_stack[..interior])
    } else {
        m_heap = vec![0.; n];
        cp_heap = vec![0.; interior];
        dp_heap = vec![0.; interior];
        (&mut m_heap, &mut cp_heap, &mut dp_heap)
    };
    for row in 0..interior {
        let i = row + 1;
        let h0 = points[i].0 - points[i - 1].0;
        let h1 = points[i + 1].0 - points[i].0;
        let rhs = 6. * ((points[i + 1].1 - points[i].1) / h1 - (points[i].1 - points[i - 1].1) / h0);
        let denom = if row == 0 {
            2. * (h0 + h1)
        } else {
            2. * (h0 + h1) - h0 * cp[row - 1]
        };
        cp[row] = h1 / denom;
        dp[row] = (rhs - if row == 0 { 0. } else { h0 * dp[row - 1] }) / denom;
    }
    for row in (0..interior).rev() {
        m[row + 1] = dp[row] - cp[row] * m[row + 2];
    }
    let i = (1..n).find(|&i| clamped <= points[i].0).unwrap_or(n - 1);
    let h = points[i].0 - points[i - 1].0;
    let a = (points[i].0 - clamped) / h;
    let b = 1. - a;
    a * points[i - 1].1 + b * points[i].1 + ((a.powi(3) - a) * m[i - 1] + (b.powi(3) - b) * m[i]) * h * h / 6.
}

/// Shared fillet options.
#[derive(Clone, Copy, Debug)]
pub struct FilletOptions {
    /// Side of surface A the ball rolls on: `+1.` follows `S_u × S_v`, `-1.`
    /// the opposite side. Only the sign is used.
    pub side_a: f64,
    /// Side of surface B the ball rolls on.
    pub side_b: f64,
    /// Spatial Newton/section tolerance (absolute, model units).
    pub tolerance: f64,
    /// Total marching step budget across both directions.
    pub max_march_steps: usize,
    /// Skin section count cap (clamped to `2..=11`).
    pub max_sections: usize,
}

impl Default for FilletOptions {
    fn default() -> Self {
        Self {
            side_a: 1.,
            side_b: 1.,
            tolerance: 1e-7,
            max_march_steps: 256,
            max_sections: 9,
        }
    }
}

impl FilletOptions {
    fn validated(&self) -> Result<(f64, f64)> {
        require_finite_f64(self.side_a, "side_a")?;
        require_finite_f64(self.side_b, "side_b")?;
        check(
            self.side_a != 0. && self.side_b != 0.,
            "Fillet side selectors must be nonzero",
        )?;
        require_finite_f64(self.tolerance, "tolerance")?;
        check(
            self.tolerance > 0.,
            "Fillet tolerance must be positive",
        )?;
        check(
            (1..=4096).contains(&self.max_march_steps),
            "Fillet march step budget must be in 1..=4096",
        )?;
        Ok((self.side_a.signum(), self.side_b.signum()))
    }
}

/// One spine station of a built fillet.
#[derive(Clone, Copy, Debug)]
pub struct SpineSample {
    /// Normalized arc-length parameter in `[0, 1]`.
    pub t: f64,
    /// Ball center on the spine.
    pub center: [f64; 3],
    /// Tangency point on support A.
    pub contact_a: [f64; 3],
    /// Tangency point on support B.
    pub contact_b: [f64; 3],
    /// Local section radius.
    pub radius: f64,
    /// Surface A parameter of the contact.
    pub uv_a: [f64; 2],
    /// Surface B parameter of the contact.
    pub uv_b: [f64; 2],
}

/// Why a fillet could not be produced.
#[derive(Clone, Debug)]
pub enum FailureReason {
    /// No converging offset-intersection seed was found.
    SeedNotFound,
    /// The predictor-corrector march stopped converging (or the Jacobian went
    /// singular) before the spine gained any extent.
    MarchingDiverged,
    /// The requested radius is below the numerical minimum for this pair.
    RadiusBelowMinimum { radius: f64, minimum: f64 },
    /// The requested radius exceeds the certified fold-free offset reach of a
    /// support: the ball cannot stay tangent there.
    RadiusExceedsReachability { radius: f64, limiting: f64 },
    /// The section would self-intersect or collapse (contact angle out of
    /// `(0, π)`): the radius is wrong for the local dihedral.
    SectionSelfIntersection,
    /// Final skinning of the sections failed.
    AssemblyFailed,
}

/// Detailed geometric failure with spine position context.
#[derive(Clone, Debug)]
pub struct FilletFailure {
    pub reason: FailureReason,
    /// Normalized spine parameter where the failure surfaced, when known.
    pub t: Option<f64>,
    pub detail: String,
}

/// Fillet outcome: a built surface, or a detailed failure, plus evidence.
pub struct FilletReport {
    /// Authored UV contact proposals using the same station interpolation as the skin.
    /// These are unqualified until the original-support lifts pass.
    pub contact_pcurves: Option<[Curve; 2]>,
    /// The fillet surface (`None` iff `failure` is set).
    pub surface: Option<Surface>,
    pub failure: Option<FilletFailure>,
    /// Spine stations used for the skin (empty on early failure).
    pub spine: Vec<SpineSample>,
    /// Sampled `[min, max]` of the section radius along the edge.
    pub radius_range: [f64; 2],
    /// Non-fatal diagnostics: reachability excess, polish fallbacks, etc.
    pub warnings: Vec<String>,
    /// Sampled estimate of the skin's deviation from the true pipe of radius
    /// `r(t)` around the spine. Section and inter-section radius accuracy require
    /// an independent whole-skin coefficient proof.
    pub skin_deviation_estimate: f64,
    /// True when the spine closed into a loop and was cut at the seed.
    pub closed_spine: bool,
}

impl FilletReport {
    fn failed(reason: FailureReason, t: Option<f64>, detail: impl Into<String>, warnings: Vec<String>) -> Self {
        Self {
            surface: None,
            contact_pcurves: None,
            failure: Some(FilletFailure { reason, t, detail: detail.into() }),
            spine: Vec::new(),
            radius_range: [0.; 2],
            warnings,
            skin_deviation_estimate: 0.,
            closed_spine: false,
        }
    }
}

/// Chordal fillet outcome with the recovered radius/dihedral evidence.
pub struct ChordalFilletReport {
    pub fillet: FilletReport,
    /// `[min, max]` of the recovered radius `c / (2 sin(φ/2))` along the edge.
    pub radius_range: [f64; 2],
    /// `[min, max]` of the local dihedral angle `φ` (radians) between the
    /// oriented support normals at the contact points.
    pub dihedral_range: [f64; 2],
}

struct RawPoint {
    x: [f64; 4],
    center: [f64; 3],
    contact_a: [f64; 3],
    contact_b: [f64; 3],
    s: f64,
}

/// The implicit offset pair `F(u,v,s,t) = A + r·sa·nA − B − r·sb·nB`.
struct Marcher<'a> {
    a: &'a Surface,
    b: &'a Surface,
    sa: f64,
    sb: f64,
    tol: f64,
    dom_a: ([f64; 2], [f64; 2]),
    dom_b: ([f64; 2], [f64; 2]),
}

impl<'a> Marcher<'a> {
    fn new(a: &'a Surface, b: &'a Surface, sa: f64, sb: f64, tol: f64) -> Self {
        Self { a, b, sa, sb, tol, dom_a: domain(a), dom_b: domain(b) }
    }

    fn value(&self, x: &[f64; 4], r: f64) -> Result<Option<[f64; 3]>> {
        let Some((pa, na)) = point_normal(self.a, x[0], x[1])? else { return Ok(None) };
        let Some((pb, nb)) = point_normal(self.b, x[2], x[3])? else { return Ok(None) };
        Ok(Some(sub(add(pa, scale(na, r * self.sa)), add(pb, scale(nb, r * self.sb)))))
    }

    /// Central-difference 3×4 Jacobian of the offset map.
    fn jacobian(&self, x: &[f64; 4], r: f64) -> Result<Option<[[f64; 4]; 3]>> {
        let lengths = [
            self.dom_a.0[1] - self.dom_a.0[0],
            self.dom_a.1[1] - self.dom_a.1[0],
            self.dom_b.0[1] - self.dom_b.0[0],
            self.dom_b.1[1] - self.dom_b.1[0],
        ];
        let mut j = [[0.; 4]; 3];
        for (column, &length) in lengths.iter().enumerate() {
            let h = (1e-6 * length.max(1e-3)).max(1e-9);
            let mut xp = *x;
            let mut xm = *x;
            xp[column] += h;
            xm[column] -= h;
            let (fp, fm) = match (self.value(&xp, r)?, self.value(&xm, r)?) {
                (Some(fp), Some(fm)) => (fp, fm),
                _ => {
                    let Some(f0) = self.value(x, r)? else { return Ok(None) };
                    match (self.value(&xp, r)?, self.value(&xm, r)?) {
                        (Some(fp), None) => {
                            for k in 0..3 {
                                j[k][column] = (fp[k] - f0[k]) / h;
                            }
                            continue;
                        }
                        (None, Some(fm)) => {
                            for k in 0..3 {
                                j[k][column] = (f0[k] - fm[k]) / h;
                            }
                            continue;
                        }
                        _ => return Ok(None),
                    }
                }
            };
            for k in 0..3 {
                j[k][column] = (fp[k] - fm[k]) / (2. * h);
            }
        }
        Ok(Some(j))
    }

    /// Least-norm Newton on the 3×4 underdetermined system.
    fn refine(&self, seed: [f64; 4], r: f64) -> Result<Option<([f64; 4], [f64; 3])>> {
        Ok(self.refine_with_contacts(seed, r)?.map(|(x, center, _, _)| (x, center)))
    }

    fn refine_with_contacts(
        &self,
        seed: [f64; 4],
        r: f64,
    ) -> Result<Option<([f64; 4], [f64; 3], [f64; 3], [f64; 3])>> {
        let mut x = seed;
        let mut budget = Budget::with_iterations(NEWTON_ITERS + 1)?.guard("fillet_newton");
        for _ in 0..NEWTON_ITERS {
            budget.tick()?;
            let Some((pa, na)) = point_normal(self.a, x[0], x[1])? else { return Ok(None) };
            let Some((pb, nb)) = point_normal(self.b, x[2], x[3])? else { return Ok(None) };
            let ca = add(pa, scale(na, r * self.sa));
            let cb = add(pb, scale(nb, r * self.sb));
            let f = sub(ca, cb);
            if norm(f) <= self.tol {
                let center = scale(add(ca, cb), 0.5);
                return Ok(Some((x, center, pa, pb)));
            }
            let Some(j) = self.jacobian(&x, r)? else { return Ok(None) };
            let mut jjt = [[0.; 3]; 3];
            for row in 0..3 {
                for col in 0..3 {
                    jjt[row][col] = (0..4).map(|i| j[row][i] * j[col][i]).sum();
                }
            }
            let Some(mu) = solve3(jjt, f) else { return Ok(None) };
            let mut delta = [0.; 4];
            for i in 0..4 {
                delta[i] = (0..3).map(|k| j[k][i] * mu[k]).sum();
            }
            let step = delta.iter().fold(0_f64, |a, &d| a.max(d.abs()));
            if !step.is_finite() || step > 1e6 {
                return Ok(None);
            }
            for i in 0..4 {
                x[i] -= delta[i];
            }
            if !x.iter().all(|v| v.is_finite()) {
                return Ok(None);
            }
        }
        let Some((pa, na)) = point_normal(self.a, x[0], x[1])? else { return Ok(None) };
        let Some((pb, nb)) = point_normal(self.b, x[2], x[3])? else { return Ok(None) };
        let ca = add(pa, scale(na, r * self.sa));
        let cb = add(pb, scale(nb, r * self.sb));
        let f = sub(ca, cb);
        if norm(f) <= self.tol {
            let center = scale(add(ca, cb), 0.5);
            Ok(Some((x, center, pa, pb)))
        } else {
            Ok(None)
        }
    }

    /// Null-space tangent of the 3×4 Jacobian (4D cross product of rows).
    fn tangent(&self, x: &[f64; 4], r: f64) -> Result<Option<[f64; 4]>> {
        let Some(j) = self.jacobian(x, r)? else { return Ok(None) };
        let det3 = |c0: usize, c1: usize, c2: usize| {
            j[0][c0] * (j[1][c1] * j[2][c2] - j[1][c2] * j[2][c1])
                - j[0][c1] * (j[1][c0] * j[2][c2] - j[1][c2] * j[2][c0])
                + j[0][c2] * (j[1][c0] * j[2][c1] - j[1][c1] * j[2][c0])
        };
        let t = [
            det3(1, 2, 3),
            -det3(0, 2, 3),
            det3(0, 1, 3),
            -det3(0, 1, 2),
        ];
        let n = norm4(t);
        if !n.is_finite() || n <= 1e-300 {
            return Ok(None);
        }
        Ok(Some(t.map(|v| v / n)))
    }

    fn inside(&self, x: &[f64; 4]) -> bool {
        let eps = 1e-9;
        self.dom_a.0[0] - eps <= x[0]
            && x[0] <= self.dom_a.0[1] + eps
            && self.dom_a.1[0] - eps <= x[1]
            && x[1] <= self.dom_a.1[1] + eps
            && self.dom_b.0[0] - eps <= x[2]
            && x[2] <= self.dom_b.0[1] + eps
            && self.dom_b.1[0] - eps <= x[3]
            && x[3] <= self.dom_b.1[1] + eps
    }
}

mod marching;
use marching::*;


mod sections;
use sections::*;





/// Re-polish each raw point with the radius of the final law at its measured
/// normalized arc position (variable-radius second pass).
fn polish_with_law(
    m: &Marcher<'_>,
    points: &mut [RawPoint],
    law: &RadiusLaw,
    warnings: &mut Vec<String>,
) -> Result<Option<FilletFailure>> {
    let length = points.last().map(|p| p.s).unwrap_or(0.);
    if length <= 0. {
        return Ok(Some(FilletFailure {
            reason: FailureReason::MarchingDiverged,
            t: Some(0.),
            detail: "spine has no extent".into(),
        }));
    }
    for p in points.iter_mut() {
        let t = (p.s / length).clamp(0., 1.);
        let r = law.evaluate(t);
        match m.refine(p.x, r)? {
            Some((x, center)) if m.inside(&x) => {
                p.x = x;
                p.center = center;
                if let (Some((pa, _)), Some((pb, _))) = (
                    point_normal(m.a, x[0], x[1])?,
                    point_normal(m.b, x[2], x[3])?,
                ) {
                    p.contact_a = pa;
                    p.contact_b = pb;
                }
            }
            _ => warnings.push(format!("polish with r({t:.4}) = {r:.6} did not converge; kept first-pass point")),
        }
    }
    Ok(None)
}

/// Resample the raw spine polyline into `n` arc-length-uniform stations and
/// polish each at its own radius.
fn stations(
    m: &Marcher<'_>,
    points: &[RawPoint],
    n: usize,
    law: &RadiusLaw,
) -> Result<Vec<SpineSample>> {
    (0..n)
        .map(|k| station_at(m, points, k as f64 / (n - 1) as f64, law))
        .collect()
}

fn station_at(
    m: &Marcher<'_>,
    points: &[RawPoint],
    t: f64,
    law: &RadiusLaw,
) -> Result<SpineSample> {
    let length = points.last().map(|p| p.s).unwrap_or(0.);
    let mut span = 0;
    let target = t * length;
    while span + 1 < points.len() && points[span + 1].s < target {
        span += 1;
    }
    let (p0, p1) = (&points[span], &points[(span + 1).min(points.len() - 1)]);
    let w = if p1.s > p0.s {
        ((target - p0.s) / (p1.s - p0.s)).clamp(0., 1.)
    } else {
        0.
    };
    let mut x = [0.; 4];
    for i in 0..4 {
        x[i] = p0.x[i] + w * (p1.x[i] - p0.x[i]);
    }
    let r = law.evaluate(t);
    let (x, center, ca, cb) = match m.refine(x, r)? {
        Some((rx, rc)) if m.inside(&rx) => {
            let ca = point_normal(m.a, rx[0], rx[1])?
                .map(|(p, _)| p)
                .unwrap_or(p0.contact_a);
            let cb = point_normal(m.b, rx[2], rx[3])?
                .map(|(p, _)| p)
                .unwrap_or(p0.contact_b);
            (rx, rc, ca, cb)
        }
        _ => (
            x,
            add(p0.center, scale(sub(p1.center, p0.center), w)),
            add(p0.contact_a, scale(sub(p1.contact_a, p0.contact_a), w)),
            add(p0.contact_b, scale(sub(p1.contact_b, p0.contact_b), w)),
        ),
    };
    Ok(SpineSample {
        t,
        center,
        contact_a: ca,
        contact_b: cb,
        radius: r,
        uv_a: [x[0], x[1]],
        uv_b: [x[2], x[3]],
    })
}



/// Signed normal curvature of `surface` at `uv` in the unit direction `dir`
/// (Meusnier; sign relative to the `S_u × S_v` normal).
fn normal_curvature(surface: &Surface, uv: [f64; 2], dir: [f64; 3]) -> Result<Option<f64>> {
    let e = surface.evaluate(uv[0], uv[1])?;
    let (Some(n), Some((su, sv)), Some((suu, suv, svv))) =
        (e.unit_normal(), e.first_derivatives(), e.second_derivatives())
    else {
        return Ok(None);
    };
    let (ee, ff, gg) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let det = ee * gg - ff * ff;
    if det <= 1e-300 {
        return Ok(None);
    }
    let (ru, rv) = (dot(dir, su), dot(dir, sv));
    let alpha = (gg * ru - ff * rv) / det;
    let beta = (-ff * ru + ee * rv) / det;
    let (ll, mm, nn) = (dot(suu, n), dot(suv, n), dot(svv, n));
    Ok(Some(ll * alpha * alpha + 2. * mm * alpha * beta + nn * beta * beta))
}



/// Skin the sections and estimate the inter-section deviation from the true
/// pipe of radius `r(t)` around the spine.
fn assemble(
    sections: Vec<Curve>,
    spine: &[SpineSample],
    stations_s: &[f64],
) -> Result<(Surface, f64)> {
    let params: Vec<f64> = stations_s.to_vec();
    let surface = natural_loft::interpolate(&sections, &params)?;
    let deviation = estimate_skin_deviation(&surface, spine, stations_s)?;
    Ok((surface, deviation))
}

fn estimate_skin_deviation(
    surface: &Surface,
    spine: &[SpineSample],
    stations_s: &[f64],
) -> Result<f64> {
    let params = stations_s;
    // Deviation estimate: mid-span samples against the spine chord.
    let mut deviation = 0_f64;
    let (du, dv) = domain(&surface);
    for k in 0..spine.len().saturating_sub(1) {
        let v_mid = (stations_s[k] + stations_s[k + 1]) * 0.5;
        let v =
            dv[0] + (dv[1] - dv[0]) * (v_mid - params[0]) / (params[params.len() - 1] - params[0]);
        let r_mid = (spine[k].radius + spine[k + 1].radius) * 0.5;
        for &u_frac in &[0.25, 0.5, 0.75] {
            let u = du[0] + (du[1] - du[0]) * u_frac;
            let Ok(e) = surface.evaluate(u, v) else {
                continue;
            };
            let p = e.point;
            let c0 = spine[k].center;
            let c1 = spine[k + 1].center;
            let axis = sub(c1, c0);
            let len2 = dot(axis, axis);
            let w = if len2 > 1e-300 {
                (dot(sub(p, c0), axis) / len2).clamp(0., 1.)
            } else {
                0.
            };
            let nearest = add(c0, scale(axis, w));
            deviation = deviation.max((norm(sub(p, nearest)) - r_mid).abs());
        }
    }
    Ok(deviation)
}



#[derive(Clone, Copy, PartialEq)]
enum SectionKind {
    Arc,
    G2,
}

fn prepare<'a>(
    a: &'a Surface,
    b: &'a Surface,
    options: &FilletOptions,
) -> Result<(Marcher<'a>, f64, f64)> {
    a.validate()?;
    b.validate()?;
    let (sa, sb) = options.validated()?;
    Ok((Marcher::new(a, b, sa, sb, options.tolerance), sa, sb))
}

fn reachability_warnings(a: &Surface, b: &Surface, sa: f64, sb: f64, r_max: f64) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(w) = reachability_warning("support A", a, r_max * sa) {
        warnings.push(w);
    }
    if let Some(w) = reachability_warning("support B", b, r_max * sb) {
        warnings.push(w);
    }
    warnings
}











mod projection;
use projection::*;




#[cfg(test)]
#[path = "fillet_surfaces/tests/tests_integration.rs"]
mod tests;

#[path = "fillet_surfaces/fillet_construction.rs"]
mod fillet_construction;
pub use fillet_construction::hold_line_fillet;
pub use fillet_construction::chordal_fillet;
pub use fillet_construction::rolling_ball_fillet;
pub use fillet_construction::variable_radius_fillet;
pub use fillet_construction::variable_g2_fillet;
use fillet_construction::variable_radius_sections;
pub use fillet_construction::g2_fillet;
use fillet_construction::build_report;

#[path = "fillet_surfaces/section_geometry.rs"]
mod section_geometry;
use section_geometry::g2_section;
use section_geometry::arc_section;
use section_geometry::march_direction;
use section_geometry::trace_spine;
