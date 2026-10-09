//! One-shot analytic surface classification and trimmed face area for AAG
//! face attributes (checklist 865).
//!
//! The classifier is deliberately *best-fit with tolerance*, not exact-type
//! matching: an imported "almost cylinder" (a B-spline deviating from a true
//! cylinder by less than the fit tolerance) must still read as
//! [`SurfaceClass::Cylinder`]. `nurbs-core/src/foundation/fitting` only fits
//! B-spline curves/surfaces to points — it has no analytic-primitive best-fit —
//! so the primitive fits (plane/sphere/cylinder/cone/torus) are implemented
//! locally over a sampled grid of points and unit normals:
//!
//! 1. **Plane** — normals mutually parallel and points coplanar.
//! 2. **Sphere** — algebraic center/radius fit; gated on point residual and
//!    radial normal alignment.
//! 3. **Cylinder** — axis = smallest-eigenvalue eigenvector of the normal
//!    covariance (normals sweep a circle in the plane ⊥ axis for *any* arc
//!    extent), then a circular fit of the axis-projected points.
//! 4. **Cone** — same covariance axis, but normals hold a constant nonzero
//!    angle to it; apex located by a 1-D search minimizing the spread of the
//!    generator angle.
//! 5. **Torus** — axis = largest-eigenvalue eigenvector of the point inertia
//!    tensor (valid for patches spanning most of the ring; narrower patches
//!    fall through to freeform), then a circular fit in (ρ, z).
//!
//! Every acceptance gate compares a maximum sample residual against the
//! caller-supplied fit tolerance, so the exact-type fast path and the noisy
//! best-fit path share one code path with one threshold.
//!
//! Face area uses Green's theorem over the authored UV trims (outer wire
//! minus holes), with adaptive Gauss subdivision until relative convergence
//! better than 1e-9, bounded by the caller's [`Budget`]. It converges to the
//! `mass_properties` integral estimate; the module tests pin 1e-6 relative
//! agreement on analytic solids.

use crate::{Error, Model, Result};
use nurbs_core::core::surface::Surface;
use nurbs_core::foundation::guards::{Budget, BudgetGuard, require_finite_f64};
use nurbs_core::surface::SurfaceSampler;

/// Sample grid side per face (9×9 = 81 evaluations). Fixed so classification
/// cost is predictable and budget-chargeable.
pub const SURFACE_CLASSIFY_SAMPLES: usize = 9;
/// Minimum valid (non-pole) samples required for a verdict.
const MIN_SAMPLES: usize = 12;
/// Normal-direction gates: acceptance angles for axis/normal alignment checks.
/// 0.05 rad ≈ 2.9° absorbs the normal tilt of a best-fit-noisy surface while
/// still separating the analytic classes decisively.
const NORMAL_GATE: f64 = 0.05;
/// Gauss-Legendre 5-point rule (same nodes as `analysis.rs` mass quadrature).
const GAUSS: [(f64, f64); 5] = [
    (-0.906179845938664, 0.236926885056189),
    (-0.538469310105683, 0.478628670499366),
    (0., 0.568888888888889),
    (0.538469310105683, 0.478628670499366),
    (0.906179845938664, 0.236926885056189),
];

/// Analytic surface class of one face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceClass {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Torus,
    /// Not an analytic primitive within the fit tolerance.
    Freeform,
}

/// 3D axis line: a point on the line plus a unit direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis {
    pub point: [f64; 3],
    pub direction: [f64; 3],
}

/// Best-fit classification result for one surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceClassification {
    pub class: SurfaceClass,
    /// Axis of rotational surfaces (cylinder/cone/torus).
    pub axis: Option<Axis>,
    /// Center for spheres.
    pub center: Option<[f64; 3]>,
    /// Radius for cylinder/sphere; minor (tube) radius for torus. Cones have
    /// no single radius: `None`.
    pub radius: Option<f64>,
    /// Major (ring) radius for torus; `None` otherwise.
    pub secondary_radius: Option<f64>,
    /// Maximum sampled point deviation from the fitted primitive.
    pub max_deviation: f64,
    /// Valid samples that supported the verdict.
    pub samples: usize,
}

