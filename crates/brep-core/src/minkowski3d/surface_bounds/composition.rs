//! Exact tensor composition S(A+r(C(t)-A)) and affine-chart shortcuts.
use super::*;
type Scalar = Vec<Vec<Q>>;
fn multiply(a: &Scalar, b: &Scalar, work: &mut Work) -> Result<Scalar> {
    let (n, m, p, q) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    work.charge(a.len() * a[0].len() * b.len() * b[0].len())?;
    let u = (0..=n)
        .map(|i| {
            (0..=p)
                .map(|j| choose(n, i) * choose(p, j) / choose(n + p, i + j))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let v = (0..=m)
        .map(|i| {
            (0..=q)
                .map(|j| choose(m, i) * choose(q, j) / choose(m + q, i + j))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut out = vec![vec![Q::zero(); m + q + 1]; n + p + 1];
    for i in 0..=n {
        for j in 0..=m {
            for k in 0..=p {
                for l in 0..=q {
                    if !a[i][j].is_zero() && !b[k][l].is_zero() {
                        out[i + k][j + l] += &a[i][j] * &b[k][l] * &u[i][k] * &v[j][l];
                    }
                }
            }
        }
    }
    Ok(out)
}
pub(in crate::minkowski3d) fn affine(s: &Surface) -> Result<Option<[P; 3]>> {
    let weight = s.weights[0][0];
    if s.weights.iter().flatten().any(|w| *w != weight) {
        return Ok(None);
    }
    let a = [
        Q::from_float(s.knots_u[s.degree_u]).unwrap(),
        Q::from_float(s.knots_v[s.degree_v]).unwrap(),
    ];
    let b = [
        Q::from_float(s.knots_u[s.control_points.len()]).unwrap(),
        Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap(),
    ];
    let origin = at(s, &a)?;
    let x = at(s, &[b[0].clone(), a[1].clone()])?;
    let y = at(s, &[a[0].clone(), b[1].clone()])?;
    let u: P = std::array::from_fn(|k| (&x[k] - &origin[k]) / (&b[0] - &a[0]));
    let v: P = std::array::from_fn(|k| (&y[k] - &origin[k]) / (&b[1] - &a[1]));
    let c: P = std::array::from_fn(|k| &origin[k] - &a[0] * &u[k] - &a[1] * &v[k]);
    for (i, row) in s.control_points.iter().enumerate() {
        let x = s.knots_u[i + 1..=i + s.degree_u]
            .iter()
            .map(|q| Q::from_float(*q).unwrap())
            .sum::<Q>()
            / integer(s.degree_u as i64);
        for (j, p) in row.iter().enumerate() {
            let y = s.knots_v[j + 1..=j + s.degree_v]
                .iter()
                .map(|q| Q::from_float(*q).unwrap())
                .sum::<Q>()
                / integer(s.degree_v as i64);
            if (0..3).any(|k| &c[k] + &x * &u[k] + &y * &v[k] != Q::from_float(p[k]).unwrap()) {
                return Ok(None);
            }
        }
    }
    Ok(Some([c, u, v]))
}
pub(in crate::minkowski3d) fn affine_curve_error(a: &[P; 3], pc: &[H]) -> P {
    let h = pc
        .iter()
        .map(|p| {
            [
                &a[0][0] * &p[3] + &a[1][0] * &p[0] + &a[2][0] * &p[1],
                &a[0][1] * &p[3] + &a[1][1] * &p[0] + &a[2][1] * &p[1],
                &a[0][2] * &p[3] + &a[1][2] * &p[0] + &a[2][2] * &p[1],
                p[3].clone(),
            ]
        })
        .collect::<Vec<_>>();
    let n = h.len() - 1;
    let a = point(&h[0]);
    let b = point(h.last().unwrap());
    let wmin = h.iter().map(|h| &h[3]).min().unwrap();
    std::array::from_fn(|k| {
        (0..=n + 1)
            .map(|i| {
                let t = Q::new(i.into(), (n + 1).into());
                ((if i <= n {
                    (Q::one() - &t) * (&h[i][k] - &a[k] * &h[i][3])
                } else {
                    Q::zero()
                }) + (if i > 0 {
                    &t * (&h[i - 1][k] - &b[k] * &h[i - 1][3])
                } else {
                    Q::zero()
                }))
                .abs()
                    / wmin
            })
            .max()
            .unwrap()
    })
}
pub(in crate::minkowski3d) fn fan(
    s: &Surface,
    pc: &[H],
    anchor: &[Q; 2],
    radial: &[Q; 2],
    work: &mut Work,
) -> Result<Option<Net>> {
    let uv = radial
        .iter()
        .map(|r| {
            pc.iter()
                .map(|p| {
                    [
                        &anchor[0] * &p[3] + r * (&p[0] - &anchor[0] * &p[3]),
                        &anchor[1] * &p[3] + r * (&p[1] - &anchor[1] * &p[3]),
                        Q::zero(),
                        p[3].clone(),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let span = |knots: &[f64], degree: usize, count: usize, axis: usize| {
        let values = uv
            .iter()
            .flatten()
            .map(|p| &p[axis] / &p[3])
            .collect::<Vec<_>>();
        let lo = values.iter().min().unwrap();
        let hi = values.iter().max().unwrap();
        (degree..count)
            .filter(|i| knots[*i] < knots[*i + 1])
            .map(|i| {
                [
                    Q::from_float(knots[i]).unwrap(),
                    Q::from_float(knots[i + 1]).unwrap(),
                ]
            })
            .find(|d| d[0] <= *lo && *hi <= d[1])
    };
    let (Some(du), Some(dv)) = (
        span(&s.knots_u, s.degree_u, s.control_points.len(), 0),
        span(&s.knots_v, s.degree_v, s.control_points[0].len(), 1),
    ) else {
        return Ok(None);
    };
    let h = cell(s, du.clone(), dv.clone())?;
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let basis =
        |axis: usize, domain: &[Q; 2], degree: usize, work: &mut Work| -> Result<Vec<Scalar>> {
            let u = uv
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|p| (&p[axis] - &domain[0] * &p[3]) / (&domain[1] - &domain[0]))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let v = uv
                .iter()
                .zip(&u)
                .map(|(r, u)| r.iter().zip(u).map(|(p, u)| &p[3] - u).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            let mut powers = |p: &Scalar| -> Result<Vec<Scalar>> {
                let mut powers = vec![vec![vec![Q::one()]]];
                for _ in 0..degree {
                    powers.push(multiply(powers.last().unwrap(), p, work)?)
                }
                Ok(powers)
            };
            let a = powers(&u)?;
            let b = powers(&v)?;
            (0..=degree)
                .map(|i| {
                    Ok(multiply(&a[i], &b[degree - i], work)?
                        .iter()
                        .map(|r| r.iter().map(|x| x * choose(degree, i)).collect())
                        .collect())
                })
                .collect()
        };
    let u = basis(0, &du, n, work)?;
    let v = basis(1, &dv, m, work)?;
    let mut out =
        vec![vec![std::array::from_fn(|_| Q::zero()); (pc.len() - 1) * (n + m) + 1]; n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            let p = multiply(&u[i], &v[j], work)?;
            for (r, row) in p.iter().enumerate() {
                for (c, p) in row.iter().enumerate() {
                    for k in 0..4 {
                        out[r][c][k] += &h[i][j][k] * p;
                    }
                }
            }
        }
    }
    if out.iter().flatten().any(|h| {
        h[3] <= Q::zero()
            || h.iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    }) {
        return Err(fail("trim fan tensor coefficient budget exceeded"));
    }
    Ok(Some(out))
}
