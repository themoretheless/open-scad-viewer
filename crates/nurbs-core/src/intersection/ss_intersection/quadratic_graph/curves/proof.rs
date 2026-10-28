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
                b.tick(2)?;
                if v[0].lo == v[0].hi && v[1].lo == v[1].hi {
                    if let Some(a) = exact_add(v[0].lo, v[1].lo).and_then(|s| exact_mul(s, 0.5)) {
                        return Ok(I::point(a));
                    }
                }
                v[0].add(v[1])?.mul(I::point(0.5))
            })
            .collect::<Result<P>>()?;
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
// W is positive. Proving N-lower*W >= 0 and upper*W-N >= 0 avoids
// losing exact source boundary contacts through independently rounded N/W.
fn margin(a: f64, c: f64, weight: f64, b: &mut B) -> Result<I> {
    b.tick(2)?;
    if let Some(v) = exact_add(a, -c).and_then(|d| exact_mul(d, weight)) {
        return Ok(I::point(v));
    }
    I::point(a).sub(I::point(c))?.mul(I::point(weight))
}
pub(super) fn domain(curve: &Curve, domain: [[f64; 2]; 2], b: &mut B) -> Result<()> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 2
            && domain
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
        "Native pcurve domain needs finite ordered 2D bounds",
    )?;
    if curve
        .control_points
        .iter()
        .all(|p| (0..2).all(|a| domain[a][0] <= p[a] && p[a] <= domain[a][1]))
    {
        return Ok(());
    }
    // Also validate the complete unit Bezier clock before using its coefficients.
    super::expressions::actual(curve, b)?;
    let mut margins = Vec::new();
    for a in 0..2 {
        margins.push(
            curve
                .control_points
                .iter()
                .zip(&curve.weights)
                .map(|(p, w)| margin(p[a], domain[a][0], *w, b))
                .collect::<Result<P>>()?,
        );
        margins.push(
            curve
                .control_points
                .iter()
                .zip(&curve.weights)
                .map(|(p, w)| margin(domain[a][1], p[a], *w, b))
                .collect::<Result<P>>()?,
        );
    }
    let mut pending = vec![(margins, 0)];
    while let Some((polynomials, depth)) = pending.pop() {
        b.tick(1)?;
        let unresolved: Vec<_> = polynomials
            .into_iter()
            .filter(|p| !p.iter().all(|v| v.lo >= 0.))
            .collect();
        if unresolved.is_empty() {
            continue;
        }
        check(
            !unresolved.iter().any(|p| p.iter().all(|v| v.hi < 0.)),
            "Actual pcurve leaves the original native domain",
        )?;
        check(depth < 16, "Native pcurve domain containment unresolved")?;
        let mut left = Vec::new();
        let mut right = Vec::new();
        for p in unresolved {
            let (l, r) = split(&p, b)?;
            left.push(l);
            right.push(r);
        }
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
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
