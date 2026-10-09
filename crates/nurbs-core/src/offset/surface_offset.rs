//! Signed surface offset along the unit normal with an honest deviation report.
//!
//! A true NURBS surface offset is not a NURBS surface in general. This module
//! therefore offsets the control net along normals sampled at the Greville
//! abscissae, detects planar/spherical/circular-cylindrical surfaces whose
//! offsets are exact affine images of the control net, and certifies the
//! residual deviation on a dense sample grid. Degenerate regions (undefined
//! normals, focal crossings) are recorded as UV rectangles, never fatal.
//!
//! Sign convention: positive `distance` moves along the surface orientation
//! `S_u x S_v`; negative `distance` moves to the opposite side.
use crate::{
    Result, check, numeric,
    foundation::fitting::fit_surface_cloud_certified_report,
    foundation::guards::{Budget, require_finite_f64},
    surface::{Axis, Surface},
};
use math_core::{dot, norm, next_down, next_up};

mod deviation;
mod validity;
pub use validity::{OffsetValidityReport, offset_validity};
use deviation::sampled_deviation;
use validity::{principal, validity_cell};
/// Result of a signed surface offset.
pub struct OffsetReport {
    /// Offset approximation (exact for plane/sphere/circular cylinder).
    pub surface: Surface,
    /// Certified upper bound of the sampled deviation against the true offset:
    /// combines normal error `|<S_off - S, n> - distance|` and lateral drift,
    /// maximized over a dense grid, outward rounded.
    pub max_deviation: f64,
    /// UV rectangles `[u0, u1, v0, v1]` where the normal degenerated or the
    /// offset crossed a focal surface and would need self-trimming.
    pub degenerate_regions: Vec<[f64; 4]>,
    /// True when `|distance|` exceeds the certified fold-free bound reported
    /// by [`offset_validity`]: the offset may self-intersect near
    /// `OffsetValidityReport::limiting_uv` (also recorded as a degenerate
    /// region). Informational only; the offset is still produced.
    pub exceeds_fold_free_bound: bool,
    /// True when the offset is exact by construction (plane, sphere,
    /// circular cylinder, or zero distance).
    pub exact: bool,
}

struct Jet {
    point: [f64; 3],
    normal: Option<[f64; 3]>,
    gaussian: Option<f64>,
    mean: Option<f64>,
}

fn jet(surface: &Surface, u: f64, v: f64) -> Result<Jet> {
    let e = surface.evaluate_validated(u, v)?;
    // Normals whose cross product is dominated by roundoff (poles) are
    // treated as undefined everywhere in this module.
    let reliable = e.unit_normal().is_some()
        && e.first_derivatives().is_some_and(|(du, dv)| {
            let su = norm(du);
            let sv = norm(dv);
            su > 0.
                && sv > 0.
                && su.min(sv) > 1e-9 * su.max(sv)
                && norm([
                    du[1] * dv[2] - du[2] * dv[1],
                    du[2] * dv[0] - du[0] * dv[2],
                    du[0] * dv[1] - du[1] * dv[0],
                ]) > 1e-6 * su * sv
        });
    Ok(Jet {
        point: e.point,
        normal: e.unit_normal().filter(|_| reliable),
        gaussian: e.curvatures().map(|(k, _)| k).filter(|_| reliable),
        mean: e.curvatures().map(|(_, h)| h).filter(|_| reliable),
    })
}

fn domain(surface: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    )
}

/// Greville abscissae: control point i owns the mean of its p support knots.
fn greville(degree: usize, knots: &[f64], count: usize) -> Vec<f64> {
    (0..count)
        .map(|i| knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64)
        .collect()
}

/// Support of basis function i, clamped to the active domain.
fn support(degree: usize, knots: &[f64], count: usize, i: usize) -> [f64; 2] {
    [
        knots[i].max(knots[degree]),
        knots[i + degree + 1].min(knots[count]),
    ]
}
enum Exact {
    Plane([f64; 3]),
    Sphere { center: [f64; 3], radius: f64 },
    Cylinder {
        axis_point: [f64; 3],
        axis: [f64; 3],
        radius: f64,
        /// True when the unit normal points away from the axis.
        outward: bool,
    },
}

