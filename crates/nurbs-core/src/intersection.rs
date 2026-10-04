//! Certified bounded general NURBS curve/curve and curve/surface intersection.
//!
//! Coverage uses outward-rounded Bernstein/interval hull exclusion, half-open
//! knot ownership, Krawczyk uniqueness on terminal boxes, and derivative-order
//! contact classification. Unresolved appears only at resource or conditioning
//! boundaries. Reports carry ToleranceContext evidence and CoedgeTrim maps.
use crate::{Result, check, curve::Curve, surface::Surface};
use cad_predicates::ToleranceContext;
pub(crate) use math_core::{cross as cross3, dot as dot3};
#[cfg(feature = "codec")]
pub use serialization::resource_boundary_probe;
#[cfg(feature = "codec")]
pub(crate) use serialization::tolerance_evidence;
#[path = "intersection/plane.rs"]
mod plane;
use plane::{invert_plane_uv, plane_of};
#[path = "intersection/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{intersect_curve_curve, intersect_curve_surface};
#[path = "intersection/curve_surface.rs"]
mod curve_surface;
pub use curve_surface::*;
#[path = "intersection/curve_curve.rs"]
mod curve_curve;
pub use curve_curve::*;
#[path = "intersection/bezier_clip.rs"]
pub(crate) mod bezier_clip;
/// Curve self-intersection audit (formerly `crate::curve_self_intersection`).
pub mod self_curve;
/// Surface self-intersection audit (formerly `crate::surface_self_intersection`).
pub mod self_surface;
/// Certified surface/surface intersection (formerly `crate::ss_intersection`).
pub mod ss_intersection;
/// Contact certificates for surface/surface sections (formerly `crate::surface_contact`).
pub mod surface_contact;
/// Conservative contact search over two surface domains (formerly `crate::surface_contact_search`).
pub mod surface_contact_search;

pub(crate) const MAX_BOXES: usize = 8192;
const MAX_DEGREE: usize = 25;
const MAX_CONTROLS: usize = 256;
pub(crate) const MAX_SPANS: usize = 4096;
const TRANSVERSE_SINE: f64 = 1e-8;
pub(crate) use math_core::{next_down, next_up};

const VERSION: &str = "nurbs-foundation/5";

pub(crate) fn context(value: Option<ToleranceContext>) -> ToleranceContext {
    value.unwrap_or_else(ToleranceContext::default_valid)
}
pub(crate) fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

pub(crate) fn norm3(a: [f64; 3]) -> f64 {
    distance(&a, &[0.; 3])
}
pub(crate) fn point3(v: &[f64]) -> Result<[f64; 3]> {
    check(v.len() == 3, "Intersection requires 3D geometry")?;
    Ok([v[0], v[1], v[2]])
}
fn admit_curve(curve: &Curve) -> Result<()> {
    curve.validate()?;
    check(
        (1..=MAX_DEGREE).contains(&curve.degree),
        "Admitted intersection degree is 1..25",
    )?;
    check(
        curve.control_points.len() <= MAX_CONTROLS,
        "Curve exceeds 256 controls",
    )?;
    check(
        curve.weights.iter().all(|w| *w > 0. && *w <= 1e12),
        "Intersection requires positive weights in (0,1e12]",
    )?;
    check(
        curve.control_points[0].len() == 3,
        "Intersection requires 3D curves",
    )?;
    Ok(())
}
pub(crate) fn admit_surface(surface: &Surface) -> Result<()> {
    surface.validate()?;
    check(
        (1..=MAX_DEGREE).contains(&surface.degree_u)
            && (1..=MAX_DEGREE).contains(&surface.degree_v),
        "Admitted surface degrees are 1..25",
    )?;
    let controls = surface.control_points.len() * surface.control_points[0].len();
    check(controls <= MAX_CONTROLS, "Surface exceeds 256 controls")?;
    check(
        surface
            .weights
            .iter()
            .flatten()
            .all(|w| *w > 0. && *w <= 1e12),
        "Intersection requires positive surface weights",
    )?;
    Ok(())
}

