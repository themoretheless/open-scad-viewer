//! A leading conic fan and a quantitative small-neighborhood quotient.
use super::*;
pub(super) struct Fan {
    pub angle: f64,
    pub radial: f64,
    pub remainder: f64,
}
pub(super) fn calculate(
    p: &prepared::Prepared,
    v: [f64; 2],
    j: &jets::Jets,
    sign: i8,
) -> Result<Option<Fan>> {
    let zero = jets::evaluate_homogeneous_jets(&p.net, [I::point(0.), I::new(v[0], v[1])?], 2)?;
    let g = [zero[0][0][0], zero[0][0][1]];
    let gv = [zero[0][1][0], zero[0][1][1]];
    let determinant = g[0].mul(gv[1])?.sub(g[1].mul(gv[0])?)?;
    if !(if sign > 0 {
        determinant.lo > 0.
    } else {
        determinant.hi < 0.
    }) {
        return Ok(None);
    }
    let radial = box_distance(&g, &[I::point(0.); 2])?.0;
    if radial <= 0. {
        return Ok(None);
    }
    let angular = I::point(determinant.lo.abs().max(determinant.hi.abs()))
        .div(I::point(radial).mul(I::point(radial))?)?
        .mul(I::point(v[1]).sub(I::point(v[0]))?)?
        .hi;
    let neg = |x: I| I::new(-x.hi, -x.lo);
    let inverse = [
        [
            gv[1].div_signed(determinant)?,
            neg(gv[0])?.div_signed(determinant)?,
        ],
        [
            neg(g[1])?.div_signed(determinant)?,
            g[0].div_signed(determinant)?,
        ],
    ];
    // (F-F0)_W = u * [2 G_u, G_uv] [G0,G0_v]^-1, with the first
    // column enclosing the mean-value average plus endpoint derivative.
    let mut bound = [I::point(0.); 4];
    for a in 0..2 {
        for b in 0..2 {
            bound[2 * a + b] = I::point(2.)
                .mul(j[1][0][a])?
                .mul(inverse[0][b])?
                .add(j[1][1][a].mul(inverse[1][b])?)?;
        }
    }
    let remainder = box_distance(&bound, &[I::point(0.); 4])?.1;
    Ok(Some(Fan {
        angle: angular,
        radial,
        remainder,
    }))
}
