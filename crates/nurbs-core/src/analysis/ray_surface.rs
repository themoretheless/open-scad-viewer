//! Complete-domain line/surface root isolation. Only strict interval Krawczyk
//! inclusions with an invertible preconditioner and contraction certify roots.
//! Unresolved cells remain explicit; this module does not classify solid volume.
use crate::{
    check,
    distance_bounds::Interval as I,
    resource,
    surface::Surface,
    surface_distance::{interpolate, rectangle_bounds},
    Result,
};
type Net = Vec<Vec<[I; 2]>>;
#[derive(Clone, Debug)]
pub struct Root {
    pub uv: [[f64; 2]; 2],
    pub parameter: [f64; 2],
}
#[derive(Debug)]
pub struct Report {
    pub profile_parameterization:
        Option<crate::surface_linear_monotonicity::ProfileParameterization>,
    pub roots: Vec<Root>,
    pub unresolved: Vec<[[f64; 2]; 2]>,
    pub cells: usize,
    pub complete: bool,
}
struct Cell {
    net: Net,
    domain: [[f64; 2]; 2],
    span: [[f64; 2]; 2],
}
fn global_domain(cell: &Cell, local: [I; 2]) -> Result<[[f64; 2]; 2]> {
    let mut out = [[0.; 2]; 2];
    for k in 0..2 {
        let d = cell.span[k];
        let x = I::point(d[0])
            .add(I::point(d[1]).sub(I::point(d[0]))?.mul(local[k])?)?
            .intersect(d[0], d[1])?;
        out[k] = [x.lo, x.hi];
    }
    Ok(out)
}
fn unresolved_domain(cell: &Cell) -> Result<[[f64; 2]; 2]> {
    global_domain(cell, cell.domain.map(|d| I { lo: d[0], hi: d[1] }))
}
fn hull(values: impl Iterator<Item = I>) -> I {
    values.fold(
        I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        },
        |a, b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        },
    )
}
fn midpoint(x: I) -> f64 {
    crate::distance_bounds::midpoint(x.lo, x.hi)
}
fn signed_div(a: I, b: f64) -> Result<I> {
    if b > 0. {
        a.div(I::point(b))
    } else {
        I::point(0.).sub(a)?.div(I::point(-b))
    }
}

