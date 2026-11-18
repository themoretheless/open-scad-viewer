//! Shared-clock ORIGINAL normal polynomials on original Bezier charts.
//! Complete spatial lift remains independently mandatory in the public owner.
use super::*;
use crate::curve_surface_composition as composition;
type Poly = Vec<I>;
struct Budget<'a> {
    work: &'a mut u64,
    max: u64,
}
impl Budget<'_> {
    fn charge(&mut self, n: usize) -> bool {
        let n = n as u64;
        if n > self.max.saturating_sub(*self.work) {
            false
        } else {
            *self.work += n;
            true
        }
    }
    fn mul(&mut self, a: &Poly, b: &Poly) -> Result<Option<Poly>> {
        if !self.charge(a.len() * b.len()) {
            return Ok(None);
        }
        Ok(Some(composition::mul(a, b)?))
    }
    fn lift(
        &mut self,
        h: &[Poly],
        net: &[Vec<[I; 4]>],
        degree: [usize; 2],
        domain: [[f64; 2]; 2],
    ) -> Result<Option<Vec<Poly>>> {
        let d = h[0].len() - 1;
        let [p, q] = degree;
        let mut cost = (p + 1) * (q + 1) * (p * d + 1) * (q * d + 1);
        for n in degree {
            cost += 2 * (0..n).map(|i| (i * d + 1) * (d + 1)).sum::<usize>();
            cost += (0..=n)
                .map(|i| (i * d + 1) * ((n - i) * d + 1))
                .sum::<usize>();
        }
        if !self.charge(cost) {
            return Ok(None);
        }
        Ok(Some(composition::lift_coefficients(
            h, net, degree, domain,
        )?))
    }
}
fn sub(a: Poly, b: Poly) -> Result<Poly> {
    a.into_iter().zip(b).map(|(a, b)| a.sub(b)).collect()
}
fn hull(p: &Poly) -> Result<I> {
    I::new(
        p.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min),
        p.iter().map(|v| v.hi).fold(f64::NEG_INFINITY, f64::max),
    )
}
fn lower_squared(n: &[Poly]) -> Result<f64> {
    let mut sum = I::point(0.);
    for p in n {
        let a = hull(p)?;
        let x = if a.lo > 0. {
            a.lo
        } else if a.hi < 0. {
            -a.hi
        } else {
            0.
        };
        sum = sum.add(I::point(x).mul(I::point(x))?)?;
    }
    Ok(sum.lo.max(0.))
}
fn normal(
    p: &Curve,
    s: &Surface,
    reverse: bool,
    t: [f64; 2],
    b: &mut Budget<'_>,
) -> Result<Option<Vec<Poly>>> {
    if p.control_points.len() != p.degree + 1
        || s.control_points.len() != s.degree_u + 1
        || s.control_points[0].len() != s.degree_v + 1
        || p.degree > 4
        || (4 * (s.degree_u + s.degree_v) - 2) * p.degree > 64
    {
        return Ok(None);
    }
    let pd = p.domain();
    let map = |t: f64| -> Result<I> {
        let t = if reverse { 1. - t } else { t };
        if t == 0. {
            return Ok(I::point(pd[0]));
        }
        if t == 1. {
            return Ok(I::point(pd[1]));
        }
        I::point(pd[0])
            .add(I::point(pd[1]).sub(I::point(pd[0]))?.mul(I::point(t))?)?
            .intersect(pd[0], pd[1])
    };
    let Some(h) = composition::restricted_ends(p, [map(t[0])?, map(t[1])?])? else {
        return Ok(None);
    };
    if hull(&h[2])?.lo <= 0. {
        return Ok(None);
    }
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let degree = [s.degree_u, s.degree_v];
    let net = composition::surface_net_on_centered(
        s,
        degree,
        domain,
        [
            s.control_points[0][0][0],
            s.control_points[0][0][1],
            s.control_points[0][0][2],
        ],
    )?;
    let du = crate::analysis::normal_alignment::differentiate_net(&net, degree, 0, domain)?;
    let dv = crate::analysis::normal_alignment::differentiate_net(&net, degree, 1, domain)?;
    let Some(x) = b.lift(&h, &net, degree, domain)? else {
        return Ok(None);
    };
    let Some(u) = b.lift(&h, &du, [degree[0] - 1, degree[1]], domain)? else {
        return Ok(None);
    };
    let Some(v) = b.lift(&h, &dv, [degree[0], degree[1] - 1], domain)? else {
        return Ok(None);
    };
    macro_rules! product {
        ($a:expr,$c:expr) => {
            match b.mul(&$a, &$c)? {
                Some(p) => p,
                None => return Ok(None),
            }
        };
    }
    let mut a = Vec::new();
    let mut c = Vec::new();
    for k in 0..3 {
        a.push(sub(product!(x[3], u[k]), product!(x[k], u[3]))?);
        c.push(sub(product!(x[3], v[k]), product!(x[k], v[3]))?);
    }
    let mut out = Vec::new();
    for k in 0..3 {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        out.push(sub(product!(a[i], c[j]), product!(a[j], c[i]))?);
    }
    Ok(Some(out))
}
pub(super) fn bounds(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut u64,
    max: u64,
) -> Result<Option<[f64; 2]>> {
    let mut b = Budget { work, max };
    let Some(a) = normal(p[0], s[0], reverse[0], t, &mut b)? else {
        return Ok(None);
    };
    let Some(c) = normal(p[1], s[1], reverse[1], t, &mut b)? else {
        return Ok(None);
    };
    let denominator = I::point(lower_squared(&a)?).mul(I::point(lower_squared(&c)?))?;
    if denominator.lo <= 0. {
        return Ok(None);
    }
    macro_rules! product {
        ($a:expr,$c:expr) => {
            match b.mul(&$a, &$c)? {
                Some(p) => p,
                None => return Ok(None),
            }
        };
    }
    let mut numerator = I::point(0.);
    let mut scalar = None::<Poly>;
    for k in 0..3 {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        let cross = hull(&sub(product!(a[i], c[j]), product!(a[j], c[i]))?)?;
        let m = I::point(cross.lo.abs().max(cross.hi.abs()));
        numerator = numerator.add(m.mul(m)?)?;
        let term = product!(a[k], c[k]);
        scalar = Some(if let Some(old) = scalar {
            old.into_iter()
                .zip(term)
                .map(|(a, b)| a.add(b))
                .collect::<Result<Poly>>()?
        } else {
            term
        });
    }
    let dot = hull(&scalar.unwrap())?;
    let branch = if dot.lo > 0. {
        0
    } else if dot.hi < 0. {
        1
    } else {
        return Ok(None);
    };
    let sine = numerator.div(denominator)?.hi.min(1.);
    let cosine = I::point(1.)
        .sub(I::point(sine))?
        .lo
        .max(0.)
        .sqrt()
        .next_down()
        .max(0.);
    let chord_squared = I::point(2.)
        .mul(I::point(sine))?
        .div(I::point(1.).add(I::point(cosine))?)?;
    let mut out = [2.; 2];
    out[branch] = chord_squared.hi.sqrt().next_up().min(2.);
    Ok(Some(out))
}