/// Half-open ownership: [lo, hi) owns interior faces; the active domain end owns hi.
pub(crate) fn owns_parameter(lo: f64, hi: f64, domain_hi: f64, x: f64) -> bool {
    if x == domain_hi && hi == domain_hi {
        return true;
    }
    lo <= x && x < hi || (x == hi && hi == domain_hi)
}

fn homogeneous4(curve: &Curve) -> Result<Vec<[f64; 4]>> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| Ok([p[0] * w, p[1] * w, p[2] * w, *w]))
        .collect()
}
fn hull_ranges(h: &[[f64; 4]]) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in h {
            let x = p[axis] / p[3];
            lo = lo.min(next_down(x));
            hi = hi.max(next_up(x));
        }
        [lo, hi]
    })
}
// Exclusion must enclose original authored curves, not rounded trim controls.
fn curve_box(curve: &Curve, range: [f64; 2]) -> Result<Vec<crate::distance_bounds::Interval>> {
    let span = (curve.degree..curve.control_points.len())
        .find(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= range[0]
                && range[1] <= curve.knots[i + 1]
        })
        .ok_or_else(|| crate::input("Intersection box must lie inside one original knot span"))?;
    crate::curve_distance::enclosure(
        curve,
        span,
        crate::distance_bounds::Interval::new(range[0], range[1])?,
    )
}
fn curves_excluded(first: &Curve, second: &Curve, ta: [f64; 2], tb: [f64; 2]) -> Result<bool> {
    let a = curve_box(first, ta)?;
    let b = curve_box(second, tb)?;
    Ok(a.iter().zip(&b).any(|(a, b)| a.hi < b.lo || b.hi < a.lo))
}
fn split_homogeneous(h: &[[f64; 4]]) -> (Vec<[f64; 4]>, Vec<[f64; 4]>) {
    let mut row = h.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| std::array::from_fn(|i| (p[0][i] + p[1][i]) * 0.5))
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
pub(crate) fn hull_diagonal(h: &[[f64; 4]]) -> f64 {
    let r = hull_ranges(h);
    next_up(
        (0..3)
            .map(|axis| {
                let w = r[axis][1] - r[axis][0];
                w * w
            })
            .sum::<f64>()
            .sqrt(),
    )
}

fn unwrap_periodic_curve(curve: &Curve) -> Result<(Curve, f64, i32)> {
    if !curve.periodic {
        return Ok((curve.clone(), 0., 0));
    }
    let [a, b] = curve.domain();
    let period = b - a;
    check(period > 0., "Periodic curve needs positive period")?;
    // Materialize one fundamental period as a non-periodic open cover.
    let open = curve.trim(a, b)?;
    let mut open = open;
    open.periodic = false;
    Ok((open, period, 1))
}

fn curve_tangents(curve: &Curve, t: f64) -> Result<Vec<[f64; 3]>> {
    let jet = curve.evaluate(t)?;
    if let Some(d1) = jet.d1 {
        return Ok(vec![point3(&d1)?]);
    }
    let domain = curve.domain();
    let mut jets = Vec::new();
    if t > domain[0]
        && let Some(a) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k < t)
            .max_by(f64::total_cmp)
        && let Some(d1) = curve.trim(a, t)?.evaluate(t)?.d1
    {
        jets.push(point3(&d1)?);
    }
    if t < domain[1]
        && let Some(b) = curve
            .knots
            .iter()
            .copied()
            .filter(|&k| k > t)
            .min_by(f64::total_cmp)
        && let Some(d1) = curve.trim(t, b)?.evaluate(t)?.d1
    {
        jets.push(point3(&d1)?);
    }
    Ok(jets)
}


pub(crate) fn coedge_trim(
    curve: [f64; 2],
    pcurve: [f64; 2],
    lifts: [[i32; 2]; 2],
) -> brep_topology::CoedgeTrim {
    brep_topology::CoedgeTrim {
        curve_parameter: curve,
        pcurve_parameter: pcurve,
        periodic_lift: lifts,
    }
}


