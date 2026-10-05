//! Correlated Bernstein bounds for |S(u,v)-C(u)|^2-r(u)^2.
//! All original knot cells participate; no fitted surface or sample admission.
use crate::{check, curve::Curve, interval_eval::Interval as I, surface::Surface, Result};
type Poly = Vec<Vec<I>>;
pub struct Certificate {
    surface: Surface,
    centers: Curve,
    radius: Curve,
    error_upper: f64,
    residual: [f64; 2],
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn centers(&self) -> &Curve {
        &self.centers
    }
    /// Radius is coordinate zero of the original two-dimensional law curve.
    pub fn radius(&self) -> &Curve {
        &self.radius
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn squared_residual(&self) -> [f64; 2] {
        self.residual
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub squared_residual: Option<[f64; 2]>,
    pub error_upper: Option<f64>,
    pub cells: usize,
    pub work: u64,
    pub uncertain_uv: Option<[[f64; 2]; 2]>,
    pub reason: &'static str,
}
// Product degree is at most 48. These binomial integers and their u64
// recurrence intermediates are exact; ratios are formed outward below.
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}
fn mul(a: &Poly, b: &Poly, work: &mut u64, max: u64) -> Result<Option<Poly>> {
    let count = (a.len() * a[0].len() * b.len() * b[0].len()) as u64;
    if count > max - *work {
        return Ok(None);
    }
    *work += count;
    let degrees = [a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1];
    let mut out =
        vec![vec![I::point(0.); degrees[1] + degrees[3] + 1]; degrees[0] + degrees[2] + 1];
    for (i, row) in a.iter().enumerate() {
        for (j, &x) in row.iter().enumerate() {
            for (k, other) in b.iter().enumerate() {
                for (l, &y) in other.iter().enumerate() {
                    let f = I::point(choose(degrees[0], i))
                        .mul(I::point(choose(degrees[2], k)))?
                        .div(I::point(choose(degrees[0] + degrees[2], i + k)))?
                        .mul(I::point(choose(degrees[1], j)))?
                        .mul(I::point(choose(degrees[3], l)))?
                        .div(I::point(choose(degrees[1] + degrees[3], j + l)))?;
                    out[i + k][j + l] = out[i + k][j + l].add(x.mul(y)?.mul(f)?)?;
                }
            }
        }
    }
    Ok(Some(out))
}
fn sub(a: Poly, b: &Poly) -> Result<Poly> {
    a.into_iter()
        .zip(b)
        .map(|(row, other)| row.into_iter().zip(other).map(|(x, &y)| x.sub(y)).collect())
        .collect()
}
fn curve_net(c: &Curve, span: usize, domain: [f64; 2], origin: &[f64]) -> Result<Vec<Vec<I>>> {
    let raw = crate::curve_distance::restricted_controls(c, span, I::new(domain[0], domain[1])?)?;
    let d = c.control_points[0].len();
    raw.into_iter()
        .map(|mut h| {
            for k in 0..d {
                h[k] = h[k].add(
                    I::point(c.control_points[span - c.degree][k])
                        .sub(I::point(origin[k]))?
                        .mul(h[d])?,
                )?;
            }
            Ok(h)
        })
        .collect()
}
fn span(knots: &[f64], degree: usize, n: usize, d: [f64; 2]) -> usize {
    (degree..n)
        .find(|&i| knots[i] < knots[i + 1] && knots[i] <= d[0] && d[1] <= knots[i + 1])
        .unwrap()
}
fn cell(
    s: &Surface,
    c: &Curve,
    r: &Curve,
    uv: [[f64; 2]; 2],
    work: &mut u64,
    max: u64,
    reason: &mut &'static str,
) -> Result<Option<([f64; 2], f64)>> {
    let indices = [
        span(&s.knots_u, s.degree_u, s.control_points.len(), uv[0]),
        span(&s.knots_v, s.degree_v, s.control_points[0].len(), uv[1]),
    ];
    let cs = span(&c.knots, c.degree, c.control_points.len(), uv[0]);
    let rs = span(&r.knots, r.degree, r.control_points.len(), uv[0]);
    let origin = &c.control_points[cs - c.degree];
    let mut net = crate::curve_surface_composition::surface_net_on(s, indices, uv)?;
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let centers = curve_net(c, cs, uv[0], origin)?;
    let radius = curve_net(r, rs, uv[0], &[0., 0.])?;
    let sp = |k| {
        net.iter()
            .map(|row| row.iter().map(|h| h[k]).collect())
            .collect::<Poly>()
    };
    let cp = |k| centers.iter().map(|h| vec![h[k]]).collect::<Poly>();
    let rp = |k| radius.iter().map(|h| vec![h[k]]).collect::<Poly>();
    macro_rules! product {
        ($a:expr,$b:expr) => {
            match mul(&$a, &$b, work, max)? {
                Some(p) => p,
                None => return Ok(None),
            }
        };
    }
    let sc = product!(sp(3), cp(3));
    let denominator = product!(sc, rp(2));
    let denominator = product!(denominator, denominator);
    let radial = product!(sc, rp(0));
    let radial = product!(radial, radial);
    let mut residual = radial
        .iter()
        .map(|row| row.iter().map(|x| I::new(-x.hi, -x.lo)).collect())
        .collect::<Result<Poly>>()?;
    for k in 0..3 {
        let delta = sub(product!(sp(k), cp(3)), &product!(cp(k), sp(3)))?;
        let delta = product!(delta, rp(2));
        let squared = product!(delta, delta);
        for (row, other) in residual.iter_mut().zip(squared) {
            for (a, b) in row.iter_mut().zip(other) {
                *a = a.add(b)?;
            }
        }
    }
    let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
    for (row, other) in residual.iter().zip(&denominator) {
        for (a, b) in row.iter().zip(other) {
            if b.lo <= 0. {
                *reason = "moving-radius-denominator-unresolved";
                return Ok(None);
            }
            let q = a.div(*b)?;
            bounds[0] = bounds[0].min(q.lo);
            bounds[1] = bounds[1].max(q.hi);
        }
    }
    let max_residual = bounds[0].abs().max(bounds[1].abs());
    let rlo = r
        .control_points
        .iter()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    let mut error = max_residual.sqrt().next_up();
    if rlo > 0. {
        error = error.min(I::point(max_residual).div(I::point(rlo))?.hi);
    }
    Ok(Some((bounds, error)))
}
/// Radius-law coordinate zero is nonnegative; all source U domains must agree.
/// Only complete coverage with a distance error <= tolerance admits a certificate.
/// This is a radius relation, not normal alignment, curvature, or regularity.
pub fn qualify(
    s: &Surface,
    c: &Curve,
    r: &Curve,
    tolerance: f64,
    max_cells: usize,
    max_work: u64,
) -> Result<Report> {
    s.validate()?;
    c.validate()?;
    r.validate()?;
    let du = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
    check(
        s.control_points[0][0].len() == 3
            && c.control_points[0].len() == 3
            && r.control_points[0].len() == 2
            && !s.periodic_u
            && !s.periodic_v
            && !c.periodic
            && !r.periodic
            && du == c.domain()
            && du == r.domain()
            && r.control_points.iter().all(|p| p[0] >= 0.)
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells)
            && (1..=100_000_000).contains(&max_work),
        "Choose original charts and nonnegative radius law on the same U domain with bounded work",
    )?;
    let mut out = Report {
        certificate: None,
        squared_residual: None,
        error_upper: None,
        cells: 0,
        work: 0,
        uncertain_uv: None,
        reason: "moving-radius-degree-unproven",
    };
    if [s.degree_u, s.degree_v, c.degree, r.degree]
        .into_iter()
        .any(|d| d > 8)
    {
        return Ok(out);
    }
    let mut cuts = s
        .knots_u
        .iter()
        .chain(&c.knots)
        .chain(&r.knots)
        .copied()
        .filter(|&t| du[0] <= t && t <= du[1])
        .collect::<Vec<_>>();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut bounds = [f64::INFINITY, f64::NEG_INFINITY];
    let mut error = 0_f64;
    let mut worst_uv = None;
    for u in cuts.windows(2) {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] >= s.knots_v[v + 1] {
                continue;
            }
            let uv = [[u[0], u[1]], [s.knots_v[v], s.knots_v[v + 1]]];
            out.uncertain_uv = Some(uv);
            out.reason = "moving-radius-work-limit";
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let Some((b, e)) = cell(s, c, r, uv, &mut out.work, max_work, &mut out.reason)? else {
                return Ok(out);
            };
            bounds[0] = bounds[0].min(b[0]);
            bounds[1] = bounds[1].max(b[1]);
            if e > error {
                worst_uv = Some(uv);
            }
            error = error.max(e);
        }
    }
    out.squared_residual = Some(bounds);
    out.error_upper = Some(error);
    if error <= tolerance {
        out.certificate = Some(Certificate {
            surface: s.clone(),
            centers: c.clone(),
            radius: r.clone(),
            error_upper: error,
            residual: bounds,
        });
        out.uncertain_uv = None;
        out.reason = "moving-radius-qualified";
    } else {
        out.reason = "moving-radius-error-unqualified";
        out.uncertain_uv = worst_uv;
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tube() -> (Surface, Curve, Curve) {
        let weight = 0.5_f64.sqrt();
        let s = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![2., 2., 3., 4., 4.],
            knots_v: vec![5., 5., 5., 9., 9., 9.],
            control_points: [0., 1., 2.]
                .into_iter()
                .map(|z| vec![vec![1., 0., z], vec![1., 1., z], vec![0., 1., z]])
                .collect(),
            weights: vec![vec![1., weight, 1.]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let c = Curve {
            degree: 1,
            knots: vec![2., 2., 2.5, 4., 4.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 0.5], vec![0., 0., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let r = Curve {
            degree: 1,
            knots: vec![2., 2., 3.5, 4., 4.],
            control_points: vec![vec![1., 0.]; 3],
            weights: vec![1.; 3],
            periodic: false,
        };
        (s, c, r)
    }
    #[test]
    fn all_original_knots_and_independent_law_partitions_are_covered() {
        let (s, c, r) = tube();
        let out = qualify(&s, &c, &r, 1e-9, 4, 100_000_000).unwrap();
        assert!(
            out.certificate.is_some(),
            "{} {:?}",
            out.reason,
            out.error_upper
        );
        assert_eq!(out.cells, 4);
        let cert = out.certificate.unwrap();
        assert_eq!(cert.surface(), &s);
        assert_eq!(cert.centers(), &c);
        assert_eq!(cert.radius(), &r);
        // Independent point checks are regressions; the certificate authority
        // is the complete correlated original Bernstein enclosure.
        for u in [2., 2.2, 2.5, 3., 3.7, 4.] {
            for v in [5., 6., 8., 9.] {
                let p = s.evaluate(u, v).unwrap().point;
                let center = c.evaluate(u).unwrap().point;
                let d = (0..3)
                    .map(|k| (p[k] - center[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!((d - 1.).abs() <= cert.error_upper());
            }
        }
        let stopped = qualify(&s, &c, &r, 1e-9, 3, 100_000_000).unwrap();
        assert!(stopped.certificate.is_none());
        assert!(stopped.squared_residual.is_none());
        assert!(stopped.uncertain_uv.is_some());
        let stopped = qualify(&s, &c, &r, 1e-9, 4, 1).unwrap();
        assert!(stopped.certificate.is_none());
        assert!(stopped.work <= 1);
        let mut displaced = c.clone();
        for p in &mut displaced.control_points {
            p[0] = 0.1;
        }
        let fail = qualify(&s, &displaced, &r, 1e-9, 4, 100_000_000).unwrap();
        assert!(fail.certificate.is_none());
        assert_eq!(fail.reason, "moving-radius-error-unqualified");
    }
    #[test]
    fn invalid_radius_and_mismatched_parameter_domains_cannot_be_certified() {
        let (s, c, mut r) = tube();
        r.control_points[1][0] = -1.;
        assert!(qualify(&s, &c, &r, 1e-6, 4, 10000).is_err());
        let (_, mut c, r) = tube();
        c.knots = vec![1., 1., 2., 4., 4.];
        assert!(qualify(&s, &c, &r, 1e-6, 4, 10000).is_err());
    }
}
