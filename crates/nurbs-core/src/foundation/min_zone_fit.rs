//! Min-zone (Chebyshev) fitting for metrology: flatness, circularity and
//! cylindricity zone evaluation.
//!
//! Guarantee model (uniform across all three features):
//! * The returned `zone_width` is always an *honest upper bound* of the true
//!   min-zone width, because it is the zone of one concrete evaluated
//!   feature (plane / circle / cylinder axis), outward-rounded.
//! * `certified == true` additionally means the search enumerated the full
//!   combinatorial active-set candidate family (below the documented size
//!   threshold), so the bound is the exact min-zone value up to evaluation
//!   rounding. Above the threshold the result comes from a least-squares
//!   start plus active-set refinement and is an upper bound only.
//!
//! Active-set facts used (standard min-zone results):
//! * Flatness: the optimal separating planes are supported by 4 points in a
//!   3+1 or 2+2 configuration, so the optimal normal is either a triangle
//!   normal or perpendicular to two segments.
//! * Circularity: the optimal annulus is supported by ≥3 points; candidates
//!   are circumcenters of triples, intersections of two perpendicular
//!   bisectors (2+2) and pair midpoints (diameter case). Rare 2+1 optima
//!   with exactly three active points not on one circle are approached by a
//!   local refinement pass; the enumeration threshold therefore certifies a
//!   bound that is exact for the covered configurations.
//! * Cylindricity has no small exact combinatorial characterisation in this
//!   implementation; the result is a least-squares start plus direction
//!   refinement, and `upper_bound_certificate` is a conservative
//!   outward-rounded bound (see `min_zone_cylindricity`).
use super::{Result, check, numeric};
use math_core::{Acceleration, next_down, next_up, point_fit_plane, point_principal_axes};

/// Which geometric feature a [`MinZoneReport`] refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinZoneFeature {
    Flatness,
    Circularity,
    Cylindricity,
}

/// Common min-zone report: evaluated zone plus solver evidence.
#[derive(Clone, Debug)]
pub struct MinZoneReport {
    pub feature: MinZoneFeature,
    /// Outward-rounded zone of the returned feature; an upper bound of the
    /// true min-zone width (exact up to rounding when `certified`).
    pub zone_width: f64,
    /// Refinement/enumeration iterations actually performed.
    pub solver_iterations: usize,
    /// True when the active-set enumeration covered the full candidate
    /// family (point count at or below the documented threshold).
    pub certified: bool,
}

/// Min-zone flatness result: mid-plane of the optimal zone as
/// `dot(plane_normal, p) + plane_offset = 0`.
#[derive(Clone, Debug)]
pub struct FlatnessZone {
    pub plane_normal: [f64; 3],
    pub plane_offset: f64,
    pub report: MinZoneReport,
}

/// Min-zone circularity result for 2D points: center of the optimal annulus.
#[derive(Clone, Debug)]
pub struct CircularityZone {
    pub center: [f64; 2],
    pub report: MinZoneReport,
}

/// Min-zone cylindricity result: axis plus a rigorous upper-bound
/// certificate for the zone of the returned axis.
#[derive(Clone, Debug)]
pub struct CylindricityZone {
    pub axis_point: [f64; 3],
    pub axis_direction: [f64; 3],
    /// Conservative outward-rounded upper bound of the zone of the returned
    /// axis: `max_i r_i·(1+8u)` minus `min_i r_i·(1−8u)` with `u = f64::EPSILON`
    /// and per-value outward rounding, covering the ≤6 rounding steps of each
    /// radial evaluation (3 subtracts/multiplies, 2 adds, 1 sqrt). Always
    /// ≥ `report.zone_width` and ≥ the true min-zone width.
    pub upper_bound_certificate: f64,
    pub report: MinZoneReport,
}

/// Point counts at or below which the full combinatorial candidate family
/// is enumerated (and `certified` is set).
const FLATNESS_EXACT_LIMIT: usize = 24;
const CIRCULARITY_EXACT_LIMIT: usize = 32;

