//! Sufficient global injectivity certificate for a complete NURBS chart.
//! If a fixed projection F and constant Y satisfy ||I-Y DF||_inf < 1 over
//! the entire convex parameter rectangle, F(x)=F(y) implies x=y. Every knot
//! rectangle contributes to the same Jacobian hull; local success is not enough.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
mod shared_boundary;
pub use shared_boundary::certify_shared_natural_boundary;
#[derive(Clone, Debug)]
pub struct Report {
    pub proven: bool,
    pub projection: Option<[usize; 2]>,
    pub contraction_upper: Option<f64>,
    pub spans: usize,
    pub reason: &'static str,
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
/// Derivatives of the restricted section. A collapsed parameter axis is fixed
/// and its column is zero; it is not a bound on transverse surface derivatives.
pub(crate) fn section_jacobian(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<[[I; 2]; 3]> {
    let (p, q) = (s.degree_u, s.degree_v);
    let mut net = match crate::curve_surface_composition::surface_net_on_bernstein(s, span, domain)?
    {
        Some(net) => net,
        None => crate::curve_surface_composition::surface_net_on(s, span, domain)?,
    };
    // Translation does not change derivatives. Reducing the numerator's
    // magnitude improves the rational quotient bound without changing geometry.
    let origin = &s.control_points[span[0] - p][span[1] - q];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    jacobian_from_net(&net, [p, q], domain)
}

pub(crate) fn jacobian_from_net(
    net: &[Vec<[I; 4]>],
    degrees: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<[[I; 2]; 3]> {
    let [p, q] = degrees;
    let values: [I; 4] = std::array::from_fn(|k| hull(net.iter().flatten().map(|h| h[k])));
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if domain[axis][0] == domain[axis][1] {
            continue;
        }
        let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
        let mut diffs = Vec::with_capacity((p + 1) * (q + 1));
        for i in 0..=p {
            for j in 0..=q {
                if (axis == 0 && i == p) || (axis == 1 && j == q) {
                    continue;
                }
                let a = net[i][j];
                let b = net[i + usize::from(axis == 0)][j + usize::from(axis == 1)];
                let mut d = [I::point(0.); 4];
                for k in 0..4 {
                    d[k] = b[k].sub(a[k])?.mul(I::point(degree as f64))?.div(width)?;
                }
                diffs.push(d);
            }
        }
        let d: [I; 4] = std::array::from_fn(|k| hull(diffs.iter().map(|h| h[k])));
        for k in 0..3 {
            result[k][axis] = d[k]
                .mul(values[3])?
                .sub(values[k].mul(d[3])?)?
                .div(denominator)?;
        }
    }
    Ok(result)
}
fn contraction(j: [[I; 2]; 2]) -> Result<Option<f64>> {
    let a = j.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let y = [
        [a[1][1] / det, -a[0][1] / det],
        [-a[1][0] / det, a[0][0] / det],
    ];
    if y.iter().flatten().any(|x| !x.is_finite()) {
        return Ok(None);
    }
    let mut maximum = 0_f64;
    for i in 0..2 {
        let mut row = I::point(0.);
        for k in 0..2 {
            let product = I::point(y[i][0])
                .mul(j[0][k])?
                .add(I::point(y[i][1]).mul(j[1][k])?)?;
            let residual = I::point(if i == k { 1. } else { 0. }).sub(product)?;
            row = row.add(I::point(residual.lo.abs().max(residual.hi.abs())))?;
        }
        maximum = maximum.max(row.hi);
    }
    Ok(Some(maximum))
}
/// Failure of this sufficient condition is unresolved, never evidence of an
/// intersection. This certifies the whole natural chart, hence any trimmed
/// subset, but does not compare it with other faces or certify a whole solid.
pub fn certify(s: &Surface, max_spans: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_spans > 0 && max_spans <= 100000,
        "Injectivity span budget must be in 1..100000",
    )?;
    if s.periodic_u || s.periodic_v {
        return Ok(Report {
            proven: false,
            projection: None,
            contraction_upper: None,
            spans: 0,
            reason: "periodic-domain",
        });
    }
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut report = certify_on(s, domain, max_spans)?;
    if report.proven {
        report.reason = "global-projection-contraction";
    }
    Ok(report)
}
/// Prove injectivity and rank on one complete convex rectangle of the ORIGINAL
/// surface, including a local chart of a periodic support. No rounded trim or
/// candidate surface grants evidence. The result concerns only this rectangle.
pub fn certify_on(s: &Surface, domain: [[f64; 2]; 2], max_spans: usize) -> Result<Report> {
    certify_on_refined(s, domain, max_spans, 0)
}
/// Refine complete original-domain covers before testing one COMMON Jacobian
/// hull and projection. Independent local successes never imply this result.
/// All evaluated rectangles, including previous levels, share max_spans.
pub fn certify_on_refined(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
    max_depth: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        max_spans > 0 && max_spans <= 100_000,
        "Injectivity span budget must be in 1..100000",
    )?;
    check(max_depth <= 6, "Injectivity refinement depth must be 0..6")?;
    let report = Report {
        proven: false,
        projection: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    for axis in 0..2 {
        let (k, d, n) = if axis == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        check(
            domain[axis].iter().all(|x| x.is_finite())
                && domain[axis][0] < domain[axis][1]
                && domain[axis][0] >= k[d]
                && domain[axis][1] <= k[n],
            "Injectivity rectangle must be noncollapsed and inside the original surface domain",
        )?;
    }
    let us =
        crate::certificates::audit::nonempty_spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = crate::certificates::audit::nonempty_spans(
        &s.knots_v,
        s.degree_v,
        s.control_points[0].len(),
    );
    let mut cover = Vec::new();
    for &u in &us {
        for &v in &vs {
            let section = [
                [
                    domain[0][0].max(s.knots_u[u]),
                    domain[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    domain[1][0].max(s.knots_v[v]),
                    domain[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if section.iter().all(|r| r[0] < r[1]) {
                cover.push(([u, v], section));
            }
        }
    }
    certify_original_cover(s, cover, report, max_spans, max_depth, "rectangle-projection-contraction")
}
fn certify_original_cover(
    s: &Surface,
    cover: Vec<([usize; 2], [[f64; 2]; 2])>,
    report: Report,
    max_spans: usize,
    max_depth: usize,
    success_reason: &'static str,
) -> Result<Report> {
    Ok(certify_original_cover_with_projection(
        s,
        cover,
        report,
        max_spans,
        max_depth,
        success_reason,
        false,
        [1., 1.],
    )?
    .0)
}
fn certify_original_cover_with_projection(
    s: &Surface,
    mut cover: Vec<([usize; 2], [[f64; 2]; 2])>,
    mut report: Report,
    max_spans: usize,
    max_depth: usize,
    success_reason: &'static str,
    allow_oblique: bool,
    column_scale: [f64; 2],
) -> Result<(Report, Option<[[f64; 3]; 2]>, [f64; 2])> {
    let mut projection_matrix = None;
    let mut proven_scale = column_scale;
    for depth in 0..=max_depth {
        let mut section_jacobians = Vec::new();
        let mut j = [[I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        }; 2]; 3];
        for &(span, section) in &cover {
            if report.spans == max_spans {
                report.reason = "work-limit";
                return Ok((report, projection_matrix, proven_scale));
            }
            let next = section_jacobian(s, span, section)?;
            if allow_oblique {
                let mut local = next;
                for k in 0..3 {
                    for axis in 0..2 {
                        if column_scale[axis] != 1. {
                            local[k][axis] = local[k][axis].mul(I::point(column_scale[axis]))?;
                        }
                    }
                }
                section_jacobians.push(local);
            }
            report.spans += 1;
            for k in 0..3 {
                for axis in 0..2 {
                    j[k][axis] = I {
                        lo: j[k][axis].lo.min(next[k][axis].lo),
                        hi: j[k][axis].hi.max(next[k][axis].hi),
                    };
                }
            }
        }
        for world in 0..3 {
            for axis in 0..2 {
                if column_scale[axis] != 1. {
                    j[world][axis] = j[world][axis].mul(I::point(column_scale[axis]))?;
                }
            }
        }
        for axes in [[0, 1], [0, 2], [1, 2]] {
            if let Some(q) = contraction([j[axes[0]], j[axes[1]]])? {
                if q < 1. {
                    projection_matrix = Some(std::array::from_fn(|row| {
                        std::array::from_fn(|world| if axes[row] == world { 1. } else { 0. })
                    }));
                    report.proven = true;
                    report.projection = Some(axes);
                    report.contraction_upper = Some(q);
                    report.reason = success_reason;
                    return Ok((report, projection_matrix, proven_scale));
                }
            }
        }
        if allow_oblique {
            for nx in -1i32..=1 {
                for ny in -1i32..=1 {
                    for nz in -1i32..=1 {
                        let n = [nx, ny, nz];
                        let Some(pivot) = n.iter().position(|v| *v != 0) else {
                            continue;
                        };
                        if n[pivot] != 1 || n.iter().filter(|v| **v != 0).count() < 2 {
                            continue;
                        }
                        let other = (0..3).filter(|k| *k != pivot).collect::<Vec<_>>();
                        let matrix: [[f64; 3]; 2] = std::array::from_fn(|row| {
                            std::array::from_fn(|k| {
                                if k == pivot {
                                    f64::from(n[other[row]])
                                } else if k == other[row] {
                                    -1.
                                } else {
                                    0.
                                }
                            })
                        });
                        let mut projected = [[I {
                            lo: f64::INFINITY,
                            hi: f64::NEG_INFINITY,
                        }; 2]; 2];
                        for local in &section_jacobians {
                            for row in 0..2 {
                                for axis in 0..2 {
                                    let mut value = I::point(0.);
                                    for k in 0..3 {
                                        if matrix[row][k] != 0. {
                                            value = value.add(
                                                local[k][axis].mul(I::point(matrix[row][k]))?,
                                            )?;
                                        }
                                    }
                                    projected[row][axis].lo = projected[row][axis].lo.min(value.lo);
                                    projected[row][axis].hi = projected[row][axis].hi.max(value.hi);
                                }
                            }
                        }
                        for extra in [1., 0.5, 2., 0.25, 4., 0.125, 8.] {
                            let effective = column_scale[0] * extra;
                            if !effective.is_finite()
                                || effective <= 0.
                                || crate::exact_products::sum_sign(&[
                                    (column_scale[0], extra),
                                    (-effective, 1.),
                                ]) != std::cmp::Ordering::Equal
                            {
                                continue;
                            }
                            let mut weighted = projected;
                            if extra != 1. {
                                for row in 0..2 {
                                    weighted[row][0] = weighted[row][0].mul(I::point(extra))?;
                                }
                            }
                            if let Some(q) = contraction(weighted)? {
                                if q < 1. {
                                    report.proven = true;
                                    report.projection = None;
                                    report.contraction_upper = Some(q);
                                    report.reason = success_reason;
                                    projection_matrix = Some(matrix);
                                    proven_scale = [effective, column_scale[1]];
                                    return Ok((report, projection_matrix, proven_scale));
                                }
                            }
                        }
                    }
                }
            }
        }
        if depth == max_depth {
            break;
        }
        if cover.len() * 4 > max_spans - report.spans {
            report.reason = "work-limit";
            return Ok((report, projection_matrix, proven_scale));
        }
        let mut refined = Vec::with_capacity(cover.len() * 4);
        for (span, section) in cover {
            let mid = section.map(|r| 0.5 * r[0] + 0.5 * r[1]);
            if (0..2).any(|a| mid[a] <= section[a][0] || mid[a] >= section[a][1]) {
                report.reason = "subdivision-limit";
                return Ok((report, projection_matrix, proven_scale));
            }
            for u in [[section[0][0], mid[0]], [mid[0], section[0][1]]] {
                for v in [[section[1][0], mid[1]], [mid[1], section[1][1]]] {
                    refined.push((span, [u, v]));
                }
            }
        }
        cover = refined;
    }
    Ok((report, projection_matrix, proven_scale))
}
/// Enclose (dS/du cross dS/dv) dot direction over the requested rectangle.
/// At a knot, include both one-sided derivatives. A point rectangle uses a
/// non-collapsed knot span for derivative bounds, never a zero column.
/// None means the span budget did not cover the entire rectangle.
pub fn normal_direction_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    direction: [f64; 3],
    max_spans: usize,
) -> Result<Option<[f64; 2]>> {
    s.validate()?;
    let knots = [&s.knots_u, &s.knots_v];
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    check(
        max_spans > 0 && max_spans <= 100000 && direction.iter().all(|x| x.is_finite()),
        "Normal bounds require a finite direction and bounded positive span budget",
    )?;
    for axis in 0..2 {
        let d = domain[axis];
        check(
            d.iter().all(|x| x.is_finite())
                && d[0] <= d[1]
                && d[0] >= knots[axis][degrees[axis]]
                && d[1] <= knots[axis][counts[axis]],
            "Normal rectangle must be inside the natural surface domain",
        )?;
    }
    let mut spans = 0;
    let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
    for u in crate::certificates::audit::nonempty_spans(knots[0], degrees[0], counts[0]) {
        for v in crate::certificates::audit::nonempty_spans(knots[1], degrees[1], counts[1]) {
            let indices = [u, v];
            let mut section = [[0.; 2]; 2];
            let mut outside = false;
            for axis in 0..2 {
                let k = indices[axis];
                let lo = knots[axis][k];
                let hi = knots[axis][k + 1];
                if domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
                if section[axis][0] == section[axis][1] {
                    section[axis] = [lo, hi];
                }
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok(None);
            }
            spans += 1;
            let j = section_jacobian(s, indices, section)?;
            let mut dot = I::point(0.);
            for axis in 0..3 {
                let b = (axis + 1) % 3;
                let c = (axis + 2) % 3;
                let component = j[b][0].mul(j[c][1])?.sub(j[c][0].mul(j[b][1])?)?;
                dot = dot.add(component.mul(I::point(direction[axis]))?)?;
            }
            bounds[0] = bounds[0].min(dot.lo);
            bounds[1] = bounds[1].max(dot.hi);
        }
    }
    Ok(Some(bounds))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., (i * j) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn normal_bounds_cover_graph_derivatives_and_do_not_collapse_at_points() {
        // S(u,v)=(u,v,4uv), hence (Su cross Sv).d = -4v*dx-4u*dy+dz.
        let s = graph();
        for domain in [
            [[0., 1.], [0., 1.]],
            [[0.2, 0.4], [0.3, 0.7]],
            [[0.25, 0.25], [0.5, 0.5]],
        ] {
            for direction in [[0., 0., 1.], [0.3, -0.2, 0.7], [0., 0., -1.]] {
                let b = normal_direction_bounds(&s, domain, direction, 10)
                    .unwrap()
                    .unwrap();
                for u in domain[0] {
                    for v in domain[1] {
                        let value = -4. * v * direction[0] - 4. * u * direction[1] + direction[2];
                        assert!(b[0] <= value && value <= b[1], "{b:?}: {value}");
                    }
                }
                if direction[2] == 1. {
                    assert!(b[0] > 0.);
                }
                if direction[2] == -1. {
                    assert!(b[1] < 0.);
                }
            }
        }
    }
    #[test]
    fn rational_normal_bounds_contain_an_independent_separable_formula() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        // x=2u/(1+u), y=3v/(1+2v); normal Z is dx/du * dy/dv.
        for i in 0..10 {
            for j in 0..10 {
                let domain = [
                    [i as f64 / 10., (i + 1) as f64 / 10.],
                    [j as f64 / 10., (j + 1) as f64 / 10.],
                ];
                let b = normal_direction_bounds(&s, domain, [0., 0., 1.], 1)
                    .unwrap()
                    .unwrap();
                for u in domain[0] {
                    for v in domain[1] {
                        let z = 6. / ((1. + u).powi(2) * (1. + 2. * v).powi(2));
                        assert!(b[0] <= z && z <= b[1]);
                    }
                }
            }
        }
    }
    #[test]
    fn normal_bounds_keep_both_knot_sides_and_refuse_partial_coverage() {
        let mut s = graph();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        for p in &mut s.control_points[2] {
            p[0] = 0.;
        }
        let d = [[0.5, 0.5], [0.3, 0.7]];
        assert!(
            normal_direction_bounds(&s, d, [0., 0., 1.], 1)
                .unwrap()
                .is_none()
        );
        let b = normal_direction_bounds(&s, d, [0., 0., 1.], 2)
            .unwrap()
            .unwrap();
        assert!(b[0] < 0. && b[1] > 0.);
        assert!(normal_direction_bounds(&s, [[-1., 0.], [0., 1.]], [0., 0., 1.], 2).is_err());
    }
    #[test]
    fn nonlinear_graph_and_vertical_rotation_are_globally_injective() {
        let mut s = graph();
        let r = certify(&s, 10).unwrap();
        assert!(r.proven);
        assert_eq!(r.projection, Some([0, 1]));
        assert!(r.contraction_upper.unwrap() < 1e-10);
        for row in &mut s.control_points {
            for p in row {
                p.swap(1, 2);
            }
        }
        assert!(certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn folded_and_collapsed_charts_never_claim_injectivity() {
        let mut s = graph();
        for (i, row) in s.control_points.iter_mut().enumerate() {
            for p in row {
                p[0] = [1., -1., 1.][i];
                p[2] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
        for row in &mut s.control_points {
            for p in row {
                p[0] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn all_spans_must_pass_the_same_global_test() {
        let mut s = graph();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.3, 1., 1.];
        let partial = certify(&s, 1).unwrap();
        assert!(!partial.proven);
        assert_eq!(partial.reason, "work-limit");
        let r = certify(&s, 2).unwrap();
        assert!(r.proven);
        assert_eq!(r.spans, 2);
        // Both individual spans are regular; their union folds back on itself.
        for p in &mut s.control_points[2] {
            p[0] = 0.;
            p[2] = 0.;
        }
        for p in &mut s.control_points[1] {
            p[2] = 0.;
        }
        assert!(!certify(&s, 2).unwrap().proven);
    }
}

mod seam_glued;
pub use seam_glued::{GluedRectangleReport,certify_seam_glued_rectangle};