fn classify_error(message: impl Into<String>) -> Error {
    Error::new("BREP_SURFACE_CLASSIFY_INPUT", message)
}

// ---------------------------------------------------------------------------
// small vector helpers (mirroring aag.rs conventions)
// ---------------------------------------------------------------------------
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

mod fitting;
use fitting::*;


#[derive(Clone, Copy)]
struct Sample {
    point: [f64; 3],
    normal: [f64; 3],
}

/// Sample a regular interior grid of points and unit normals. Pole/singular
/// samples (no unit normal) are skipped; budget ticks once per evaluation.
fn sample_surface(
    sampler: &SurfaceSampler,
    guard: &mut BudgetGuard,
) -> Result<Vec<Sample>> {
    let surface = sampler.definition();
    let domain_u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - 1 - surface.degree_u],
    ];
    let domain_v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - 1 - surface.degree_v],
    ];
    let n = SURFACE_CLASSIFY_SAMPLES;
    let mut out = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            guard.tick()?;
            let u = domain_u[0] + (i as f64 + 0.5) / n as f64 * (domain_u[1] - domain_u[0]);
            let v = domain_v[0] + (j as f64 + 0.5) / n as f64 * (domain_v[1] - domain_v[0]);
            let eval = sampler.evaluate(u, v)?;
            let Some(normal) = eval.unit_normal() else {
                continue;
            };
            let point = eval.point;
            require_finite_f64(point[0], "sample.point.x")?;
            require_finite_f64(point[1], "sample.point.y")?;
            require_finite_f64(point[2], "sample.point.z")?;
            out.push(Sample { point, normal });
        }
    }
    Ok(out)
}

fn freeform(samples: usize) -> SurfaceClassification {
    SurfaceClassification {
        class: SurfaceClass::Freeform,
        axis: None,
        center: None,
        radius: None,
        secondary_radius: None,
        max_deviation: 0.,
        samples,
    }
}

fn try_plane(samples: &[Sample], tol: f64) -> Option<f64> {
    let reference = samples[0].normal;
    if samples
        .iter()
        .any(|s| 1. - dot(s.normal, reference).abs() > NORMAL_GATE)
    {
        return None;
    }
    let origin = samples[0].point;
    let deviation = samples
        .iter()
        .map(|s| dot(sub(s.point, origin), reference).abs())
        .fold(0., f64::max);
    (deviation <= tol).then_some(deviation)
}

fn try_sphere(samples: &[Sample], tol: f64) -> Option<([f64; 3], f64, f64)> {
    let points: Vec<[f64; 3]> = samples.iter().map(|s| s.point).collect();
    let (center, radius) = fit_sphere(&points)?;
    let deviation = samples
        .iter()
        .map(|s| (norm(sub(s.point, center)) - radius).abs())
        .fold(0., f64::max);
    if deviation > tol {
        return None;
    }
    // Radial normal alignment.
    let misalignment = samples
        .iter()
        .map(|s| {
            let radial = sub(s.point, center);
            let length = norm(radial);
            if length <= 0. {
                return 1.;
            }
            1. - dot(s.normal, scale(radial, 1. / length)).abs()
        })
        .fold(0., f64::max);
    (misalignment <= NORMAL_GATE).then_some((center, radius, deviation))
}

