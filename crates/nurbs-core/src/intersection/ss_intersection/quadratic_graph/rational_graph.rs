//! Exact denominator cancellation on unchanged original rational Bezier supports.
//! A successful polynomial quotient retains the original surface as authority.
use super::*;
type Grid = Vec<Vec<f64>>;

pub(super) fn power(values: &[f64], w: &mut Work) -> Result<Vec<f64>> {
    let n = values.len() - 1;
    let mut row = values.to_vec();
    let mut out = vec![row[0]];
    let mut binomial = 1usize;
    for k in 1..=n {
        row = row
            .windows(2)
            .map(|a| w.add(a[1], -a[0]))
            .collect::<Result<_>>()?;
        binomial = binomial * (n + 1 - k) / k;
        out.push(w.mul(row[0], binomial as f64)?);
    }
    Ok(out)
}
pub(super) fn homogeneous(s: &Surface, axis: usize, w: &mut Work) -> Result<Grid> {
    let mut rows = Vec::new();
    for i in 0..=s.degree_u {
        let values = (0..=s.degree_v)
            .map(|j| {
                if axis == 3 {
                    Ok(s.weights[i][j])
                } else {
                    w.mul(s.control_points[i][j][axis], s.weights[i][j])
                }
            })
            .collect::<Result<Vec<_>>>()?;
        rows.push(power(&values, w)?);
    }
    let mut result = vec![vec![0.; s.degree_v + 1]; s.degree_u + 1];
    for j in 0..=s.degree_v {
        let values = rows.iter().map(|r| r[j]).collect::<Vec<_>>();
        for (i, c) in power(&values, w)?.into_iter().enumerate() {
            result[i][j] = c;
        }
    }
    Ok(result)
}
fn coefficient(g: &Grid, i: usize, j: usize) -> f64 {
    g.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.)
}
fn product(
    q: &Poly,
    den: &Grid,
    i: usize,
    j: usize,
    skip_constant: bool,
    w: &mut Work,
) -> Result<f64> {
    let mut sum = 0.;
    for a in 0..=i.min(2) {
        for b in 0..=j.min(2) {
            if skip_constant && a == i && b == j {
                continue;
            }
            let d = coefficient(den, i - a, j - b);
            if q[a][b] == 0. || d == 0. {
                continue;
            }
            let term = w.mul(q[a][b], d)?;
            sum = w.add(sum, term)?;
        }
    }
    Ok(sum)
}
fn verify(numerator: &Grid, den: &Grid, q: &Poly, w: &mut Work) -> Result<()> {
    // Including every possible product tail is essential: a lower-order
    // Taylor match alone does not prove global rational cancellation.
    for i in 0..numerator.len().max(den.len() + 2) {
        for j in 0..numerator[0].len().max(den[0].len() + 2) {
            let expected = product(q, den, i, j, false, w)?;
            w.tick()?;
            check(
                expected == coefficient(numerator, i, j),
                "Original rational numerator does not equal the proposed polynomial times its denominator",
            )?;
        }
    }
    Ok(())
}
pub(super) fn quotient(numerator: &Grid, den: &Grid, w: &mut Work) -> Result<Poly> {
    check(
        den[0][0] > 0.,
        "Original rational graph corner denominator must be positive",
    )?;
    let mut q = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let known = product(&q, den, i, j, true, w)?;
            let residual = w.add(coefficient(numerator, i, j), -known)?;
            q[i][j] = w.div(residual, den[0][0])?;
        }
    }
    verify(numerator, den, &q, w)?;
    Ok(q)
}

pub(super) fn graph(s: &Surface, w: &mut Work) -> Result<([f64; 4], [[f64; 2]; 2], Poly)> {
    check(
        (1..=4).contains(&s.degree_u)
            && (1..=4).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points
                .iter()
                .all(|r| r.len() == s.degree_v + 1 && r.iter().all(|p| p.len() == 3))
            && s.knots_u.len() == 2 * (s.degree_u + 1)
            && s.knots_v.len() == 2 * (s.degree_v + 1)
            && !s.periodic_u
            && !s.periodic_v,
        "Rational graph cancellation needs one original degree1..4 unwrapped Bezier support",
    )?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for (axis, (knots, degree)) in [(&s.knots_u, s.degree_u), (&s.knots_v, s.degree_v)]
        .into_iter()
        .enumerate()
    {
        check(
            knots[..=degree].iter().all(|&t| t == domain[axis][0])
                && knots[degree + 1..].iter().all(|&t| t == domain[axis][1]),
            "Rational graph must retain complete original clamped knot spans",
        )?;
        w.is(I::point(domain[axis][1]), I::point(domain[axis][0]))?;
    }
    // Positive original Bernstein weights prove a nonzero denominator over the
    // complete rectangle. Surface::validate has checked every authored weight.
    let x = s.control_points[0][0][0];
    let y = s.control_points[0][0][1];
    let dx = w.add(s.control_points[s.degree_u][0][0], -x)?;
    let dy = w.add(s.control_points[0][s.degree_v][1], -y)?;
    check(
        dx != 0. && dy != 0.,
        "Original rational graph XY projection must be invertible",
    )?;
    let den = homogeneous(s, 3, w)?;
    let mut xq = [[0.; 3]; 3];
    xq[0][0] = x;
    xq[1][0] = dx;
    let mut yq = [[0.; 3]; 3];
    yq[0][0] = y;
    yq[0][1] = dy;
    verify(&homogeneous(s, 0, w)?, &den, &xq, w)?;
    verify(&homogeneous(s, 1, w)?, &den, &yq, w)?;
    let z = quotient(&homogeneous(s, 2, w)?, &den, w)?;
    Ok(([x, dx, y, dy], domain, z))
}

#[cfg(test)]
mod tests;
