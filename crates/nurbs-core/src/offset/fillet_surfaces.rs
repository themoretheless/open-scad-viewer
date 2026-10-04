//! Surface-pair fillet surfaces (no B-rep): rolling-ball, variable-radius,
//! chordal, G2 and hold-line fillets between two NURBS support surfaces.
//!
//! The ball-center spine is traced as the intersection of the two offset
//! surfaces: seed pairs come from sampling [`super::surface_offset::offset`]
//! results, then a least-norm Newton predictor-corrector marches the exact
//! implicit offset map `A(u,v) + r·sa·nA − B(s,t) − r·sb·nB = 0` with a
//! curvature-adaptive step. Cross sections are exact rational circular arcs
//! (rolling-ball / variable / chordal), G2 quintic Bézier sections, or
//! tangent cubic hold-line sections, skinned by
//! [`crate::natural_loft::interpolate`].
//!
//! Graceful failure: geometric breakdowns never panic and never abort with a
//! bare error; they return a [`FilletReport`] whose `failure` field carries
//! the spine parameter `t` and the reason. Only invalid *inputs* (non-finite
//! radii, bad options, invalid surfaces) produce `Err` via `check`.
use crate::{
    Result, check,
    curve::Curve,
    natural_loft,
    offset::surface_offset::{offset, offset_validity},
    primitives,
    surface::Surface,
};
use math_core::{dot, norm};

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

/// Radius law along the normalized spine parameter `t ∈ [0, 1]`.
#[derive(Clone, Debug)]
pub enum RadiusLaw {
    /// Constant rolling-ball radius.
    Constant(f64),
    /// Linear interpolation between the start and end radii.
    Linear { start: f64, end: f64 },
    /// Natural cubic B-spline interpolation through `(t, r)` control points
    /// (`t` strictly increasing, at least two points; clamped outside).
    Spline(Vec<(f64, f64)>),
}

impl RadiusLaw {
    fn validate(&self) -> Result<()> {
        match self {
            RadiusLaw::Constant(r) => check(
                r.is_finite() && *r > 0.,
                "Fillet radius must be finite and positive",
            )?,
            RadiusLaw::Linear { start, end } => check(
                start.is_finite() && *start > 0. && end.is_finite() && *end > 0.,
                "Fillet linear law radii must be finite and positive",
            )?,
            RadiusLaw::Spline(points) => {
                check(
                    points.len() >= 2 && points.len() <= 64,
                    "Fillet spline law needs 2..=64 control points",
                )?;
                for w in points.windows(2) {
                    check(
                        w[0].0.is_finite() && w[1].0.is_finite() && w[1].0 > w[0].0,
                        "Fillet spline law parameters must be finite and strictly increasing",
                    )?;
                }
                check(
                    points.iter().all(|&(_, r)| r.is_finite() && r > 0.),
                    "Fillet spline law radii must be finite and positive",
                )?;
            }
        }
        Ok(())
    }

    /// Evaluate the law at `t` (clamped to the control range for splines).
    pub fn evaluate(&self, t: f64) -> f64 {
        match self {
            RadiusLaw::Constant(r) => *r,
            RadiusLaw::Linear { start, end } => start + (end - start) * t.clamp(0., 1.),
            RadiusLaw::Spline(points) => natural_cubic(points, t),
        }
    }

    /// `[min, max, mean]` over a budgeted uniform sample.
    fn sampled_range(&self) -> [f64; 3] {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut sum = 0.;
        for i in 0..=LAW_SAMPLES {
            let r = self.evaluate(i as f64 / LAW_SAMPLES as f64);
            lo = lo.min(r);
            hi = hi.max(r);
            sum += r;
        }
        [lo, hi, sum / (LAW_SAMPLES + 1) as f64]
    }
}

