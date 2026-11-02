//! Exact polynomial tensor-surface composition with an authored UV line.
//! Construction data only; callers must validate actual saved topology/identity.
use super::*;
fn powers(p: &[Q], n: usize, b: &mut Budget) -> Result<Vec<Vec<Q>>> {
    let mut out = vec![vec![Q::one()]];
    for i in 0..n {
        out.push(product(&out[i], p, b)?);
    }
    Ok(out)
}
fn basis(p: &[Q], degree: usize, b: &mut Budget) -> Result<Vec<Vec<Q>>> {
    let opposite = p
        .iter()
        .map(|x| b.keep(Q::one() - x))
        .collect::<Result<Vec<_>>>()?;
    let a = powers(p, degree, b)?;
    let z = powers(&opposite, degree, b)?;
    (0..=degree)
        .map(|i| {
            product(&a[i], &z[degree - i], b)?
                .into_iter()
                .map(|x| b.keep(x * Q::from_integer(choose(degree, i).into())))
                .collect()
        })
        .collect()
}
pub fn polynomial_surface_line(
    s: &Surface,
    uv: [[Q; 2]; 2],
    b: &mut Budget,
) -> Result<Vec<[Q; 3]>> {
    s.validate()?;
    let n = s.degree_u;
    let m = s.degree_v;
    check(
        !s.periodic_u
            && !s.periodic_v
            && n + m <= 32
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == m + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Source lift requires a polynomial Bezier tensor support",
    )?;
    let mut normalized = Vec::new();
    for (axis, knots, degree, count) in [
        (0, &s.knots_u, n, s.control_points.len()),
        (1, &s.knots_v, m, s.control_points[0].len()),
    ] {
        let low = b.authored(knots[degree])?;
        let high = b.authored(knots[count])?;
        let span = b.keep(&high - &low)?;
        check(
            knots[..=degree].iter().all(|x| *x == knots[degree])
                && knots[count..].iter().all(|x| *x == knots[count]),
            "Source support must preserve original clamped Bezier knots",
        )?;
        let line = uv
            .iter()
            .map(|p| {
                let x = b.keep(p[axis].clone())?;
                check(x >= low && x <= high, "Original UV line left source chart")?;
                b.keep((x - &low) / &span)
            })
            .collect::<Result<Vec<_>>>()?;
        normalized.push(line);
    }
    let u = basis(&normalized[0], n, b)?;
    let v = basis(&normalized[1], m, b)?;
    let mut points = vec![std::array::from_fn(|_| Q::zero()); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            let factor = product(&u[i], &v[j], b)?;
            for k in 0..3 {
                let original = b.authored(s.control_points[i][j][k])?;
                for t in 0..points.len() {
                    let value = b.keep(&original * &factor[t])?;
                    points[t][k] = b.keep(&points[t][k] + value)?;
                }
            }
        }
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_source_lift_preserves_polynomial_and_reversed_clock() {
        // S(u,v) = (u,v,u^2-v/4). Diagonal gives t^2-t/4,
        // independently represented as degree-two Bernstein controls.
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., -0.25]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., -0.25]],
                vec![vec![1., 0., 1.], vec![1., 1., 0.75]],
            ],
            weights: vec![vec![2.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let uv = [[Q::zero(), Q::zero()], [Q::one(), Q::one()]];
        let expected = vec![
            [Q::zero(), Q::zero(), Q::zero()],
            [ratio(1, 2), ratio(1, 2), -ratio(1, 8)],
            [Q::one(), Q::one(), ratio(3, 4)],
        ];
        let mut b = Budget::new(100_000).unwrap();
        let actual = polynomial_surface_line(&s, uv.clone(), &mut b).unwrap();
        assert!(polynomial_points_equal(&actual, &expected, &mut b).unwrap());
        let reversed = polynomial_surface_line(&s, [uv[1].clone(), uv[0].clone()], &mut b).unwrap();
        assert_eq!(reversed, actual.into_iter().rev().collect::<Vec<_>>());
        assert!(polynomial_surface_line(&s, uv.clone(), &mut Budget::new(0).unwrap()).is_err());
        let mut bad = s.clone();
        bad.weights[1][1] = 3.;
        assert!(polynomial_surface_line(&bad, uv.clone(), &mut b).is_err());
        let mut outside = uv;
        outside[1][0] = ratio(3, 2);
        assert!(polynomial_surface_line(&s, outside, &mut b).is_err());
    }
}
