//! Exact triangular Bernstein arithmetic. All domains are the closed standard
//! triangle; rational maps retain their original positive homogeneous weights.
use super::*;
pub(super) type Scalar = Vec<Vec<Q>>;
pub(super) type Net = Vec<Vec<sb::H>>;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |x, i| {
        x * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn multi(n: usize, i: usize, j: usize) -> u64 {
    let choose = |n: usize, k: usize| (0..k).fold(1u64, |v, a| v * (n - a) as u64 / (a + 1) as u64);
    // n <= 33, hence every multinomial is bounded by 3^33 < 2^53.
    choose(n, i) * choose(n - i, j)
}
fn blank(n: usize) -> Scalar {
    (0..=n).map(|i| vec![Q::zero(); n + 1 - i]).collect()
}
pub(super) fn scalar(h: &Net, k: usize) -> Scalar {
    h.iter()
        .map(|r| r.iter().map(|p| p[k].clone()).collect())
        .collect()
}
pub(super) fn product(a: &Scalar, b: &Scalar, work: &mut Work) -> Result<Scalar> {
    let (n, m) = (a.len() - 1, b.len() - 1);
    if n + m > 33 {
        return Err(fail("rational atlas degree budget exceeded"));
    }
    let mut c = blank(n + m);
    for (i, r) in a.iter().enumerate() {
        for (j, x) in r.iter().enumerate() {
            if x.is_zero() {
                continue;
            }
            for (k, s) in b.iter().enumerate() {
                for (l, y) in s.iter().enumerate() {
                    if y.is_zero() {
                        continue;
                    }
                    work.charge(1)?;
                    let numerator = num_bigint::BigInt::from(multi(n, i, j))
                        * num_bigint::BigInt::from(multi(m, k, l));
                    let factor = Q::new(numerator, multi(n + m, i + k, j + l).into());
                    c[i + k][j + l] += x * y * factor;
                }
            }
        }
    }
    if c.iter()
        .flatten()
        .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(fail("rational atlas coefficient bit budget exceeded"));
    }
    Ok(c)
}
pub(super) fn derivative(a: &Scalar, axis: usize) -> Scalar {
    let n = a.len() - 1;
    if n == 0 {
        return blank(0);
    }
    (0..n)
        .map(|i| {
            (0..n - i)
                .map(|j| {
                    (&a[i + usize::from(axis == 0)][j + usize::from(axis == 1)] - &a[i][j])
                        * integer(n as i64)
                })
                .collect()
        })
        .collect()
}
pub(super) fn difference(a: &Scalar, b: &Scalar) -> Scalar {
    a.iter()
        .zip(b)
        .map(|(r, s)| r.iter().zip(s).map(|(x, y)| x - y).collect())
        .collect()
}
/// Compose tensor Bernstein bases with an affine triangle in the cell.
pub(super) fn triangle(h: &sb::Net, uv: [[i64; 2]; 3], work: &mut Work) -> Result<Net> {
    let basis = |degree: usize, axis: usize, work: &mut Work| -> Result<Vec<Scalar>> {
        let u = vec![
            vec![integer(uv[0][axis]), integer(uv[2][axis])],
            vec![integer(uv[1][axis])],
        ];
        let v = u
            .iter()
            .map(|r| r.iter().map(|x| Q::one() - x).collect())
            .collect::<Scalar>();
        let mut up = vec![vec![vec![Q::one()]]];
        let mut vp = up.clone();
        for i in 0..degree {
            up.push(product(&up[i], &u, work)?);
            vp.push(product(&vp[i], &v, work)?);
        }
        (0..=degree)
            .map(|i| {
                let mut p = product(&up[i], &vp[degree - i], work)?;
                for x in p.iter_mut().flatten() {
                    *x *= choose(degree, i);
                }
                Ok(p)
            })
            .collect()
    };
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let (u, v) = (basis(n, 0, work)?, basis(m, 1, work)?);
    let mut out = (0..=n + m)
        .map(|i| vec![std::array::from_fn(|_| Q::zero()); n + m + 1 - i])
        .collect::<Net>();
    for i in 0..=n {
        for j in 0..=m {
            let p = product(&u[i], &v[j], work)?;
            for (k, r) in p.iter().enumerate() {
                for (l, q) in r.iter().enumerate() {
                    for d in 0..4 {
                        out[k][l][d] += q * &h[i][j][d];
                    }
                }
            }
        }
    }
    for p in out.iter().flatten() {
        if p[3] <= Q::zero() {
            return Err(fail("nonpositive triangular atlas weight"));
        }
        guard(&sb::point(p))?;
    }
    Ok(out)
}
/// Positive first weight layer proves a uniform single limit at a zero-weight
/// vertex. Every zero weight must have zero numerator; no pole is discarded.
pub(super) fn limits(h: &Net) -> Result<[P; 3]> {
    if h.is_empty()
        || h.iter().enumerate().any(|(i, r)| r.len() != h.len() - i)
        || h.iter()
            .flatten()
            .any(|p| p[3] < Q::zero() || (p[3].is_zero() && p[..3].iter().any(|x| !x.is_zero())))
    {
        return Err(fail("invalid homogeneous atlas pole controls"));
    }
    let n = h.len() - 1;
    let mut out = Vec::new();
    for corner in 0..3 {
        let p = match corner {
            0 => &h[0][0],
            1 => &h[n][0],
            _ => &h[0][n],
        };
        if p[3] > Q::zero() {
            out.push(sb::point(p));
            continue;
        }
        let mut found = None;
        for layer in 1..=n {
            let row = h
                .iter()
                .enumerate()
                .flat_map(|(i, r)| {
                    r.iter().enumerate().filter_map(move |(j, p)| {
                        let distance = match corner {
                            0 => i + j,
                            1 => n - i,
                            _ => n - j,
                        };
                        (distance == layer).then_some(p)
                    })
                })
                .collect::<Vec<_>>();
            if row.iter().all(|p| p[3].is_zero()) {
                continue;
            }
            if row.iter().any(|p| p[3] <= Q::zero()) {
                return Err(fail("pole limit lacks a positive complete leading layer"));
            }
            let limit = sb::point(row[0]);
            if row.iter().any(|p| sb::point(p) != limit) {
                return Err(fail("atlas pole has direction-dependent limits"));
            }
            found = Some(limit);
            break;
        }
        out.push(found.ok_or_else(|| fail("atlas pole has no finite proved limit"))?);
    }
    Ok(out.try_into().unwrap())
}
pub(super) fn corners(h: &Net) -> [P; 3] {
    limits(h).expect("atlas charts must have validated homogeneous corner limits")
}
/// Cancel only exact common endpoint powers in all four homogeneous coordinates.
fn cancel_endpoints(mut h: Vec<sb::H>) -> Vec<sb::H> {
    for side in 0..2 {
        if side == 1 {
            h.reverse();
        }
        let n = h.len() - 1;
        let zeros = h.iter().take_while(|p| p.iter().all(Q::is_zero)).count();
        if zeros > 0 && zeros <= n {
            h = (zeros..=n)
                .map(|i| {
                    std::array::from_fn(|k| &h[i][k] * choose(n, i) / choose(n - zeros, i - zeros))
                })
                .collect();
        }
        if side == 1 {
            h.reverse();
        }
    }
    h
}
/// Same denominator in both endpoints of S -> L. Degree elevation is exact.
pub(super) fn homotopy(h: &Net, work: &mut Work) -> Result<[Net; 2]> {
    let n = h.len() - 1;
    let c = corners(h);
    let w = scalar(h, 3);
    let one = vec![vec![Q::one(), Q::one()], vec![Q::one()]];
    let elevated = (0..4)
        .map(|k| product(&scalar(h, k), &one, work))
        .collect::<Result<Vec<_>>>()?;
    let target = (0..3)
        .map(|k| {
            product(
                &w,
                &vec![
                    vec![c[0][k].clone(), c[2][k].clone()],
                    vec![c[1][k].clone()],
                ],
                work,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let a = (0..=n + 1)
        .map(|i| {
            (0..=n + 1 - i)
                .map(|j| std::array::from_fn(|k| elevated[k][i][j].clone()))
                .collect()
        })
        .collect::<Net>();
    let b = (0..=n + 1)
        .map(|i| {
            (0..=n + 1 - i)
                .map(|j| {
                    std::array::from_fn(|k| {
                        if k == 3 {
                            elevated[3][i][j].clone()
                        } else {
                            target[k][i][j].clone()
                        }
                    })
                })
                .collect()
        })
        .collect::<Net>();
    Ok([a, b])
}
pub(super) fn edges(h: &Net) -> [Vec<sb::H>; 3] {
    let n = h.len() - 1;
    [
        cancel_endpoints((0..=n).map(|i| h[i][0].clone()).collect()),
        cancel_endpoints((0..=n).map(|i| h[n - i][i].clone()).collect()),
        cancel_endpoints((0..=n).map(|i| h[0][n - i].clone()).collect()),
    ]
}
/// Rational derivative numerators, with the common strictly positive W².
pub(super) fn derivatives(h: &Net, work: &mut Work) -> Result<[Vec<Scalar>; 2]> {
    let w = scalar(h, 3);
    let mut out = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let dw = derivative(&w, axis);
        for k in 0..3 {
            let p = scalar(h, k);
            out[axis].push(difference(
                &product(&derivative(&p, axis), &w, work)?,
                &product(&p, &dw, work)?,
            ));
        }
    }
    Ok(out)
}
/// Reparameterize a triangular net by its three barycentric vertices, using
/// the triangular de Casteljau blossom. This also works for scalar nets.
pub(super) fn restrict(h: &Net, vertices: [[Q; 3]; 3], work: &mut Work) -> Result<Net> {
    let n = h.len() - 1;
    let mut out = Vec::new();
    for i in 0..=n {
        let mut row = Vec::new();
        for j in 0..=n - i {
            let mut p = h.clone();
            for (v, count) in [(0, n - i - j), (1, i), (2, j)] {
                for _ in 0..count {
                    work.charge(1)?;
                    let d = p.len() - 1;
                    p = (0..d)
                        .map(|a| {
                            (0..d - a)
                                .map(|b| {
                                    std::array::from_fn(|k| {
                                        &vertices[v][0] * &p[a][b][k]
                                            + &vertices[v][1] * &p[a + 1][b][k]
                                            + &vertices[v][2] * &p[a][b + 1][k]
                                    })
                                })
                                .collect()
                        })
                        .collect();
                }
            }
            row.push(p[0][0].clone());
        }
        out.push(row);
    }
    Ok(out)
}
pub(super) fn subdivide(h: &Net, work: &mut Work) -> Result<Vec<Net>> {
    let v = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn(|j| if i == j { Q::one() } else { Q::zero() })
    });
    let mid = |i: usize, j: usize| std::array::from_fn(|k| (&v[i][k] + &v[j][k]) / integer(2));
    let (a, b, c) = (mid(0, 1), mid(1, 2), mid(2, 0));
    [
        [v[0].clone(), a.clone(), c.clone()],
        [a.clone(), v[1].clone(), b.clone()],
        [c.clone(), b.clone(), v[2].clone()],
        [a, b, c],
    ]
    .into_iter()
    .map(|p| restrict(h, p, work))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evaluate(h: &Net, p: [Q; 3]) -> sb::H {
        let n = h.len() - 1;
        std::array::from_fn(|k| {
            h.iter()
                .enumerate()
                .map(|(i, r)| {
                    r.iter()
                        .enumerate()
                        .map(|(j, h)| {
                            &h[k]
                                * Q::from_integer(multi(n, i, j).into())
                                * p[0].pow((n - i - j) as i32)
                                * p[1].pow(i as i32)
                                * p[2].pow(j as i32)
                        })
                        .sum::<Q>()
                })
                .sum()
        })
    }
    #[test]
    fn tensor_restriction_and_homotopy_keep_the_original_rational_map() {
        let h = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let w = Q::new((i + j + 2).into(), 2.into());
                        [
                            integer(i as i64) * &w,
                            integer(j as i64) * &w,
                            integer((i * j) as i64) * &w,
                            w,
                        ]
                    })
                    .collect()
            })
            .collect::<sb::Net>();
        let bary = [
            Q::new(1.into(), 6.into()),
            Q::new(1.into(), 3.into()),
            Q::new(1.into(), 2.into()),
        ];
        for uv in [
            [[0, 0], [1, 0], [1, 1]],
            [[0, 0], [1, 1], [0, 1]],
            [[0, 0], [1, 0], [0, 1]],
            [[1, 0], [1, 1], [0, 1]],
        ] {
            let t = triangle(&h, uv, &mut Work::default()).unwrap();
            let (u, v) = (
                (0..3).map(|i| &bary[i] * integer(uv[i][0])).sum::<Q>(),
                (0..3).map(|i| &bary[i] * integer(uv[i][1])).sum::<Q>(),
            );
            let original: sb::H = std::array::from_fn(|k| {
                h.iter()
                    .enumerate()
                    .map(|(i, r)| {
                        r.iter()
                            .enumerate()
                            .map(|(j, h)| {
                                &h[k]
                                    * choose(2, i)
                                    * choose(2, j)
                                    * u.pow(i as i32)
                                    * (Q::one() - &u).pow((2 - i) as i32)
                                    * v.pow(j as i32)
                                    * (Q::one() - &v).pow((2 - j) as i32)
                            })
                            .sum::<Q>()
                    })
                    .sum()
            });
            assert_eq!(evaluate(&t, bary.clone()), original);
            let [from, to] = homotopy(&t, &mut Work::default()).unwrap();
            assert_eq!(
                sb::point(&evaluate(&from, bary.clone())),
                sb::point(&original)
            );
            let c = corners(&t);
            let linear = std::array::from_fn(|k| (0..3).map(|i| &bary[i] * &c[i][k]).sum::<Q>());
            assert_eq!(sb::point(&evaluate(&to, bary.clone())), linear);
            for child in subdivide(&t, &mut Work::default()).unwrap() {
                assert!(child.iter().flatten().all(|p| p[3] > Q::zero()));
            }
        }
    }
    #[test]
    fn rational_quotient_derivatives_are_exact() {
        // S(u,v)=(u,v,0)/(1+u+2v), all homogeneous linear data.
        let h = vec![
            vec![
                [integer(0), integer(0), integer(0), integer(1)],
                [integer(0), integer(1), integer(0), integer(3)],
            ],
            vec![[integer(1), integer(0), integer(0), integer(2)]],
        ];
        let d = derivatives(&h, &mut Work::default()).unwrap();
        let p = [
            Q::new(1.into(), 2.into()),
            Q::new(1.into(), 3.into()),
            Q::new(1.into(), 6.into()),
        ];
        let eval = |a: &Scalar| {
            let net = a
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|q| [q.clone(), Q::zero(), Q::zero(), Q::one()])
                        .collect()
                })
                .collect();
            evaluate(&net, p.clone())[0].clone()
        };
        assert_eq!(eval(&d[0][0]), Q::one() + integer(2) * &p[2]);
        assert_eq!(eval(&d[0][1]), -&p[2]);
        assert_eq!(eval(&d[1][0]), -integer(2) * &p[1]);
        assert_eq!(eval(&d[1][1]), Q::one() + &p[1]);
    }
}