/// Solve a symmetric 3x3 system; None when singular.
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

/// Detect plane/sphere/circular-cylinder structure from an interior jet grid.
/// Conservative by design: any doubt falls through to the general path.
fn detect_exact(surface: &Surface) -> Result<Option<Exact>> {
    let ([u0, u1], [v0, v1]) = domain(surface);
    let (nu, nv) = (5, 5);
    let mut jets = Vec::new();
    for i in 0..=nu {
        for j in 0..=nv {
            let u = u0 + (u1 - u0) * i as f64 / nu as f64;
            let v = v0 + (v1 - v0) * j as f64 / nv as f64;
            let j = jet(surface, u, v)?;
            if let Some(n) = j.normal {
                jets.push((j.point, n, j.gaussian, j.mean));
            }
        }
    }
    if jets.is_empty() {
        return Ok(None);
    }
    // Plane: all unit normals agree.
    let first = jets[0].1;
    if jets
        .iter()
        .all(|(_, n, _, _)| (0..3).all(|k| (n[k] - first[k]).abs() < 1e-9))
    {
        return Ok(Some(Exact::Plane(first)));
    }
    let curvature: Vec<(f64, f64)> = jets
        .iter()
        .filter_map(|(_, _, k, h)| k.zip(*h))
        .filter(|(k, h)| k.is_finite() && h.is_finite())
        .collect();
    if curvature.len() < jets.len() / 2 {
        return Ok(None);
    }
    // Near-degenerate normals (poles) can produce finite but absurd curvature
    // numbers. Use medians and require at least 80% inliers instead of
    // trusting every sample.
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values[values.len() / 2]
    };
    let k_med = median(curvature.iter().map(|c| c.0).collect());
    let h_med = median(curvature.iter().map(|c| c.1).collect());
    let scale = k_med.abs().max(h_med * h_med).max(1e-300);
    let inliers: Vec<(f64, f64)> = curvature
        .iter()
        .copied()
        .filter(|&(k, h)| {
            (k - k_med).abs() <= 1e-5 * scale && (h - h_med).abs() <= 1e-5 * scale.sqrt()
        })
        .collect();
    if inliers.len() * 5 < curvature.len() * 4 {
        return Ok(None);
    }
    let k0 = inliers.iter().map(|c| c.0).sum::<f64>() / inliers.len() as f64;
    let h0 = inliers.iter().map(|c| c.1).sum::<f64>() / inliers.len() as f64;
    // Points/normals of curvature inliers only; near-pole normals are corrupt.
    let good: Vec<([f64; 3], [f64; 3])> = jets
        .iter()
        .filter(|(_, _, k, h)| {
            match (k, h) {
                (Some(k), Some(h)) => {
                    (k - k_med).abs() <= 1e-5 * scale
                        && (h - h_med).abs() <= 1e-5 * scale.sqrt()
                }
                _ => false,
            }
        })
        .map(|&(p, n, _, _)| (p, n))
        .collect();
    if good.is_empty() {
        return Ok(None);
    }
    // Sphere: K > 0 constant and H^2 == K (umbilic everywhere).
    if k0 > 0. && (h0 * h0 - k0).abs() <= 1e-6 * scale {
        // Least-squares intersection of the normal lines p + t n.
        let mut matrix = [[0.; 3]; 3];
        let mut rhs = [0.; 3];
        for &(p, n) in &good {
            for a in 0..3 {
                for b in 0..3 {
                    matrix[a][b] += if a == b { 1. } else { 0. } - n[a] * n[b];
                }
                rhs[a] += p[a] - n[a] * dot(n, p);
            }
        }
        if let Some(center) = solve3(matrix, rhs) {
            let radii: Vec<f64> = good
                .iter()
                .map(|&(p, _)| norm(std::array::from_fn(|k| p[k] - center[k])))
                .collect();
            let radius = radii.iter().sum::<f64>() / radii.len() as f64;
            if radius > 0.
                && radii
                    .iter()
                    .all(|r| (r - radius).abs() <= 1e-6 * radius.max(1.))
            {
                return Ok(Some(Exact::Sphere { center, radius }));
            }
        }
        return Ok(None);
    }
    // Circular cylinder: K == 0 and H constant nonzero.
    if k0.abs() <= 1e-9 * scale.sqrt().max(1e-300) && h0.abs() > 0. {
        let radius = 1. / (2. * h0.abs());
        // Axis direction: least-aligned unit vector with all normals, i.e. the
        // smallest eigenvector of M = sum(n n^T). Find the two dominant
        // eigenvectors by orthogonalized power iteration; the axis is their
        // cross product (robust under a degenerate dominant eigenspace).
        let mut m = [[0.; 3]; 3];
        for &(_, n) in &good {
            for a in 0..3 {
                for b in 0..3 {
                    m[a][b] += n[a] * n[b];
                }
            }
        }
        let mut detect_budget = Budget::with_iterations(3 * 128 + 1)?.guard("offset_exact_detect");
        let mut dominant = |start: [f64; 3], against: Option<[f64; 3]>| -> Option<[f64; 3]> {
            let mut v = start;
            for _ in 0..128 {
                if detect_budget.tick().is_err() {
                    return None;
                }
                let mut next = std::array::from_fn::<_, 3, _>(|a| {
                    m[a][0] * v[0] + m[a][1] * v[1] + m[a][2] * v[2]
                });
                if let Some(o) = against {
                    let proj = dot(next, o);
                    for k in 0..3 {
                        next[k] -= proj * o[k];
                    }
                }
                let length = norm(next);
                if length <= 1e-14 {
                    return None;
                }
                v = next.map(|x| x / length);
            }
            Some(v)
        };
        let starts = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let mut first = None;
        for s in starts {
            if let Some(v) = dominant(s, None) {
                first = Some(v);
                break;
            }
        }
        let Some(first) = first else { return Ok(None) };
        let mut second = None;
        for s in starts {
            if let Some(v) = dominant(s, Some(first)) {
                if norm(std::array::from_fn(|k| {
                    let c = [
                        first[1] * v[2] - first[2] * v[1],
                        first[2] * v[0] - first[0] * v[2],
                        first[0] * v[1] - first[1] * v[0],
                    ];
                    let _ = k;
                    c[k]
                })) > 1e-9
                {
                    second = Some(v);
                    break;
                }
            }
        }
        let Some(second) = second else { return Ok(None) };
        let axis = [
            first[1] * second[2] - first[2] * second[1],
            first[2] * second[0] - first[0] * second[2],
            first[0] * second[1] - first[1] * second[0],
        ];
        let axis = axis.map(|x| x / norm(axis));
        // Axis point candidates from both possible normal orientations.
        let mut best: Option<(f64, [f64; 3], bool)> = None;
        for outward in [true, false] {
            let sign = if outward { 1. } else { -1. };
            let centers: Vec<[f64; 3]> = good
                .iter()
                .map(|&(p, n)| std::array::from_fn(|k| p[k] - sign * radius * n[k]))
                .collect();
            let mut mean = [0.; 3];
            for c in &centers {
                for k in 0..3 {
                    mean[k] += c[k] / centers.len() as f64;
                }
            }
            // Residual spread perpendicular to the axis.
            let spread = centers
                .iter()
                .map(|c| {
                    let d: [f64; 3] = std::array::from_fn(|k| c[k] - mean[k]);
                    let axial = dot(d, axis);
                    norm(std::array::from_fn(|k| d[k] - axial * axis[k]))
                })
                .fold(0., f64::max);
            if best.as_ref().is_none_or(|(s, _, _)| spread < *s) {
                best = Some((spread, mean, outward));
            }
        }
        if let Some((spread, axis_point, outward)) = best {
            // Also require radial distances from the axis to be uniform.
            let radial_uniform = good.iter().all(|&(p, _)| {
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - axis_point[k]);
                let axial = dot(d, axis);
                let r = norm(std::array::from_fn(|k| d[k] - axial * axis[k]));
                (r - radius).abs() <= 1e-6 * radius.max(1.)
            });
            if spread <= 1e-6 * radius.max(1.) && radial_uniform {
                return Ok(Some(Exact::Cylinder {
                    axis_point,
                    axis,
                    radius,
                    outward,
                }));
            }
        }
    }
    Ok(None)
}

