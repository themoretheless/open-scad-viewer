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
#[derive(Debug)]
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
    x.lo * 0.5 + x.hi * 0.5
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
                    let xyz = rectangle_bounds(s, uv)?;
                    let t = signed_div(
                        I::new(xyz[dominant][0], xyz[dominant][1])?
                            .sub(I::point(origin[dominant]))?,
                        direction[dominant],
                    )?;
                    report.roots.push(Root {
                        uv,
                        parameter: [t.lo, t.hi],
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

/// Conservative line parameter range of a complete surface rectangle, using
/// the ray's dominant coordinate. Useful for rejecting regions behind a ray.
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
mod tests {
    use super::*;
    #[test]
    fn rational_quintic_ray_isolation_returns_original_parameter_enclosures() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| (0..6).map(|v| vec![u as f64, v as f64 / 5., 0.]).collect())
                .collect(),
            weights: vec![vec![1.; 6], vec![2.; 6]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = surface.clone();
        let result = intersections(&surface, [0.25, 0.37, -1.], [0., 0., 1.], 1e-8, 10000).unwrap();
        assert!(result.complete && result.profile_parameterization.is_some());
        assert_eq!(result.roots.len(), 1);
        let root = &result.roots[0];
        assert!(root.uv[0][0] <= 1. / 7. && root.uv[0][1] >= 1. / 7.);
        assert!(root.parameter[0] <= 1. && root.parameter[1] >= 1.);
        assert!(root.uv[0][1] - root.uv[0][0] <= 1e-8);
        assert_eq!(surface, before);
        let mut folded = surface.clone();
        for row in &mut folded.control_points {
            for (p, y) in row.iter_mut().zip([0., 1., 1., -1., -1., 0.]) {
                p[1] = y;
            }
        }
        let denied = intersections(&folded, [0.25, 0.2, -1.], [0., 0., 1.], 1e-8, 1).unwrap();
        assert!(!denied.complete && !denied.unresolved.is_empty() && denied.cells <= 1);
    }
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 2.], vec![0., 1., 2.]],
                vec![vec![1., 0., 2.], vec![1., 1., 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn transverse_plane_root_and_exclusion() {
        let s = plane();
        let r = intersections(&s, [0.37, 0.62, 0.], [0., 0., 1.], 1e-8, 1000).unwrap();
        assert!(r.complete);
        assert_eq!(r.roots.len(), 1);
        for (k, t) in [0.37, 0.62].into_iter().enumerate() {
            assert!(r.roots[0].uv[k][0] <= t && r.roots[0].uv[k][1] >= t)
        }
        assert!(r.roots[0].parameter[0] <= 2. && r.roots[0].parameter[1] >= 2.);
        let r = intersections(&s, [2., 0.62, 0.], [0., 0., 1.], 1e-8, 1000).unwrap();
        assert!(r.complete && r.roots.is_empty());
    }
    fn folded() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|j| vec![[0.16, -0.34, 0.16][i], j as f64, i as f64 * 0.5])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn covers_two_separate_roots_without_seed_sampling() {
        let s = folded();
        let r = intersections(&s, [0., 0.37, -1.], [0., 0., 1.], 1e-7, 10000).unwrap();
        assert!(r.complete, "{} {:?}", r.cells, r.unresolved);
        assert_eq!(r.roots.len(), 2);
        for (root, u) in r.roots.iter().zip([0.2, 0.8]) {
            assert!(root.uv[0][0] <= u && root.uv[0][1] >= u);
            assert!(root.parameter[0] <= 1. + u && root.parameter[1] >= 1. + u)
        }
    }
    #[test]
    fn mid_plane_crossings_survive_subdivision_and_rounding_stays_explicit() {
        let r = intersections(&folded(), [0., 0.5, -1.], [0., 0., 1.], 1e-7, 10000).unwrap();
        assert!(r.complete);
        assert_eq!(r.roots.len(), 2);
        for root in &r.roots {
            assert!(root.uv[1][0] <= 0.5 && root.uv[1][1] >= 0.5);
        }
        assert_eq!(exact_split(0., 1.), Some(0.375));
        assert_eq!(exact_split(0.5, 0.5_f64.next_up()), None);
    }
    #[test]
    fn tangency_coincidence_and_limits_never_claim_empty_coverage() {
        let mut tangent = folded();
        for (i, row) in tangent.control_points.iter_mut().enumerate() {
            for p in row {
                p[0] = [0.25, -0.25, 0.25][i]
            }
        }
        let r = intersections(&tangent, [0., 0.37, -1.], [0., 0., 1.], 1e-7, 100).unwrap();
        assert!(!r.complete);
        assert!(r.roots.is_empty());
        assert!(!r.unresolved.is_empty());
        let r = intersections(&plane(), [0.37, 0.62, 2.], [1., 0., 0.], 1e-7, 100).unwrap();
        assert!(!r.complete && !r.unresolved.is_empty());
        let r = intersections(&folded(), [0., 0.37, -1.], [0., 0., 1.], 1e-7, 1).unwrap();
        assert!(!r.complete);
        assert_eq!(r.cells, 1);
    }
    #[test]
    fn nonbinary_knots_and_negative_direction_keep_parameter_enclosures() {
        let mut s = folded();
        s.knots_u = vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17];
        s.knots_v = vec![-0.71, -0.71, 0.29, 0.29];
        let r = intersections(&s, [0., 0.37, 2.], [0., 0., -2.], 1e-7, 10000).unwrap();
        assert!(r.complete);
        assert_eq!(r.roots.len(), 2);
        for (root, u) in r.roots.iter().zip([0.8, 0.2]) {
            let uv = 0.13 + (3.17 - 0.13) * u;
            let t = (2. - u) / 2.;
            assert!(root.uv[0][0] <= uv && root.uv[0][1] >= uv);
            assert!(root.parameter[0] <= t && root.parameter[1] >= t)
        }
    }
    #[test]
    fn rational_weights_and_multiple_spans_are_covered() {
        let mut rational = plane();
        rational.weights = vec![vec![1., 2.], vec![3., 4.]];
        let p = rational.evaluate(0.31, 0.67).unwrap().point;
        let r = intersections(&rational, [p[0], p[1], 0.], [0., 0., 1.], 1e-7, 10000).unwrap();
        assert!(r.complete);
        assert_eq!(r.roots.len(), 1);
        for (k, t) in [0.31, 0.67].into_iter().enumerate() {
            assert!(r.roots[0].uv[k][0] <= t && r.roots[0].uv[k][1] >= t)
        }
        let mut multi = plane();
        multi.knots_u = vec![0., 0., 0.3, 1., 1.];
        multi.control_points = vec![
            vec![vec![0., 0., 2.], vec![0., 1., 2.]],
            vec![vec![0.3, 0., 2.], vec![0.3, 1., 2.]],
            vec![vec![1., 0., 2.], vec![1., 1., 2.]],
        ];
        multi.weights = vec![vec![1.; 2]; 3];
        let r = intersections(&multi, [0.2, 0.63, 0.], [0., 0., 1.], 1e-7, 1000).unwrap();
        assert!(r.complete);
        assert_eq!(r.roots.len(), 1);
        assert!(r.cells >= 2);
    }
    #[test]
    fn outer_boundary_roots_cannot_be_counted_as_isolated_interior_crossings() {
        let r = intersections(&plane(), [0., 0.63, 0.], [0., 0., 1.], 1e-7, 100).unwrap();
        assert!(!r.complete);
        assert!(r.roots.is_empty());
    }
}