/// Exact rational composition of a tensor chart with a triangular UV map.
/// UV homogeneous numerators are normalized to the tensor's unit square.
pub(super) fn compose(h: &sb::Net, uv: &Net, work: &mut Work) -> Result<Net> {
    let d = uv.len() - 1;
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    if d * (n + m) > 33 {
        return Err(fail("trimmed atlas composition degree budget exceeded"));
    }
    let basis = |degree: usize, axis: usize, work: &mut Work| -> Result<Vec<Scalar>> {
        let x = scalar(uv, axis);
        let y = difference(&scalar(uv, 3), &x);
        let mut xp = vec![vec![vec![Q::one()]]];
        let mut yp = xp.clone();
        for i in 0..degree {
            xp.push(product(&xp[i], &x, work)?);
            yp.push(product(&yp[i], &y, work)?);
        }
        (0..=degree)
            .map(|i| {
                let mut p = product(&xp[i], &yp[degree - i], work)?;
                for q in p.iter_mut().flatten() {
                    *q *= choose(degree, i);
                }
                Ok(p)
            })
            .collect()
    };
    let (u, v) = (basis(n, 0, work)?, basis(m, 1, work)?);
    let degree = d * (n + m);
    let mut out = (0..=degree)
        .map(|i| vec![std::array::from_fn(|_| Q::zero()); degree + 1 - i])
        .collect::<Net>();
    for i in 0..=n {
        for j in 0..=m {
            let p = product(&u[i], &v[j], work)?;
            for (a, row) in p.iter().enumerate() {
                for (b, q) in row.iter().enumerate() {
                    for k in 0..4 {
                        out[a][b][k] += &h[i][j][k] * q;
                    }
                }
            }
        }
    }
    if out.iter().flatten().any(|p| p[3] <= Q::zero()) {
        return Err(fail("trimmed atlas composed weights are not positive"));
    }
    Ok(out)
}