/// Natural cubic spline scalar interpolation (Thomas solve, budgeted by the
/// 64-point validation cap).
fn natural_cubic(points: &[(f64, f64)], t: f64) -> f64 {
    let n = points.len();
    let clamped = t.clamp(points[0].0, points[n - 1].0);
    // Second derivatives m[i]; natural boundary m[0] = m[n-1] = 0. Thomas
    // forward sweep over the interior rows of the tridiagonal system.
    let mut m = vec![0.; n];
    let interior = n.saturating_sub(2);
    let mut cp = vec![0.; interior];
    let mut dp = vec![0.; interior];
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
        check(
            self.side_a.is_finite() && self.side_a != 0. && self.side_b.is_finite() && self.side_b != 0.,
            "Fillet side selectors must be nonzero and finite",
        )?;
        check(
            self.tolerance.is_finite() && self.tolerance > 0.,
            "Fillet tolerance must be positive and finite",
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
    /// `r(t)` around the spine (sections themselves are exact by construction;
    /// this measures only the inter-section skin).
    pub skin_deviation_estimate: f64,
    /// True when the spine closed into a loop and was cut at the seed.
    pub closed_spine: bool,
}

impl FilletReport {
    fn failed(reason: FailureReason, t: Option<f64>, detail: impl Into<String>, warnings: Vec<String>) -> Self {
        Self {
            surface: None,
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
        let finish = |x: [f64; 4]| -> Result<Option<([f64; 4], [f64; 3])>> {
            let Some((pa, na)) = point_normal(self.a, x[0], x[1])? else { return Ok(None) };
            let Some((pb, nb)) = point_normal(self.b, x[2], x[3])? else { return Ok(None) };
            let center = scale(
                add(add(pa, scale(na, r * self.sa)), add(pb, scale(nb, r * self.sb))),
                0.5,
            );
            Ok(Some((x, center)))
        };
        let mut x = seed;
        for _ in 0..NEWTON_ITERS {
            let Some(f) = self.value(&x, r)? else { return Ok(None) };
            if norm(f) <= self.tol {
                return finish(x);
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
        let Some(f) = self.value(&x, r)? else { return Ok(None) };
        if norm(f) <= self.tol {
            finish(x)
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

/// Minimum meaningful radius for this pair: resolves the "graceful failure on
/// a ridiculously small radius" case before any marching happens.
fn minimum_radius(a: &Surface, b: &Surface, tol: f64) -> f64 {
    let mut scale = 0_f64;
    for s in [a, b] {
        for row in &s.control_points {
            for p in row {
                scale = scale.max(p.iter().fold(0., |m, x| m.max(x.abs())));
            }
        }
    }
    (1e-9 * scale.max(1.)).max(100. * tol)
}

/// Reachability warning via the certified offset validity: returns a warning
/// string when `r_max` on a side exceeds the fold-free bound.
fn reachability_warning(
    name: &str,
    surface: &Surface,
    signed: f64,
) -> Option<String> {
    let report = offset_validity(surface, signed).ok()?;
    if report.requested_ok {
        return None;
    }
    let ([u0, u1], [v0, v1]) = domain(surface);
    let point = surface
        .evaluate(
            report.limiting_uv[0].clamp(u0, u1),
            report.limiting_uv[1].clamp(v0, v1),
        )
        .map(|e| e.point)
        .unwrap_or([f64::NAN; 3]);
    Some(format!(
        "{name}: requested radius |{signed:.6}| exceeds the certified fold-free reach {:.6} near {:?} (limiting radius)",
        report.max_offset, point
    ))
}

/// Offset-surface seed search: sample both ready-made offset surfaces on a
/// grid, take the closest pairs, and Newton-refine them on the exact map.
fn find_seed(m: &Marcher<'_>, r: f64) -> Result<Option<[f64; 4]>> {
    let sample_grid = |surface: &Surface, distance: f64| -> Result<Vec<([f64; 2], [f64; 3])>> {
        // Ready-made offset surface when available; exact normal offset as a
        // fallback (identical for planes/spheres/cylinders by construction).
        let report = offset(surface, distance, (0.05 * r.abs()).max(10. * m.tol)).ok();
        let (du, dv) = domain(surface);
        let mut out = Vec::with_capacity(SEED_GRID * SEED_GRID);
        for i in 0..SEED_GRID {
            for j in 0..SEED_GRID {
                let u = du[0] + (du[1] - du[0]) * (i as f64 + 0.5) / SEED_GRID as f64;
                let v = dv[0] + (dv[1] - dv[0]) * (j as f64 + 0.5) / SEED_GRID as f64;
                let point = match &report {
                    Some(rep) => rep.surface.evaluate(u, v).ok().map(|e| e.point),
                    None => None,
                };
                let point = match point {
                    Some(p) => Some(p),
                    None => point_normal(surface, u, v)?.map(|(p, n)| add(p, scale(n, distance))),
                };
                if let Some(p) = point {
                    out.push(([u, v], p));
                }
            }
        }
        Ok(out)
    };
    let grid_a = sample_grid(m.a, r * m.sa)?;
    let grid_b = sample_grid(m.b, r * m.sb)?;
    if grid_a.is_empty() || grid_b.is_empty() {
        return Ok(None);
    }
    // Keep the 8 closest pairs as seed candidates (selection is budgeted).
    let mut best: Vec<(f64, [f64; 4])> = Vec::with_capacity(8);
    for &(uva, pa) in &grid_a {
        for &(uvb, pb) in &grid_b {
            let d = norm(sub(pa, pb));
            let candidate = (d, [uva[0], uva[1], uvb[0], uvb[1]]);
            if best.len() < 8 {
                best.push(candidate);
                best.sort_by(|x, y| x.0.total_cmp(&y.0));
            } else if d < best[7].0 {
                best[7] = candidate;
                best.sort_by(|x, y| x.0.total_cmp(&y.0));
            }
        }
    }
    for (_, x0) in best {
        if let Some((x, _)) = m.refine(x0, r)? {
            if m.inside(&x) {
                return Ok(Some(x));
            }
        }
    }
    Ok(None)
}

/// March one direction from the seed with a curvature-adaptive step.
fn march_direction(
    m: &Marcher<'_>,
    seed: [f64; 4],
    sign: f64,
    radius_at: &dyn Fn(f64) -> f64,
    max_steps: usize,
    closed_at: [f64; 3],
) -> Result<(Vec<RawPoint>, bool)> {
    let mut points = Vec::new();
    let mut x = seed;
    let r0 = radius_at(0.);
    let Some(mut tangent) = m.tangent(&x, r0)? else {
        return Ok((points, false));
    };
    let h_min = 1e-4 * r0;
    let h_max = r0;
    let mut h = 0.5 * r0;
    let mut s = 0.;
    let mut closed = false;
    for _ in 0..max_steps {
        let r_here = radius_at(s);
        let h_min = h_min.min(1e-4 * r_here);
        let h_max = h_max.max(2. * r_here);
        let mut accepted = None;
        let mut step = h;
        for _ in 0..6 {
            let mut candidate = x;
            for i in 0..4 {
                candidate[i] += sign * step * tangent[i];
            }
            if let Some((refined, _)) = m.refine(candidate, radius_at(s + sign * step))? {
                if m.inside(&refined) {
                    accepted = Some(refined);
                    break;
                }
            }
            step *= 0.5;
            if step < h_min * 1e-3 {
                break;
            }
        }
        let Some(next) = accepted else { break };
        let s_next = s + sign * step;
        let r_next = radius_at(s_next);
        let Some((pa, na)) = point_normal(m.a, next[0], next[1])? else { break };
        let Some((pb, nb)) = point_normal(m.b, next[2], next[3])? else { break };
        let center = scale(add(add(pa, scale(na, r_next * m.sa)), add(pb, scale(nb, r_next * m.sb))), 0.5);
        // Curvature-adaptive next step: chord error κ h²/8 ≤ chord tolerance.
        let mut h_next = (h * 1.5).min(h_max);
        if let Some(t_next) = m.tangent(&next, r_next)? {
            let d = dot4(t_next, tangent);
            let t_next = if d >= 0. { t_next } else { t_next.map(|v| -v) };
            let kappa = d.abs().clamp(-1., 1.).acos() / step.max(1e-300);
            if kappa > 1e-12 {
                let chord_tol = (0.02 * r_next).max(10. * m.tol);
                h_next = h_next.min((8. * chord_tol / kappa).sqrt()).clamp(h_min, h_max);
            }
            tangent = t_next;
        }
        points.push(RawPoint {
            x: next,
            center,
            contact_a: pa,
            contact_b: pb,
            s: s_next,
        });
        x = next;
        s = s_next;
        h = h_next;
        // Loop closure back at the seed after enough travel.
        if points.len() > 8 && norm(sub(center, closed_at)) < 0.25 * r_next {
            closed = true;
            break;
        }
    }
    Ok((points, closed))
}

/// Full two-direction spine trace with a radius law evaluated on the
/// normalized arc-length parameter of a previous pass (`law_length`), or a
/// constant radius when `law` is None and `seed_r` is used.
fn trace_spine(
    m: &Marcher<'_>,
    seed_r: f64,
    max_steps: usize,
) -> Result<(Vec<RawPoint>, Option<[f64; 4]>, bool)> {
    let Some(seed) = find_seed(m, seed_r)? else {
        return Ok((Vec::new(), None, false));
    };
    let (Some((pa, na)), Some((pb, nb))) = (
        point_normal(m.a, seed[0], seed[1])?,
        point_normal(m.b, seed[2], seed[3])?,
    ) else {
        return Ok((Vec::new(), None, false));
    };
    let seed_center = scale(add(add(pa, scale(na, seed_r * m.sa)), add(pb, scale(nb, seed_r * m.sb))), 0.5);
    let radius_at = |_: f64| seed_r;
    let (mut backward, closed_b) = march_direction(m, seed, -1., &radius_at, max_steps / 2, seed_center)?;
    let (forward, closed_f) = march_direction(m, seed, 1., &radius_at, max_steps / 2, seed_center)?;
    backward.reverse();
    let mut points = backward;
    points.push(RawPoint { x: seed, center: seed_center, contact_a: pa, contact_b: pb, s: 0. });
    points.extend(forward);
    // Re-base arc lengths to start at zero.
    let s0 = points[0].s;
    for p in &mut points {
        p.s -= s0;
    }
    Ok((points, Some(seed), closed_b || closed_f))
}

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
    let length = points.last().map(|p| p.s).unwrap_or(0.);
    let mut out = Vec::with_capacity(n);
    let mut span = 0;
    for k in 0..n {
        let t = k as f64 / (n - 1) as f64;
        let target = t * length;
        while span + 1 < points.len() && points[span + 1].s < target {
            span += 1;
        }
        let (p0, p1) = (&points[span], &points[(span + 1).min(points.len() - 1)]);
        let w = if p1.s > p0.s { ((target - p0.s) / (p1.s - p0.s)).clamp(0., 1.) } else { 0. };
        let mut x = [0.; 4];
        for i in 0..4 {
            x[i] = p0.x[i] + w * (p1.x[i] - p0.x[i]);
        }
        let r = law.evaluate(t);
        let (x, center, ca, cb) = match m.refine(x, r)? {
            Some((rx, rc)) if m.inside(&rx) => {
                let ca = point_normal(m.a, rx[0], rx[1])?.map(|(p, _)| p).unwrap_or(p0.contact_a);
                let cb = point_normal(m.b, rx[2], rx[3])?.map(|(p, _)| p).unwrap_or(p0.contact_b);
                (rx, rc, ca, cb)
            }
            _ => (
                x,
                add(p0.center, scale(sub(p1.center, p0.center), w)),
                add(p0.contact_a, scale(sub(p1.contact_a, p0.contact_a), w)),
                add(p0.contact_b, scale(sub(p1.contact_b, p0.contact_b), w)),
            ),
        };
        out.push(SpineSample {
            t,
            center,
            contact_a: ca,
            contact_b: cb,
            radius: r,
            uv_a: [x[0], x[1]],
            uv_b: [x[2], x[3]],
        });
    }
    Ok(out)
}

/// Exact rational circular arc section from contact A to contact B.
fn arc_section(sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let phi = angle_between(a_vec, b_vec);
    if !(1e-6..std::f64::consts::PI - 1e-9).contains(&phi) || !phi.is_finite() {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let Some(a_hat) = unit(a_vec) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let perp = sub(b_vec, scale(a_hat, dot(b_vec, a_hat)));
    let Some(v_hat) = unit(perp) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    match primitives::ellipse_arc(
        sample.center,
        a_vec,
        scale(v_hat, sample.radius),
        0.,
        phi.to_degrees(),
    ) {
        Ok(c) => Ok(Ok(c)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
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

/// G2 quintic Bézier section: position, tangent and curvature vector matched
/// to both support normal sections at the contacts.
fn g2_section(m: &Marcher<'_>, sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let Some(section_normal) = unit(cross(a_vec, b_vec)) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let Some((_, na)) = point_normal(m.a, sample.uv_a[0], sample.uv_a[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let Some((_, nb)) = point_normal(m.b, sample.uv_b[0], sample.uv_b[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let chord = sub(sample.contact_b, sample.contact_a);
    let chord_len = norm(chord);
    if chord_len <= 1e-12 {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let tangent_at = |n: [f64; 3]| -> Option<[f64; 3]> {
        let mut t = unit(cross(section_normal, n))?;
        if dot(t, chord) < 0. {
            t = scale(t, -1.);
        }
        Some(t)
    };
    let (Some(t0), Some(t1)) = (tangent_at(na), tangent_at(nb)) else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let k0 = normal_curvature(m.a, sample.uv_a, t0)?.unwrap_or(0.);
    let k1 = normal_curvature(m.b, sample.uv_b, t1)?.unwrap_or(0.);
    let handle = 0.35 * chord_len;
    // Bézier end condition: curvature vector K = (4/5)·(P2−P1)⊥/a² — solve
    // for the inner controls with K matched to k_n·n of each support.
    let k0_vec = scale(na, k0);
    let k1_vec = scale(nb, k1);
    let p0 = sample.contact_a;
    let p5 = sample.contact_b;
    let p1 = add(p0, scale(t0, handle));
    let p4 = sub(p5, scale(t1, handle));
    let p2 = add(add(p1, scale(t0, handle)), scale(k0_vec, 1.25 * handle * handle));
    let p3 = add(sub(p4, scale(t1, handle)), scale(k1_vec, 1.25 * handle * handle));
    let curve = Curve {
        degree: 5,
        knots: vec![0.; 6].into_iter().chain(vec![1.; 6]).collect(),
        control_points: vec![p0, p1, p2, p3, p4, p5]
            .iter()
            .map(|p| p.to_vec())
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    match curve.validate() {
        Ok(()) => Ok(Ok(curve)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
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
    // Deviation estimate: mid-span samples against the spine chord.
    let mut deviation = 0_f64;
    let (du, dv) = domain(&surface);
    for k in 0..spine.len().saturating_sub(1) {
        let v_mid = (stations_s[k] + stations_s[k + 1]) * 0.5;
        let v = dv[0] + (dv[1] - dv[0]) * (v_mid - params[0]) / (params[params.len() - 1] - params[0]);
        let r_mid = (spine[k].radius + spine[k + 1].radius) * 0.5;
        for &u_frac in &[0.25, 0.5, 0.75] {
            let u = du[0] + (du[1] - du[0]) * u_frac;
            let Ok(e) = surface.evaluate(u, v) else { continue };
            let p = e.point;
            let c0 = spine[k].center;
            let c1 = spine[k + 1].center;
            let axis = sub(c1, c0);
            let len2 = dot(axis, axis);
            let w = if len2 > 1e-300 { (dot(sub(p, c0), axis) / len2).clamp(0., 1.) } else { 0. };
            let nearest = add(c0, scale(axis, w));
            deviation = deviation.max((norm(sub(p, nearest)) - r_mid).abs());
        }
    }
    Ok((surface, deviation))
}

/// Shared tail: stations, section construction, skinning, report assembly.
fn build_report(
    m: &Marcher<'_>,
    raw: &[RawPoint],
    law: &RadiusLaw,
    options: &FilletOptions,
    warnings: Vec<String>,
    closed: bool,
    section_kind: SectionKind,
) -> Result<FilletReport> {
    let mut warnings = warnings;
    let length = raw.last().map(|p| p.s).unwrap_or(0.);
    if raw.len() < 2 || length <= 1e-12 {
        return Ok(FilletReport::failed(
            FailureReason::MarchingDiverged,
            Some(0.),
            "spine trace produced fewer than two points",
            warnings,
        ));
    }
    let n = options.max_sections.clamp(2, MAX_SECTIONS);
    let spine = stations(m, raw, n, law)?;
    let stations_s: Vec<f64> = spine.iter().map(|s| s.t * length).collect();
    let minimum = minimum_radius(m.a, m.b, m.tol);
    let mut sections = Vec::with_capacity(n);
    let mut radius_range = [f64::INFINITY, f64::NEG_INFINITY];
    for sample in &spine {
        radius_range[0] = radius_range[0].min(sample.radius);
        radius_range[1] = radius_range[1].max(sample.radius);
        if sample.radius < minimum {
            return Ok(FilletReport::failed(
                FailureReason::RadiusBelowMinimum { radius: sample.radius, minimum },
                Some(sample.t),
                format!("section radius {:.3e} below the numerical minimum {:.3e}", sample.radius, minimum),
                warnings,
            ));
        }
        let built = match section_kind {
            SectionKind::Arc => arc_section(sample)?,
            SectionKind::G2 => g2_section(m, sample)?,
        };
        let curve = match built {
            Ok(c) => c,
            Err(reason) => {
                return Ok(FilletReport::failed(
                    reason,
                    Some(sample.t),
                    format!("section construction failed at t = {:.4}", sample.t),
                    warnings,
                ));
            }
        };
        sections.push(curve);
    }
    match assemble(sections, &spine, &stations_s) {
        Ok((surface, deviation)) => Ok(FilletReport {
            surface: Some(surface),
            failure: None,
            spine,
            radius_range,
            warnings,
            skin_deviation_estimate: deviation,
            closed_spine: closed,
        }),
        Err(e) => {
            warnings.push(format!("skinning error: {e}"));
            Ok(FilletReport::failed(
                FailureReason::AssemblyFailed,
                None,
                format!("natural loft of the sections failed: {e}"),
                warnings,
            ))
        }
    }
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

/// Rolling-ball fillet of constant `radius` between two NURBS surfaces.
///
/// The spine is the intersection of the two offset surfaces; sections are
/// exact circular arcs of `radius` tangent to both supports at the contact
/// points (the projections of the ball center onto the supports).
pub fn rolling_ball_fillet(
    a: &Surface,
    b: &Surface,
    radius: f64,
    options: &FilletOptions,
) -> Result<FilletReport> {
    variable_radius_fillet(a, b, RadiusLaw::Constant(radius), options)
}

/// Variable-radius fillet: the radius follows `law` along the normalized
/// spine arc parameter. Radii must stay positive (validated); when the
/// sampled maximum exceeds a support's certified fold-free reach a warning
/// names the limiting radius and location.
pub fn variable_radius_fillet(
    a: &Surface,
    b: &Surface,
    law: RadiusLaw,
    options: &FilletOptions,
) -> Result<FilletReport> {
    law.validate()?;
    let (m, sa, sb) = prepare(a, b, options)?;
    let [r_min, r_max, r_mean] = law.sampled_range();
    let minimum = minimum_radius(a, b, m.tol);
    if r_min < minimum {
        return Ok(FilletReport::failed(
            FailureReason::RadiusBelowMinimum { radius: r_min, minimum },
            None,
            format!("law minimum radius {r_min:.3e} is below the numerical minimum {minimum:.3e}"),
            Vec::new(),
        ));
    }
    let warnings = reachability_warnings(a, b, sa, sb, r_max);
    let (mut raw, seed, closed) = trace_spine(&m, r_mean, options.max_march_steps)?;
    if seed.is_none() {
        return Ok(FilletReport::failed(
            FailureReason::SeedNotFound,
            None,
            "no converging intersection of the two offset surfaces (wrong side selectors, or the ball of this radius never touches both supports)",
            warnings,
        ));
    }
    let mut warnings = warnings;
    if !matches!(law, RadiusLaw::Constant(_)) {
        if let Some(failure) = polish_with_law(&m, &mut raw, &law, &mut warnings)? {
            return Ok(FilletReport {
                surface: None,
                failure: Some(failure),
                spine: Vec::new(),
                radius_range: [0.; 2],
                warnings,
                skin_deviation_estimate: 0.,
                closed_spine: false,
            });
        }
    }
    build_report(&m, &raw, &law, options, warnings, closed, SectionKind::Arc)
}

/// Chordal fillet: the chord `chord` between the two contact points is fixed;
/// the radius is recovered station-wise as `r = c / (2 sin(φ/2))` from the
/// local dihedral angle `φ` between the oriented support normals.
pub fn chordal_fillet(
    a: &Surface,
    b: &Surface,
    chord: f64,
    options: &FilletOptions,
) -> Result<ChordalFilletReport> {
    check(
        chord.is_finite() && chord > 0.,
        "Chordal fillet chord must be finite and positive",
    )?;
    let (m, _, _) = prepare(a, b, options)?;
    let minimum = minimum_radius(a, b, m.tol);
    if chord * 0.5 < minimum {
        let report = FilletReport::failed(
            FailureReason::RadiusBelowMinimum { radius: chord * 0.5, minimum },
            None,
            "chord is below the numerical minimum for this surface pair",
            Vec::new(),
        );
        return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
    }
    let warnings = Vec::new();
    // First pass: guess radius from a 60° dihedral, measure the true angles.
    let guess = chord / (2. * (std::f64::consts::FRAC_PI_3).sin());
    let (raw, seed, _) = trace_spine(&m, guess, options.max_march_steps)?;
    if seed.is_none() || raw.len() < 2 {
        let report = FilletReport::failed(
            FailureReason::SeedNotFound,
            None,
            "no converging offset intersection for the chordal seed pass",
            warnings,
        );
        return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
    }
    let n = options.max_sections.clamp(2, MAX_SECTIONS);
    let probe_law = RadiusLaw::Constant(guess);
    let probes = stations(&m, &raw, n, &probe_law)?;
    let mut law_points = Vec::with_capacity(n);
    let mut dihedral_range = [f64::INFINITY, f64::NEG_INFINITY];
    for sample in &probes {
        let na = point_normal(m.a, sample.uv_a[0], sample.uv_a[1])?.map(|(_, n)| n);
        let nb = point_normal(m.b, sample.uv_b[0], sample.uv_b[1])?.map(|(_, n)| n);
        let (Some(na), Some(nb)) = (na, nb) else {
            let report = FilletReport::failed(
                FailureReason::MarchingDiverged,
                Some(sample.t),
                "normal unavailable at a chordal probe station",
                warnings,
            );
            return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range: [0.; 2] });
        };
        let phi = angle_between(scale(na, m.sa), scale(nb, m.sb));
        dihedral_range[0] = dihedral_range[0].min(phi);
        dihedral_range[1] = dihedral_range[1].max(phi);
        let half = (phi * 0.5).sin();
        if half <= 1e-9 {
            let report = FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(sample.t),
                format!("dihedral angle {phi:.6} rad cannot carry a finite chordal radius"),
                warnings,
            );
            return Ok(ChordalFilletReport { fillet: report, radius_range: [0.; 2], dihedral_range });
        }
        law_points.push((sample.t, chord / (2. * half)));
    }
    let law = RadiusLaw::Spline(law_points);
    let [r_min, r_max, _] = law.sampled_range();
    let radius_range = [r_min, r_max];
    let fillet = variable_radius_fillet(a, b, law, options)?;
    Ok(ChordalFilletReport { fillet, radius_range, dihedral_range })
}

/// G2 fillet: same rolling-ball spine, but each section is a quintic Bézier
/// with position, tangent and curvature matched to both support normal
/// sections at the contacts (true curvature continuity across the contact
/// curves, unlike the G1 rolling ball).
pub fn g2_fillet(
    a: &Surface,
    b: &Surface,
    radius: f64,
    options: &FilletOptions,
) -> Result<FilletReport> {
    check(
        radius.is_finite() && radius > 0.,
        "G2 fillet radius must be finite and positive",
    )?;
    let (m, sa, sb) = prepare(a, b, options)?;
    let minimum = minimum_radius(a, b, m.tol);
    if radius < minimum {
        return Ok(FilletReport::failed(
            FailureReason::RadiusBelowMinimum { radius, minimum },
            None,
            format!("radius {radius:.3e} below the numerical minimum {minimum:.3e}"),
            Vec::new(),
        ));
    }
    let warnings = reachability_warnings(a, b, sa, sb, radius);
    let law = RadiusLaw::Constant(radius);
    let (raw, seed, closed) = trace_spine(&m, radius, options.max_march_steps)?;
    if seed.is_none() {
        return Ok(FilletReport::failed(
            FailureReason::SeedNotFound,
            None,
            "no converging intersection of the two offset surfaces",
            warnings,
        ));
    }
    build_report(&m, &raw, &law, options, warnings, closed, SectionKind::G2)
}

/// Arc-length resampling of a polyline into `n` uniform stations.
fn resample_polyline(line: &[[f64; 3]], n: usize) -> Result<Vec<[f64; 3]>> {
    check(
        line.len() >= 2 && line.len() <= 4096,
        "Hold lines need 2..=4096 points",
    )?;
    check(
        line.iter().flatten().all(|x| x.is_finite()),
        "Hold line points must be finite",
    )?;
    let mut s = vec![0.; line.len()];
    for i in 1..line.len() {
        s[i] = s[i - 1] + norm(sub(line[i], line[i - 1]));
    }
    let total = s[line.len() - 1];
    check(total > 0., "Hold line must have positive length")?;
    let mut out = Vec::with_capacity(n);
    let mut span = 0;
    for k in 0..n {
        let target = total * k as f64 / (n - 1) as f64;
        while span + 1 < line.len() && s[span + 1] < target {
            span += 1;
        }
        let next = (span + 1).min(line.len() - 1);
        let w = if s[next] > s[span] { ((target - s[span]) / (s[next] - s[span])).clamp(0., 1.) } else { 0. };
        out.push(add(line[span], scale(sub(line[next], line[span]), w)));
    }
    Ok(out)
}

/// Local closest-point recovery for hold-line stations: coarse grid seed plus
/// Gauss-Newton polish. Returns `(uv, point, normal)`.
fn project_point(surface: &Surface, target: [f64; 3], tol: f64) -> Result<Option<([f64; 2], [f64; 3], [f64; 3])>> {
    let (du, dv) = domain(surface);
    let mut best: Option<(f64, f64, f64)> = None;
    for i in 0..SEED_GRID {
        for j in 0..SEED_GRID {
            let u = du[0] + (du[1] - du[0]) * (i as f64 + 0.5) / SEED_GRID as f64;
            let v = dv[0] + (dv[1] - dv[0]) * (j as f64 + 0.5) / SEED_GRID as f64;
            if let Some((p, _)) = point_normal(surface, u, v)? {
                let d = norm(sub(p, target));
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, u, v));
                }
            }
        }
    }
    let Some((_, mut u, mut v)) = best else { return Ok(None) };
    for _ in 0..NEWTON_ITERS {
        let e = match surface.evaluate(u, v) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };
        let Some((su, sv)) = e.first_derivatives() else { return Ok(None) };
        let r = sub(e.point, target);
        if norm(r) <= tol * 0.1 {
            break;
        }
        let g = [[dot(su, su), dot(su, sv)], [dot(su, sv), dot(sv, sv)]];
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        if det.abs() <= 1e-300 {
            return Ok(None);
        }
        let rhs = [dot(r, su), dot(r, sv)];
        let du_ = (g[1][1] * rhs[0] - g[0][1] * rhs[1]) / det;
        let dv_ = (-g[0][1] * rhs[0] + g[0][0] * rhs[1]) / det;
        u -= du_;
        v -= dv_;
        if !(du[0] - 1e-9..=du[1] + 1e-9).contains(&u) || !(dv[0] - 1e-9..=dv[1] + 1e-9).contains(&v) {
            return Ok(None);
        }
    }
    match point_normal(surface, u, v)? {
        Some((p, n)) if norm(sub(p, target)) <= 100. * tol => Ok(Some(([u, v], p, n))),
        _ => Ok(None),
    }
}

/// Simplified hold-line fillet: sections are tangent cubics through the given
/// hold-line points on the two supports (tangent to both supports at the hold
/// points). Both hold lines are resampled by arc length for a consistent
/// station correspondence. The spine/radius fields of the report are not
/// meaningful here; `radius_range` is `[0, 0]`.
pub fn hold_line_fillet(
    a: &Surface,
    b: &Surface,
    line_a: &[[f64; 3]],
    line_b: &[[f64; 3]],
    options: &FilletOptions,
) -> Result<FilletReport> {
    a.validate()?;
    b.validate()?;
    options.validated()?;
    let n = line_a.len().min(line_b.len()).clamp(2, MAX_SECTIONS).min(options.max_sections.max(2));
    let stations_a = resample_polyline(line_a, n)?;
    let stations_b = resample_polyline(line_b, n)?;
    let warnings = Vec::new();
    let mut sections = Vec::with_capacity(n);
    let mut spine = Vec::with_capacity(n);
    for k in 0..n {
        let t = k as f64 / (n - 1) as f64;
        let pa = project_point(a, stations_a[k], options.tolerance)?;
        let pb = project_point(b, stations_b[k], options.tolerance)?;
        let (Some((uva, pa_pt, na)), Some((uvb, pb_pt, nb))) = (pa, pb) else {
            return Ok(FilletReport::failed(
                FailureReason::MarchingDiverged,
                Some(t),
                format!("hold-line station {k} does not project onto its support within tolerance"),
                warnings,
            ));
        };
        let chord = sub(pb_pt, pa_pt);
        let chord_len = norm(chord);
        if chord_len <= 1e-12 {
            return Ok(FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(t),
                "hold-line station contacts coincide",
                warnings,
            ));
        }
        // Tangent directions: chord projected into each support tangent plane.
        let ta = unit(sub(chord, scale(na, dot(chord, na))));
        let tb = unit(sub(chord, scale(nb, dot(chord, nb))));
        let (Some(ta), Some(tb)) = (ta, tb) else {
            return Ok(FilletReport::failed(
                FailureReason::SectionSelfIntersection,
                Some(t),
                "hold-line chord is normal to a support; no tangent cubic exists",
                warnings,
            ));
        };
        let handle = chord_len / 3.;
        let p1 = add(pa_pt, scale(ta, handle));
        let p2 = sub(pb_pt, scale(tb, handle));
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![pa_pt.to_vec(), p1.to_vec(), p2.to_vec(), pb_pt.to_vec()],
            weights: vec![1.; 4],
            periodic: false,
        };
        curve.validate()?;
        sections.push(curve);
        spine.push(SpineSample {
            t,
            center: scale(add(pa_pt, pb_pt), 0.5),
            contact_a: pa_pt,
            contact_b: pb_pt,
            radius: 0.,
            uv_a: uva,
            uv_b: uvb,
        });
    }
    let params: Vec<f64> = (0..n).map(|k| k as f64).collect();
    match natural_loft::interpolate(&sections, &params) {
        Ok(surface) => Ok(FilletReport {
            surface: Some(surface),
            failure: None,
            spine,
            radius_range: [0.; 2],
            warnings,
            skin_deviation_estimate: 0.,
            closed_spine: false,
        }),
        Err(e) => Ok(FilletReport::failed(
            FailureReason::AssemblyFailed,
            None,
            format!("natural loft of hold-line sections failed: {e}"),
            warnings,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Axis;

    /// Bilinear plane patch through the origin spanned by two unit axes.
    fn plane(u_dir: [f64; 3], v_dir: [f64; 3], size: f64) -> Surface {
        let p = |su: f64, sv: f64| {
            vec![
                u_dir[0] * su + v_dir[0] * sv,
                u_dir[1] * su + v_dir[1] * sv,
                u_dir[2] * su + v_dir[2] * sv,
            ]
        };
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![p(0., 0.), p(0., size)],
                vec![p(size, 0.), p(size, size)],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Cylinder of radius 3 about the Y axis through (0, ·, 4), y in [0, 8].
    fn cylinder_y() -> Surface {
        let profile = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![3., 0., 4.], vec![3., 8., 4.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        crate::surface::revolve(&profile, [0., 0., 4.], [0., 1., 0.], 360.).unwrap()
    }

    /// Perpendicular planes z=0 (normal +z) and x=0 (normal +x).
    fn corner_planes() -> (Surface, Surface) {
        (
            plane([1., 0., 0.], [0., 1., 0.], 10.),
            plane([0., 1., 0.], [0., 0., 1.], 10.),
        )
    }

    /// Signed side of the cylinder's normal (outward = +1).
    fn cylinder_outward_sign(c: &Surface) -> f64 {
        let (du, dv) = domain(c);
        // Avoid knot seams (revolve joints) where the normal is undefined.
        let e = c
            .evaluate(du[0] + 0.37 * (du[1] - du[0]), dv[0] + 0.41 * (dv[1] - dv[0]))
            .unwrap();
        let n = e.unit_normal().unwrap();
        let radial = sub(e.point, [0., e.point[1], 4.]);
        if dot(n, radial) >= 0. { 1. } else { -1. }
    }

    #[test]
    fn rolling_ball_between_perpendicular_planes_is_exact_cylinder() {
        let (a, b) = corner_planes();
        let r = 1.5;
        let report = rolling_ball_fillet(&a, &b, r, &FilletOptions::default()).unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        let surface = report.surface.as_ref().unwrap();
        let (du, dv) = domain(surface);
        // Every surface point sits on the pipe of radius r around the spine
        // line x = r, z = r (the analytic rolling-ball locus); tangency edges
        // lie in the two planes.
        for i in 0..=8 {
            for j in 0..=8 {
                let p = surface
                    .evaluate(du[0] + (du[1] - du[0]) * i as f64 / 8., dv[0] + (dv[1] - dv[0]) * j as f64 / 8.)
                    .unwrap()
                    .point;
                let to_axis = ((p[0] - r).powi(2) + (p[2] - r).powi(2)).sqrt();
                assert!((to_axis - r).abs() < 1e-6, "p = {p:?}, |p − axis| = {to_axis}");
                assert!(p[0] >= -1e-9 && p[2] >= -1e-9);
            }
        }
        // Contact edges: u = 0 on plane z = 0, u = 1 on plane x = 0.
        for j in 0..=4 {
            let v = dv[0] + (dv[1] - dv[0]) * j as f64 / 4.;
            let p0 = surface.evaluate(du[0], v).unwrap().point;
            let p1 = surface.evaluate(du[1], v).unwrap().point;
            assert!(p0[2].abs() < 1e-9, "start contact: {p0:?}");
            assert!(p1[0].abs() < 1e-9, "end contact: {p1:?}");
        }
        assert_eq!(report.radius_range, [r, r]);
        assert!(!report.spine.is_empty());
        // Spine centers are at distance r from both planes.
        for s in &report.spine {
            assert!((s.center[0] - r).abs() < 1e-6 && (s.center[2] - r).abs() < 1e-6);
        }
    }

    #[test]
    fn rolling_ball_plane_cylinder_matches_analytic_spine() {
        let a = plane([1., 0., 0.], [0., 1., 0.], 10.); // z = 0, normal +z
        let c = cylinder_y();
        let side_c = cylinder_outward_sign(&c);
        let r = 1.0;
        let options = FilletOptions { side_a: 1., side_b: side_c, ..Default::default() };
        let report = rolling_ball_fillet(&a, &c, r, &options).unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        // Analytic spine: z = r, distance from the axis = R + r = 4 → x = √7.
        let x0 = 7_f64.sqrt();
        for s in &report.spine {
            assert!((s.center[2] - r).abs() < 1e-6, "center {:?}", s.center);
            let to_axis = (s.center[0].powi(2) + (s.center[2] - 4.).powi(2)).sqrt();
            assert!((to_axis - 4.).abs() < 1e-6, "center {:?}", s.center);
            assert!((s.center[0] - x0).abs() < 1e-6);
            // Contact on the plane lies in z = 0, contact on the cylinder at
            // radius 3 from its axis.
            assert!(s.contact_a[2].abs() < 1e-6);
            let cb = (s.contact_b[0].powi(2) + (s.contact_b[2] - 4.).powi(2)).sqrt();
            assert!((cb - 3.).abs() < 1e-6);
        }
        // Surface points stay on the pipe of radius r around the spine line.
        let surface = report.surface.as_ref().unwrap();
        let (du, dv) = domain(surface);
        for i in 0..=6 {
            for j in 0..=6 {
                let p = surface
                    .evaluate(du[0] + (du[1] - du[0]) * i as f64 / 6., dv[0] + (dv[1] - dv[0]) * j as f64 / 6.)
                    .unwrap()
                    .point;
                let to_axis = ((p[0] - x0).powi(2) + (p[2] - r).powi(2)).sqrt();
                assert!((to_axis - r).abs() < 1e-5, "p = {p:?}, |p − axis| = {to_axis}");
            }
        }
    }

    #[test]
    fn variable_radius_linear_law_averages_at_midpoint() {
        let (a, b) = corner_planes();
        let options = FilletOptions { max_sections: 9, ..Default::default() };
        let report = variable_radius_fillet(
            &a,
            &b,
            RadiusLaw::Linear { start: 0.5, end: 1.5 },
            &options,
        )
        .unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        // Odd station count puts a station exactly at t = 0.5.
        let mid = report
            .spine
            .iter()
            .find(|s| (s.t - 0.5).abs() < 1e-9)
            .expect("middle station");
        assert!((mid.radius - 1.0).abs() < 1e-12, "{}", mid.radius);
        // The mid section is an exact arc of radius 1.0: its points sit at
        // distance 1.0 from the mid center.
        let surface = report.surface.as_ref().unwrap();
        let (_du, dv) = domain(surface);
        let iso = surface.iso(Axis::V, dv[0] + (dv[1] - dv[0]) * 0.5).unwrap();
        for i in 0..=10 {
            let p = iso.evaluate(i as f64 / 10.).unwrap().point;
            let d = norm(sub(
                [p[0], p[1], p[2]],
                mid.center,
            ));
            assert!((d - 1.0).abs() < 1e-6, "d = {d}");
        }
        // Radii grow monotonically along the spine.
        assert!((report.radius_range[0] - 0.5).abs() < 1e-9);
        assert!((report.radius_range[1] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn chordal_on_right_angle_planes_recovers_c_over_sqrt2() {
        let (a, b) = corner_planes();
        let chord = 1.4;
        let report = chordal_fillet(&a, &b, chord, &FilletOptions::default()).unwrap();
        assert!(report.fillet.failure.is_none(), "{:?}", report.fillet.failure);
        let expected = chord / 2_f64.sqrt();
        assert!((report.radius_range[0] - expected).abs() < 1e-6, "{:?}", report.radius_range);
        assert!((report.radius_range[1] - expected).abs() < 1e-6, "{:?}", report.radius_range);
        let right = std::f64::consts::FRAC_PI_2;
        assert!((report.dihedral_range[0] - right).abs() < 1e-6);
        assert!((report.dihedral_range[1] - right).abs() < 1e-6);
        assert!(report.fillet.surface.is_some());
    }

    #[test]
    fn g2_section_curvature_matches_supports() {
        // Plane (curvature 0) against the radius-3 cylinder (normal curvature
        // magnitude 1/3 in the circumferential section direction).
        let a = plane([1., 0., 0.], [0., 1., 0.], 10.);
        let c = cylinder_y();
        let side_c = cylinder_outward_sign(&c);
        let options = FilletOptions { side_a: 1., side_b: side_c, max_sections: 5, ..Default::default() };
        let report = g2_fillet(&a, &c, 1.0, &options).unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        let surface = report.surface.as_ref().unwrap();
        let (du, dv) = domain(surface);
        let curvature = |curve: &Curve, u: f64| {
            let e = curve.evaluate(u).unwrap();
            let d1 = e.d1.unwrap();
            let d2 = e.d2.unwrap();
            norm(cross(
                [d1[0], d1[1], d1[2]],
                [d2[0], d2[1], d2[2]],
            )) / norm([d1[0], d1[1], d1[2]]).powi(3)
        };
        // Check several stations: curvature at u = 0 ≈ 0 (plane), at u = 1
        // ≈ 1/3 (cylinder, sign carried by direction not magnitude).
        for k in 1..4 {
            let v = dv[0] + (dv[1] - dv[0]) * k as f64 / 4.;
            let iso = surface.iso(Axis::V, v).unwrap();
            let k_plane = curvature(&iso, du[0]);
            let k_cyl = curvature(&iso, du[1]);
            assert!(k_plane < 1e-3, "station {k}: plane end k = {k_plane}");
            assert!((k_cyl - 1. / 3.).abs() < 1e-3, "station {k}: cylinder end k = {k_cyl}");
        }
    }

    #[test]
    fn hold_line_sections_pass_through_resampled_lines_and_stay_tangent() {
        let (a, b) = corner_planes();
        let line_a: Vec<[f64; 3]> = (0..=4).map(|i| [1., 8. * i as f64 / 4., 0.]).collect();
        let line_b: Vec<[f64; 3]> = (0..=4).map(|i| [0., 8. * i as f64 / 4., 1.]).collect();
        let report = hold_line_fillet(&a, &b, &line_a, &line_b, &FilletOptions::default()).unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        let surface = report.surface.as_ref().unwrap();
        let (du, dv) = domain(surface);
        // Station iso curves hit the hold points and start/end tangent to the
        // supports (section tangent perpendicular to each plane normal).
        for k in 0..report.spine.len() {
            let v = dv[0] + (dv[1] - dv[0]) * report.spine[k].t;
            let iso = surface.iso(Axis::V, v).unwrap();
            let start = iso.evaluate(iso.domain()[0]).unwrap();
            let end = iso.evaluate(iso.domain()[1]).unwrap();
            for (p, q) in [(start.point.clone(), report.spine[k].contact_a), (end.point.clone(), report.spine[k].contact_b)] {
                assert!(norm(sub([p[0], p[1], p[2]], q)) < 1e-8, "{p:?} vs {q:?}");
            }
            let t0 = start.d1.unwrap();
            let t1 = end.d1.unwrap();
            // Plane z = 0 normal is +z; plane x = 0 normal is +x.
            assert!(t0[2].abs() / norm([t0[0], t0[1], t0[2]]) < 1e-6, "t0 = {t0:?}");
            assert!(t1[0].abs() / norm([t1[0], t1[1], t1[2]]) < 1e-6, "t1 = {t1:?}");
        }
    }

    #[test]
    fn tiny_radius_fails_gracefully_without_panic() {
        let (a, b) = corner_planes();
        let report = rolling_ball_fillet(&a, &b, 1e-12, &FilletOptions::default()).unwrap();
        assert!(report.surface.is_none());
        let failure = report.failure.as_ref().expect("failure must be reported");
        assert!(matches!(
            failure.reason,
            FailureReason::RadiusBelowMinimum { .. } | FailureReason::SeedNotFound
        ));
        // Sensible radius on the wrong side also fails gracefully.
        let wrong = rolling_ball_fillet(
            &a,
            &b,
            1.5,
            &FilletOptions { side_a: -1., side_b: 1., ..Default::default() },
        )
        .unwrap();
        if let Some(f) = wrong.failure {
            assert!(f.detail.len() > 0);
        }
    }

    #[test]
    fn spline_law_and_invalid_inputs() {
        let (a, b) = corner_planes();
        // Invalid inputs are errors, not reports.
        assert!(rolling_ball_fillet(&a, &b, f64::NAN, &FilletOptions::default()).is_err());
        assert!(rolling_ball_fillet(&a, &b, -1., &FilletOptions::default()).is_err());
        assert!(chordal_fillet(&a, &b, 0., &FilletOptions::default()).is_err());
        assert!(
            variable_radius_fillet(&a, &b, RadiusLaw::Spline(vec![(0., 1.), (0., 2.)]), &FilletOptions::default()).is_err()
        );
        // A smooth spline law builds a surface.
        let report = variable_radius_fillet(
            &a,
            &b,
            RadiusLaw::Spline(vec![(0., 0.8), (0.5, 1.2), (1., 0.8)]),
            &FilletOptions::default(),
        )
        .unwrap();
        assert!(report.failure.is_none(), "{:?}", report.failure);
        assert!(report.radius_range[0] >= 0.8 - 0.1 && report.radius_range[1] >= 1.1);
    }

}
