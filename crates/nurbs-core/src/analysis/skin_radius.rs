//! Whole-patch radius-law residual for original rational Bernstein coefficients.
//! Correspondence is normalized station parameter, not nearest spine distance.
use crate::{
    Error, Result, check,
    curve::Curve,
    interval_eval::Interval as I,
    surface::{Axis, Surface},
};
type Poly = Vec<Vec<I>>;
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn charge(&mut self, n: usize) -> Result<()> {
        if n > self.max.saturating_sub(self.used) {
            return Err(Error::new(
                "SKIN_RADIUS_WORK_LIMIT",
                "Skin radius coefficient budget exhausted",
            ));
        }
        self.used += n;
        Ok(())
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut value = 1u64;
    for j in 0..k.min(n - k) {
        value = value * (n - j) as u64 / (j + 1) as u64;
    }
    value as f64
}
fn mul(a: &Poly, b: &Poly, w: &mut Work) -> Result<Poly> {
    let (p, q, r, s) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    let mut out = vec![vec![I::point(0.); q + s + 1]; p + r + 1];
    for i in 0..=p {
        for j in 0..=q {
            for k in 0..=r {
                for l in 0..=s {
                    w.charge(9)?;
                    let f = I::point(choose(p, i))
                        .mul(I::point(choose(r, k)))?
                        .div(I::point(choose(p + r, i + k)))?
                        .mul(I::point(choose(q, j)))?
                        .mul(I::point(choose(s, l)))?
                        .div(I::point(choose(q + s, j + l)))?;
                    out[i + k][j + l] = out[i + k][j + l].add(a[i][j].mul(b[k][l])?.mul(f)?)?;
                }
            }
        }
    }
    Ok(out)
}
fn combine(a: &Poly, b: &Poly, subtract: bool, w: &mut Work) -> Result<Poly> {
    check(
        a.len() == b.len() && a[0].len() == b[0].len(),
        "Radius polynomial basis mismatch",
    )?;
    a.iter()
        .zip(b)
        .map(|(a, b)| {
            a.iter()
                .zip(b)
                .map(|(a, b)| {
                    w.charge(1)?;
                    if subtract { a.sub(*b) } else { a.add(*b) }
                })
                .collect()
        })
        .collect()
}
fn hull(p: &Poly) -> Result<I> {
    I::new(
        p.iter()
            .flatten()
            .map(|x| x.lo)
            .fold(f64::INFINITY, f64::min),
        p.iter()
            .flatten()
            .map(|x| x.hi)
            .fold(f64::NEG_INFINITY, f64::max),
    )
}
fn bezier(k: &[f64], p: usize, n: usize) -> bool {
    p <= 8
        && n == p + 1
        && k[..=p].iter().all(|x| *x == k[p])
        && k[p + 1..].iter().all(|x| *x == k[p + 1])
}
pub struct Report {
    /// None is incomplete proof, never a radius certificate.
    pub radial_error_upper_mm: Option<f64>,
    pub within_tolerance: bool,
    pub coefficient_work: usize,
}
/// Bound |norm(S(u,v)-C(t))-r(t)| over the WHOLE patch. `t` is the normalized
/// selected station axis; C and r use their own complete normalized domains.
/// Radius is encoded [r,0,0]. Only positive single-Bezier rational charts are
/// currently supported. This is not a curvature-radius, nearest-distance,
/// contact, regularity, embedding or solid certificate.
pub fn inspect(
    s: &Surface,
    center: &Curve,
    radius: &Curve,
    axis: Axis,
    tolerance_mm: f64,
    max_work: usize,
) -> Result<Report> {
    s.validate()?;
    center.validate()?;
    radius.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && !center.periodic
            && !radius.periodic
            && bezier(&s.knots_u, s.degree_u, s.control_points.len())
            && bezier(&s.knots_v, s.degree_v, s.control_points[0].len())
            && bezier(&center.knots, center.degree, center.control_points.len())
            && bezier(&radius.knots, radius.degree, radius.control_points.len())
            && s.control_points[0][0].len() == 3
            && center.control_points[0].len() == 3
            && radius.control_points[0].len() == 3
            && s.weights
                .iter()
                .flatten()
                .chain(center.weights.iter())
                .chain(radius.weights.iter())
                .all(|w| *w > 0.)
            && radius
                .control_points
                .iter()
                .all(|p| p[0] > 0. && p[1] == 0. && p[2] == 0.)
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=2000000).contains(&max_work),
        "Need positive bounded rational Bezier skin, center and radius charts",
    )?;
    let mut work = Work {
        used: 0,
        max: max_work,
    };
    let result = (|| -> Result<f64> {
        let lift_curve = |c: &Curve, k: usize, w: &mut Work| -> Result<Poly> {
            let mut values = Vec::new();
            for i in 0..c.control_points.len() {
                w.charge(1)?;
                values.push(if k == 3 {
                    I::point(c.weights[i])
                } else {
                    I::point(c.control_points[i][k]).mul(I::point(c.weights[i]))?
                });
            }
            Ok(match axis {
                Axis::U => values.into_iter().map(|v| vec![v]).collect(),
                Axis::V => vec![values],
            })
        };
        let mut skin = Vec::new();
        let mut spine = Vec::new();
        for k in 0..4 {
            let mut net =
                vec![vec![I::point(0.); s.control_points[0].len()]; s.control_points.len()];
            for i in 0..net.len() {
                for j in 0..net[0].len() {
                    work.charge(1)?;
                    net[i][j] = if k == 3 {
                        I::point(s.weights[i][j])
                    } else {
                        I::point(s.control_points[i][j][k]).mul(I::point(s.weights[i][j]))?
                    };
                }
            }
            skin.push(net);
            spine.push(lift_curve(center, k, &mut work)?);
        }
        let denominator = mul(&skin[3], &spine[3], &mut work)?;
        let mut squared = None;
        for k in 0..3 {
            let a = mul(&skin[k], &spine[3], &mut work)?;
            let b = mul(&spine[k], &skin[3], &mut work)?;
            let d = combine(&a, &b, true, &mut work)?;
            let q = mul(&d, &d, &mut work)?;
            squared = Some(if let Some(old) = squared {
                combine(&old, &q, false, &mut work)?
            } else {
                q
            });
        }
        let rw = lift_curve(radius, 3, &mut work)?;
        let rh = lift_curve(radius, 0, &mut work)?;
        let den2 = mul(&denominator, &denominator, &mut work)?;
        let rw2 = mul(&rw, &rw, &mut work)?;
        let rh2 = mul(&rh, &rh, &mut work)?;
        let a = mul(&squared.unwrap(), &rw2, &mut work)?;
        let b = mul(&rh2, &den2, &mut work)?;
        let residual = combine(&a, &b, true, &mut work)?;
        let den = mul(&den2, &rw2, &mut work)?;
        work.charge(3)?;
        let e = hull(&residual)?.div(hull(&den)?)?;
        let absolute = e.lo.abs().max(e.hi.abs());
        let rmin = radius
            .control_points
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min);
        Ok(I::point(absolute).div(I::point(rmin))?.hi)
    })();
    match result {
        Ok(upper) => Ok(Report {
            radial_error_upper_mm: Some(upper),
            within_tolerance: upper <= tolerance_mm,
            coefficient_work: work.used,
        }),
        Err(e) if e.code == "SKIN_RADIUS_WORK_LIMIT" => Ok(Report {
            radial_error_upper_mm: None,
            within_tolerance: false,
            coefficient_work: work.used,
        }),
        Err(e) => Err(e),
    }
}

