//! Exact graph reconstruction over a proved collapsed-edge planar disk.
//! This admits nonlinear rational angular/radial clocks without inverting them.
use super::planar_pole::{Tensor, product};
use super::*;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |q, i| {
        q * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn fit(mut rows: Vec<Vec<Q>>, count: usize, work: &mut Work) -> Result<Option<Vec<Q>>> {
    let mut rank = 0;
    let mut pivots = Vec::new();
    for col in 0..count {
        let Some(p) = (rank..rows.len()).find(|&i| !rows[i][col].is_zero()) else {
            continue;
        };
        rows.swap(rank, p);
        let scale = rows[rank][col].clone();
        for q in &mut rows[rank][col..] {
            *q /= &scale;
        }
        for i in 0..rows.len() {
            if i == rank {
                continue;
            }
            let scale = rows[i][col].clone();
            if scale.is_zero() {
                continue;
            }
            for j in col..=count {
                work.charge(1)?;
                let q = &scale * &rows[rank][j];
                rows[i][j] -= q;
                if rows[i][j].numer().bits() > 8192 || rows[i][j].denom().bits() > 8192 {
                    return Err(fail("pole graph elimination bit budget exceeded"));
                }
            }
        }
        pivots.push(col);
        rank += 1;
    }
    if rows
        .iter()
        .any(|r| r[..count].iter().all(Q::is_zero) && !r[count].is_zero())
    {
        return Ok(None);
    }
    let mut out = vec![Q::zero(); count];
    for (i, c) in pivots.into_iter().enumerate() {
        out[c] = rows[i][count].clone();
    }
    Ok(Some(out))
}
pub(super) fn chart(h: &sb::Net, work: &mut Work) -> Result<Option<Net>> {
    for axis in 0..3 {
        if let Some(chart) = chart_axis(h, axis, work)? {
            return Ok(Some(chart));
        }
    }
    Ok(None)
}
fn chart_axis(h: &sb::Net, axis: usize, work: &mut Work) -> Result<Option<Net>> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    if n == 0 || m == 0 {
        return Ok(None);
    }
    let a = sb::point(&h[0][0]);
    let b = sb::point(&h[n][m]);
    let c = sb::point(&h[0][m]);
    if h.iter().any(|r| sb::point(&r[0]) != a) {
        return Ok(None);
    }
    let e = sub(&c, &a);
    let f = sub(&b, &a);
    let normal = cross(&e, &f);
    if normal[axis].is_zero() {
        return Ok(None);
    }
    let direction: P = std::array::from_fn(|k| if k == axis { Q::one() } else { Q::zero() });
    let (ee, ff, ef) = (dot(&e, &e), dot(&f, &f), dot(&e, &f));
    let determinant = &ee * &ff - &ef * &ef;
    if determinant <= Q::zero() {
        return Ok(None);
    }
    let coordinates = h
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| {
                    let v = std::array::from_fn(|k| &p[k] - &a[k] * &p[3]);
                    let height = dot(&v, &normal) / &normal[axis];
                    let v = std::array::from_fn(|k| &v[k] - &direction[k] * &height);
                    [
                        (dot(&v, &e) * &ff - dot(&v, &f) * &ef) / &determinant,
                        (dot(&v, &f) * &ee - dot(&v, &e) * &ef) / &determinant,
                        height,
                        p[3].clone(),
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<sb::Net>();
    let projected = h
        .iter()
        .zip(&coordinates)
        .map(|(r, s)| {
            r.iter()
                .zip(s)
                .map(|(p, q)| {
                    std::array::from_fn(|k| {
                        if k == 3 {
                            p[3].clone()
                        } else {
                            &p[k] - &direction[k] * &q[2]
                        }
                    })
                })
                .collect()
        })
        .collect::<sb::Net>();
    if super::planar_pole::chart(&projected, work)?.is_none() {
        return Ok(None);
    }
    let scalar = |k: usize| {
        coordinates
            .iter()
            .map(|r| r.iter().map(|p| p[k].clone()).collect())
            .collect::<Tensor>()
    };
    let (x, y, z, w) = (scalar(0), scalar(1), scalar(2), scalar(3));
    let powers = |v: &Tensor, work: &mut Work| -> Result<Vec<Tensor>> {
        let mut out = vec![vec![vec![Q::one()]]];
        for i in 0..3 {
            out.push(product(&out[i], v, work)?);
        }
        Ok(out)
    };
    let (xp, yp, wp) = (powers(&x, work)?, powers(&y, work)?, powers(&w, work)?);
    for degree in 1..=3 {
        let indices = (0..=degree)
            .flat_map(|i| (0..=degree - i).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let terms = indices
            .iter()
            .map(|&(i, j)| product(&product(&xp[i], &yp[j], work)?, &wp[degree - i - j], work))
            .collect::<Result<Vec<_>>>()?;
        let rhs = product(&z, &wp[degree - 1], work)?;
        let rows = (0..rhs.len())
            .flat_map(|i| (0..rhs[0].len()).map(move |j| (i, j)))
            .map(|(i, j)| {
                let mut r = terms.iter().map(|t| t[i][j].clone()).collect::<Vec<_>>();
                r.push(rhs[i][j].clone());
                r
            })
            .collect();
        let Some(coefficients) = fit(rows, indices.len(), work)? else {
            continue;
        };
        // Independently check every tensor coefficient; interpolation alone has
        // no authority. This is Z W^(d-1)=g(X,Y,W), over the complete source cell.
        for i in 0..rhs.len() {
            for j in 0..rhs[0].len() {
                let sum: Q = terms
                    .iter()
                    .zip(&coefficients)
                    .map(|(t, c)| &t[i][j] * c)
                    .sum();
                if sum != rhs[i][j] {
                    return Err(fail("pole graph identity mismatch"));
                }
            }
        }
        let mut out = (0..=degree)
            .map(|i| vec![std::array::from_fn(|_| Q::zero()); degree + 1 - i])
            .collect::<Net>();
        for (i, row) in out.iter_mut().enumerate() {
            for (j, p) in row.iter_mut().enumerate() {
                let mut height = Q::zero();
                for (&(r, s), q) in indices.iter().zip(&coefficients) {
                    if r <= i && s <= j {
                        height += q * choose(i, r) * choose(j, s)
                            / (choose(degree, r + s) * choose(r + s, r));
                    }
                }
                for k in 0..3 {
                    p[k] = &a[k]
                        + &e[k] * Q::new(i.into(), degree.into())
                        + &f[k] * Q::new(j.into(), degree.into())
                        + &direction[k] * &height;
                }
                p[3] = Q::one();
            }
        }
        transport::check(&out)?;
        return Ok(Some(out));
    }
    // Rational graph Z/W=N(X/W,Y/W)/D(X/W,Y/W). A nullspace
    // candidate has no authority until its full tensor identity and positive
    // denominator on the entire target triangle have been checked.
    for degree in 1..=3 {
        let indices = (0..=degree)
            .flat_map(|i| (0..=degree - i).map(move |j| (i, j)))
            .collect::<Vec<_>>();
        let terms = indices
            .iter()
            .map(|&(i, j)| product(&product(&xp[i], &yp[j], work)?, &wp[degree - i - j], work))
            .collect::<Result<Vec<_>>>()?;
        if let Some(h) = rational_graph(
            &terms, &z, &w, &indices, degree, &a, &e, &f, &direction, work,
        )? {
            return Ok(Some(h));
        }
    }
    Ok(None)
}
fn polynomial(coefficients: &[Q], indices: &[(usize, usize)], degree: usize) -> Scalar {
    (0..=degree)
        .map(|i| {
            (0..=degree - i)
                .map(|j| {
                    indices
                        .iter()
                        .zip(coefficients)
                        .filter(|((r, s), _)| *r <= i && *s <= j)
                        .map(|(&(r, s), q)| {
                            q * choose(i, r) * choose(j, s)
                                / (choose(degree, r + s) * choose(r + s, r))
                        })
                        .sum()
                })
                .collect()
        })
        .collect()
}
#[allow(clippy::too_many_arguments)]
fn rational_graph(
    terms: &[Tensor],
    z: &Tensor,
    w: &Tensor,
    indices: &[(usize, usize)],
    degree: usize,
    a: &P,
    e: &P,
    f: &P,
    direction: &P,
    work: &mut Work,
) -> Result<Option<Net>> {
    let nw = terms
        .iter()
        .map(|t| product(w, t, work))
        .collect::<Result<Vec<_>>>()?;
    let dz = terms
        .iter()
        .map(|t| product(z, t, work))
        .collect::<Result<Vec<_>>>()?;
    let count = terms.len();
    let rows = (0..nw[0].len())
        .flat_map(|i| (0..nw[0][0].len()).map(move |j| (i, j)))
        .map(|(i, j)| {
            nw.iter()
                .map(|t| -t[i][j].clone())
                .chain(dz.iter().map(|t| t[i][j].clone()))
                .collect::<Vec<_>>()
        })
        .collect();
    for coefficients in super::linear::kernel(rows, 2 * count, work)? {
        let mut numerator = polynomial(&coefficients[..count], indices, degree);
        let mut denominator = polynomial(&coefficients[count..], indices, degree);
        let negative = denominator.iter().flatten().all(|q| *q < Q::zero());
        if negative {
            for q in numerator
                .iter_mut()
                .flatten()
                .chain(denominator.iter_mut().flatten())
            {
                *q = -q.clone();
            }
        }
        if !denominator.iter().flatten().all(|q| *q > Q::zero()) {
            continue;
        }
        for i in 0..nw[0].len() {
            for j in 0..nw[0][0].len() {
                let residual: Q = (0..count)
                    .map(|k| {
                        &dz[k][i][j] * &coefficients[count + k] - &nw[k][i][j] * &coefficients[k]
                    })
                    .sum();
                if !residual.is_zero() {
                    return Err(fail("rational pole graph identity mismatch"));
                }
            }
        }
        let one = vec![vec![Q::one(), Q::one()], vec![Q::one()]];
        let den = bernstein::product(&denominator, &one, work)?;
        let num = bernstein::product(&numerator, &one, work)?;
        let coordinates = (0..3)
            .map(|k| {
                bernstein::product(
                    &denominator,
                    &vec![vec![a[k].clone(), &a[k] + &f[k]], vec![&a[k] + &e[k]]],
                    work,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let mut out = (0..=degree + 1)
            .map(|i| vec![std::array::from_fn(|_| Q::zero()); degree + 2 - i])
            .collect::<Net>();
        for (i, row) in out.iter_mut().enumerate() {
            for (j, p) in row.iter_mut().enumerate() {
                for k in 0..3 {
                    p[k] = &coordinates[k][i][j] + &direction[k] * &num[i][j];
                }
                p[3] = den[i][j].clone();
            }
        }
        transport::check(&out)?;
        return Ok(Some(out));
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonpolynomial_rational_graph_retains_the_complete_source() {
        // S=(u v,v,v²/(1+v)), with a genuinely rational graph lift.
        let h = (0..2)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let v = Q::new(j.into(), 2.into());
                        let vw = match j {
                            0 => Q::zero(),
                            1 => Q::new(1.into(), 2.into()),
                            _ => integer(2),
                        };
                        [
                            integer(i) * &vw,
                            vw,
                            if j == 2 { Q::one() } else { Q::zero() },
                            Q::one() + v,
                        ]
                    })
                    .collect()
            })
            .collect::<sb::Net>();
        let out = chart(&h, &mut Work::default()).unwrap().unwrap();
        assert!(out.iter().flatten().all(|p| p[3] > Q::zero()));
        assert!(out.iter().flatten().any(|p| p[3] != out[0][0][3]));
        let mut bad = h;
        bad[1][1][0] = integer(40);
        assert!(chart(&bad, &mut Work::default()).unwrap().is_none());
    }
    #[test]
    fn nonplanar_graph_pole_removes_a_nonmonomial_angular_clock() {
        // x=u² v/(2-u²), y=v, z=y². The source is curved and has
        // u-dependent rational weight at the collapsed edge, but its image
        // is the regular graph (x,y,y²) on 0<=x<=y<=1.
        let num = [integer(0), integer(0), integer(1)];
        let den = [integer(2), integer(2), integer(1)];
        let h = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let v = Q::new(j.into(), 2.into());
                        [
                            &num[i] * &v,
                            &den[i] * &v,
                            if j == 2 { den[i].clone() } else { Q::zero() },
                            den[i].clone(),
                        ]
                    })
                    .collect()
            })
            .collect::<sb::Net>();
        assert!(
            super::super::planar_pole::chart(&h, &mut Work::default())
                .unwrap()
                .is_none()
        );
        let out = chart(&h, &mut Work::default()).unwrap().unwrap();
        assert_eq!(out.len(), 3);
        // Lifted triangular controls must still prove the full atlas predicates.
        let p = Patch {
            ids: [0, 1, 2],
            derivatives: bernstein::derivatives(&out, &mut Work::default()).unwrap(),
            motion: bernstein::homotopy(&out, &mut Work::default()).unwrap(),
            h: out,
        };
        assert!(
            regular(
                &p,
                &[integer(0), integer(0), integer(-1)],
                &mut Work::default()
            )
            .unwrap()
        );
        let mut bad = h;
        bad[1][1][0] = integer(40);
        assert!(chart(&bad, &mut Work::default()).unwrap().is_none());
    }
}
