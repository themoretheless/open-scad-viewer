use crate::numerics::interval_vec3::{add, cross, div, dot, norm, scale, sub};
use crate::{Result, distance_bounds::Interval as I, surface::Surface, surface_measure::jets};
pub(super) struct Proof {
    pub lower: f64,
    pub upper: f64,
    pub center: [f64; 2],
    pub fold_free: bool,
    pub candidate_regular: bool,
}
fn widths(s: &Surface, span: [usize; 2]) -> Result<[I; 2]> {
    Ok([
        I::point(s.knots_u[span[0] + 1]).sub(I::point(s.knots_u[span[0]]))?,
        I::point(s.knots_v[span[1] + 1]).sub(I::point(s.knots_v[span[1]]))?,
    ])
}
fn unit(j: &jets::Jets) -> Result<Option<([I; 3], I)>> {
    let c = cross(j[1][0], j[0][1])?;
    let length = norm(c)?;
    if length.lo <= 0. {
        return Ok(None);
    }
    let mut n = div(c, length)?;
    for x in &mut n {
        *x = x.intersect(-1., 1.)?;
    }
    Ok(Some((n, length)))
}
fn position_delta(
    s: &Surface,
    c: &Surface,
    si: [usize; 2],
    ci: [usize; 2],
    sj: [I; 3],
    cj: [I; 3],
) -> Result<[I; 3]> {
    let so = &s.control_points[si[0] - s.degree_u][si[1] - s.degree_v];
    let co = &c.control_points[ci[0] - c.degree_u][ci[1] - c.degree_v];
    let mut out = sub(cj, sj)?;
    for k in 0..3 {
        out[k] = out[k].add(I::point(co[k]).sub(I::point(so[k]))?)?;
    }
    Ok(out)
}
// Positive definiteness of I-d*shape along the normal (eigenvalue 1) and
// both tangent directions proves no focal crossing for the entire segment
// from zero to the requested signed offset. It is a local chart statement.
fn fold_free(j: &jets::Jets, d: f64) -> Result<bool> {
    let Some(shape) = crate::surface_shape_operator::from_jets(j)? else {
        return Ok(false);
    };
    let mut a = [[I::point(0.); 3]; 3];
    for i in 0..3 {
        for k in 0..3 {
            a[i][k] =
                I::point(if i == k { 1. } else { 0. }).sub(shape.tensor[i][k].mul(I::point(d))?)?;
        }
    }
    if a[0][0].lo <= 0. {
        return Ok(false);
    }
    let p = a[1][1].sub(a[1][0].mul(a[0][1])?.div(a[0][0])?)?;
    if p.lo <= 0. {
        return Ok(false);
    }
    let x = a[2][1].sub(a[2][0].mul(a[0][1])?.div(a[0][0])?)?;
    let y = a[1][2].sub(a[1][0].mul(a[0][2])?.div(a[0][0])?)?;
    Ok(a[2][2]
        .sub(a[2][0].mul(a[0][2])?.div(a[0][0])?)?
        .sub(x.mul(y)?.div(p)?)?
        .lo
        > 0.)
}
pub(super) fn inspect(
    s: &Surface,
    c: &Surface,
    si: [usize; 2],
    ci: [usize; 2],
    rect: [[f64; 2]; 2],
    d: f64,
) -> Result<Option<Proof>> {
    let center = rect.map(|v| v[0] * 0.5 + v[1] * 0.5);
    let point = center.map(|v| [v, v]);
    let js = jets::calculate_partial_stable_up_to(s, si, rect, [None; 2], 2)?;
    let Some((n, length)) = unit(&js)? else {
        return Ok(None);
    };
    let jc = jets::calculate_first_stable(c, ci, rect)?;
    let ps = jets::calculate_partial_stable_up_to(s, si, point, [None; 2], 1)?;
    let Some((nc, _)) = unit(&ps)? else {
        return Ok(None);
    };
    let pc = jets::calculate_first_stable(c, ci, point)?;
    let residual = sub(
        position_delta(s, c, si, ci, ps[0][0], pc[0])?,
        scale(nc, I::point(d))?,
    )?;
    let error = norm(residual)?;
    let sw = widths(s, si)?;
    let cw = widths(c, ci)?;
    let mut upper = I::point(error.hi);
    for axis in 0..2 {
        let normal_derivative = if axis == 0 {
            add(cross(js[2][0], js[0][1])?, cross(js[1][0], js[1][1])?)?
        } else {
            add(cross(js[1][1], js[0][1])?, cross(js[1][0], js[0][2])?)?
        };
        let dn = div(
            sub(normal_derivative, scale(n, dot(n, normal_derivative)?)?)?,
            length,
        )?;
        let source = if axis == 0 { js[1][0] } else { js[0][1] };
        let derivative = sub(
            sub(div(jc[axis + 1], cw[axis])?, div(source, sw[axis])?)?,
            scale(div(dn, sw[axis])?, I::point(d))?,
        )?;
        let radius = I::point(rect[axis][0])
            .sub(I::point(center[axis]))?
            .lo
            .abs()
            .max(
                I::point(rect[axis][1])
                    .sub(I::point(center[axis]))?
                    .hi
                    .abs(),
            );
        upper = upper.add(I::point(norm(derivative)?.hi).mul(I::point(radius))?)?;
    }
    let candidate_regular = dot(cross(jc[1], jc[2])?, n)?.lo > 0.;
    Ok(Some(Proof {
        lower: error.lo,
        upper: upper.hi,
        center,
        fold_free: fold_free(&js, d)?,
        candidate_regular,
    }))
}
