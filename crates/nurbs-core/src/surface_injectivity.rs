//! Sufficient global injectivity certificate for a complete NURBS chart.
//! If a fixed projection F and constant Y satisfy ||I-Y DF||_inf < 1 over
//! the entire convex parameter rectangle, F(x)=F(y) implies x=y. Every knot
//! rectangle contributes to the same Jacobian hull; local success is not enough.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
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
fn jacobian(s: &Surface, span: [usize; 2]) -> Result<[[I; 2]; 3]> {
    section_jacobian(
        s,
        span,
        [
            [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
            [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
        ],
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
    let mut net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
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
    let values: [I; 4] = std::array::from_fn(|k| hull(net.iter().flatten().map(|h| h[k])));
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if domain[axis][0] == domain[axis][1] {
            continue;
        }
        let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
        let mut diffs = Vec::new();
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
        max_spans > 0 && max_spans <= 100_000,
        "Injectivity span budget must be in 1..100000",
    )?;
    let mut report = Report {
        proven: false,
        projection: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    if s.periodic_u || s.periodic_v {
        report.reason = "periodic-domain";
        return Ok(report);
    }
    let mut j = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_u[u] >= s.knots_u[u + 1] || s.knots_v[v] >= s.knots_v[v + 1] {
                continue;
            }
            if report.spans == max_spans {
                report.reason = "work-limit";
                return Ok(report);
            }
            let next = jacobian(s, [u, v])?;
            report.spans += 1;
            for k in 0..3 {
                for axis in 0..2 {
                    j[k][axis] = hull([j[k][axis], next[k][axis]].into_iter());
                }
            }
        }
    }
    for axes in [[0, 1], [0, 2], [1, 2]] {
        if let Some(q) = contraction([j[axes[0]], j[axes[1]]])? {
            if q < 1. {
                report.proven = true;
                report.projection = Some(axes);
                report.contraction_upper = Some(q);
                report.reason = "global-projection-contraction";
                return Ok(report);
            }
        }
    }
    Ok(report)
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
