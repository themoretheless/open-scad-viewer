//! Original homogeneous normal/radial cross product. Coefficient differences
//! precede interval hulls, retaining cancellations on curved canal surfaces.
use super::*;
use crate::{curve::Curve, surface::Surface};
type Poly = Vec<Vec<I>>;
fn derivative(p: &Poly, axis: usize, uv: [[f64; 2]; 2]) -> Result<Poly> {
    let degree = if axis == 0 {
        p.len() - 1
    } else {
        p[0].len() - 1
    };
    let mut out = vec![
        vec![I::point(0.); p[0].len() - usize::from(axis == 1)];
        p.len() - usize::from(axis == 0)
    ];
    let factor = I::point(degree as f64).div(I::point(uv[axis][1]).sub(I::point(uv[axis][0]))?)?;
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            out[i][j] = p[i + usize::from(axis == 0)][j + usize::from(axis == 1)]
                .sub(p[i][j])?
                .mul(factor)?;
        }
    }
    Ok(out)
}
fn hull(p: &Poly) -> Result<I> {
    I::new(
        p.iter()
            .flatten()
            .map(|v| v.lo)
            .fold(f64::INFINITY, f64::min),
        p.iter()
            .flatten()
            .map(|v| v.hi)
            .fold(f64::NEG_INFINITY, f64::max),
    )
}
fn span(k: &[f64], p: usize, n: usize, t: [f64; 2]) -> Option<usize> {
    (p..n).find(|i| k[*i] < k[*i + 1] && k[*i] <= t[0] && t[1] <= k[*i + 1])
}
pub(super) fn upper(
    s: &Surface,
    c: &Curve,
    uv: [[f64; 2]; 2],
    j: &crate::analysis::normal_alignment::JetBounds,
    radial_lower: f64,
    work: &mut u64,
    max: u64,
) -> Result<Option<f64>> {
    if 5 * s.degree_u + c.degree > 49 || 5 * s.degree_v > 49 || radial_lower <= 0. || *work >= max {
        return Ok(None);
    }
    let n2 = squared_norm(cross(j.first[0], j.first[1])?)?;
    if n2.lo <= 0. {
        return Ok(None);
    }
    let Some(u) = span(&s.knots_u, s.degree_u, s.control_points.len(), uv[0]) else {
        return Ok(None);
    };
    let Some(v) = span(&s.knots_v, s.degree_v, s.control_points[0].len(), uv[1]) else {
        return Ok(None);
    };
    let Some(cs) = span(&c.knots, c.degree, c.control_points.len(), uv[0]) else {
        return Ok(None);
    };
    let origin = &c.control_points[cs - c.degree];
    let mut net = crate::curve_surface_composition::surface_net_on(s, [u, v], uv)?;
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let centers = crate::moving_radius::curve_net(c, cs, uv[0], origin)?;
    let sp = |k| {
        net.iter()
            .map(|r| r.iter().map(|h| h[k]).collect())
            .collect::<Poly>()
    };
    let cp = |k| centers.iter().map(|h| vec![h[k]]).collect::<Poly>();
    let sw = sp(3);
    let cw = cp(3);
    let swlo = hull(&sw)?.lo;
    let cwlo = hull(&cw)?.lo;
    if swlo <= 0. || cwlo <= 0. {
        return Ok(None);
    }
    macro_rules! product {
        ($a:expr,$b:expr) => {
            match crate::moving_radius::mul(&$a, &$b, work, max)? {
                Some(p) => p,
                None => return Ok(None),
            }
        };
    }
    let wu = derivative(&sw, 0, uv)?;
    let wv = derivative(&sw, 1, uv)?;
    let mut a = Vec::new();
    let mut b = Vec::new();
    let mut d = Vec::new();
    for k in 0..3 {
        let x = sp(k);
        a.push(crate::moving_radius::sub(
            product!(sw, derivative(&x, 0, uv)?),
            &product!(x, wu),
        )?);
        b.push(crate::moving_radius::sub(
            product!(sw, derivative(&x, 1, uv)?),
            &product!(x, wv),
        )?);
        d.push(crate::moving_radius::sub(
            product!(x, cw),
            &product!(cp(k), sw),
        )?);
    }
    let mut n = Vec::new();
    for k in 0..3 {
        let i = (k + 1) % 3;
        let l = (k + 2) % 3;
        n.push(crate::moving_radius::sub(
            product!(a[i], b[l]),
            &product!(a[l], b[i]),
        )?);
    }
    let mut numerator = I::point(0.);
    for k in 0..3 {
        let i = (k + 1) % 3;
        let l = (k + 2) % 3;
        let residual = crate::moving_radius::sub(product!(n[i], d[l]), &product!(n[l], d[i]))?;
        let bound = hull(&residual)?;
        let magnitude = I::point(bound.lo.abs().max(bound.hi.abs()));
        numerator = numerator.add(magnitude.mul(magnitude)?)?;
    }
    // A×B=W_s^4(S_u×S_v); D=W_s W_c(S-C). All denominator
    // factors are fresh ORIGINAL positive Bernstein hull lower bounds.
    let sw = I::point(swlo);
    let wc = I::point(cwlo);
    let mut normal_factor = I::point(1.);
    for _ in 0..8 {
        normal_factor = normal_factor.mul(sw)?;
    }
    let radial = I::point(radial_lower).mul(sw)?.mul(wc)?;
    let denominator = I::point(n2.lo)
        .mul(normal_factor)?
        .mul(radial.mul(radial)?)?;
    if denominator.lo <= 0. {
        return Ok(None);
    }
    Ok(Some(numerator.div(denominator)?.hi.min(1.)))
}