/// Compose two rational triangular maps. The inner coordinates are barycentric
/// (1-u-v,u,v); all homogeneous controls must lie in the closed source triangle.
pub(super) fn compose_triangle(h: &Net, uv: &Net, work: &mut Work) -> Result<Net> {
    let (n, d) = (h.len() - 1, uv.len() - 1);
    if n * d > 16 {
        return Err(fail("nonlinear chart composition degree budget exceeded"));
    }
    if uv
        .iter()
        .flatten()
        .any(|p| p[3] <= Q::zero() || p[0] < Q::zero() || p[1] < Q::zero() || &p[0] + &p[1] > p[3])
    {
        return Err(fail("nonlinear chart transport leaves its source triangle"));
    }
    let u = scalar(uv, 0);
    let v = scalar(uv, 1);
    let w = difference(&difference(&scalar(uv, 3), &u), &v);
    let powers = |p: &Scalar, work: &mut Work| -> Result<Vec<Scalar>> {
        let mut out = vec![vec![vec![Q::one()]]];
        for i in 0..n {
            out.push(product(&out[i], p, work)?);
        }
        Ok(out)
    };
    let (u, v, w) = (powers(&u, work)?, powers(&v, work)?, powers(&w, work)?);
    let mut out = (0..=n * d)
        .map(|i| vec![std::array::from_fn(|_| Q::zero()); n * d + 1 - i])
        .collect::<Net>();
    for i in 0..=n {
        for j in 0..=n - i {
            let p = product(&product(&u[i], &v[j], work)?, &w[n - i - j], work)?;
            let c = integer(multi(n, i, j) as i64);
            for (a, row) in p.iter().enumerate() {
                for (b, q) in row.iter().enumerate() {
                    for k in 0..4 {
                        out[a][b][k] += &h[i][j][k] * q * &c;
                    }
                }
            }
        }
    }
    transport::check(&out)?;
    Ok(out)
}