/// Whole multi-span skin qualification against affine world-coordinate fields:
/// C(P)[k]=sum(center[k][j]*P[j])+center[k][3], r(P)=sum(radius[j]*P[j])+radius[3].
/// This does not establish that C is a contact-ball center or nearest point.
/// All original nonempty knot cells are covered; no rounded Bezier proxy is used.
pub fn inspect_affine_field(
    s: &Surface,
    center: [[f64; 4]; 3],
    radius: [f64; 4],
    tolerance_mm: f64,
    max_work: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && s.degree_u <= 8
            && s.degree_v <= 8
            && s.control_points[0][0].len() == 3
            && s.control_points.len() * s.control_points[0].len() <= 4096
            && s.weights.iter().flatten().all(|w| *w > 0.)
            && center
                .iter()
                .flatten()
                .chain(radius.iter())
                .all(|x| x.is_finite())
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=2000000).contains(&max_work),
        "Need finite bounded original skin and affine radius fields",
    )?;
    let mut work = Work {
        used: 0,
        max: max_work,
    };
    let result = (|| -> Result<f64> {
        let mut maximum = 0_f64;
        for u in s.degree_u..s.control_points.len() {
            for v in s.degree_v..s.control_points[0].len() {
                let domain = [
                    [s.knots_u[u], s.knots_u[u + 1]],
                    [s.knots_v[v], s.knots_v[v + 1]],
                ];
                if domain.iter().any(|d| d[0] >= d[1]) {
                    continue;
                }
                // Conservative debit for original blossom plans, homogeneous
                // products and both interpolation passes, before extraction.
                work.charge(
                    32 * (s.degree_u + 1).pow(2)
                        * (s.degree_v + 1).pow(2)
                        * (s.degree_u + s.degree_v + 2),
                )?;
                let net = crate::surface_measure::jets::original_span_net(s, [u, v], domain)?;
                let mut weight = vec![vec![I::point(0.); s.degree_v + 1]; s.degree_u + 1];
                let mut radial = weight.clone();
                let mut delta = vec![weight.clone(); 3];
                for i in 0..net.len() {
                    for j in 0..net[0].len() {
                        let h = net[i][j];
                        weight[i][j] = h[3];
                        let field = |coefficients: [f64; 4], work: &mut Work| -> Result<I> {
                            let mut sum = I::point(0.);
                            for k in 0..4 {
                                work.charge(2)?;
                                sum = sum.add(h[k].mul(I::point(coefficients[k]))?)?;
                            }
                            Ok(sum)
                        };
                        radial[i][j] = field(radius, &mut work)?;
                        for k in 0..3 {
                            let c = field(center[k], &mut work)?;
                            work.charge(1)?;
                            delta[k][i][j] = h[k].sub(c)?;
                        }
                    }
                }
                work.charge(2)?;
                let r = hull(&radial)?.div(hull(&weight)?)?;
                if r.lo <= 0. {
                    return Err(Error::new(
                        "SKIN_RADIUS_POSITIVITY_UNPROVEN",
                        "Affine field radius is not proved positive on an original cell",
                    ));
                }
                let mut squared = mul(&delta[0], &delta[0], &mut work)?;
                for d in &delta[1..] {
                    let q = mul(d, d, &mut work)?;
                    squared = combine(&squared, &q, false, &mut work)?;
                }
                let rr = mul(&radial, &radial, &mut work)?;
                let residual = combine(&squared, &rr, true, &mut work)?;
                let ww = mul(&weight, &weight, &mut work)?;
                work.charge(3)?;
                let e = hull(&residual)?.div(hull(&ww)?)?;
                let bound = I::point(e.lo.abs().max(e.hi.abs())).div(I::point(r.lo))?.hi;
                maximum = maximum.max(bound);
            }
        }
        Ok(maximum)
    })();
    match result {
        Ok(upper) => Ok(Report {
            radial_error_upper_mm: Some(upper),
            within_tolerance: upper <= tolerance_mm,
            coefficient_work: work.used,
        }),
        Err(e)
            if e.code == "SKIN_RADIUS_WORK_LIMIT"
                || e.code == "SKIN_RADIUS_POSITIVITY_UNPROVEN" =>
        {
            Ok(Report {
                radial_error_upper_mm: None,
                within_tolerance: false,
                coefficient_work: work.used,
            })
        }
        Err(e) => Err(e),
    }
}
