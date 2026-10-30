use super::*;
pub(super) struct Bounds {
    pub normal: f64,
    pub tensor: f64,
    pub sign: i8,
    pub fan: fan::Fan,
}
pub(super) fn calculate(p: &prepared::Prepared, u: f64, v: [f64; 2]) -> Result<Option<Bounds>> {
    let up = I::new(0., u)?;
    let u2 = up.mul(up)?.intersect(0., 1.)?;
    let j = jets::evaluate_homogeneous_jets(&p.net, [up, I::new(v[0], v[1])?], 2)?;
    let h = j[0][0][2];
    let hu = j[1][0][2];
    let hv = j[0][1][2];
    let huu = j[2][0][2];
    let huv = j[1][1][2];
    let hvv = j[0][2][2];
    let mut a = [I::point(0.); 3];
    let mut b = a;
    for k in 0..2 {
        a[k] = j[0][0][k].add(up.mul(j[1][0][k])?)?;
        b[k] = j[0][1][k];
    }
    let az = I::point(3.).mul(h)?.add(up.mul(hu)?)?;
    a[2] = u2.mul(az)?;
    b[2] = u2.mul(hv)?;
    let determinant = a[0].mul(b[1])?.sub(a[1].mul(b[0])?)?;
    let (sign, lower) = if determinant.lo > 0. {
        (1, determinant.lo)
    } else if determinant.hi < 0. {
        (-1, -determinant.hi)
    } else {
        return Ok(None);
    };
    let xy = [
        a[1].mul(hv)?.sub(az.mul(b[1])?)?,
        az.mul(b[0])?.sub(a[0].mul(hv)?)?,
    ];
    let n = [u2.mul(xy[0])?, u2.mul(xy[1])?, determinant];
    let area = box_distance(&n, &[I::point(0.); 3])?;
    if area.0 <= 0. {
        return Ok(None);
    }
    let area = I::new(area.0, area.1)?;
    let normal = n.map(|x| x.div(area));
    let normal = [normal[0].clone()?, normal[1].clone()?, normal[2].clone()?];
    let dotxy = |x: [I; 2]| xy[0].mul(x[0])?.add(xy[1].mul(x[1])?);
    let au = [
        I::point(2.).mul(j[1][0][0])?.add(up.mul(j[2][0][0])?)?,
        I::point(2.).mul(j[1][0][1])?.add(up.mul(j[2][0][1])?)?,
    ];
    let uu_height = I::point(6.)
        .mul(h)?
        .add(I::point(6.).mul(up)?.mul(hu)?)?
        .add(u2.mul(huu)?)?;
    let uu = up
        .mul(dotxy(au)?)?
        .add(determinant.mul(uu_height)?)?
        .div(area)?;
    let uv = up
        .mul(dotxy([j[1][1][0], j[1][1][1]])?)?
        .add(determinant.mul(I::point(2.).mul(hv)?.add(up.mul(huv)?)?)?)?
        .div(area)?;
    let vv = dotxy([j[0][2][0], j[0][2][1]])?
        .add(determinant.mul(hvv)?)?
        .div(area)?;
    let mut synthetic = [[[I::point(0.); 3]; 4]; 4];
    for k in 0..3 {
        synthetic[1][0][p.axes[k]] = a[k];
        synthetic[0][1][p.axes[k]] = b[k];
        synthetic[2][0][p.axes[k]] = normal[k].mul(uu)?;
        synthetic[1][1][p.axes[k]] = normal[k].mul(uv)?;
        synthetic[0][2][p.axes[k]] = normal[k].mul(vv)?;
    }
    let Some(shape) = super::super::from_jets(&synthetic)? else {
        return Ok(None);
    };
    let tensor: Vec<I> = shape.tensor.into_iter().flatten().collect();
    let tensor = box_distance(&tensor, &[I::point(0.); 9])?.1;
    let x = xy[0].lo.abs().max(xy[0].hi.abs());
    let y = xy[1].lo.abs().max(xy[1].hi.abs());
    let denom = I::point(lower);
    let z = I::point(u)
        .mul(I::point(u))?
        .mul(
            I::point(x)
                .mul(I::point(x))?
                .add(I::point(y).mul(I::point(y))?)?,
        )?
        .div(I::point(2.).mul(denom)?.mul(denom)?)?;
    let coefficient = [I::point(x).div(denom)?, I::point(y).div(denom)?, z];
    let normal = box_distance(&coefficient, &[I::point(0.); 3])?.1;
    let Some(fan) = fan::calculate(p, v, &j, sign)? else {
        return Ok(None);
    };
    Ok(Some(Bounds {
        normal,
        tensor,
        sign,
        fan,
    }))
}