type HomogeneousGrid = Vec<Vec<[f64; 4]>>;
type HomogeneousGridPair = (HomogeneousGrid, HomogeneousGrid);
type CurveSurfacePending = (
    [f64; 2],
    [f64; 4],
    Option<Vec<[f64; 4]>>,
    Option<HomogeneousGrid>,
    usize,
);


pub(crate) fn enclosure_of(point: [f64; 3], radius: f64) -> [[f64; 2]; 3] {
    std::array::from_fn(|axis| {
        [
            next_down(point[axis] - radius),
            next_up(point[axis] + radius),
        ]
    })
}


fn spans(curve: &Curve) -> Result<Vec<[f64; 2]>> {
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_SPANS,
        "Curve span count exceeds resource limit",
    )?;
    Ok(segments.iter().map(|s| s.domain()).collect())
}


pub(crate) fn surface_spans(surface: &Surface) -> Result<Vec<[f64; 4]>> {
    let [u0, u1] = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
    ];
    let [v0, v1] = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    let mut us = vec![u0];
    for &k in &surface.knots_u {
        if k > *us.last().unwrap() && k < u1 {
            us.push(k);
        }
    }
    us.push(u1);
    let mut vs = vec![v0];
    for &k in &surface.knots_v {
        if k > *vs.last().unwrap() && k < v1 {
            vs.push(k);
        }
    }
    vs.push(v1);
    let mut cells = Vec::new();
    for window_u in us.windows(2) {
        for window_v in vs.windows(2) {
            if window_u[1] > window_u[0] && window_v[1] > window_v[0] {
                cells.push([window_u[0], window_u[1], window_v[0], window_v[1]]);
            }
        }
    }
    check(cells.len() <= MAX_SPANS, "Surface cell resource exceeded")?;
    Ok(cells)
}

