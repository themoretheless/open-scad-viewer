//! Literal C0 border restrictions and signed world-coordinate plane charts.
//! Unsupported clocks stay unresolved; no rounded curve extraction is admitted.
use super::*;
use crate::exact_curve_segments::exact_add;
fn clamped(c: &Curve) -> bool {
    let n = c.control_points.len();
    c.knots[..=c.degree].iter().all(|&u| u == c.knots[c.degree])
        && c.knots[n..].iter().all(|&u| u == c.knots[n])
}
fn mirror(knots: &[f64], domain: [f64; 2]) -> Option<Vec<f64>> {
    let sum = exact_add(domain[0], domain[1])?;
    knots.iter().rev().map(|&u| exact_add(sum, -u)).collect()
}
fn bind(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    target: Vec<[f64; 4]>,
    target_knots: Vec<f64>,
    constant: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    let actual = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(p, &w)| [p[0], p[1], p[2], w])
        .collect::<Vec<_>>();
    if !constant && (actual != target || c.knots != target_knots) {
        return Ok(None);
    }
    let values = actual
        .iter()
        .chain(&target)
        .flatten()
        .copied()
        .chain(c.knots.iter().copied())
        .chain(target_knots.iter().copied())
        .chain(p.control_points.iter().flatten().copied())
        .chain(p.weights.iter().copied())
        .chain(p.knots.iter().copied())
        .chain(s.control_points.iter().flatten().flatten().copied())
        .chain(s.weights.iter().flatten().copied())
        .chain(s.knots_u.iter().chain(&s.knots_v).copied())
        .chain([
            c.degree as f64,
            p.degree as f64,
            s.degree_u as f64,
            s.degree_v as f64,
        ])
        .collect::<Vec<_>>();
    let source = SourceArena::authored(
        "literal-structured-boundary",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Invalid literal boundary context"))?;
    let mut index = 0;
    let mut leaf = || {
        let l = source.leaf(index).unwrap();
        index += 1;
        l
    };
    let a = (0..actual.len())
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect::<Vec<_>>();
    let b = (0..target.len())
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect::<Vec<_>>();
    let ak = (0..c.knots.len()).map(|_| leaf()).collect::<Vec<_>>();
    let bk = (0..target_knots.len()).map(|_| leaf()).collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    if constant {
        cad_predicates::rational_bezier_identity(&mut context, &a, &b)
    } else {
        cad_predicates::rational_bspline_same_definition(&mut context, c.degree, &a, &ak, &b, &bk)
    }
    .map(Some)
    .map_err(|_| crate::input("Invalid structured boundary identity"))
}
pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 8
        || p.degree > 8
        || s.degree_u > 8
        || s.degree_v > 8
        || c.control_points.len() > 256
        || p.control_points.len() > 256
        || !clamped(c)
        || !clamped(p)
    {
        return Ok(None);
    }
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if counts.iter().any(|&n| n > 256) || counts[0] * counts[1] > 4096 {
        return Ok(None);
    }
    if (0..2).any(|k| {
        !knots[k][..=degrees[k]]
            .iter()
            .all(|&u| u == knots[k][degrees[k]])
            || !knots[k][counts[k]..]
                .iter()
                .all(|&u| u == knots[k][counts[k]])
    }) {
        return Ok(None);
    }
    let domain = std::array::from_fn::<_, 2, _>(|k| [knots[k][degrees[k]], knots[k][counts[k]]]);
    if p.control_points
        .iter()
        .any(|v| (0..2).any(|k| v[k] < domain[k][0] || v[k] > domain[k][1]))
    {
        return Ok(None);
    }
    // A signed coordinate identity plane lifts arbitrary original C0 curves
    // by copying literal coordinates and retaining the complete rational basis.
    if s.degree_u == 1
        && s.degree_v == 1
        && counts == [2, 2]
        && s.weights.iter().flatten().all(|&w| w == s.weights[0][0])
        && c.degree == p.degree
        && c.control_points.len() == p.control_points.len()
        && c.domain() == p.domain()
    {
        for axes in [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]] {
            for signs in [[1., 1.], [1., -1.], [-1., 1.], [-1., -1.]] {
                let fixed = 3 - axes[0] - axes[1];
                let height = s.control_points[0][0][fixed];
                if !(0..2).all(|u| {
                    (0..2).all(|v| {
                        s.control_points[u][v][axes[0]] == signs[0] * domain[0][u]
                            && s.control_points[u][v][axes[1]] == signs[1] * domain[1][v]
                            && s.control_points[u][v][fixed] == height
                    })
                }) {
                    continue;
                }
                let expected_knots = if reversed {
                    let Some(k) = mirror(&p.knots, p.domain()) else {
                        continue;
                    };
                    k
                } else {
                    p.knots.clone()
                };
                let mut target = p
                    .control_points
                    .iter()
                    .zip(&p.weights)
                    .map(|(uv, &w)| {
                        let mut h = [height, height, height, w];
                        h[axes[0]] = signs[0] * uv[0];
                        h[axes[1]] = signs[1] * uv[1];
                        h
                    })
                    .collect::<Vec<_>>();
                if reversed {
                    target.reverse();
                }
                if let Some(r) = bind(c, p, s, target, expected_knots, false, max_work)? {
                    return Ok(Some(r));
                }
            }
        }
    }
    if p.degree != 1 || p.control_points.len() != 2 || p.weights[0] != p.weights[1] {
        return Ok(None);
    }
    for fixed in 0..2 {
        let free = 1 - fixed;
        let a = &p.control_points[0];
        let b = &p.control_points[1];
        if a[fixed] != b[fixed] {
            continue;
        }
        let row = if a[fixed] == domain[fixed][0] {
            0
        } else if a[fixed] == domain[fixed][1] {
            counts[fixed] - 1
        } else {
            continue;
        };
        let target = (0..counts[free])
            .map(|i| {
                let (u, v) = if fixed == 0 { (row, i) } else { (i, row) };
                let p = &s.control_points[u][v];
                [p[0], p[1], p[2], s.weights[u][v]]
            })
            .collect::<Vec<_>>();
        if target.len() <= 33
            && c.control_points.len() <= 33
            && target.iter().all(|p| p[..3] == target[0][..3])
            && c.control_points
                .iter()
                .all(|p| p.as_slice() == &target[0][..3])
        {
            if let Some(r) = bind(c, p, s, target.clone(), Vec::new(), true, max_work)? {
                return Ok(Some(r));
            }
        }
        let lo = a[free].min(b[free]);
        let hi = a[free].max(b[free]);
        let degree = degrees[free];
        let k = knots[free];
        let n = counts[free];
        if degree != c.degree || c.domain() != [lo, hi] || lo == hi {
            continue;
        }
        if [lo, hi].iter().any(|&u| {
            u != domain[free][0]
                && u != domain[free][1]
                && k.iter().filter(|&&v| v == u).count() != degree
        }) {
            continue;
        }
        let first = (degree..n).find(|&i| k[i] == lo && k[i] < k[i + 1]);
        let last = (degree..n).find(|&i| k[i + 1] == hi && k[i] < k[i + 1]);
        let (Some(first), Some(last)) = (first, last) else {
            continue;
        };
        let mut row = target[first - degree..=last].to_vec();
        let mut basis = vec![lo; degree + 1];
        basis.extend(k.iter().copied().filter(|&u| lo < u && u < hi));
        basis.extend(vec![hi; degree + 1]);
        let backwards = if reversed {
            b[free] > a[free]
        } else {
            a[free] > b[free]
        };
        if backwards {
            row.reverse();
            let Some(reflected) = mirror(&basis, [lo, hi]) else {
                continue;
            };
            basis = reflected;
        }
        if let Some(r) = bind(c, p, s, row, basis, false, max_work)? {
            return Ok(Some(r));
        }
    }
    Ok(None)
}
#[cfg(test)]
#[path = "structured_boundary/tests.rs"]
mod tests;
