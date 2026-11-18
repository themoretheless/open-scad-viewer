//! Exact restriction of the stored homogeneous B-spline to its active knot spans.
//! De Boor evaluation and Bernstein interpolation use rational arithmetic only.
//! A degree-p polynomial is uniquely determined by p+1 distinct exact values;
//! the resulting net represents the original span, not a rounded fitted curve.
use super::*;

fn homogeneous(c: &Curve) -> Vec<[Q; 3]> {
    c.control_points
        .iter()
        .zip(&c.weights)
        .map(|(p, w)| {
            let w = Q::from_float(*w).unwrap();
            [
                Q::from_float(p[0]).unwrap() * &w,
                Q::from_float(p[1]).unwrap() * &w,
                w,
            ]
        })
        .collect()
}

fn de_boor(h: &[[Q; 3]], knots: &[Q], degree: usize, span: usize, t: &Q) -> Result<[Q; 3]> {
    let mut d = h[span - degree..=span].to_vec();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denominator = &knots[i + degree - r + 1] - &knots[i];
            if denominator <= Q::zero() {
                return Err(refused(
                    "Exact B-spline restriction has an empty denominator",
                ));
            }
            let alpha = (t - &knots[i]) / denominator;
            d[j] = std::array::from_fn(|k| (&Q::one() - &alpha) * &d[j - 1][k] + &alpha * &d[j][k]);
        }
    }
    Ok(d.pop().unwrap())
}

fn choose(n: usize, k: usize) -> Q {
    let mut v = 1usize;
    for i in 0..k {
        v = v * (n - i) / (i + 1);
    }
    Q::from_integer(v.into())
}

fn restrict(h: &[[Q; 3]], knots: &[Q], degree: usize, span: usize) -> Result<Vec<[Q; 3]>> {
    restrict_interval(h, knots, degree, span, &knots[span], &knots[span + 1])
}

fn restrict_interval(
    h: &[[Q; 3]],
    knots: &[Q],
    degree: usize,
    span: usize,
    a: &Q,
    b: &Q,
) -> Result<Vec<[Q; 3]>> {
    let mut rows = Vec::new();
    for i in 0..=degree {
        let u = Q::new(i.into(), degree.into());
        let t = a + (b - a) * &u;
        let value = de_boor(h, knots, degree, span, &t)?;
        let mut row = (0..=degree)
            .map(|j| {
                choose(degree, j) * u.pow(j as i32) * (&Q::one() - &u).pow((degree - j) as i32)
            })
            .collect::<Vec<_>>();
        row.extend(value);
        rows.push(row);
    }
    for col in 0..=degree {
        let pivot = (col..=degree)
            .find(|i| !rows[*i][col].is_zero())
            .ok_or_else(|| refused("Singular exact Bernstein interpolation"))?;
        rows.swap(col, pivot);
        let scale = rows[col][col].clone();
        for x in &mut rows[col][col..] {
            *x /= &scale;
        }
        for i in 0..=degree {
            if i == col {
                continue;
            }
            let scale = rows[i][col].clone();
            for j in col..degree + 4 {
                let product = &scale * &rows[col][j];
                rows[i][j] -= product;
            }
        }
    }
    let net: Vec<[Q; 3]> = rows
        .into_iter()
        .map(|r| std::array::from_fn(|k| r[degree + 1 + k].clone()))
        .collect();
    if net.iter().any(|p| p[2] <= Q::zero()) {
        return Err(refused(
            "Exact extracted span needs positive Bernstein weights",
        ));
    }
    Ok(net)
}

/// Caller supplies one interval contained in one active knot span. Reverse
/// orientation is permitted, but crossing an unpartitioned knot is refused.
pub(super) fn interval(c: &Curve, a: &Q, b: &Q) -> Result<Vec<[Q; 3]>> {
    let knots: Vec<_> = c.knots.iter().map(|t| Q::from_float(*t).unwrap()).collect();
    let lo = a.min(b);
    let hi = a.max(b);
    let span = (c.degree..c.control_points.len())
        .find(|i| knots[*i] <= *lo && *hi <= knots[*i + 1] && lo < hi)
        .ok_or_else(|| refused("Agreement interval crosses an unpartitioned knot"))?;
    restrict_interval(&homogeneous(c), &knots, c.degree, span, a, b)
}

pub(super) fn nets(loops: &[Vec<Curve>]) -> Result<Vec<(usize, Vec<[Q; 3]>)>> {
    let mut out = Vec::new();
    for (wire, curves) in loops.iter().enumerate() {
        for c in curves {
            let h = homogeneous(c);
            if c.control_points.len() == c.degree + 1
                && c.knots[..=c.degree].iter().all(|t| *t == c.domain()[0])
                && c.knots[c.degree + 1..].iter().all(|t| *t == c.domain()[1])
            {
                out.push((wire, h));
            } else {
                let knots: Vec<_> = c.knots.iter().map(|t| Q::from_float(*t).unwrap()).collect();
                for span in c.degree..c.control_points.len() {
                    if knots[span] == knots[span + 1] {
                        continue;
                    }
                    if out.len() >= MAX_SPANS {
                        return Err(refused("Exact knot-span budget exceeded"));
                    }
                    out.push((wire, restrict(&h, &knots, c.degree, span)?));
                }
            }
            if out.len() > MAX_SPANS {
                return Err(refused("Exact knot-span budget exceeded"));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_restriction_agrees_with_original_polynomial_on_non_dyadic_knots() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 3., 3., 3.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![3., 2.], vec![4., 0.]],
            weights: vec![1., 2., 3., 1.],
            periodic: false,
        };
        c.validate().unwrap();
        let nets = nets(&[vec![c.clone()]]).unwrap();
        assert_eq!(nets.len(), 2);
        let h = homogeneous(&c);
        let knots: Vec<_> = c.knots.iter().map(|t| Q::from_float(*t).unwrap()).collect();
        for (s, (_, net)) in nets.iter().enumerate() {
            for i in 0..=7 {
                let u = Q::new(i.into(), 7.into());
                let span = s + 2;
                let t = &knots[span] + (&knots[span + 1] - &knots[span]) * &u;
                let mut level = net.clone();
                while level.len() > 1 {
                    level = level
                        .windows(2)
                        .map(|p| {
                            std::array::from_fn(|k| (&Q::one() - &u) * &p[0][k] + &u * &p[1][k])
                        })
                        .collect();
                }
                assert_eq!(level[0], de_boor(&h, &knots, 2, span, &t).unwrap());
            }
        }
    }
}