/// Control-net offset along Greville normals, with degeneracy bookkeeping.
fn offset_net(
    surface: &Surface,
    distance: f64,
    degenerate: &mut Vec<[f64; 4]>,
) -> Result<Surface> {
    let us = greville(surface.degree_u, &surface.knots_u, surface.control_points.len());
    let vs = greville(
        surface.degree_v,
        &surface.knots_v,
        surface.control_points[0].len(),
    );
    let rows = surface.control_points.len();
    let cols = surface.control_points[0].len();
    let mut normals = vec![vec![None; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            let j_ = jet(surface, us[i], vs[j])?;
            normals[i][j] = j_.normal;
            // Focal crossing: with this crate's convention `L = <n, S_uu>`
            // (outward sphere has k = -1/R), the offset S + d*n folds when
            // `d * k >= 1` — the same convention `offset_validity` certifies.
            if let (Some(k), Some(h)) = (j_.gaussian, j_.mean) {
                if let Some((k1, k2)) = principal(k, h) {
                    if distance * k1 >= 1. - 1e-9 || distance * k2 >= 1. - 1e-9 {
                        let [a, b] = support(surface.degree_u, &surface.knots_u, rows, i);
                        let [c, d] = support(surface.degree_v, &surface.knots_v, cols, j);
                        degenerate.push([a, b, c, d]);
                    }
                }
            }
            if normals[i][j].is_none() {
                let [a, b] = support(surface.degree_u, &surface.knots_u, rows, i);
                let [c, d] = support(surface.degree_v, &surface.knots_v, cols, j);
                degenerate.push([a, b, c, d]);
            }
        }
    }
    // Fill degenerate normals from valid neighbours; never panic on a pole.
    let fallback = normals
        .iter()
        .flatten()
        .flatten()
        .next()
        .copied();
    let mut control_points = surface.control_points.clone();
    for i in 0..rows {
        for j in 0..cols {
            let n = normals[i][j].or_else(|| {
                let mut sum = [0.; 3];
                let mut count = 0.;
                for (di, dj) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (r, c) = (i as i64 + di, j as i64 + dj);
                    if r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols {
                        if let Some(n) = normals[r as usize][c as usize] {
                            for k in 0..3 {
                                sum[k] += n[k];
                            }
                            count += 1.;
                        }
                    }
                }
                (count > 0. && norm(sum) > 1e-12).then(|| sum.map(|x| x / norm(sum)))
            });
            let Some(n) = n.or(fallback) else {
                // Entire net degenerate: leave the control point unmoved.
                continue;
            };
            for k in 0..3 {
                control_points[i][j][k] += distance * n[k];
            }
        }
    }
    let result = Surface {
        control_points,
        ..surface.clone()
    };
    // Result boundary: a non-finite offset net must surface as NonFinite,
    // not propagate into downstream evaluation.
    for row in &result.control_points {
        for p in row {
            crate::foundation::guards::require_finite_slice(p, "offset_control_points")?;
        }
    }
    result.validate()?;
    Ok(result)
}