/// Normal covariance eigenstructure shared by the cylinder and cone fits:
/// normals of a rotational surface sweep a (small) circle whose plane is
/// perpendicular to the axis, so the smallest-eigenvalue eigenvector of the
/// covariance is the axis for any patch extent.
fn normal_axis(samples: &[Sample]) -> Option<([f64; 3], f64, f64)> {
    let n = samples.len() as f64;
    let mean = scale(
        samples.iter().map(|s| s.normal).fold([0.; 3], add),
        1. / n,
    );
    let mut cov = [[0.; 3]; 3];
    for s in samples {
        let d = sub(s.normal, mean);
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    let (values, vectors) = eigen_symmetric3(cov);
    // Normals must actually spread; otherwise the axis is unobservable.
    if values[2] <= 1e-12 {
        return None;
    }
    let axis = eigenvector(&(values, vectors), 0);
    // Alignment of each normal with the axis candidate.
    let mean_abs = samples
        .iter()
        .map(|s| dot(s.normal, axis).abs())
        .sum::<f64>()
        / n;
    let max_abs = samples
        .iter()
        .map(|s| dot(s.normal, axis).abs())
        .fold(0., f64::max);
    Some((axis, mean_abs, max_abs))
}

/// Best-fit axis line through the projected centroid, radius from a circular
/// fit of axis-projected points. Returns (axis, radius, max radial deviation).
fn fit_axis_and_radius(samples: &[Sample], direction: [f64; 3]) -> Option<(Axis, f64, f64)> {
    let n = samples.len() as f64;
    let centroid = scale(
        samples.iter().map(|s| s.point).fold([0.; 3], add),
        1. / n,
    );
    // Project points into the plane ⊥ direction through the centroid and fit
    // a circle in an orthonormal 2D frame.
    let reference = if direction[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let e1_raw = cross(direction, reference);
    let e1 = scale(e1_raw, 1. / norm(e1_raw));
    let e2 = cross(direction, e1);
    let projected: Vec<[f64; 2]> = samples
        .iter()
        .map(|s| {
            let d = sub(s.point, centroid);
            [dot(d, e1), dot(d, e2)]
        })
        .collect();
    let (center2, radius) = fit_circle_2d(&projected)?;
    let axis_point = add(centroid, add(scale(e1, center2[0]), scale(e2, center2[1])));
    let axis = Axis {
        point: axis_point,
        direction,
    };
    let deviation = samples
        .iter()
        .map(|s| {
            let d = sub(s.point, axis_point);
            let along = dot(d, direction);
            (norm(sub(d, scale(direction, along))) - radius).abs()
        })
        .fold(0., f64::max);
    Some((axis, radius, deviation))
}

fn try_cylinder(samples: &[Sample], tol: f64) -> Option<(Axis, f64, f64)> {
    let (axis_dir, mean_abs, max_abs) = normal_axis(samples)?;
    // Cylinder normals are (nearly) perpendicular to the axis; a spherical
    // cap or a cone fails this gate even when its covariance axis is good.
    if mean_abs > NORMAL_GATE || max_abs > 2. * NORMAL_GATE {
        return None;
    }
    let (axis, radius, deviation) = fit_axis_and_radius(samples, axis_dir)?;
    if !(radius > 0.) || deviation > tol {
        return None;
    }
    // Radial normal alignment with the fitted axis.
    let misalignment = samples
        .iter()
        .map(|s| {
            let d = sub(s.point, axis.point);
            let radial = sub(d, scale(axis.direction, dot(d, axis.direction)));
            let length = norm(radial);
            if length <= 0. {
                return 1.;
            }
            1. - dot(s.normal, scale(radial, 1. / length)).abs()
        })
        .fold(0., f64::max);
    (misalignment <= NORMAL_GATE).then_some((axis, radius, deviation))
}

fn try_cone(samples: &[Sample], tol: f64, scale_hint: f64) -> Option<(Axis, f64)> {
    let _ = scale_hint;
    let (axis_dir, _mean_abs, _max_abs) = normal_axis(samples)?;
    // Cone normals hold a constant, nonzero, non-perpendicular angle to the
    // axis: the signed projections must cluster away from 0 and ±1.
    let signed: Vec<f64> = samples.iter().map(|s| dot(s.normal, axis_dir)).collect();
    let mean = signed.iter().sum::<f64>() / signed.len() as f64;
    if !(NORMAL_GATE..=1. - NORMAL_GATE).contains(&mean.abs()) {
        return None;
    }
    let spread = signed.iter().map(|q| (q - mean).abs()).fold(0., f64::max);
    if spread > NORMAL_GATE {
        return None;
    }
    // Axis line position: the axis-perpendicular part of every normal is
    // parallel to the radial direction, so the projected normal lines concur
    // at the axis (exact for any rotational surface).
    let axis_point = axis_concurrency_point(samples, axis_dir)?;
    // Radius vs height is linear for a cone: ρ(s) = a + b·s, apex at ρ = 0.
    let (a, b, deviation) = radius_height_regression(samples, axis_dir, axis_point);
    if b.abs() <= 1e-3 || deviation > tol {
        return None;
    }
    let direction = if b < 0. {
        // Radius shrinks along +d: the apex lies in the +d direction.
        axis_dir
    } else {
        scale(axis_dir, -1.)
    };
    let apex = add(axis_point, scale(axis_dir, -a / b));
    Some((
        Axis {
            point: apex,
            direction,
        },
        deviation,
    ))
}

/// Least-squares concurrency point of the axis-perpendicular normal
/// components: exact for any rotational surface (the perpendicular component
/// is parallel to the radial direction). Returns a 3D point on the axis.
fn axis_concurrency_point(samples: &[Sample], direction: [f64; 3]) -> Option<[f64; 3]> {
    let reference = if direction[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let e1_raw = cross(direction, reference);
    let e1 = scale(e1_raw, 1. / norm(e1_raw));
    let e2 = cross(direction, e1);
    // Line i in the ⊥ plane: dot(q, h_i) = dot(p_i, h_i). Least squares.
    let (mut a00, mut a01, mut a11, mut b0, mut b1) = (0., 0., 0., 0., 0.);
    let mut used = 0usize;
    for s in samples {
        let h = sub(s.normal, scale(direction, dot(s.normal, direction)));
        if norm(h) < 0.1 {
            // Normal parallel to the axis: radial direction unobservable.
            continue;
        }
        let h2 = [dot(h, e1), dot(h, e2)];
        let p2 = [dot(s.point, e1), dot(s.point, e2)];
        // The projected line runs ALONG h2 through p2, so its implicit
        // equation uses the perpendicular g = h2⟂: dot(q, g) = dot(p2, g).
        // Normalize so every sample weighs equally in the least squares.
        let g = [-h2[1], h2[0]];
        let gl = (g[0] * g[0] + g[1] * g[1]).sqrt();
        let g = [g[0] / gl, g[1] / gl];
        let rhs = g[0] * p2[0] + g[1] * p2[1];
        a00 += g[0] * g[0];
        a01 += g[0] * g[1];
        a11 += g[1] * g[1];
        b0 += g[0] * rhs;
        b1 += g[1] * rhs;
        used += 1;
    }
    if used < 3 {
        return None;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() <= 1e-14 * (a00 + a11).powi(2).max(1e-300) {
        return None;
    }
    let c0 = (b0 * a11 - b1 * a01) / det;
    let c1 = (a00 * b1 - a01 * b0) / det;
    if !c0.is_finite() || !c1.is_finite() {
        return None;
    }
    Some(add(scale(e1, c0), scale(e2, c1)))
}

/// Linear regression ρ(s) = a + b·s of axis distance against height along the
/// axis. Returns (a, b, max residual).
fn radius_height_regression(
    samples: &[Sample],
    direction: [f64; 3],
    axis_point: [f64; 3],
) -> (f64, f64, f64) {
    let mut data = Vec::with_capacity(samples.len());
    for s in samples {
        let d = sub(s.point, axis_point);
        let along = dot(d, direction);
        let rho = norm(sub(d, scale(direction, along)));
        data.push((along, rho));
    }
    let n = data.len() as f64;
    let sx: f64 = data.iter().map(|d| d.0).sum();
    let sy: f64 = data.iter().map(|d| d.1).sum();
    let sxx: f64 = data.iter().map(|d| d.0 * d.0).sum();
    let sxy: f64 = data.iter().map(|d| d.0 * d.1).sum();
    let det = sxx * n - sx * sx;
    if det.abs() <= 1e-300 {
        return (0., 0., f64::INFINITY);
    }
    let b = (sxy * n - sx * sy) / det;
    let a = (sy - b * sx) / n;
    let deviation = data
        .iter()
        .map(|d| (d.1 - (a + b * d.0)).abs())
        .fold(0., f64::max);
    (a, b, deviation)
}

/// Concurrency residual of the axis-perpendicular normal components for a
/// candidate axis direction: RMS distance from the least-squares concurrency
/// point to the projected normal lines. Zero for the true axis of any
/// rotational surface; large for wrong directions.
fn concurrency_residual(samples: &[Sample], direction: [f64; 3]) -> f64 {
    let reference = if direction[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let e1_raw = cross(direction, reference);
    let e1 = scale(e1_raw, 1. / norm(e1_raw));
    let e2 = cross(direction, e1);
    let (mut a00, mut a01, mut a11, mut b0, mut b1) = (0., 0., 0., 0., 0.);
    let mut lines = Vec::with_capacity(samples.len());
    for s in samples {
        let h = sub(s.normal, scale(direction, dot(s.normal, direction)));
        if norm(h) < 0.1 {
            continue;
        }
        let h2 = [dot(h, e1), dot(h, e2)];
        let p2 = [dot(s.point, e1), dot(s.point, e2)];
        let g = [-h2[1], h2[0]];
        let gl = (g[0] * g[0] + g[1] * g[1]).sqrt();
        let g = [g[0] / gl, g[1] / gl];
        let rhs = g[0] * p2[0] + g[1] * p2[1];
        a00 += g[0] * g[0];
        a01 += g[0] * g[1];
        a11 += g[1] * g[1];
        b0 += g[0] * rhs;
        b1 += g[1] * rhs;
        lines.push((g, rhs));
    }
    if lines.len() < 3 {
        return f64::INFINITY;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() <= 1e-14 * (a00 + a11).powi(2).max(1e-300) {
        return f64::INFINITY;
    }
    let c0 = (b0 * a11 - b1 * a01) / det;
    let c1 = (a00 * b1 - a01 * b0) / det;
    if !c0.is_finite() || !c1.is_finite() {
        return f64::INFINITY;
    }
    let sum: f64 = lines
        .iter()
        .map(|(g, rhs)| (g[0] * c0 + g[1] * c1 - rhs).powi(2))
        .sum();
    (sum / lines.len() as f64).sqrt()
}

/// Search the direction sphere for the axis minimizing the concurrency
/// residual: seeds on a coarse grid plus the inertia eigenvectors, then
/// pattern-search refinement in two tangent directions. Returns the best
/// direction and its residual.
fn rotational_axis_search(
    samples: &[Sample],
    extra_seeds: &[[f64; 3]],
    guard: &mut BudgetGuard,
) -> Result<Option<([f64; 3], f64)>> {
    let mut seeds: Vec<[f64; 3]> = vec![
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0.5773502691896258; 3],
        [0.5773502691896258, 0.5773502691896258, -0.5773502691896258],
        [0.5773502691896258, -0.5773502691896258, 0.5773502691896258],
        [-0.5773502691896258, 0.5773502691896258, 0.5773502691896258],
    ];
    seeds.extend_from_slice(extra_seeds);
    let mut best: Option<([f64; 3], f64)> = None;
    for seed in seeds {
        guard.tick()?;
        let mut d = scale(seed, 1. / norm(seed));
        let mut r = concurrency_residual(samples, d);
        if !r.is_finite() {
            continue;
        }
        // Pattern search on the sphere with a shrinking angular step.
        let mut step = 0.5f64;
        while step > 1e-9 {
            let reference = if d[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
            let t1_raw = cross(d, reference);
            let t1 = scale(t1_raw, 1. / norm(t1_raw));
            let t2 = cross(d, t1);
            let mut improved = false;
            for (a, b) in [(1., 0.), (-1., 0.), (0., 1.), (0., -1.)] {
                let candidate = add(d, add(scale(t1, a * step), scale(t2, b * step)));
                let candidate = scale(candidate, 1. / norm(candidate));
                let cr = concurrency_residual(samples, candidate);
                if cr < r {
                    d = candidate;
                    r = cr;
                    improved = true;
                }
            }
            if !improved {
                step *= 0.5;
            }
        }
        if best.as_ref().is_none_or(|b| r < b.1) {
            best = Some((d, r));
        }
    }
    Ok(best)
}

fn try_torus(
    samples: &[Sample],
    tol: f64,
    scale_hint: f64,
    guard: &mut BudgetGuard,
) -> Result<Option<(Axis, f64, f64, f64)>> {
    let _ = scale_hint;
    // Inertia eigenvectors are extra seeds; the concurrency search below
    // finds the axis even for narrow tiles where inertia is ambiguous.
    let n = samples.len() as f64;
    let centroid = scale(
        samples.iter().map(|s| s.point).fold([0.; 3], add),
        1. / n,
    );
    let mut inertia = [[0.; 3]; 3];
    for s in samples {
        let d = sub(s.point, centroid);
        let r2 = dot(d, d);
        for i in 0..3 {
            for j in 0..3 {
                inertia[i][j] += if i == j { r2 - d[i] * d[j] } else { -d[i] * d[j] };
            }
        }
    }
    let eigen = eigen_symmetric3(inertia);
    let seeds: Vec<[f64; 3]> = (0..3).map(|w| eigenvector(&eigen, w)).collect();
    let Some((direction, axis_residual)) = rotational_axis_search(samples, &seeds, guard)? else {
        return Ok(None);
    };
    // The concurrency residual is an axis-position accuracy bound; it must
    // beat the fit tolerance for the downstream circle fit to mean anything.
    if axis_residual > tol {
        return Ok(None);
    }
    let Some(axis_point) = axis_concurrency_point(samples, direction) else {
        return Ok(None);
    };
    // Circle fit in (ρ, z): ρ = distance from the axis, z = height.
    let rho_z: Vec<[f64; 2]> = samples
        .iter()
        .map(|s| {
            let d = sub(s.point, axis_point);
            let z = dot(d, direction);
            let rho = norm(sub(d, scale(direction, z)));
            [rho, z]
        })
        .collect();
    let Some((center, minor)) = fit_circle_2d(&rho_z) else {
        return Ok(None);
    };
    let (major, z0) = (center[0], center[1]);
    // Ring torus only, and the patch must not cross the axis.
    if !(minor > 0.) || !(major > 0.) || minor >= major {
        return Ok(None);
    }
    let min_rho = rho_z.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    if min_rho <= 0.1 * major {
        return Ok(None);
    }
    let deviation = rho_z
        .iter()
        .map(|p| (((p[0] - major).powi(2) + (p[1] - z0).powi(2)).sqrt() - minor).abs())
        .fold(0., f64::max);
    if deviation > tol {
        return Ok(None);
    }
    // Radial normal alignment with the fitted tube circle.
    let misalignment = samples
        .iter()
        .map(|s| {
            let d = sub(s.point, axis_point);
            let z = dot(d, direction);
            let rho_dir = sub(d, scale(direction, z));
            let rho_len = norm(rho_dir);
            if rho_len <= 1e-300 {
                return 1.;
            }
            let q = add(
                add(axis_point, scale(direction, z0)),
                scale(rho_dir, major / rho_len),
            );
            let radial = sub(s.point, q);
            let length = norm(radial);
            if length <= 1e-300 {
                return 1.;
            }
            1. - dot(s.normal, scale(radial, 1. / length)).abs()
        })
        .fold(0., f64::max);
    if misalignment > NORMAL_GATE {
        return Ok(None);
    }
    Ok(Some((
        Axis {
            point: axis_point,
            direction,
        },
        minor,
        major,
        deviation,
    )))
}

/// Classify one surface by best-fit against the analytic primitives, accepting
/// a class when the maximum sampled point deviation stays within
/// `fit_tolerance` (an absolute length, same units as the model). Falls back
/// to [`SurfaceClass::Freeform`] when nothing fits. Never fails on weird
/// geometry: degenerate sampling yields `Freeform`; only guard violations
/// (non-finite input, budget exhaustion) are errors.
pub fn classify_surface(
    surface: &Surface,
    fit_tolerance: f64,
    budget: &Budget,
) -> Result<SurfaceClassification> {
    require_finite_f64(fit_tolerance, "fit_tolerance")?;
    if fit_tolerance <= 0. {
        return Err(classify_error("fit_tolerance must be positive"));
    }
    let mut guard: BudgetGuard = budget.guard("surface-classify");
    guard.check()?;
    let sampler = SurfaceSampler::new(surface)?;
    let samples = sample_surface(&sampler, &mut guard)?;
    guard.check()?;
    if samples.len() < MIN_SAMPLES {
        return Ok(freeform(samples.len()));
    }
    let scale_hint = samples
        .iter()
        .flat_map(|a| samples.iter().map(move |b| norm(sub(a.point, b.point))))
        .fold(0., f64::max)
        .max(fit_tolerance);
    guard.tick()?;

    if let Some(deviation) = try_plane(&samples, fit_tolerance) {
        return Ok(SurfaceClassification {
            class: SurfaceClass::Plane,
            axis: None,
            center: None,
            radius: None,
            secondary_radius: None,
            max_deviation: deviation,
            samples: samples.len(),
        });
    }
    if let Some((center, radius, deviation)) = try_sphere(&samples, fit_tolerance) {
        return Ok(SurfaceClassification {
            class: SurfaceClass::Sphere,
            axis: None,
            center: Some(center),
            radius: Some(radius),
            secondary_radius: None,
            max_deviation: deviation,
            samples: samples.len(),
        });
    }
    if let Some((axis, radius, deviation)) = try_cylinder(&samples, fit_tolerance) {
        return Ok(SurfaceClassification {
            class: SurfaceClass::Cylinder,
            axis: Some(axis),
            center: None,
            radius: Some(radius),
            secondary_radius: None,
            max_deviation: deviation,
            samples: samples.len(),
        });
    }
    if let Some((axis, deviation)) = try_cone(&samples, fit_tolerance, scale_hint) {
        return Ok(SurfaceClassification {
            class: SurfaceClass::Cone,
            axis: Some(axis),
            center: None,
            radius: None,
            secondary_radius: None,
            max_deviation: deviation,
            samples: samples.len(),
        });
    }
    if let Some((axis, minor, major, deviation)) =
        try_torus(&samples, fit_tolerance, scale_hint, &mut guard)?
    {
        return Ok(SurfaceClassification {
            class: SurfaceClass::Torus,
            axis: Some(axis),
            center: None,
            radius: Some(minor),
            secondary_radius: Some(major),
            max_deviation: deviation,
            samples: samples.len(),
        });
    }
    Ok(freeform(samples.len()))
}

// ---------------------------------------------------------------------------
// Trimmed face area via Green's theorem
// ---------------------------------------------------------------------------

mod area;
pub use area::{face_area};


#[cfg(test)]
#[path = "tests/surface_classify.rs"]
mod tests;

