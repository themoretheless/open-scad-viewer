//! Directional shape derivatives through a proved original XY projection.
use super::*;
pub(super) fn bounds(
    j: &jets::Jets,
    r: [I; 2],
    frame: ([usize; 3], [f64; 2]),
) -> Result<Option<Vec<I>>> {
    let field = |a: usize, b: usize| -> Result<[D; 3]> {
        let mut out = [D::zero(); 3];
        for k in 0..3 {
            out[k] = D {
                v: j[a][b][k],
                d: j[a + 1][b][k].mul(r[0])?.add(j[a][b + 1][k].mul(r[1])?)?,
            };
        }
        Ok(out)
    };
    let (axes, plane) = frame;
    let (u, v) = (field(1, 0)?, field(0, 1)?);
    let determinant = u[0].mul(v[1])?.sub(v[0].mul(u[1])?)?;
    if determinant.v.lo <= 0. && determinant.v.hi >= 0. {
        return Ok(None);
    }
    let negative = determinant.v.hi < 0.;
    let divide = |x: D| {
        if negative {
            x.neg()?.div(determinant.neg()?)
        } else {
            x.div(determinant)
        }
    };
    let inverse = [
        [divide(v[1])?, divide(v[0].neg()?)?],
        [divide(u[1].neg()?)?, divide(u[0])?],
    ];
    let gradient = [
        u[2].mul(inverse[0][0])?.add(v[2].mul(inverse[1][0])?)?,
        u[2].mul(inverse[0][1])?.add(v[2].mul(inverse[1][1])?)?,
    ];
    let mut second = [[D::zero(); 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            let jet = field(
                usize::from(a == 0) + usize::from(b == 0),
                usize::from(a == 1) + usize::from(b == 1),
            )?;
            second[a][b] = jet[2]
                .sub(gradient[0].mul(jet[0])?)?
                .sub(gradient[1].mul(jet[1])?)?;
        }
    }
    let constant = |x| D {
        v: I::point(x),
        d: I::point(0.),
    };
    let mut graph = [[[D::zero(); 3]; 3]; 3];
    graph[1][0][axes[0]] = constant(1.);
    graph[0][1][axes[1]] = constant(1.);
    graph[1][0][axes[2]] = constant(plane[0]).add(gradient[0])?;
    graph[0][1][axes[2]] = constant(plane[1]).add(gradient[1])?;
    for a in 0..2 {
        for b in a..2 {
            let mut value = D::zero();
            for x in 0..2 {
                for y in 0..2 {
                    value = value.add(inverse[x][a].mul(second[x][y])?.mul(inverse[y][b])?)?;
                }
            }
            graph[usize::from(a == 0) + usize::from(b == 0)]
                [usize::from(a == 1) + usize::from(b == 1)][axes[2]] = value;
        }
    }
    let Some(mut out) = shape_derivative_fields(|a, b| Ok(graph[a][b]))? else {
        return Ok(None);
    };
    if negative {
        for x in &mut out {
            *x = I::new(-x.hi, -x.lo)?;
        }
    }
    Ok(Some(out))
}
