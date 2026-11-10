//! Exact pullback of knot lines when their UV coordinate is affine on the
//! parameter triangle. The other coordinate and boundary remain rational curves.
use super::*;
fn affine(h: &Net, axis: usize, work: &mut Work) -> Result<Option<[Q; 3]>> {
    let c = bernstein::corners(h).map(|p| p[axis].clone());
    let linear = vec![vec![c[0].clone(), c[2].clone()], vec![c[1].clone()]];
    let one = vec![vec![Q::one(), Q::one()], vec![Q::one()]];
    let lhs = bernstein::product(&bernstein::scalar(h, axis), &one, work)?;
    let rhs = bernstein::product(&bernstein::scalar(h, 3), &linear, work)?;
    Ok((lhs == rhs).then_some(c))
}
fn clip_line(
    poly: Vec<P>,
    values: &[Q; 3],
    limit: &Q,
    lower: bool,
    work: &mut Work,
) -> Result<Vec<P>> {
    let value = |p: &P| {
        &values[0] + (&values[1] - &values[0]) * &p[0] + (&values[2] - &values[0]) * &p[1] - limit
    };
    let mut out = Vec::new();
    for (a, b) in poly
        .iter()
        .zip(poly.iter().cycle().skip(1))
        .take(poly.len())
    {
        work.charge(1)?;
        let (va, vb) = (value(a), value(b));
        let inside = |v: &Q| {
            if lower {
                *v >= Q::zero()
            } else {
                *v <= Q::zero()
            }
        };
        let (ia, ib) = (inside(&va), inside(&vb));
        if ia {
            out.push(a.clone());
        }
        if ia != ib {
            let t = &va / (&va - &vb);
            out.push(std::array::from_fn(|k| &a[k] + &t * (&b[k] - &a[k])));
        }
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    Ok(out)
}
pub(super) fn charts(
    s: &crate::Surface,
    uv: &Net,
    us: &[[Q; 2]],
    vs: &[[Q; 2]],
    work: &mut Work,
) -> Result<Vec<Net>> {
    let affine = [affine(uv, 0, work)?, affine(uv, 1, work)?];
    for (axis, ranges) in [us, vs].iter().enumerate() {
        if affine[axis].is_none() && !ranges.iter().any(|r| contained(uv, axis, r)) {
            return Err(fail(
                "nonlinear knot-line pullback needs an algebraic branch partition",
            ));
        }
    }
    let triangle = vec![
        [Q::zero(), Q::zero(), Q::zero()],
        [Q::one(), Q::zero(), Q::zero()],
        [Q::zero(), Q::one(), Q::zero()],
    ];
    let mut covered = Q::zero();
    let mut result = Vec::new();
    for u in us {
        for v in vs {
            let mut poly = triangle.clone();
            for (axis, range) in [u, v].iter().enumerate() {
                if let Some(values) = &affine[axis] {
                    poly = clip_line(poly, values, &range[0], true, work)?;
                    poly = clip_line(poly, values, &range[1], false, work)?;
                } else if !contained(uv, axis, range) {
                    poly.clear();
                }
            }
            for i in 1..poly.len().saturating_sub(1) {
                let p = [poly[0].clone(), poly[i].clone(), poly[i + 1].clone()];
                let area = (&p[1][0] - &p[0][0]) * (&p[2][1] - &p[0][1])
                    - (&p[1][1] - &p[0][1]) * (&p[2][0] - &p[0][0]);
                if area.is_zero() {
                    continue;
                }
                if area < Q::zero() {
                    return Err(fail("curved knot pullback reversed a parameter triangle"));
                }
                covered += area;
                let vertices = p.map(|p| [Q::one() - &p[0] - &p[1], p[0].clone(), p[1].clone()]);
                let child = bernstein::restrict(uv, vertices, work)?;
                result.push(compose_cell(s, &child, u, v, work)?);
                if result.len() > 512 {
                    return Err(fail("curved knot pullback triangle budget exceeded"));
                }
            }
        }
    }
    if covered != Q::one() {
        return Err(fail(
            "curved knot pullback leaves a gap in the parameter disk",
        ));
    }
    Ok(result)
}
fn contained(uv: &Net, axis: usize, r: &[Q; 2]) -> bool {
    uv.iter()
        .flatten()
        .all(|p| p[axis] >= &r[0] * &p[3] && p[axis] <= &r[1] * &p[3])
}