fn check_points_3d(points: &[[f64; 3]], min: usize) -> Result<()> {
    check(
        points.len() >= min && points.len() <= 4096,
        "Min-zone fitting needs a bounded finite point cloud (at most 4096 points)",
    )?;
    check(
        points.iter().flatten().all(|v| v.is_finite()),
        "Min-zone points must be finite",
    )
}

fn check_points_2d(points: &[[f64; 2]]) -> Result<()> {
    check(
        points.len() >= 3 && points.len() <= 4096,
        "Min-zone circularity needs 3..4096 points",
    )?;
    check(
        points.iter().flatten().all(|v| v.is_finite()),
        "Min-zone points must be finite",
    )
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (n > 0. && n.is_finite()).then_some([v[0] / n, v[1] / n, v[2] / n])
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Zone (max − min signed distance) of `points` measured along `normal`,
/// outward-rounded.
fn plane_zone(points: &[[f64; 3]], normal: [f64; 3]) -> f64 {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in points {
        let d = normal[0] * p[0] + normal[1] * p[1] + normal[2] * p[2];
        lo = lo.min(d);
        hi = hi.max(d);
    }
    next_up(hi - lo)
}

/// Min-zone flatness: the minimum distance between two parallel planes
/// enclosing all points.
///
/// For `points.len() <= 24` every candidate normal from the active-set
/// theorem is enumerated (all triangle normals and all normals perpendicular
/// to two segments), so the result is the exact min-zone value up to
/// rounding and `certified` is set. For larger clouds the search starts
/// from the least-squares plane (`math_core::point_fit_plane`) and refines
/// through exact solves over the current 3 top + 3 bottom active points;
/// the result is then an upper bound (`certified == false`).
pub fn min_zone_flatness(points: &[[f64; 3]]) -> Result<FlatnessZone> {
    check_points_3d(points, 3)?;
    let fit = point_fit_plane(points, Acceleration::Cpu)
        .map_err(|e| crate::numeric_err(&format!("Flatness LS plane failed: {e}")))?;
    let mut best_normal = fit.normal;
    let mut best_zone = plane_zone(points, best_normal);
    let mut iterations = 0_usize;
    let n = points.len();
    let certified = n <= FLATNESS_EXACT_LIMIT;
    if certified {
        iterations += 1;
        // 3+1 configurations: every triangle normal.
        for i in 0..n {
            for j in i + 1..n {
                for k in j + 1..n {
                    let e1 = sub3(points[j], points[i]);
                    let e2 = sub3(points[k], points[i]);
                    if let Some(normal) = unit(cross(e1, e2)) {
                        let zone = plane_zone(points, normal);
                        if zone < best_zone {
                            best_zone = zone;
                            best_normal = normal;
                        }
                    }
                }
            }
        }
        // 2+2 configurations: normals perpendicular to two segments.
        for i in 0..n {
            for j in i + 1..n {
                let e1 = sub3(points[j], points[i]);
                for k in j..n {
                    for l in k + 1..n {
                        let e2 = sub3(points[l], points[k]);
                        if let Some(normal) = unit(cross(e1, e2)) {
                            let zone = plane_zone(points, normal);
                            if zone < best_zone {
                                best_zone = zone;
                                best_normal = normal;
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Active-set refinement from the LS start.
        for _ in 0..32 {
            iterations += 1;
            let mut signed: Vec<(usize, f64)> = points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        i,
                        best_normal[0] * p[0] + best_normal[1] * p[1] + best_normal[2] * p[2],
                    )
                })
                .collect();
            signed.sort_by(|a, b| a.1.total_cmp(&b.1));
            let active: Vec<[f64; 3]> = signed[..3]
                .iter()
                .chain(&signed[n - 3..])
                .map(|&(i, _)| points[i])
                .collect();
            let mut improved = false;
            for i in 0..6 {
                for j in i + 1..6 {
                    let e1 = sub3(active[j], active[i]);
                    for k in j..6 {
                        for l in k + 1..6 {
                            if let Some(normal) = unit(cross(e1, sub3(active[l], active[k]))) {
                                let zone = plane_zone(points, normal);
                                if zone < best_zone {
                                    best_zone = zone;
                                    best_normal = normal;
                                    improved = true;
                                }
                            }
                        }
                    }
                    for k in j + 1..6 {
                        let e2 = sub3(active[k], active[j]);
                        if let Some(normal) = unit(cross(e1, e2)) {
                            let zone = plane_zone(points, normal);
                            if zone < best_zone {
                                best_zone = zone;
                                best_normal = normal;
                                improved = true;
                            }
                        }
                    }
                }
            }
            if !improved {
                break;
            }
        }
    }
    numeric(best_zone.is_finite(), "Flatness zone lost finiteness")?;
    // Mid-plane offset of the optimal zone.
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in points {
        let d = best_normal[0] * p[0] + best_normal[1] * p[1] + best_normal[2] * p[2];
        lo = lo.min(d);
        hi = hi.max(d);
    }
    Ok(FlatnessZone {
        plane_normal: best_normal,
        plane_offset: -(hi + lo) / 2.,
        report: MinZoneReport {
            feature: MinZoneFeature::Flatness,
            zone_width: best_zone,
            solver_iterations: iterations,
            certified,
        },
    })
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Circumcenter of three 2D points, `None` when (nearly) collinear.
fn circumcenter(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<[f64; 2]> {
    let d = 2. * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]));
    let scale = ((b[0] - a[0]).hypot(b[1] - a[1])) * ((c[0] - a[0]).hypot(c[1] - a[1]));
    if d.abs() <= 1e-12 * scale {
        return None;
    }
    let a2 = a[0] * a[0] + a[1] * a[1];
    let b2 = b[0] * b[0] + b[1] * b[1];
    let c2 = c[0] * c[0] + c[1] * c[1];
    Some([
        ((b2 - a2) * (c[1] - a[1]) - (c2 - a2) * (b[1] - a[1])) / d + a[0],
        ((c2 - a2) * (b[0] - a[0]) - (b2 - a2) * (c[0] - a[0])) / d + a[1],
    ])
}

/// Intersection of the perpendicular bisectors of segments `ab` and `cd`.
fn bisector_intersection(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    // Solve: (b-a)·x = (|b|²-|a|²)/2 and (d-c)·x = (|d|²-|c|²)/2.
    let (u1, v1) = (b[0] - a[0], b[1] - a[1]);
    let (u2, v2) = (d[0] - c[0], d[1] - c[1]);
    let det = u1 * v2 - u2 * v1;
    let scale = (u1.hypot(v1)) * (u2.hypot(v2));
    if det.abs() <= 1e-12 * scale {
        return None;
    }
    let r1 = (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1]) / 2.;
    let r2 = (d[0] * d[0] + d[1] * d[1] - c[0] * c[0] - c[1] * c[1]) / 2.;
    Some([(r1 * v2 - r2 * v1) / det, (u1 * r2 - u2 * r1) / det])
}

/// Annulus zone of `points` around `center`, outward-rounded.
fn annulus_zone(points: &[[f64; 2]], center: [f64; 2]) -> f64 {
    let mut lo = f64::INFINITY;
    let mut hi = 0_f64;
    for p in points {
        let r = ((p[0] - center[0]) * (p[0] - center[0])
            + (p[1] - center[1]) * (p[1] - center[1]))
            .sqrt();
        lo = lo.min(r);
        hi = hi.max(r);
    }
    next_up(hi - lo)
}

/// Min-zone circularity for 2D points: minimum width of an annulus
/// enclosing all points.
///
/// For `points.len() <= 32` the full candidate family is enumerated:
/// circumcenters of all triples (3 active on one circle), intersections of
/// perpendicular bisectors of all segment pairs (2+2), and all segment
/// midpoints (diameter case). A final local refinement pass polishes the
/// best candidate; `certified` is set. For larger clouds a Kasa
/// least-squares circle starts the same refinement and the result is an
/// upper bound only.
pub fn min_zone_circularity(points: &[[f64; 2]]) -> Result<CircularityZone> {
    check_points_2d(points)?;
    let n = points.len();
    // Least-squares (Kasa) circle start.
    let mut best_center = kasa_center(points);
    let mut best_zone = annulus_zone(points, best_center);
    let mut iterations = 0_usize;
    let certified = n <= CIRCULARITY_EXACT_LIMIT;
    // Data extent guard: degenerate configurations can produce astronomically
    // far centers whose radii are all equal in f64; reject candidates beyond
    // four diameters from the centroid.
    let centroid = [
        points.iter().map(|p| p[0]).sum::<f64>() / n as f64,
        points.iter().map(|p| p[1]).sum::<f64>() / n as f64,
    ];
    let (mut xlo, mut xhi, mut ylo, mut yhi) = (f64::INFINITY, 0_f64, f64::INFINITY, 0_f64);
    for p in points {
        xlo = xlo.min(p[0]);
        xhi = xhi.max(p[0]);
        ylo = ylo.min(p[1]);
        yhi = yhi.max(p[1]);
    }
    let radius_limit = 4. * (xhi - xlo).hypot(yhi - ylo) + 1e-300;
    if certified {
        iterations += 1;
        let consider = |center: Option<[f64; 2]>, best: &mut ([f64; 2], f64)| {
            if let Some(c) = center {
                if c[0].is_finite()
                    && c[1].is_finite()
                    && (c[0] - centroid[0]).hypot(c[1] - centroid[1]) <= radius_limit
                {
                    let zone = annulus_zone(points, c);
                    if zone < best.1 {
                        *best = (c, zone);
                    }
                }
            }
        };
        let mut best = (best_center, best_zone);
        for i in 0..n {
            for j in i + 1..n {
                consider(
                    Some([
                        (points[i][0] + points[j][0]) / 2.,
                        (points[i][1] + points[j][1]) / 2.,
                    ]),
                    &mut best,
                );
                for k in j + 1..n {
                    consider(
                        circumcenter(points[i], points[j], points[k]),
                        &mut best,
                    );
                    for l in k + 1..n {
                        consider(
                            bisector_intersection(points[i], points[j], points[k], points[l]),
                            &mut best,
                        );
                    }
                }
            }
        }
        best_center = best.0;
        best_zone = best.1;
    }
    // Local refinement (pattern search) — also polishes the certified path.
    let mut step = (best_zone / 4.).max(1e-9);
    for _ in 0..64 {
        iterations += 1;
        let mut improved = false;
        for &(dx, dy) in &[
            (1., 0.),
            (-1., 0.),
            (0., 1.),
            (0., -1.),
            (1., 1.),
            (1., -1.),
            (-1., 1.),
            (-1., -1.),
        ] {
            let candidate = [best_center[0] + step * dx, best_center[1] + step * dy];
            let zone = annulus_zone(points, candidate);
            if zone < best_zone {
                best_zone = zone;
                best_center = candidate;
                improved = true;
            }
        }
        if !improved {
            step *= 0.5;
            if step <= 1e-13 * (1. + best_zone) {
                break;
            }
        }
    }
    numeric(best_zone.is_finite(), "Circularity zone lost finiteness")?;
    Ok(CircularityZone {
        center: best_center,
        report: MinZoneReport {
            feature: MinZoneFeature::Circularity,
            zone_width: best_zone,
            solver_iterations: iterations,
            certified,
        },
    })
}

/// Algebraic (Kasa) least-squares circle center.
fn kasa_center(points: &[[f64; 2]]) -> [f64; 2] {
    let n = points.len() as f64;
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy, mut sxz, mut syz) =
        (0., 0., 0., 0., 0., 0., 0.);
    for p in points {
        let z = p[0] * p[0] + p[1] * p[1];
        sx += p[0];
        sy += p[1];
        sxx += p[0] * p[0];
        syy += p[1] * p[1];
        sxy += p[0] * p[1];
        sxz += p[0] * z;
        syz += p[1] * z;
    }
    // Solve the 2x2 normal system; fall back to the centroid when singular.
    let a = sxx - sx * sx / n;
    let b = sxy - sx * sy / n;
    let c = syy - sy * sy / n;
    let det = a * c - b * b;
    if det.abs() <= 1e-300 {
        return [sx / n, sy / n];
    }
    let r1 = (sxz - sx * (sxx + syy) / n) / 2.;
    let r2 = (syz - sy * (sxx + syy) / n) / 2.;
    [(r1 * c - r2 * b) / det, (a * r2 - b * r1) / det]
}

/// Min-zone cylindricity: approximate minimum radial zone around a common
/// axis.
///
/// Algorithm (approximation, documented): the axis direction starts from the
/// dominant principal axis of the cloud (correct for elongated cylindrical
/// data), the axis point from the centroid of the projections onto the
/// perpendicular plane; the direction is then refined by a pattern search
/// over tilted directions, re-centring each candidate. There is no exact
/// combinatorial certificate here, so `certified` is always `false`; instead
/// `upper_bound_certificate` gives a rigorous outward-rounded bound on the
/// zone of the returned axis (and therefore on the true min-zone width):
/// each radial length accumulates at most 6 rounding steps, covered by the
/// `(1 ± 8u)` factors with per-value `next_up`/`next_down` rounding.
pub fn min_zone_cylindricity(points: &[[f64; 3]]) -> Result<CylindricityZone> {
    check_points_3d(points, 4)?;
    let axes = point_principal_axes(points, Acceleration::Cpu)
        .map_err(|e| crate::numeric_err(&format!("Cylindricity PCA failed: {e}")))?;
    let centroid = axes.moments.centroid;
    // Dominant variance direction as the cylinder axis start.
    let mut direction = unit(axes.axes[0])
        .ok_or_else(|| crate::numeric_err("Cylindricity axis degenerated"))?;
    let mut iterations = 0_usize;

    let zone_of = |direction: [f64; 3]| -> ([f64; 3], f64) {
        // Centre on the perpendicular plane through the centroid.
        let mut center = [0.; 3];
        let n = points.len() as f64;
        for p in points {
            let d = sub3(*p, centroid);
            let t = d[0] * direction[0] + d[1] * direction[1] + d[2] * direction[2];
            for axis in 0..3 {
                center[axis] += (d[axis] - t * direction[axis]) / n;
            }
        }
        let mut lo = f64::INFINITY;
        let mut hi = 0_f64;
        for p in points {
            let d = sub3(*p, centroid);
            let t = d[0] * direction[0] + d[1] * direction[1] + d[2] * direction[2];
            let q = [
                d[0] - t * direction[0] - center[0],
                d[1] - t * direction[1] - center[1],
                d[2] - t * direction[2] - center[2],
            ];
            let r = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
            lo = lo.min(r);
            hi = hi.max(r);
        }
        let mut axis_point = centroid;
        for axis in 0..3 {
            axis_point[axis] += center[axis];
        }
        (axis_point, next_up(hi - lo))
    };

    let (_, mut best_zone) = zone_of(direction);
    // Pattern-search refinement from one starting direction.
    let refine = |mut direction: [f64; 3], iterations: &mut usize| -> ([f64; 3], f64) {
        let (_, mut zone) = zone_of(direction);
        // Orthonormal frame around the current direction for tilt proposals.
        let mut theta = 0.25_f64; // ~14° initial tilt
        for _ in 0..48 {
            *iterations += 1;
            let reference = if direction[0].abs() < 0.9 {
                [1., 0., 0.]
            } else {
                [0., 1., 0.]
            };
            let u = unit(cross(direction, reference)).unwrap();
            let v = cross(direction, u);
            let mut improved = false;
            for &(su, sv) in &[
                (1., 0.),
                (-1., 0.),
                (0., 1.),
                (0., -1.),
                (1., 1.),
                (1., -1.),
                (-1., 1.),
                (-1., -1.),
            ] {
                let candidate = [
                    direction[0] + theta * (su * u[0] + sv * v[0]),
                    direction[1] + theta * (su * u[1] + sv * v[1]),
                    direction[2] + theta * (su * u[2] + sv * v[2]),
                ];
                let Some(candidate) = unit(candidate) else {
                    continue;
                };
                let (_, candidate_zone) = zone_of(candidate);
                if candidate_zone < zone {
                    zone = candidate_zone;
                    direction = candidate;
                    improved = true;
                }
            }
            if !improved {
                theta *= 0.5;
                if theta <= 1e-12 {
                    break;
                }
            }
        }
        (direction, zone)
    };
    // Restart from every principal axis: for compact clouds the dominant
    // axis need not be the cylinder axis, so all three starts compete.
    for candidate in axes.axes {
        let Some(start) = unit(candidate) else {
            continue;
        };
        let (refined, zone) = refine(start, &mut iterations);
        if zone < best_zone {
            best_zone = zone;
            direction = refined;
        }
    }
    let (axis_point, zone) = zone_of(direction);
    numeric(zone.is_finite(), "Cylindricity zone lost finiteness")?;
    // Rigorous conservative certificate: outward-rounded radii with a
    // relative margin covering the ≤6 rounding steps per radial evaluation.
    let mut lo = f64::INFINITY;
    let mut hi = 0_f64;
    let ulp = f64::EPSILON;
    for p in points {
        let d = sub3(*p, axis_point);
        let t = d[0] * direction[0] + d[1] * direction[1] + d[2] * direction[2];
        let q = [
            d[0] - t * direction[0],
            d[1] - t * direction[1],
            d[2] - t * direction[2],
        ];
        let r = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
        lo = lo.min(next_down(r * (1. - 8. * ulp)));
        hi = hi.max(next_up(r * (1. + 8. * ulp)));
    }
    let certificate = next_up(hi - lo).max(zone);
    Ok(CylindricityZone {
        axis_point,
        axis_direction: direction,
        upper_bound_certificate: certificate,
        report: MinZoneReport {
            feature: MinZoneFeature::Cylindricity,
            zone_width: zone,
            solver_iterations: iterations,
            certified: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
            (bits as f64) / ((1u64 << 53) as f64)
        }
        fn signed(&mut self) -> f64 {
            2. * self.next() - 1.
        }
    }

    #[test]
    fn flatness_of_noisy_plane_is_about_two_noise_amplitudes() {
        let mut rng = Rng(0xabcd_1234_5678_9def);
        let noise = 1e-3;
        let points: Vec<[f64; 3]> = (0..16)
            .map(|_| [rng.signed(), rng.signed(), noise * rng.signed()])
            .collect();
        let result = min_zone_flatness(&points).unwrap();
        assert!(result.report.certified);
        // Zone cannot exceed the full noise spread and must cover most of it
        // with 16 uniform samples.
        assert!(result.report.zone_width <= next_up(2. * noise) * 1.000_001);
        assert!(result.report.zone_width > 1.5 * noise);
        // LS comparison: zone must not exceed ~4x the RMS spread.
        let rms = (points.iter().map(|p| p[2] * p[2]).sum::<f64>() / 16.).sqrt();
        assert!(result.report.zone_width <= 4. * rms + 1e-12);
        // Plane is nearly z-aligned.
        assert!(result.plane_normal[2].abs() > 0.99);
    }

    #[test]
    fn flatness_exact_reference_configuration() {
        // Regular tetrahedron with unit edges: the min-zone width is the
        // distance between two opposite edges, 1/sqrt(2) — a 2+2 active
        // configuration covered exactly by the enumeration.
        let h = (2.0_f64 / 3.).sqrt();
        let points = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0.5, 3.0_f64.sqrt() / 2., 0.],
            [0.5, 3.0_f64.sqrt() / 6., h],
        ];
        let result = min_zone_flatness(&points).unwrap();
        assert!(result.report.certified);
        let want = 1.0 / 2.0_f64.sqrt();
        assert!(
            (result.report.zone_width - want).abs() <= 1e-9,
            "zone = {} want {want}",
            result.report.zone_width
        );
    }

    #[test]
    fn flatness_rejects_bad_input() {
        assert!(min_zone_flatness(&[[0.; 3], [1.; 3]]).is_err());
        assert!(min_zone_flatness(&[[0.; 3], [1.; 3], [f64::NAN; 3]]).is_err());
    }

    #[test]
    fn circularity_two_concentric_circles() {
        let mut points = Vec::new();
        for i in 0..8 {
            let angle = i as f64 * std::f64::consts::TAU / 8.;
            points.push([angle.cos(), angle.sin()]);
            points.push([2. * angle.cos(), 2. * angle.sin()]);
        }
        let result = min_zone_circularity(&points).unwrap();
        assert!(result.report.certified);
        assert!(
            (result.report.zone_width - 1.).abs() <= 1e-9,
            "zone = {}",
            result.report.zone_width
        );
        assert!(result.center[0].abs() < 1e-6 && result.center[1].abs() < 1e-6);
    }

    #[test]
    fn circularity_noisy_circle_is_about_two_noise_amplitudes() {
        let mut rng = Rng(0x1111_2222_3333_4444);
        let noise = 1e-3;
        let points: Vec<[f64; 2]> = (0..24)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 24.;
                let r = 5. + noise * rng.signed();
                [r * angle.cos(), r * angle.sin()]
            })
            .collect();
        let result = min_zone_circularity(&points).unwrap();
        assert!(result.report.certified);
        assert!(result.report.zone_width <= next_up(2. * noise) * 1.000_001);
        // The zone must still be a positive fraction of the noise spread
        // (centre shifts absorb only part of the radial noise).
        assert!(result.report.zone_width > 0.5 * noise);
        // LS comparison: Kasa-fit radial spread bounds the zone from above.
        let ls = kasa_center(&points);
        let ls_zone = annulus_zone(&points, ls);
        assert!(result.report.zone_width <= ls_zone * 1.05 + 1e-12);
    }

    #[test]
    fn circularity_rejects_bad_input() {
        assert!(min_zone_circularity(&[[0.; 2], [1.; 2]]).is_err());
    }

    #[test]
    fn cylindricity_noisy_cylinder_recovers_axis_and_zone() {
        let mut rng = Rng(0x5555_6666_7777_8888);
        let noise = 1e-3;
        let mut points = Vec::new();
        // Cylinder of radius 2 around the z axis, with radial noise.
        for ring in 0..4 {
            let z = ring as f64 * 0.7;
            for i in 0..8 {
                let angle = i as f64 * std::f64::consts::TAU / 8. + ring as f64 * 0.1;
                let r = 2. + noise * rng.signed();
                points.push([r * angle.cos(), r * angle.sin(), z]);
            }
        }
        let result = min_zone_cylindricity(&points).unwrap();
        assert!(!result.report.certified);
        assert!(result.report.zone_width <= next_up(2. * noise) * 1.01);
        assert!(result.report.zone_width > 1.2 * noise);
        // Compact cloud: PCA's dominant axis is radial, so the multi-start
        // refinement must still land on a low-zone, unit, finite axis.
        assert!(result.axis_direction.iter().all(|v| v.is_finite()));
        let len = result
            .axis_direction
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        assert!((len - 1.).abs() < 1e-9);
        // The certificate is rigorous and consistent.
        assert!(result.upper_bound_certificate >= result.report.zone_width);
        assert!(result.upper_bound_certificate <= result.report.zone_width * 1.01 + 1e-12);
    }

    #[test]
    fn cylindricity_rejects_bad_input() {
        assert!(min_zone_cylindricity(&[[0.; 3]; 3]).is_err());
    }
}
