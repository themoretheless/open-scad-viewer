//! Exact oriented z-flux. This is an integral, not an embedding/solid proof.
use super::*;
use num_bigint::BigInt;
type Net = Vec<Vec<Q>>;
fn binomial(n: usize, k: usize) -> BigInt {
    (0..k.min(n - k)).fold(BigInt::one(), |a, i| a * (n - i) / (i + 1))
}
fn mul(a: &[Q], z: &[Q], b: &mut Budget) -> Result<Vec<Q>> {
    let n = a.len() - 1;
    let m = z.len() - 1;
    check(
        n + m <= 256,
        "Flux composition polynomial degree exceeds limit",
    )?;
    let mut out = vec![Q::zero(); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            let weight = b.keep(Q::new(
                binomial(n, i) * binomial(m, j),
                binomial(n + m, i + j),
            ))?;
            let term = b.keep(&a[i] * &z[j])?;
            let term = b.keep(term * weight)?;
            out[i + j] = b.keep(&out[i + j] + term)?;
        }
    }
    Ok(out)
}
fn compact(a: Net) -> Net {
    let mut a = if a.iter().all(|r| r == &a[0]) {
        vec![a[0].clone()]
    } else {
        a
    };
    if a.iter().all(|r| r.iter().all(|x| x == &r[0])) {
        a = a.into_iter().map(|r| vec![r[0].clone()]).collect();
    }
    a
}
fn derivative(a: &Net, axis: usize, b: &mut Budget) -> Result<Net> {
    let n = a.len() - 1;
    let m = a[0].len() - 1;
    if (axis == 0 && n == 0) || (axis == 1 && m == 0) {
        return Ok(vec![vec![Q::zero()]]);
    }
    let mut out = Vec::new();
    for i in 0..if axis == 0 { n } else { n + 1 } {
        let mut row = Vec::new();
        for j in 0..if axis == 1 { m } else { m + 1 } {
            row.push(b.keep(if axis == 0 {
                (&a[i + 1][j] - &a[i][j]) * Q::from_integer(n.into())
            } else {
                (&a[i][j + 1] - &a[i][j]) * Q::from_integer(m.into())
            })?);
        }
        out.push(row);
    }
    Ok(compact(out))
}
fn tensor_mul(a: &Net, z: &Net, b: &mut Budget) -> Result<Net> {
    let (n, m, p, q) = (a.len() - 1, a[0].len() - 1, z.len() - 1, z[0].len() - 1);
    check(
        n + p <= 64 && m + q <= 64,
        "Flux tensor product degree exceeds limit",
    )?;
    let mut out = vec![vec![Q::zero(); m + q + 1]; n + p + 1];
    for i in 0..=n {
        for j in 0..=m {
            for k in 0..=p {
                for l in 0..=q {
                    let w = b.keep(Q::new(
                        binomial(n, i) * binomial(p, k) * binomial(m, j) * binomial(q, l),
                        binomial(n + p, i + k) * binomial(m + q, j + l),
                    ))?;
                    let term = b.keep(&a[i][j] * &z[k][l])?;
                    let term = b.keep(term * w)?;
                    out[i + k][j + l] = b.keep(&out[i + k][j + l] + term)?;
                }
            }
        }
    }
    Ok(compact(out))
}
fn elevate_big(a: &[Q], degree: usize, b: &mut Budget) -> Result<Vec<Q>> {
    let n = a.len() - 1;
    let mut out = vec![Q::zero(); degree + 1];
    for i in 0..=n {
        for j in 0..=degree - n {
            let w = b.keep(Q::new(
                binomial(n, i) * binomial(degree - n, j),
                binomial(degree, i + j),
            ))?;
            let term = b.keep(&a[i] * w)?;
            out[i + j] = b.keep(&out[i + j] + term)?;
        }
    }
    Ok(out)
}
fn tensor_sub(a: &Net, z: &Net, b: &mut Budget) -> Result<Net> {
    let n = (a.len() - 1).max(z.len() - 1);
    let m = (a[0].len() - 1).max(z[0].len() - 1);
    fn lift(a: &Net, n: usize, m: usize, b: &mut Budget) -> Result<Net> {
        let rows = a
            .iter()
            .map(|r| elevate_big(r, m, b))
            .collect::<Result<Vec<_>>>()?;
        let cols = (0..=m)
            .map(|j| elevate_big(&rows.iter().map(|r| r[j].clone()).collect::<Vec<_>>(), n, b))
            .collect::<Result<Vec<_>>>()?;
        Ok((0..=n)
            .map(|i| cols.iter().map(|c| c[i].clone()).collect())
            .collect())
    }
    let a = lift(a, n, m, b)?;
    let z = lift(z, n, m, b)?;
    Ok(compact(
        a.iter()
            .zip(z)
            .map(|(a, z)| a.iter().zip(z).map(|(a, z)| b.keep(a - z)).collect())
            .collect::<Result<Net>>()?,
    ))
}
pub struct PolynomialFlux {
    net: Net,
}
/// Exact face-flux accumulation under the same shared coefficient budget.
pub fn sum_fluxes(values: &[Q], b: &mut Budget) -> Result<Q> {
    let mut sum = Q::zero();
    for value in values {
        let value = b.keep(value.clone())?;
        sum = b.keep(sum + value)?;
    }
    Ok(sum)
}
fn from_net(net: &[Vec<[Q; 3]>], b: &mut Budget) -> Result<PolynomialFlux> {
    check(
        !net.is_empty()
            && !net[0].is_empty()
            && net.len() <= 33
            && net.iter().all(|r| r.len() == net[0].len()),
        "Invalid flux source tensor",
    )?;
    let columns = (0..3)
        .map(|k| {
            net.iter()
                .map(|r| r.iter().map(|p| b.keep(p[k].clone())).collect())
                .collect()
        })
        .collect::<Result<Vec<Net>>>()?;
    let xu = derivative(&columns[0], 0, b)?;
    let xv = derivative(&columns[0], 1, b)?;
    let yu = derivative(&columns[1], 0, b)?;
    let yv = derivative(&columns[1], 1, b)?;
    let a = tensor_mul(&xu, &yv, b)?;
    let z = tensor_mul(&xv, &yu, b)?;
    let jac = tensor_sub(&a, &z, b)?;
    Ok(PolynomialFlux {
        net: tensor_mul(&columns[2], &jac, b)?,
    })
}
/// Polynomial original support, in its normalized [0,1]^2 parameter chart.
pub fn polynomial_surface_flux(s: &Surface, b: &mut Budget) -> Result<PolynomialFlux> {
    s.validate()?;
    let (n, m) = (s.degree_u, s.degree_v);
    check(
        !s.periodic_u
            && !s.periodic_v
            && n <= 32
            && m <= 32
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == m + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Flux source requires polynomial Bezier tensor support",
    )?;
    for (knots, d) in [(&s.knots_u, n), (&s.knots_v, m)] {
        check(
            knots[..=d].iter().all(|x| *x == knots[d])
                && knots[d + 1..].iter().all(|x| *x == knots[d + 1]),
            "Flux source must preserve clamped Bezier knots",
        )?;
    }
    let net = s
        .control_points
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| Ok([b.authored(p[0])?, b.authored(p[1])?, b.authored(p[2])?]))
                .collect()
        })
        .collect::<Result<Vec<_>>>()?;
    from_net(&net, b)
}
impl Patch {
    pub fn flux_polynomial(&self, b: &mut Budget) -> Result<PolynomialFlux> {
        from_net(&self.coefficients, b)
    }
}
fn reduce_line(a: Vec<Q>, b: &mut Budget) -> Result<Vec<Q>> {
    if a.iter().all(|x| x == &a[0]) {
        return Ok(vec![b.keep(a[0].clone())?]);
    }
    let n = a.len() - 1;
    let delta = b.keep(a[n].clone() - &a[0])?;
    for (i, x) in a.iter().enumerate() {
        let expected = b.keep(&a[0] + &delta * ratio(i as u64, n as u64))?;
        if *x != expected {
            return Ok(a);
        }
    }
    Ok(vec![a[0].clone(), a[n].clone()])
}
fn basis(p: &[Q], degree: usize, b: &mut Budget) -> Result<Vec<Vec<Q>>> {
    let other = p
        .iter()
        .map(|x| b.keep(Q::one() - x))
        .collect::<Result<Vec<_>>>()?;
    let (mut a, mut z) = (vec![vec![Q::one()]], vec![vec![Q::one()]]);
    for i in 0..degree {
        a.push(mul(&a[i], p, b)?);
        z.push(mul(&z[i], &other, b)?);
    }
    (0..=degree)
        .map(|i| {
            mul(&a[i], &z[degree - i], b)?
                .into_iter()
                .map(|x| b.keep(x * Q::from_integer(binomial(degree, i))))
                .collect()
        })
        .collect()
}
impl PolynomialFlux {
    pub fn integrate_rectangle(&self, b: &mut Budget) -> Result<Q> {
        let mut sum = Q::zero();
        for x in self.net.iter().flatten() {
            sum = b.keep(sum + x)?;
        }
        b.keep(sum / Q::from_integer((self.net.len() * self.net[0].len()).into()))
    }
    /// Green integral on a complete directed, polynomial, normalized UV loop.
    /// Loop orientation determines sign; no simplicity/embedding claim is made.
    pub fn integrate_loop(&self, uv: &[Vec<[Q; 2]>], b: &mut Budget) -> Result<Q> {
        check(
            (3..=4096).contains(&uv.len()),
            "Flux requires a complete UV loop",
        )?;
        for (i, c) in uv.iter().enumerate() {
            check(
                (2..=33).contains(&c.len()) && c.last() == uv[(i + 1) % uv.len()].first(),
                "Flux UV loop is not exactly closed",
            )?;
            for p in c {
                for x in p {
                    b.keep(x.clone())?;
                    check(
                        *x >= Q::zero() && *x <= Q::one(),
                        "Flux UV control left normalized source chart",
                    )?;
                }
            }
        }
        let n = self.net.len() - 1;
        let m = self.net[0].len() - 1;
        // P(U,V)=-integral_0^V F(U,v)dv; integral F = integral P dU.
        let mut primitive = vec![vec![Q::zero(); m + 2]; n + 1];
        for i in 0..=n {
            for j in 0..=m {
                primitive[i][j + 1] =
                    b.keep(&primitive[i][j] + &self.net[i][j] / Q::from_integer((m + 1).into()))?;
            }
        }
        let mut total = Q::zero();
        for curve in uv {
            let u = reduce_line(curve.iter().map(|p| p[0].clone()).collect(), b)?;
            if u.len() == 1 {
                continue;
            }
            let v = reduce_line(curve.iter().map(|p| p[1].clone()).collect(), b)?;
            let degree = n * (u.len() - 1) + (m + 1) * (v.len() - 1);
            check(
                degree + u.len() - 2 <= 256,
                "Flux trim composition degree exceeds limit",
            )?;
            let ub = basis(&u, n, b)?;
            let vb = basis(&v, m + 1, b)?;
            let mut composed = vec![Q::zero(); degree + 1];
            for i in 0..=n {
                for j in 0..=m + 1 {
                    if primitive[i][j].is_zero() {
                        continue;
                    }
                    for (k, x) in mul(&ub[i], &vb[j], b)?.into_iter().enumerate() {
                        let term = b.keep(&primitive[i][j] * x)?;
                        composed[k] = b.keep(&composed[k] + term)?;
                    }
                }
            }
            let du = u
                .windows(2)
                .map(|w| b.keep((&w[1] - &w[0]) * Q::from_integer((u.len() - 1).into())))
                .collect::<Result<Vec<_>>>()?;
            let line = mul(&composed, &du, b)?;
            let mut integral = Q::zero();
            for x in &line {
                integral = b.keep(integral + x)?;
            }
            total = b.keep(total - integral / Q::from_integer(line.len().into()))?;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![-1., -1., -1., 3., 3., 3.],
            knots_v: vec![2., 2., 5., 5.],
            control_points: vec![
                vec![vec![0., 0., 1.], vec![0., 1., 4.]],
                vec![vec![0.5, 0., 1.], vec![0.5, 1., 5.]],
                vec![vec![1., 0., 2.], vec![1., 1., 7.]],
            ],
            weights: vec![vec![2.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn p(x: i64, y: i64) -> [Q; 2] {
        [Q::from_integer(x.into()), Q::from_integer(y.into())]
    }
    fn parabola() -> Vec<Vec<[Q; 2]>> {
        vec![
            vec![p(0, 0), p(1, 0)],
            vec![p(1, 0), p(1, 1)],
            vec![p(1, 1), [ratio(1, 2), Q::zero()], p(0, 0)],
        ]
    }
    #[test]
    fn exact_flux_matches_independent_rectangle_and_curved_domain_integrals() {
        // z=1+u^2+2uv+3v. Full square integral=10/3; 0<v<u^2 integral=1.
        let mut b = Budget::new(1_000_000).unwrap();
        let flux = polynomial_surface_flux(&source(), &mut b).unwrap();
        assert_eq!(flux.integrate_rectangle(&mut b).unwrap(), ratio(10, 3));
        let loop_ = parabola();
        assert_eq!(flux.integrate_loop(&loop_, &mut b).unwrap(), Q::one());
        let reversed = loop_
            .into_iter()
            .rev()
            .map(|c| c.into_iter().rev().collect())
            .collect::<Vec<_>>();
        assert_eq!(flux.integrate_loop(&reversed, &mut b).unwrap(), -Q::one());
        let square = vec![
            vec![p(0, 0), p(1, 0)],
            vec![p(1, 0), p(1, 1)],
            vec![p(1, 1), p(0, 1)],
            vec![p(0, 1), p(0, 0)],
        ];
        assert_eq!(flux.integrate_loop(&square, &mut b).unwrap(), ratio(10, 3));
    }
    #[test]
    fn exact_flux_uses_both_projected_jacobian_terms_and_orientation() {
        let mut s = source();
        for row in &mut s.control_points {
            for p in row {
                let (u, v) = (p[0], p[1]);
                p[0] = 2. * u + v;
                p[1] = u + 3. * v;
            }
        }
        let mut b = Budget::new(1_000_000).unwrap();
        let f = polynomial_surface_flux(&s, &mut b).unwrap();
        assert_eq!(f.integrate_rectangle(&mut b).unwrap(), ratio(50, 3));
        assert_eq!(f.integrate_loop(&parabola(), &mut b).unwrap(), ratio(5, 1));
        for row in &mut s.control_points {
            for p in row {
                p.swap(0, 1);
            }
        }
        assert_eq!(
            polynomial_surface_flux(&s, &mut b)
                .unwrap()
                .integrate_rectangle(&mut b)
                .unwrap(),
            -ratio(50, 3)
        );
    }
    #[test]
    fn flux_limits_invalid_loops_and_rational_supports_fail_closed() {
        let mut b = Budget::new(100_000).unwrap();
        let mut s = source();
        let f = polynomial_surface_flux(&s, &mut b).unwrap();
        assert!(f.integrate_rectangle(&mut Budget::new(0).unwrap()).is_err());
        assert!(
            f.integrate_loop(&parabola(), &mut Budget::new(0).unwrap())
                .is_err()
        );
        let mut broken = parabola();
        broken[0][1] = p(0, 0);
        assert!(f.integrate_loop(&broken, &mut b).is_err());
        let mut outside = parabola();
        outside[2][1][1] = ratio(2, 1);
        assert!(f.integrate_loop(&outside, &mut b).is_err());
        s.weights[1][1] = 3.;
        assert!(polynomial_surface_flux(&s, &mut b).is_err());
        assert!(mul(&vec![Q::one(); 129], &vec![Q::one(); 130], &mut b).is_err());
    }
    #[test]
    fn flux_binomial_products_are_not_limited_to_machine_integer_coefficients() {
        let mut b = Budget::new(100_000).unwrap();
        let a = vec![Q::one(); 41];
        assert_eq!(mul(&a, &a, &mut b).unwrap(), vec![Q::one(); 81]);
    }
}