/// One knot-insertion pass at every active span midpoint of both axes.
fn refine(surface: &Surface) -> Result<Surface> {
    let midpoint_knots = |degree: usize, knots: &[f64], count: usize| -> Vec<f64> {
        (degree..count)
            .filter(|&i| knots[i] < knots[i + 1])
            .map(|i| (knots[i] + knots[i + 1]) / 2.)
            .collect()
    };
    let mut result = surface.clone();
    for axis in [Axis::U, Axis::V] {
        let (degree, knots, count) = match axis {
            Axis::U => (
                result.degree_u,
                result.knots_u.clone(),
                result.control_points.len(),
            ),
            Axis::V => (
                result.degree_v,
                result.knots_v.clone(),
                result.control_points[0].len(),
            ),
        };
        let mids = midpoint_knots(degree, &knots, count);
        // Respect the 2..=32 control net budget.
        let growth = match axis {
            Axis::U => count + mids.len() <= 32,
            Axis::V => count + mids.len() <= 32,
        };
        if !growth || mids.is_empty() {
            continue;
        }
        result = result.edit_axis(axis, |c| {
            let mut refined = c.clone();
            for m in &mids {
                refined = refined.insert(*m, 1)?;
            }
            Ok(refined)
        })?;
    }
    Ok(result)
}