/// Exact original-span Bernstein coefficients, enclosed without first rounding
/// a derived control net. Both projected polynomials have the same positive W.
fn net(
    s: &Surface,
    span: [usize; 2],
    origin: [f64; 3],
    direction: [f64; 3],
    dominant: usize,
) -> Result<Net> {
    let [iu, iv] = span;
    let p = s.degree_u;
    let q = s.degree_v;
    let mut scale = 0_f64;
    for i in iu - p..=iu {
        for j in iv - q..=iv {
            scale = scale.max(s.weights[i][j])
        }
    }
    let mut weights = [f64::INFINITY, 0_f64];
    let mut controls = Vec::new();
    for i in iu - p..=iu {
        let mut row = Vec::new();
        for j in iv - q..=iv {
            let w = if s.weights[i][j] == scale {
                I::point(1.)
            } else {
                I::point(s.weights[i][j]).div(I::point(scale))?
            };
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let mut h = [I::point(0.); 4];
            h[3] = w;
            for k in 0..3 {
                h[k] = I::point(s.control_points[i][j][k])
                    .sub(I::point(origin[k]))?
                    .mul(w)?
            }
            row.push(h);
        }
        controls.push(row)
    }
    let axes = (0..3).filter(|&k| k != dominant).collect::<Vec<_>>();
    let mut out = vec![vec![[I::point(0.); 2]; q + 1]; p + 1];
    for v in 0..=q {
        let vs = (0..q)
            .map(|k| s.knots_v[if k < q - v { iv } else { iv + 1 }])
            .collect::<Vec<_>>();
        let rows = controls
            .iter()
            .map(|r| interpolate(r.clone(), q, &s.knots_v, iv, &vs, weights))
            .collect::<Result<Vec<_>>>()?;
        for u in 0..=p {
            let us = (0..p)
                .map(|k| s.knots_u[if k < p - u { iu } else { iu + 1 }])
                .collect::<Vec<_>>();
            let h = interpolate(rows.clone(), p, &s.knots_u, iu, &us, weights)?;
            for k in 0..2 {
                out[u][v][k] = h[axes[k]]
                    .mul(I::point(direction[dominant]))?
                    .sub(h[dominant].mul(I::point(direction[axes[k]]))?)?
            }
        }
    }
    Ok(out)
}
fn blend(a: [I; 2], b: [I; 2], t: f64) -> Result<[I; 2]> {
    Ok([
        a[0].mul(I::point(1. - t))?.add(b[0].mul(I::point(t))?)?,
        a[1].mul(I::point(1. - t))?.add(b[1].mul(I::point(t))?)?,
    ])
}
fn split_line(row: Vec<[I; 2]>) -> Result<[Vec<[I; 2]>; 2]> {
    split_line_at(row, 0.5)
}
fn split_line_at(mut row: Vec<[I; 2]>, t: f64) -> Result<[Vec<[I; 2]>; 2]> {
    let n = row.len();
    let mut left = vec![row[0]];
    let mut right = vec![row[n - 1]];
    for count in (1..n).rev() {
        for j in 0..count {
            row[j] = blend(row[j], row[j + 1], t)?
        }
        left.push(row[0]);
        right.push(row[count - 1])
    }
    right.reverse();
    Ok([left, right])
}
fn split(net: &Net, axis: usize) -> Result<[Net; 2]> {
    let (nu, nv) = (net.len(), net[0].len());
    let mut out = [net.clone(), net.clone()];
    if axis == 0 {
        for v in 0..nv {
            let pair = split_line_at((0..nu).map(|u| net[u][v]).collect(), 0.375)?;
            for side in 0..2 {
                for u in 0..nu {
                    out[side][u][v] = pair[side][u]
                }
            }
        }
    } else {
        for u in 0..nu {
            let pair = split_line_at(net[u].clone(), 0.375)?;
            for side in 0..2 {
                out[side][u] = pair[side].clone()
            }
        }
    }
    Ok(out)
}
fn center(net: &Net) -> Result<[I; 2]> {
    let mut rows = Vec::new();
    for row in net {
        rows.push(*split_line(row.clone())?[0].last().unwrap())
    }
    Ok(*split_line(rows)?[0].last().unwrap())
}
enum Verdict {
    Excluded,
    Unique([I; 2]),
    Unresolved,
}
// Domain endpoints must agree exactly with the Bernstein subdivision recipe.
// Rounded endpoints would silently move a certified/excluded cell. Stop with
// explicit unresolved coverage when the next dyadic split is not representable.
fn exact_split(lo: f64, hi: f64) -> Option<f64> {
    fn sum_error(a: f64, b: f64, sum: f64) -> f64 {
        let z = sum - a;
        (a - (sum - z)) + (b - z)
    }
    let width = hi - lo;
    let part = width * 0.375;
    let mid = lo + part;
    if !width.is_finite()
        || !mid.is_finite()
        || width < f64::MIN_POSITIVE * 18014398509481984.
        || sum_error(hi, -lo, width) != 0.
        || width.mul_add(0.375, -part) != 0.
        || sum_error(lo, part, mid) != 0.
        || mid <= lo
        || mid >= hi
    {
        None
    } else {
        Some(mid)
    }
}
fn krawczyk(net: &Net) -> Result<Verdict> {
    if (0..2).any(|k| {
        let h = hull(net.iter().flatten().map(|p| p[k]));
        h.lo > 0. || h.hi < 0.
    }) {
        return Ok(Verdict::Excluded);
    }
    let (p, q) = (net.len() - 1, net[0].len() - 1);
    if p == 0 || q == 0 {
        return Ok(Verdict::Unresolved);
    }
    let mut jac = [[I::point(0.); 2]; 2];
    for k in 0..2 {
        let mut u = Vec::new();
        let mut v = Vec::new();
        for i in 0..p {
            for j in 0..=q {
                u.push(
                    net[i + 1][j][k]
                        .sub(net[i][j][k])?
                        .mul(I::point(p as f64))?,
                )
            }
        }
        for i in 0..=p {
            for j in 0..q {
                v.push(
                    net[i][j + 1][k]
                        .sub(net[i][j][k])?
                        .mul(I::point(q as f64))?,
                )
            }
        }
        jac[k] = [hull(u.into_iter()), hull(v.into_iter())];
    }
    let a = jac.map(|r| r.map(midpoint));
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(Verdict::Unresolved);
    }
    let y = [
        [a[1][1] / det, -a[0][1] / det],
        [-a[1][0] / det, a[0][0] / det],
    ];
    if !y.iter().flatten().all(|x| x.is_finite()) {
        return Ok(Verdict::Unresolved);
    }
    let determinant = I::point(y[0][0])
        .mul(I::point(y[1][1]))?
        .sub(I::point(y[0][1]).mul(I::point(y[1][0]))?)?;
    if determinant.lo <= 0. && determinant.hi >= 0. {
        return Ok(Verdict::Unresolved);
    }
    let f = center(net)?;
    let mut image = [I::point(0.5); 2];
    let mut contraction = 0_f64;
    for i in 0..2 {
        let mut row_norm = I::point(0.);
        for j in 0..2 {
            image[i] = image[i].sub(I::point(y[i][j]).mul(f[j])?)?
        }
        for k in 0..2 {
            let mut m = I::point(if i == k { 1. } else { 0. });
            for j in 0..2 {
                m = m.sub(I::point(y[i][j]).mul(jac[j][k])?)?
            }
            row_norm = row_norm.add(I::point(m.lo.abs().max(m.hi.abs())))?;
            image[i] = image[i].add(m.mul(I::new(-0.5, 0.5)?)?)?;
        }
        contraction = contraction.max(row_norm.hi);
    }
    if image.iter().any(|x| x.hi < 0. || x.lo > 1.) {
        return Ok(Verdict::Excluded);
    }
    if contraction < 0.5 && image.iter().all(|x| x.lo > 0. && x.hi < 1.) {
        Ok(Verdict::Unique(image))
    } else {
        Ok(Verdict::Unresolved)
    }
}
/// Intersections with the complete infinite line origin + t direction. Positive
/// t selection and UV trimming belong to the caller. Boundary roots, tangency,
/// coincidence and exhausted budgets remain unresolved rather than disappearing.
pub fn intersections(
    s: &Surface,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    if let Some((polynomial, parameterization)) =
        crate::surface_linear_monotonicity::polynomial_profile_image(s)?
    {
        let tolerance = I::point(tolerance_uv)
            .mul(I::point(parameterization.derivative_lower))?
            .lo;
        if tolerance > 0. && tolerance.is_finite() {
            let mut report =
                intersections_original(&polynomial, origin, direction, tolerance, max_cells)?;
            // Isolation and complete-domain coverage are preserved by the exact
            // positive bijection. UV enclosures refer to the original source;
            // geometric ray-parameter intervals remain unchanged.
            for root in &mut report.roots {
                root.uv[0] = parameterization.inverse_interval(root.uv[0])?;
            }
            for uv in &mut report.unresolved {
                uv[0] = parameterization.inverse_interval(uv[0])?;
            }
            report.profile_parameterization = Some(parameterization);
            return Ok(report);
        }
    }
    intersections_original(s, origin, direction, tolerance_uv, max_cells)
}
fn intersections_original(
    s: &Surface,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|&x| x != 0.),
        "Line origin/direction must be finite and direction nonzero",
    )?;
    check(
        tolerance_uv.is_finite() && tolerance_uv > 0. && (1..=100000).contains(&max_cells),
        "Line isolation needs positive UV tolerance and 1..100000 cells",
    )?;
    let dominant = (0..3)
        .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
        .unwrap();
    let spans = |knots: &[f64], degree: usize, count: usize| {
        (degree..count)
            .filter(|&i| knots[i] < knots[i + 1])
            .collect::<Vec<_>>()
    };
    let us = spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = spans(&s.knots_v, s.degree_v, s.control_points[0].len());
    if us.len().saturating_mul(vs.len()) > max_cells {
        return Err(resource(
            "Initial line/surface knot cells exceed the budget",
        ));
    }
    let mut stack = Vec::new();
    for u in us {
        for &v in &vs {
            let domain = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            stack.push(Cell {
                net: net(s, [u, v], origin, direction, dominant)?,
                domain: [[0., 1.]; 2],
                span: domain,
            })
        }
    }
    isolate(stack, tolerance_uv, max_cells, |uv| {
        let xyz = rectangle_bounds(s, uv)?;
        let t = signed_div(
            I::new(xyz[dominant][0], xyz[dominant][1])?.sub(I::point(origin[dominant]))?,
            direction[dominant],
        )?;
        Ok([t.lo, t.hi])
    })
}
fn isolate(
    mut stack: Vec<Cell>,
    tolerance_uv: f64,
    max_cells: usize,
    mut parameter_bounds: impl FnMut([[f64; 2]; 2]) -> Result<[f64; 2]>,
) -> Result<Report> {
    let mut report = Report {
        profile_parameterization: None,
        roots: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        complete: false,
    };
    while let Some(cell) = stack.pop() {
        if report.cells == max_cells {
            report.unresolved.push(unresolved_domain(&cell)?);
            for pending in stack {
                report.unresolved.push(unresolved_domain(&pending)?)
            }
            break;
        }
        report.cells += 1;
        match krawczyk(&cell.net)? {
            Verdict::Excluded => continue,
            Verdict::Unique(image) => {
                let mut unit = [I::point(0.); 2];
                for k in 0..2 {
                    let d = cell.domain[k];
                    let mapped = I::point(d[0])
                        .add(I::point(d[1]).sub(I::point(d[0]))?.mul(image[k])?)?
                        .intersect(d[0], d[1])?;
                    unit[k] = mapped
                }
                let uv = global_domain(&cell, unit)?;
                if uv.iter().all(|d| (d[1] - d[0]).next_up() <= tolerance_uv) {
                    report.roots.push(Root {
                        uv,
                        parameter: parameter_bounds(uv)?,
                    });
                    continue;
                }
            }
            Verdict::Unresolved => {}
        }
        let axis =
            if (cell.domain[0][1] - cell.domain[0][0]) >= (cell.domain[1][1] - cell.domain[1][0]) {
                0
            } else {
                1
            };
        let [lo, hi] = cell.domain[axis];
        // A dyadic asymmetric split keeps common mid-plane roots inside cells.
        // Roots on any actual cell boundary still require strict certification;
        // changing the split never turns an unresolved root into an exclusion.
        let Some(mid) = exact_split(lo, hi) else {
            report.unresolved.push(unresolved_domain(&cell)?);
            continue;
        };
        let [left, right] = split(&cell.net, axis)?;
        for (net, range) in [(left, [lo, mid]), (right, [mid, hi])] {
            let mut domain = cell.domain;
            domain[axis] = range;
            stack.push(Cell {
                net,
                domain,
                span: cell.span,
            })
        }
    }
    report.complete = report.unresolved.is_empty();
    report
        .roots
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    Ok(report)
}