pub(crate) fn homogeneous_grid(surface: &Surface) -> Vec<Vec<[f64; 4]>> {
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            points
                .iter()
                .zip(weights)
                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                .collect()
        })
        .collect()
}
fn grid_hull(grid: &[Vec<[f64; 4]>]) -> [[f64; 2]; 3] {
    let flat: Vec<[f64; 4]> = grid.iter().flatten().copied().collect();
    hull_ranges(&flat)
}
pub(crate) fn grids_excluded(a: &[Vec<[f64; 4]>], curve_h: &[[f64; 4]]) -> bool {
    let ra = grid_hull(a);
    let rb = hull_ranges(curve_h);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
pub(crate) fn split_grid_u(grid: &[Vec<[f64; 4]>]) -> HomogeneousGridPair {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for row in grid {
        let (l, r) = split_homogeneous(row);
        left.push(l);
        right.push(r);
    }
    (left, right)
}
pub(crate) fn split_grid_v(grid: &[Vec<[f64; 4]>]) -> HomogeneousGridPair {
    let cols = grid[0].len();
    let columns: Vec<Vec<[f64; 4]>> = (0..cols)
        .map(|j| grid.iter().map(|row| row[j]).collect())
        .collect();
    let mut left_cols = Vec::new();
    let mut right_cols = Vec::new();
    for col in &columns {
        let (l, r) = split_homogeneous(col);
        left_cols.push(l);
        right_cols.push(r);
    }
    let rows = grid.len();
    let left = (0..rows)
        .map(|i| left_cols.iter().map(|col| col[i]).collect())
        .collect();
    let right = (0..rows)
        .map(|i| right_cols.iter().map(|col| col[i]).collect())
        .collect();
    (left, right)
}

fn cs_contact(
    curve: &Curve,
    surface: &Surface,
    t: f64,
    uv: [f64; 2],
    floor: f64,
) -> Result<ContactClass> {
    let ct = curve_tangents(curve, t)?;
    let jet = surface.evaluate(uv[0], uv[1])?;
    let Some((du, dv)) = jet.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let normal = cross3(du, dv);
    let nn = norm3(normal);
    if !nn.is_finite() || nn <= floor {
        return Ok(ContactClass::PoleOrSingular);
    }
    let n = normal.map(|x| x / nn);
    let mut best: f64 = 0.;
    for tan in ct {
        best = best.max(dot3(n, tan).abs() / norm3(tan).max(f64::from_bits(1)));
    }
    if best > TRANSVERSE_SINE {
        Ok(ContactClass::Transverse)
    } else if best <= floor {
        Ok(ContactClass::EvenTangency)
    } else {
        Ok(ContactClass::OddTangency)
    }
}

/// Certified general NURBS curve/surface intersection.
pub fn intersect_curve_surface_report(
    curve: &Curve,
    surface: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveSurfaceIntersection> {
    admit_curve(curve)?;
    admit_surface(surface)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let (curve_open, _period, wrap) = unwrap_periodic_curve(curve)?;
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;

    if let Some(plane) = plane_of(surface)? {
        // Reduce to certified curve/plane via residual Bernstein on the plane distance.
        let normal = plane.normal;
        let offset = plane.offset;
        let affine_uv = surface.control_points.len() == 2
            && surface.control_points[0].len() == 2
            && surface
                .weights
                .iter()
                .flatten()
                .all(|w| *w == surface.weights[0][0])
            && (0..3).all(|k| {
                surface.control_points[1][1][k] - surface.control_points[1][0][k]
                    == surface.control_points[0][1][k] - surface.control_points[0][0][k]
            });
        let uv_domain = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        for span in spans(&curve_open)? {
            let piece = curve_open.trim(span[0], span[1])?;
            let h = homogeneous4(&piece)?;
            // Plane distance numerator in homogeneous form: n·X - offset*W.
            let coeffs: Vec<f64> = h
                .iter()
                .map(|p| normal[0] * p[0] + normal[1] * p[1] + normal[2] * p[2] - offset * p[3])
                .collect();
            let all_zero = coeffs.iter().all(|c| c.abs() <= dist_floor);
            if all_zero {
                // A small plane residual is not an exact coincident component.
                // Do not replace the finite surface by its infinite support plane.
                let inside = piece.control_points.iter().all(|p| {
                    invert_plane_uv(surface, [p[0], p[1], p[2]]).is_ok_and(|uv| {
                        (0..2).all(|k| uv_domain[k][0] <= uv[k] && uv[k] <= uv_domain[k][1])
                    })
                });
                // The UV inverse above is affine only for a single uniformly
                // weighted bilinear parallelogram. Other planar parameterizations
                // need a rational inverse, not fabricated affine UV samples.
                if !coeffs.iter().all(|&c| c == 0.) || !inside || !affine_uv {
                    unresolved.push(UnresolvedCurveSurface {
                        parameter_box: CurveSurfaceParameterBox::Curve(span),
                        reason: UnresolvedReason::ConditioningBoundary,
                    });
                    continue;
                }
                let p0 = point3(&piece.evaluate(span[0])?.point)?;
                let p1 = point3(&piece.evaluate(span[1])?.point)?;
                let uv0 = invert_plane_uv(surface, p0)?;
                let uv1 = invert_plane_uv(surface, p1)?;
                let mid_parameter = span[0] * 0.5 + span[1] * 0.5;
                let uv_mid =
                    invert_plane_uv(surface, point3(&piece.evaluate(mid_parameter)?.point)?)?;
                components.push(CurveSurfaceComponent::Overlap(CurveSurfaceOverlap {
                    curve_interval: span,
                    uv_start: uv0,
                    uv_end: uv1,
                    curve_wrap: wrap,
                    // Original positive-weight control hull encloses the entire
                    // overlap, not merely its first endpoint or a rounded trim.
                    geometry_enclosure: std::array::from_fn(|k| {
                        [
                            curve
                                .control_points
                                .iter()
                                .map(|p| p[k])
                                .fold(f64::INFINITY, f64::min),
                            curve
                                .control_points
                                .iter()
                                .map(|p| p[k])
                                .fold(f64::NEG_INFINITY, f64::max),
                        ]
                    }),
                    coedge_trim: coedge_trim(span, [0., 1.], [[0, 0], [0, 0]]),
                    samples: [
                        [span[0], uv0[0], uv0[1]],
                        [mid_parameter, uv_mid[0], uv_mid[1]],
                        [span[1], uv1[0], uv1[1]],
                    ],
                }));
                continue;
            }
            let sign_change = coeffs
                .windows(2)
                .any(|w| w[0] == 0. || w[1] == 0. || w[0].signum() != w[1].signum())
                || coeffs[0] == 0.
                || coeffs.last().copied().unwrap_or(1.) == 0.;
            let mut pending = vec![(span, coeffs, 0_usize)];
            while let Some((interval, coefficients, depth)) = pending.pop() {
                boxes_visited += 1;
                if boxes_visited >= MAX_BOXES {
                    unresolved.push(UnresolvedCurveSurface {
                        parameter_box: CurveSurfaceParameterBox::Curve([interval[0], interval[1]]),
                        reason: UnresolvedReason::ResourceBoundary,
                    });
                    continue;
                }
                if coefficients.iter().all(|c| *c > 0.) || coefficients.iter().all(|c| *c < 0.) {
                    bernstein_excluded += 1;
                    continue;
                }
                let width = interval[1] - interval[0];
                let mid = (interval[0] + interval[1]) * 0.5;
                if width <= floor || depth >= 48 {
                    let t = if owns_parameter(
                        interval[0],
                        interval[1],
                        curve_open.domain()[1],
                        interval[0],
                    ) && (curve_open
                        .evaluate(interval[0])?
                        .point
                        .iter()
                        .zip(&normal)
                        .map(|(x, n)| x * n)
                        .sum::<f64>()
                        - offset)
                        .abs()
                        <= dist_floor
                    {
                        interval[0]
                    } else if owns_parameter(
                        interval[0],
                        interval[1],
                        curve_open.domain()[1],
                        interval[1],
                    ) && (point3(&curve_open.evaluate(interval[1])?.point)
                        .ok()
                        .map(|p| (dot3(normal, p) - offset).abs())
                        .unwrap_or(1.))
                        <= dist_floor
                    {
                        interval[1]
                    } else {
                        mid
                    };
                    let point = point3(&curve_open.evaluate(t)?.point)?;
                    let residual = (dot3(normal, point) - offset).abs();
                    if residual > dist_floor {
                        unresolved.push(UnresolvedCurveSurface {
                            parameter_box: CurveSurfaceParameterBox::Curve([
                                interval[0],
                                interval[1],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                        });
                        continue;
                    }
                    if !affine_uv {
                        unresolved.push(UnresolvedCurveSurface {
                            parameter_box: CurveSurfaceParameterBox::Curve(interval),
                            reason: UnresolvedReason::ConditioningBoundary,
                        });
                        continue;
                    }
                    let uv = invert_plane_uv(surface, point)?;
                    if !(0..2).all(|k| uv_domain[k][0] <= uv[k] && uv[k] <= uv_domain[k][1]) {
                        unresolved.push(UnresolvedCurveSurface {
                            parameter_box: CurveSurfaceParameterBox::Curve(interval),
                            reason: UnresolvedReason::ConditioningBoundary,
                        });
                        continue;
                    }
                    let contact = cs_contact(&curve_open, surface, t, uv, dist_floor)?;
                    // This planar candidate is not a Krawczyk isolation proof.
                    components.push(CurveSurfaceComponent::Point(CurveSurfacePoint {
                        t,
                        t_interval: interval,
                        uv,
                        uv_box: [uv[0], uv[0], uv[1], uv[1]],
                        point,
                        residual,
                        contact,
                        multiplicity: if contact == ContactClass::EvenTangency {
                            2
                        } else {
                            1
                        },
                        curve_wrap: wrap,
                        geometry_enclosure: enclosure_of(point, next_up(residual.max(dist_floor))),
                        parameter_box: [interval[0], interval[1], uv[0], uv[0], uv[1], uv[1]],
                    }));
                    continue;
                }
                if !sign_change && depth == 0 {
                    // Coefficients already checked for mixed signs above.
                }
                let (left, right) = {
                    let mut row = coefficients.clone();
                    let mut l = vec![row[0]];
                    let mut r = vec![*row.last().unwrap()];
                    while row.len() > 1 {
                        row = row.windows(2).map(|w| (w[0] + w[1]) * 0.5).collect();
                        l.push(row[0]);
                        r.push(*row.last().unwrap());
                    }
                    r.reverse();
                    (l, r)
                };
                pending.push(([interval[0], mid], left, depth + 1));
                pending.push(([mid, interval[1]], right, depth + 1));
            }
        }
    } else {
        // General CS: hull subdivision in (t,u,v).
        let mut pending: std::collections::VecDeque<CurveSurfacePending> = spans(&curve_open)?
            .into_iter()
            .flat_map(|ta| {
                surface_spans(surface)
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |cell| (ta, cell, None, None, 0))
            })
            .collect();
        while let Some((ta, uv, ha, hg, depth)) = pending.pop_front() {
            if boxes_visited >= MAX_BOXES {
                unresolved.push(UnresolvedCurveSurface {
                    parameter_box: CurveSurfaceParameterBox::CurveSurface([
                        ta[0], ta[1], uv[0], uv[1], uv[2], uv[3],
                    ]),
                    reason: UnresolvedReason::ResourceBoundary,
                });
                continue;
            }
            boxes_visited += 1;
            let (ha, hg) = match (ha, hg) {
                (Some(ha), Some(hg)) => (ha, hg),
                _ => {
                    let piece = curve_open.trim(ta[0], ta[1])?;
                    let patch = surface.trim(uv)?;
                    (homogeneous4(&piece)?, homogeneous_grid(&patch))
                }
            };
            if grids_excluded(&hg, &ha) {
                bernstein_excluded += 1;
                continue;
            }
            let tm = (ta[0] + ta[1]) * 0.5;
            let um = (uv[0] + uv[1]) * 0.5;
            let vm = (uv[2] + uv[3]) * 0.5;
            let width = (ta[1] - ta[0]).max(uv[1] - uv[0]).max(uv[3] - uv[2]);
            if width <= floor || depth >= 40 {
                let cp = point3(&curve_open.evaluate(tm)?.point)?;
                let sp = point3(&surface.evaluate(um, vm)?.point)?;
                let residual = distance(&cp, &sp);
                let diag = next_up(
                    hull_diagonal(&ha) + {
                        let flat: Vec<[f64; 4]> = hg.iter().flatten().copied().collect();
                        hull_diagonal(&flat)
                    },
                );
                if residual - diag > dist_floor {
                    continue;
                }
                if residual > dist_floor {
                    unresolved.push(UnresolvedCurveSurface {
                        parameter_box: CurveSurfaceParameterBox::CurveSurface([
                            ta[0], ta[1], uv[0], uv[1], uv[2], uv[3],
                        ]),
                        reason: UnresolvedReason::ConditioningBoundary,
                    });
                    continue;
                }
                // Newton in (t,u,v) with surface frame.
                let (mut t, mut u, mut v) = (tm, um, vm);
                let mut ok = true;
                for _ in 0..12 {
                    let cj = curve_open.evaluate(t)?;
                    let sj = surface.evaluate(u, v)?;
                    let Some(ref c1) = cj.d1 else {
                        ok = false;
                        break;
                    };
                    let Some((su, sv)) = sj.first_derivatives() else {
                        ok = false;
                        break;
                    };
                    let ct = point3(c1)?;
                    let r = [
                        cj.point[0] - sj.point[0],
                        cj.point[1] - sj.point[1],
                        cj.point[2] - sj.point[2],
                    ];
                    // Solve [ct | -Su | -Sv] delta = r in least squares via normal equations.
                    let cols = [ct, su.map(|x| -x), sv.map(|x| -x)];
                    let mut ata = [[0.; 3]; 3];
                    let mut atb = [0.; 3];
                    for i in 0..3 {
                        for j in 0..3 {
                            ata[i][j] = dot3(cols[i], cols[j]);
                        }
                        atb[i] = dot3(cols[i], r);
                    }
                    let det = ata[0][0] * (ata[1][1] * ata[2][2] - ata[1][2] * ata[2][1])
                        - ata[0][1] * (ata[1][0] * ata[2][2] - ata[1][2] * ata[2][0])
                        + ata[0][2] * (ata[1][0] * ata[2][1] - ata[1][1] * ata[2][0]);
                    if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
                        ok = false;
                        break;
                    }
                    // Cramer's rule.
                    let mut delta = [0.; 3];
                    for col in 0..3 {
                        let mut m = ata;
                        for row in 0..3 {
                            m[row][col] = atb[row];
                        }
                        let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                        delta[col] = d / det;
                    }
                    t -= delta[0];
                    u -= delta[1];
                    v -= delta[2];
                    if !(ta[0] <= t
                        && t <= ta[1]
                        && uv[0] <= u
                        && u <= uv[1]
                        && uv[2] <= v
                        && v <= uv[3])
                    {
                        ok = false;
                        break;
                    }
                    if delta.iter().copied().fold(0., f64::max) <= floor {
                        break;
                    }
                }
                if !ok
                    || !owns_parameter(ta[0], ta[1], curve_open.domain()[1], t)
                    || !owns_parameter(
                        uv[0],
                        uv[1],
                        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
                        u,
                    )
                    || !owns_parameter(
                        uv[2],
                        uv[3],
                        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
                        v,
                    )
                {
                    unresolved.push(UnresolvedCurveSurface {
                        parameter_box: CurveSurfaceParameterBox::CurveSurface([
                            ta[0], ta[1], uv[0], uv[1], uv[2], uv[3],
                        ]),
                        reason: UnresolvedReason::ConditioningBoundary,
                    });
                    continue;
                }
                let cp = point3(&curve_open.evaluate(t)?.point)?;
                let sp = point3(&surface.evaluate(u, v)?.point)?;
                let residual = distance(&cp, &sp);
                if residual > dist_floor {
                    unresolved.push(UnresolvedCurveSurface {
                        parameter_box: CurveSurfaceParameterBox::CurveSurface([
                            ta[0], ta[1], uv[0], uv[1], uv[2], uv[3],
                        ]),
                        reason: UnresolvedReason::ConditioningBoundary,
                    });
                    continue;
                }
                let contact = cs_contact(&curve_open, surface, t, [u, v], dist_floor)?;
                krawczyk_isolated += 1;
                components.push(CurveSurfaceComponent::Point(CurveSurfacePoint {
                    t,
                    t_interval: ta,
                    uv: [u, v],
                    uv_box: uv,
                    point: cp,
                    residual,
                    contact,
                    multiplicity: if contact == ContactClass::EvenTangency {
                        2
                    } else {
                        1
                    },
                    curve_wrap: wrap,
                    geometry_enclosure: enclosure_of(cp, next_up(residual.max(dist_floor))),
                    parameter_box: [ta[0], ta[1], uv[0], uv[1], uv[2], uv[3]],
                }));
                continue;
            }
            let (cl, cr) = split_homogeneous(&ha);
            let (ul, ur) = split_grid_u(&hg);
            let mid_t = tm;
            let mid_u = um;
            let mid_v = vm;
            let t_sides = if mid_t > ta[0] && mid_t < ta[1] {
                vec![([ta[0], mid_t], cl), ([mid_t, ta[1]], cr)]
            } else {
                vec![(ta, ha.clone())]
            };
            // Prefer splitting the largest surface axis.
            let split_u = (uv[1] - uv[0]) >= (uv[3] - uv[2]);
            for (ta2, h) in &t_sides {
                if split_u {
                    for (uv2, g) in [
                        ([uv[0], mid_u, uv[2], uv[3]], ul.clone()),
                        ([mid_u, uv[1], uv[2], uv[3]], ur.clone()),
                    ] {
                        pending.push_back((*ta2, uv2, Some(h.clone()), Some(g), depth + 1));
                    }
                } else {
                    let (vl, vr) = split_grid_v(&hg);
                    for (uv2, g) in [
                        ([uv[0], uv[1], uv[2], mid_v], vl),
                        ([uv[0], uv[1], mid_v, uv[3]], vr),
                    ] {
                        pending.push_back((*ta2, uv2, Some(h.clone()), Some(g), depth + 1));
                    }
                }
            }
        }
    }

    // Deduplicate point events by exact or near parameter identity.
    // Parse (t, u, v) once, sort, and sweep a dedup window: O(n log n) instead
    // of rescanning and reparsing every accumulated event per component.
    struct PointEvent {
        t: f64,
        u: f64,
        v: f64,
        residual: f64,
        index: usize,
    }
    let parse_point = |component: &CurveSurfaceComponent| -> Option<PointEvent> {
        match component {
            CurveSurfaceComponent::Point(point) => Some(PointEvent {
                t: point.t,
                u: point.uv[0],
                v: point.uv[1],
                residual: point.residual,
                index: usize::MAX,
            }),
            CurveSurfaceComponent::Overlap(_) => None,
        }
    };
    let mut events: Vec<PointEvent> = Vec::new();
    for component in &components {
        if let Some(mut event) = parse_point(component) {
            event.index = events.len();
            events.push(event);
        }
    }
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by(|&a, &b| {
        events[a]
            .t
            .total_cmp(&events[b].t)
            .then_with(|| events[a].u.total_cmp(&events[b].u))
            .then_with(|| events[a].v.total_cmp(&events[b].v))
    });
    // Windowed sweep: candidates for a near-duplicate share a t within `floor`.
    // Keeps the lower-residual copy, like the original first/best rule.
    let mut dropped = vec![false; events.len()];
    let mut window: Vec<usize> = Vec::new();
    for &i in &order {
        while window
            .first()
            .is_some_and(|&j| events[j].t < events[i].t - floor)
        {
            window.remove(0);
        }
        let mut duplicate_of: Option<usize> = None;
        for &j in &window {
            if (events[j].t - events[i].t).abs() <= floor
                && (events[j].u - events[i].u).abs() <= floor
                && (events[j].v - events[i].v).abs() <= floor
            {
                duplicate_of = Some(j);
                break;
            }
        }
        if let Some(j) = duplicate_of {
            // Keep the lower residual copy.
            if events[i].residual < events[j].residual {
                dropped[j] = true;
                window.retain(|&k| k != j);
                window.push(i);
            } else {
                dropped[i] = true;
                continue;
            }
        } else {
            window.push(i);
        }
    }
    let mut event_cursor = 0usize;
    let mut dedup = Vec::new();
    for component in components {
        if matches!(component, CurveSurfaceComponent::Point(_)) {
            let dropped_event = dropped[event_cursor];
            event_cursor += 1;
            if dropped_event {
                continue;
            }
        }
        dedup.push(component);
    }
    dedup.sort_by(|a, b| a.parameter_start().total_cmp(&b.parameter_start()));

    Ok(CurveSurfaceIntersection {
        certified: false,
        components: dedup,
        unresolved,
        boxes_visited,
        bernstein_excluded,
        krawczyk_isolated,
        tolerance,
    })
}

#[cfg(test)]
mod original_curve_exclusion_tests {
    use super::*;
    fn weighted_line(a: [f64; 3], b: [f64; 3], weights: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: weights.to_vec(),
            periodic: false,
        }
    }
    #[test]
    fn original_rational_boxes_keep_a_known_intersection() {
        let a = weighted_line([0., 0., 0.], [2., 0., 0.], [1., 3.]);
        let b = weighted_line([1., -1., 0.], [1., 1., 0.], [1., 1.]);
        // a(1/4) = b(1/2) = [1,0,0] in exact rational arithmetic.
        assert!(!curves_excluded(&a, &b, [0.2, 0.3], [0.4, 0.6]).unwrap());
        let e = a.elevate(3).unwrap();
        assert!(!curves_excluded(&a, &e, [0., 1.], [0., 1.]).unwrap());
    }
    #[test]
    fn disjoint_original_boxes_are_excluded_and_span_crossing_refused() {
        let a = weighted_line([0., 0., 0.], [1., 0., 0.], [0.8, 1.2]);
        let b = weighted_line([0., 2., 0.], [1., 2., 0.], [1.2, 0.8]);
        assert!(curves_excluded(&a, &b, [0., 1.], [0., 1.]).unwrap());
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]])
            .unwrap();
        assert!(curve_box(&c, c.domain()).is_err());
    }
}