/// Offset a NURBS surface by signed `distance` along the unit normal
/// (`S_u x S_v` orientation; negative distances flip the side).
///
/// Plane, sphere and circular-cylinder offsets are exact affine control-net
/// images and report `exact = true`. Other surfaces use a Greville-normal
/// control-net offset, refined once by knot insertion when the dense sampled
/// deviation exceeds `tolerance`, with a certified cloud fit as a final
/// fallback; `max_deviation` always reports the honest final bound.
pub fn offset(surface: &Surface, distance: f64, tolerance: f64) -> Result<OffsetReport> {
    surface.validate()?;
    require_finite_f64(distance, "distance")?;
    require_finite_f64(tolerance, "tolerance")?;
    check(
        tolerance > 0.,
        "Surface offset tolerance must be positive",
    )?;
    let mut degenerate = Vec::new();
    if distance == 0. {
        return Ok(OffsetReport {
            surface: surface.clone(),
            max_deviation: 0.,
            degenerate_regions: degenerate,
            exceeds_fold_free_bound: false,
            exact: true,
        });
    }
    // Fold-free precondition: certify the requested magnitude against the
    // minimum opposing principal curvature radius and surface any excess as
    // both a flag and a degenerate region at the limiting location.
    let validity = offset_validity(surface, distance)?;
    let exceeds_fold_free_bound = !validity.requested_ok;
    if exceeds_fold_free_bound {
        degenerate.push(validity_cell(surface, validity.limiting_uv));
    }
    // Exact affine fast paths.
    match detect_exact(surface)? {
        Some(Exact::Plane(n)) => {
            let mut result = surface.clone();
            for row in &mut result.control_points {
                for p in row {
                    for k in 0..3 {
                        p[k] += distance * n[k];
                    }
                }
            }
            result.validate()?;
            let deviation = sampled_deviation(surface, &result, distance, &mut degenerate)?;
            return Ok(OffsetReport {
                surface: result,
                max_deviation: deviation,
                degenerate_regions: degenerate,
                exceeds_fold_free_bound,
                exact: true,
            });
        }
        Some(Exact::Sphere { center, radius }) => {
            // The offset follows the unit normal; its side relative to the
            // center decides whether the radius grows or shrinks.
            let ([u0, u1], [v0, v1]) = domain(surface);
            // Majority vote over smooth samples; seam normals may flip.
            let mut votes = 0i64;
            for i in 1..5 {
                for j in 0..5 {
                    let u = u0 + (u1 - u0) * i as f64 / 5.;
                    let v = v0 + (v1 - v0) * j as f64 / 5.;
                    let j_ = jet(surface, u, v)?;
                    if let Some(n) = j_.normal {
                        let radial: [f64; 3] =
                            std::array::from_fn(|k| j_.point[k] - center[k]);
                        let side = dot(radial, n);
                        if side.abs() > 1e-12 * radius {
                            votes += if side > 0. { 1 } else { -1 };
                        }
                    }
                }
            }
            let signed = if votes >= 0 { distance } else { -distance };
            if radius + signed <= 0. {
                // Inward offset through the center: the general path records
                // the focal crossing as a degenerate region.
                let mut candidate = offset_net(surface, distance, &mut degenerate)?;
                let deviation =
                    sampled_deviation(surface, &candidate, distance, &mut degenerate)?;
                candidate.validate()?;
                return Ok(OffsetReport {
                    surface: candidate,
                    max_deviation: deviation,
                    degenerate_regions: degenerate,
                    exceeds_fold_free_bound,
                    exact: false,
                });
            }
            let scale = (radius + signed) / radius;
            let mut result = surface.clone();
            for row in &mut result.control_points {
                for p in row {
                    for k in 0..3 {
                        p[k] = center[k] + scale * (p[k] - center[k]);
                    }
                }
            }
            result.validate()?;
            let deviation = sampled_deviation(surface, &result, distance, &mut degenerate)?;
            return Ok(OffsetReport {
                surface: result,
                max_deviation: deviation,
                degenerate_regions: degenerate,
                exceeds_fold_free_bound,
                exact: true,
            });
        }
        Some(Exact::Cylinder {
            axis_point,
            axis,
            radius,
            outward,
        }) => {
            let signed = if outward { distance } else { -distance };
            if radius + signed > 0. {
                let scale = (radius + signed) / radius;
                let mut result = surface.clone();
                for row in &mut result.control_points {
                    for p in row {
                        let w: [f64; 3] = std::array::from_fn(|k| p[k] - axis_point[k]);
                        let axial = dot(w, axis);
                        for k in 0..3 {
                            let radial = w[k] - axial * axis[k];
                            p[k] = axis_point[k] + axial * axis[k] + scale * radial;
                        }
                    }
                }
                result.validate()?;
                let deviation = sampled_deviation(surface, &result, distance, &mut degenerate)?;
                return Ok(OffsetReport {
                    surface: result,
                    max_deviation: deviation,
                    degenerate_regions: degenerate,
                    exceeds_fold_free_bound,
                    exact: true,
                });
            }
            // Inward offset through the axis: fall through to the general
            // path, which records the focal crossing as degenerate.
        }
        _ => {}
    }
    // General path: Greville-normal control-net offset.
    let mut candidate = offset_net(surface, distance, &mut degenerate)?;
    let mut deviation = sampled_deviation(surface, &candidate, distance, &mut degenerate)?;
    if deviation > tolerance {
        // One refinement pass: identical geometry, denser control net.
        let refined = refine(surface)?;
        if refined.control_points.len() != surface.control_points.len()
            || refined.control_points[0].len() != surface.control_points[0].len()
        {
            let next = offset_net(&refined, distance, &mut degenerate)?;
            let next_deviation =
                sampled_deviation(surface, &next, distance, &mut degenerate)?;
            if next_deviation < deviation {
                candidate = next;
                deviation = next_deviation;
            }
        }
    }
    if deviation > tolerance {
        // Final fallback: certified least-squares fit of densely sampled true
        // offset points; the certificate carries a Hausdorff error bound.
        let ([u0, u1], [v0, v1]) = domain(surface);
        let (nu, nv) = (24, 24);
        let mut targets = Vec::new();
        for i in 0..=nu {
            for j in 0..=nv {
                let u = u0 + (u1 - u0) * i as f64 / nu as f64;
                let v = v0 + (v1 - v0) * j as f64 / nv as f64;
                let j_ = jet(surface, u, v)?;
                if let Some(n) = j_.normal {
                    targets.push(std::array::from_fn(|k| j_.point[k] + distance * n[k]));
                }
            }
        }
        let side = (targets.len() as f64).sqrt() as usize;
        let controls = side.clamp(2, 8);
        if targets.len() >= 4 && controls * controls <= targets.len() {
            if let Ok(fit) =
                fit_surface_cloud_certified_report(targets, controls, controls, None)
            {
                let certified = fit.certificate.evidence.hausdorff_error_upper;
                if certified < deviation {
                    candidate = fit.surface;
                    deviation = certified;
                }
            }
        }
    }
    Ok(OffsetReport {
        surface: candidate,
        max_deviation: deviation,
        degenerate_regions: degenerate,
        exceeds_fold_free_bound,
        exact: false,
    })
}

#[cfg(test)]
#[path = "tests/surface_offset.rs"]
mod tests;

mod source;
pub use source::{Bounds,JacobianBounds,Evaluation,ContactBand,PairDomain,Candidates,bounds,jacobian_bounds,evaluate,certify_contact_section,certify_contact_band,intersection_candidates};
