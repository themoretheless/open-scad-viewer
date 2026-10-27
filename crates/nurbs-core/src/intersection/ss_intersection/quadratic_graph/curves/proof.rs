use super::super::*;
use super::arithmetic::{Arithmetic as B, P, Rational};
use crate::curve::Curve;
pub(super) fn deviation(a: &Rational, c: &Curve, b: &mut B) -> Result<f64> {
    let actual = super::expressions::actual(c, b)?;
    check(
        a.numerator.len() == actual.numerator.len(),
        "SS curve dimension mismatch",
    )?;
    let da = a
        .denominator
        .iter()
        .map(|v| v.lo)
        .fold(f64::INFINITY, f64::min);
    let dc = c.weights.iter().copied().fold(f64::INFINITY, f64::min);
    check(
        da > 0. && dc > 0.,
        "SS curve denominator positivity unresolved",
    )?;
    let denominator = b.mul(I::point(da), I::point(dc))?;
    let mut components = Vec::new();
    for axis in 0..a.numerator.len() {
        let left = b.product(&a.numerator[axis], &actual.denominator)?;
        let right = b.product(&actual.numerator[axis], &a.denominator)?;
        let mut upper = 0f64;
        for i in 0..left.len() {
            let e = b.sub(left[i], right[i])?;
            upper = upper.max(e.lo.abs()).max(e.hi.abs());
        }
        components.push(b.div(I::point(upper), denominator)?);
    }
    b.norm_upper(&components)
}
fn split(p: &P, b: &mut B) -> Result<(P, P)> {
    let mut row = p.clone();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|v| {
                let sum = b.add(v[0], v[1])?;
                b.mul(sum, I::point(0.5))
            })
            .collect::<Result<P>>()?;
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
pub(super) fn domain(curve: &Curve, domain: [[f64; 2]; 2], b: &mut B) -> Result<()> {
    if curve
        .control_points
        .iter()
        .all(|p| (0..2).all(|a| domain[a][0] <= p[a] && p[a] <= domain[a][1]))
    {
        return Ok(());
    }
    let original = super::expressions::actual(curve, b)?;
    let mut pending = vec![(original, 0)];
    while let Some((p, depth)) = pending.pop() {
        let mut inside = true;
        for i in 0..p.denominator.len() {
            check(
                p.denominator[i].lo > 0.,
                "Native pcurve weight positivity unresolved",
            )?;
            for a in 0..2 {
                let q = b.div(p.numerator[a][i], p.denominator[i])?;
                inside &= q.lo >= domain[a][0] && q.hi <= domain[a][1];
            }
        }
        if inside {
            continue;
        }
        check(depth < 16, "Native pcurve domain containment unresolved")?;
        let (ld, rd) = split(&p.denominator, b)?;
        let mut ln = Vec::new();
        let mut rn = Vec::new();
        for n in &p.numerator {
            let (l, r) = split(n, b)?;
            ln.push(l);
            rn.push(r);
        }
        pending.push((
            Rational {
                numerator: rn,
                denominator: rd,
            },
            depth + 1,
        ));
        pending.push((
            Rational {
                numerator: ln,
                denominator: ld,
            },
            depth + 1,
        ));
    }
    Ok(())
}
pub(super) fn vertex_error(bounds: &[[f64; 2]], point: &[f64], b: &mut B) -> Result<f64> {
    let mut delta = Vec::new();
    for (range, p) in bounds.iter().zip(point) {
        delta.push(b.sub(I::new(range[0], range[1])?, I::point(*p))?);
    }
    b.norm_upper(&delta)
}
pub(super) fn vertex_lifts(
    owner: &QualifiedQuadraticGraphSS,
    world: [f64; 3],
    native: [[f64; 2]; 2],
    b: &mut B,
) -> Result<[f64; 2]> {
    let constant = |point: Vec<f64>| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![point; 2],
        weights: vec![1., 1.],
        periodic: false,
    };
    let world = constant(world.to_vec());
    let mut out = [0.; 2];
    for side in 0..2 {
        let uv = constant(native[side].to_vec());
        let lifted = super::expressions::pullback(owner, &uv, side, b)?;
        out[side] = deviation(&lifted, &world, b)?;
    }
    Ok(out)
}
