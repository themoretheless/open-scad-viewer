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

/// Jacobi eigendecomposition of a symmetric 3×3 matrix.
/// Returns (eigenvalues, eigenvectors as columns), ascending by eigenvalue.
fn eigen_symmetric3(m: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut a = m;
    let mut v = [[0.; 3]; 3];
    for i in 0..3 {
        v[i][i] = 1.;
    }
    for _ in 0..64 {
        // Largest off-diagonal entry.
        let (mut p, mut q, mut biggest) = (0, 1, 0f64);
        for i in 0..3 {
            for j in (i + 1)..3 {
                if a[i][j].abs() > biggest {
                    biggest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if biggest <= 1e-300 {
            break;
        }
        let theta = 0.5 * (a[q][q] - a[p][p]) / a[p][q];
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
        let c = 1. / (t * t + 1.).sqrt();
        let s = t * c;
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let values = [a[order[0]][order[0]], a[order[1]][order[1]], a[order[2]][order[2]]];
    let vectors = std::array::from_fn(|r| std::array::from_fn(|c| v[r][order[c]]));
    (values, vectors)
}

fn eigenvector(eigen: &([f64; 3], [[f64; 3]; 3]), which: usize) -> [f64; 3] {
    [eigen.1[0][which], eigen.1[1][which], eigen.1[2][which]]
}

/// Solve a small dense linear system by Gaussian elimination with partial
/// pivoting. Returns `None` when the system is numerically singular.
fn solve_small(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut b = rhs.to_vec();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() <= 1e-300 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in (col + 1)..n {
            let f = a[row][col] / a[col][col];
            for k in col..n {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.; n];
    for row in (0..n).rev() {
        let tail: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - tail) / a[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Algebraic (Kasa) circle fit: returns (center, radius).
fn fit_circle_2d(points: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    let n = points.len() as f64;
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy, mut sxz, mut syz, mut sz) =
        (0., 0., 0., 0., 0., 0., 0., 0.);
    for p in points {
        let (x, y) = (p[0], p[1]);
        let z = x * x + y * y;
        sx += x;
        sy += y;
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
        sxz += x * z;
        syz += y * z;
        sz += z;
    }
    let m = vec![
        vec![sxx, sxy, sx],
        vec![sxy, syy, sy],
        vec![sx, sy, n],
    ];
    let x = solve_small(&m, &[-sxz, -syz, -sz])?;
    let (a, b, c) = (x[0], x[1], x[2]);
    let center = [-a / 2., -b / 2.];
    // x²+y² + a·x + b·y + c = 0  →  r² = (a²+b²)/4 − c.
    let r2 = center[0] * center[0] + center[1] * center[1] - c;
    (r2 > 0.).then_some((center, r2.sqrt()))
}

/// Algebraic sphere fit over 3D points: returns (center, radius).
fn fit_sphere(points: &[[f64; 3]]) -> Option<([f64; 3], f64)> {
    let n = points.len() as f64;
    let mut m = vec![vec![0.; 4]; 4];
    let mut rhs = vec![0.; 4];
    for p in points {
        let z = dot(*p, *p);
        let row = [p[0], p[1], p[2], 1.];
        for i in 0..4 {
            for j in 0..4 {
                m[i][j] += row[i] * row[j];
            }
            rhs[i] += row[i] * z;
        }
    }
    let _ = n;
    let x = solve_small(&m, &rhs)?;
    let center = [x[0] / 2., x[1] / 2., x[2] / 2.];
    let r2 = x[3] + dot(center, center);
    (r2 > 0.).then_some((center, r2.sqrt()))
}

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

/// |Su × Sv| at one UV point; zero at poles. Budget ticks per evaluation.
fn area_flux(surface: &Surface, u: f64, v: f64, guard: &mut BudgetGuard) -> Result<f64> {
    guard.tick()?;
    let e = surface.evaluate(u, v)?;
    let Some((du, dv)) = e.first_derivatives() else {
        return Err(Error::new(
            "BREP_ANALYSIS_INDETERMINATE",
            "Undefined surface derivative in face-area quadrature",
        ));
    };
    Ok(norm(cross(du, dv)))
}

/// Knot spans of `knots` strictly inside `domain` (endpoints included).
fn knot_spans(knots: &[f64], degree: usize, upto: f64) -> Vec<[f64; 2]> {
    let start = knots[degree];
    let mut breaks: Vec<f64> = knots
        .iter()
        .copied()
        .filter(|&k| k > start && k < upto)
        .collect();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let mut out = Vec::with_capacity(breaks.len() + 1);
    let mut a = start;
    for b in breaks {
        if b > a {
            out.push([a, b]);
        }
        a = b;
    }
    if upto > a {
        out.push([a, upto]);
    }
    out
}

/// One Green's-theorem area pass with `divisions` subdivisions per span:
/// area = ∮ (∫_{u0}^{u(t)} |N| du) · v'(t) dt over each trim pcurve. Holes
/// contribute with opposite sign automatically because pcurves follow their
/// loop orientation.
fn area_pass(model: &Model, face_index: usize, divisions: usize, guard: &mut BudgetGuard) -> Result<f64> {
    let face = &model.faces[face_index];
    let surface = &face.surface;
    let mut total = 0.;
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for coedge in &model.loops[wire].coedges {
            let curve = &coedge.pcurve;
            let domain = curve.domain();
            require_finite_f64(domain[0], "pcurve.domain[0]")?;
            require_finite_f64(domain[1], "pcurve.domain[1]")?;
            for part in 0..divisions {
                let a = domain[0] + (domain[1] - domain[0]) * part as f64 / divisions as f64;
                let b = domain[0] + (domain[1] - domain[0]) * (part + 1) as f64 / divisions as f64;
                for (x, w) in GAUSS {
                    let t = (a + b) / 2. + x * (b - a) / 2.;
                    let eval = curve.evaluate(t)?;
                    let Some(d1) = eval.d1 else {
                        return Err(Error::new(
                            "BREP_ANALYSIS_INDETERMINATE",
                            "Undefined trim derivative in face-area quadrature",
                        ));
                    };
                    let dv = d1[1];
                    if dv == 0. {
                        continue;
                    }
                    let (u, v) = (eval.point[0], eval.point[1]);
                    let mut inner = 0.;
                    for [low, high] in knot_spans(&surface.knots_u, surface.degree_u, u) {
                        for piece in 0..divisions {
                            let l = low + (high - low) * piece as f64 / divisions as f64;
                            let h = low + (high - low) * (piece + 1) as f64 / divisions as f64;
                            for (ix, iw) in GAUSS {
                                let flux =
                                    area_flux(surface, (l + h) / 2. + ix * (h - l) / 2., v, guard)?;
                                inner += iw * (h - l) / 2. * flux;
                            }
                        }
                    }
                    total += w * (b - a) / 2. * dv * inner;
                }
            }
        }
    }
    Ok(total)
}

/// Trimmed surface area of one face (outer wire minus holes), converging to
/// the `mass_properties` integral estimate. Adaptive subdivision refines until
/// the relative change between passes drops below 1e-9 (or 6 refinements),
/// bounded by `budget`.
pub fn face_area(model: &Model, face_index: usize, budget: &Budget) -> Result<f64> {
    if face_index >= model.faces.len() {
        return Err(classify_error("face index out of range"));
    }
    let mut guard: BudgetGuard = budget.guard("face-area");
    guard.check()?;
    let mut previous = area_pass(model, face_index, 1, &mut guard)?;
    for divisions in [2usize, 4, 8, 16, 32] {
        let current = area_pass(model, face_index, divisions, &mut guard)?;
        let delta = (current - previous).abs();
        if delta <= 1e-9 * current.abs().max(1e-300) {
            return Ok(current.abs());
        }
        previous = current;
    }
    Ok(previous.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(20_000_000, 8, 120_000).unwrap()
    }

    /// Exact cylinder side surface as a rational-quadratic B-spline patch.
    fn cylinder_patch(radius: f64, height: f64) -> Surface {
        // Quarter-circle in u (rational degree 2), linear in v.
        let r = radius;
        let w = (0.5f64).sqrt();
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![[r, 0., 0.].to_vec(), [r, 0., height].to_vec()],
                vec![[r, r, 0.].to_vec(), [r, r, height].to_vec()],
                vec![[0., r, 0.].to_vec(), [0., r, height].to_vec()],
            ],
            weights: vec![vec![1., 1.], vec![w, w], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Same quarter cylinder, degree-elevated to cubic in u (exact rational
    /// circle) with the interior control points nudged by `noise`: the exact
    /// type is a perturbed B-spline, but the geometry stays within ~`noise`
    /// of a true cylinder — the best-fit path must still say Cylinder.
    fn noisy_cylinder_patch(radius: f64, height: f64, noise: f64) -> Surface {
        // Degree-elevate the rational quadratic quarter arc 2→3 in
        // homogeneous coordinates: Q1 = P0/3 + 2P1/3, Q2 = 2P1/3 + P2/3.
        let w = (0.5f64).sqrt();
        let h0 = ([radius, 0., 0.], 1.);
        let h1 = ([radius * w, radius * w, 0.], w);
        let h2 = ([0., radius, 0.], 1.);
        let combine = |a: ([f64; 3], f64), b: ([f64; 3], f64), ta: f64| {
            let (p, q) = (
                [
                    ta * a.0[0] + (1. - ta) * b.0[0],
                    ta * a.0[1] + (1. - ta) * b.0[1],
                    0.,
                ],
                ta * a.1 + (1. - ta) * b.1,
            );
            ([p[0] / q, p[1] / q], q)
        };
        let (q1, w1) = combine(h0, h1, 1. / 3.); // 1/3·P0 + 2/3·P1
        let (q2, w2) = combine(h2, h1, 1. / 3.); // 1/3·P2 + 2/3·P1
        let bump = |p: [f64; 2], dx: f64, dy: f64| [p[0] + dx, p[1] + dy];
        let q1 = bump(q1, noise, noise * 0.5);
        let q2 = bump(q2, -noise, noise * 0.5);
        let row = |p: [f64; 2]| vec![[p[0], p[1], 0.].to_vec(), [p[0], p[1], height].to_vec()];
        Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                row([radius, 0.]),
                row(q1),
                row(q2),
                row([0., radius]),
            ],
            weights: vec![vec![1., 1.], vec![w1, w1], vec![w2, w2], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn exact_cylinder_patch_classifies_with_axis_and_radius() {
        let surface = cylinder_patch(2., 5.);
        let result = classify_surface(&surface, 1e-6, &budget()).unwrap();
        assert_eq!(result.class, SurfaceClass::Cylinder);
        let axis = result.axis.unwrap();
        assert!((axis.direction[2].abs() - 1.).abs() < 1e-6, "axis along z: {axis:?}");
        assert!((result.radius.unwrap() - 2.).abs() < 1e-6);
        assert!(result.max_deviation < 1e-9, "exact patch fits exactly");
    }

    #[test]
    fn noisy_cylinder_patch_classifies_via_best_fit() {
        // Noise ~1e-4 mm, tolerance 1e-2 mm: well inside the acceptance band.
        let surface = noisy_cylinder_patch(2., 5., 1e-4);
        let result = classify_surface(&surface, 1e-2, &budget()).unwrap();
        assert_eq!(
            result.class,
            SurfaceClass::Cylinder,
            "almost-cylinder must best-fit to Cylinder: {result:?}"
        );
        assert!((result.radius.unwrap() - 2.).abs() < 1e-2);
        // But with a tolerance below the noise level it must refuse.
        let strict = classify_surface(&surface, 1e-6, &budget()).unwrap();
        assert_eq!(strict.class, SurfaceClass::Freeform);
    }

    #[test]
    fn planar_patch_classifies_as_plane() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![[0., 0., 1.].to_vec(), [0., 2., 1.].to_vec()],
                vec![[3., 0., 1.].to_vec(), [3., 2., 1.].to_vec()],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let result = classify_surface(&surface, 1e-9, &budget()).unwrap();
        assert_eq!(result.class, SurfaceClass::Plane);
    }

    #[test]
    fn wavey_patch_is_freeform() {
        // Mild sinusoidal height field: no analytic primitive within 1e-6.
        let mut control = vec![];
        for i in 0..5 {
            let mut row = vec![];
            for j in 0..5 {
                let (x, y) = (i as f64, j as f64);
                row.push([x, y, 0.05 * (x * 1.7).sin() * (y * 1.3).cos()].to_vec());
            }
            control.push(row);
        }
        let knots = |n: usize| {
            // degree 3: knot count = n + 4; interior knots evenly spaced.
            let interior = n - 4 + 1;
            let mut k = vec![0.; 4];
            k.extend((1..interior).map(|i| i as f64 / interior as f64));
            k.extend([1.; 4]);
            k
        };
        let surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: knots(5),
            knots_v: knots(5),
            control_points: control,
            weights: vec![vec![1.; 5]; 5],
            periodic_u: false,
            periodic_v: false,
        };
        let result = classify_surface(&surface, 1e-6, &budget()).unwrap();
        assert_eq!(result.class, SurfaceClass::Freeform);
    }

    #[test]
    fn rejects_nonpositive_tolerance_and_tight_budget() {
        let surface = cylinder_patch(1., 1.);
        assert!(classify_surface(&surface, 0., &budget()).is_err());
        assert!(classify_surface(&surface, -1., &budget()).is_err());
        let tight = Budget::with_iterations(3).unwrap();
        let error = classify_surface(&surface, 1e-6, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must be a typed resource error: {error:?}"
        );
    }

    #[test]
    fn cuboid_face_areas_match_mass_properties() {
        let model = crate::cuboid([0.; 3], [2., 3., 5.]).unwrap();
        let mut sum = 0.;
        for face in 0..model.faces.len() {
            sum += face_area(&model, face, &budget()).unwrap();
        }
        let mass = crate::analysis::mass_properties(&model, 1e-9, 1_000_000).unwrap();
        let expected = 2. * (2. * 3. + 3. * 5. + 2. * 5.);
        assert!((sum - expected).abs() / expected < 1e-9, "sum={sum}");
        assert!(
            (sum - mass.surface_area_mm2).abs() / mass.surface_area_mm2 < 1e-6,
            "face areas must converge to the mass_properties integral: {sum} vs {}",
            mass.surface_area_mm2
        );
    }

    #[test]
    fn cylinder_face_areas_match_mass_properties() {
        let model = crate::analytic::cylinder(1.5, 4.).unwrap();
        let mut sum = 0.;
        for face in 0..model.faces.len() {
            sum += face_area(&model, face, &budget()).unwrap();
        }
        let mass = crate::analysis::mass_properties(&model, 1e-9, 1_900_000).unwrap();
        assert!(
            (sum - mass.surface_area_mm2).abs() / mass.surface_area_mm2 < 1e-6,
            "cylinder: {sum} vs {}",
            mass.surface_area_mm2
        );
    }

    #[test]
    fn face_area_rejects_out_of_range_and_tight_budget() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        assert!(face_area(&model, model.faces.len(), &budget()).is_err());
        let tight = Budget::with_iterations(5).unwrap();
        let error = face_area(&model, 0, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "{error:?}"
        );
    }
}

