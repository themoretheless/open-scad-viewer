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
    foundation::guards::{Budget, require_finite_at, require_finite_f64, require_finite_point},
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

mod radius_law;
pub use radius_law::{RadiusLaw};


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
        let mut budget = Budget::with_iterations(NEWTON_ITERS + 1)?.guard("fillet_newton");
        for _ in 0..NEWTON_ITERS {
            budget.tick()?;
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

mod marching;
use marching::*;


mod sections;
use sections::*;


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
    require_finite_f64(chord, "chord")?;
    check(chord > 0., "Chordal fillet chord must be positive")?;
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
    require_finite_f64(radius, "radius")?;
    check(radius > 0., "G2 fillet radius must be positive")?;
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

mod projection;
use projection::*;


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
#[path = "tests/fillet_surfaces.rs"]
mod tests;