/// Root isolation on caller-enclosed homogeneous polynomial coefficients.
/// This returns diagnostic roots, not a B-rep or material certificate.
pub(crate) fn intersections_coefficients(
    net: &[Vec<[I; 4]>],
    domain: [[f64; 2]; 2],
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    check(
        !net.is_empty()
            && !net[0].is_empty()
            && net.len() <= 17
            && net[0].len() <= 17
            && net.iter().all(|r| r.len() == net[0].len())
            && net
                .iter()
                .flatten()
                .flat_map(|h| h.iter())
                .all(|v| v.lo.is_finite() && v.hi.is_finite() && v.lo <= v.hi)
            && domain
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1])
            && origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|x| *x != 0.)
            && tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && max_cells <= 100000,
        "Bound homogeneous line intersection domains and work",
    )?;
    if net.iter().flatten().any(|h| h[3].lo <= 0.) {
        return Ok(Report {
            roots: Vec::new(),
            unresolved: vec![domain],
            cells: 0,
            complete: false,
            profile_parameterization: None,
        });
    }
    let dominant = (0..3)
        .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
        .unwrap();
    let axes: Vec<_> = (0..3).filter(|&a| a != dominant).collect();
    let projected = net
        .iter()
        .map(|row| {
            row.iter()
                .map(|h| {
                    let mut out = [I::point(0.); 2];
                    for k in 0..2 {
                        out[k] = h[axes[k]]
                            .sub(I::point(origin[axes[k]]).mul(h[3])?)?
                            .mul(I::point(direction[dominant]))?
                            .sub(
                                h[dominant]
                                    .sub(I::point(origin[dominant]).mul(h[3])?)?
                                    .mul(I::point(direction[axes[k]]))?,
                            )?;
                    }
                    Ok(out)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Net>>()?;
    let initial = Cell {
        net: projected,
        domain: [[0., 1.]; 2],
        span: domain,
    };
    isolate(vec![initial], tolerance_uv, max_cells, |uv| {
        let mut t = [I::point(0.); 2];
        for a in 0..2 {
            t[a] = I::new(uv[a][0], uv[a][1])?
                .sub(I::point(domain[a][0]))?
                .div(I::point(domain[a][1]).sub(I::point(domain[a][0]))?)?
                .intersect(0., 1.)?;
        }
        let evaluate = |mut row: Vec<[I; 4]>, parameter: I| -> Result<[I; 4]> {
            let q = I::point(1.).sub(parameter)?;
            while row.len() > 1 {
                row = row
                    .windows(2)
                    .map(|p| {
                        let mut h = [I::point(0.); 4];
                        for k in 0..4 {
                            h[k] = p[0][k].mul(q)?.add(p[1][k].mul(parameter)?)?;
                        }
                        Ok(h)
                    })
                    .collect::<Result<Vec<_>>>()?;
            }
            Ok(row[0])
        };
        let rows = net
            .iter()
            .map(|r| evaluate(r.clone(), t[1]))
            .collect::<Result<Vec<_>>>()?;
        let h = evaluate(rows, t[0])?;
        let coordinate = h[dominant].div(h[3])?;
        let parameter = signed_div(
            coordinate.sub(I::point(origin[dominant]))?,
            direction[dominant],
        )?;
        Ok([parameter.lo, parameter.hi])
    })
}

/// Conservative line parameter range of a complete surface rectangle, using
/// the ray's dominant coordinate. Useful for rejecting regions behind a ray.
/// Conservative finite-line parameters that can meet a certified position box.
/// None proves exclusion; Some does not prove a surface intersection or material.
pub fn finite_line_enclosure_parameters(
    bounds: [[f64; 2]; 3],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Result<Option<[f64; 2]>> {
    line_enclosure_parameters(bounds, origin, direction, [0., 1.])
}
/// Infinite-line box exclusion, preserving possible intersections at any t.
pub fn infinite_line_enclosure_parameters(
    bounds: [[f64; 2]; 3],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Result<Option<[f64; 2]>> {
    line_enclosure_parameters(
        bounds,
        origin,
        direction,
        [f64::NEG_INFINITY, f64::INFINITY],
    )
}
fn line_enclosure_parameters(
    bounds: [[f64; 2]; 3],
    origin: [f64; 3],
    direction: [f64; 3],
    mut parameter: [f64; 2],
) -> Result<Option<[f64; 2]>> {
    check(
        origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|&x| x != 0.),
        "Choose a finite nonzero line",
    )?;
    for axis in 0..3 {
        let box_axis = I::new(bounds[axis][0], bounds[axis][1])?;
        if direction[axis] == 0. {
            if !box_axis.contains(origin[axis]) {
                return Ok(None);
            }
            continue;
        }
        let t = box_axis
            .sub(I::point(origin[axis]))?
            .div_signed(I::point(direction[axis]))?;
        parameter[0] = parameter[0].max(t.lo);
        parameter[1] = parameter[1].min(t.hi);
        if parameter[0] > parameter[1] {
            return Ok(None);
        }
    }
    Ok(Some(parameter))
}

pub fn parameter_bounds(
    s: &Surface,
    uv: [[f64; 2]; 2],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Result<[f64; 2]> {
    check(
        origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|&x| x != 0.),
        "Line origin/direction must be finite and direction nonzero",
    )?;
    let k = (0..3)
        .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
        .unwrap();
    let xyz = rectangle_bounds(s, uv)?;
    let t = signed_div(
        I::new(xyz[k][0], xyz[k][1])?.sub(I::point(origin[k]))?,
        direction[k],
    )?;
    Ok([t.lo, t.hi])
}

#[cfg(test)]
#[path = "ray_surface/tests/mod.rs"]
mod tests;

/// Intersect the forward half-line P(t)=origin+t*direction, t>=0.
/// Roots whose certified parameter interval straddles t=0 remain unresolved;
/// no sign is inferred from a floating-point midpoint. Regions certified behind
/// the origin are excluded. Surface boundaries/tangencies retain the underlying
/// isolator's unresolved status.
pub fn ray_intersections(
    surface: &Surface,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    let line = intersections(surface, origin, direction, tolerance_uv, max_cells)?;
    let mut roots = Vec::new();
    let mut unresolved = Vec::new();
    for root in line.roots {
        if root.parameter[0] >= 0. {
            roots.push(root);
        } else if root.parameter[1] >= 0. {
            unresolved.push(root.uv);
        }
    }
    for uv in line.unresolved {
        if parameter_bounds(surface, uv, origin, direction)?[1] >= 0. {
            unresolved.push(uv);
        }
    }
    Ok(Report {
        complete: unresolved.is_empty(),
        roots,
        unresolved,
        cells: line.cells,
        profile_parameterization: line.profile_parameterization,
    })
}
