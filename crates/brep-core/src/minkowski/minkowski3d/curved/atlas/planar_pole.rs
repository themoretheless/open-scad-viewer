//! Planar collapsed-edge charts with arbitrary rational interior clocks.
//! Boundary degree plus a strictly positive interior Jacobian proves a disk
//! homeomorphism after collapsing the pole edge. No coordinate approximation.
use super::*;
pub(super) type Tensor = Vec<Vec<Q>>;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |v, i| {
        v * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
pub(super) fn product(a: &Tensor, b: &Tensor, work: &mut Work) -> Result<Tensor> {
    let (n, m, r, s) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    if n + r > 48 || m + s > 48 {
        return Err(fail("planar pole degree budget exceeded"));
    }
    let mut c = vec![vec![Q::zero(); m + s + 1]; n + r + 1];
    for i in 0..=n {
        for j in 0..=m {
            if a[i][j].is_zero() {
                continue;
            }
            for k in 0..=r {
                for l in 0..=s {
                    if b[k][l].is_zero() {
                        continue;
                    }
                    work.charge(1)?;
                    c[i + k][j + l] += &a[i][j] * &b[k][l] * choose(n, i) * choose(r, k)
                        / choose(n + r, i + k)
                        * choose(m, j)
                        * choose(s, l)
                        / choose(m + s, j + l);
                }
            }
        }
    }
    if c.iter()
        .flatten()
        .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(fail("planar pole coefficient bit budget exceeded"));
    }
    Ok(c)
}
fn derivative(a: &Tensor, axis: usize) -> Tensor {
    let (n, m) = (a.len() - 1, a[0].len() - 1);
    if axis == 0 {
        (0..n)
            .map(|i| {
                (0..=m)
                    .map(|j| integer(n as i64) * (&a[i + 1][j] - &a[i][j]))
                    .collect()
            })
            .collect()
    } else {
        (0..=n)
            .map(|i| {
                (0..m)
                    .map(|j| integer(m as i64) * (&a[i][j + 1] - &a[i][j]))
                    .collect()
            })
            .collect()
    }
}
fn difference(a: &Tensor, b: &Tensor) -> Tensor {
    a.iter()
        .zip(b)
        .map(|(r, s)| r.iter().zip(s).map(|(x, y)| x - y).collect())
        .collect()
}
fn segment(c: &[sb::H], a: &P, b: &P, work: &mut Work) -> Result<bool> {
    if sb::point(&c[0]) != *a || sb::point(c.last().unwrap()) != *b {
        return Ok(false);
    }
    let d = sub(b, a);
    if zero(&d) || c.iter().any(|p| !zero(&cross(&sub(&sb::point(p), a), &d))) {
        return Ok(false);
    }
    let x = c
        .iter()
        .map(|p| dot(&std::array::from_fn(|k| p[k].clone()), &d))
        .collect::<Vec<_>>();
    let w = c.iter().map(|p| p[3].clone()).collect::<Vec<_>>();
    let n = c.len() - 1;
    let dx = x
        .windows(2)
        .map(|p| integer(n as i64) * (&p[1] - &p[0]))
        .collect::<Vec<_>>();
    let dw = w
        .windows(2)
        .map(|p| integer(n as i64) * (&p[1] - &p[0]))
        .collect::<Vec<_>>();
    work.charge(2 * n * (n + 1))?;
    let a = sb::multiply(&dx, &w);
    let b = sb::multiply(&x, &dw);
    let coefficients = a.iter().zip(b).map(|(a, b)| a - b).collect::<Vec<_>>();
    Ok(coefficients.iter().all(|q| *q >= Q::zero()) && coefficients.iter().any(|q| *q > Q::zero()))
}
pub(super) fn chart(h: &sb::Net, work: &mut Work) -> Result<Option<Net>> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    if n == 0 || m == 0 || h.iter().flatten().any(|p| p[3] <= Q::zero()) {
        return Ok(None);
    }
    let a = sb::point(&h[0][0]);
    let b = sb::point(&h[n][m]);
    let c = sb::point(&h[0][m]);
    let normal = cross(&sub(&b, &a), &sub(&c, &a));
    if zero(&normal)
        || h.iter().any(|r| sb::point(&r[0]) != a)
        || h.iter()
            .flatten()
            .any(|p| !dot(&sub(&sb::point(p), &a), &normal).is_zero())
    {
        return Ok(None);
    }
    if !segment(&h[n], &a, &b, work)?
        || !segment(&h[0], &a, &c, work)?
        || !segment(
            &h.iter().map(|r| r[m].clone()).collect::<Vec<_>>(),
            &c,
            &b,
            work,
        )?
    {
        return Ok(None);
    }
    let w = h
        .iter()
        .map(|r| r.iter().map(|p| p[3].clone()).collect())
        .collect::<Tensor>();
    let mut ds = Vec::new();
    for axis in 0..2 {
        let mut d = Vec::new();
        for k in 0..3 {
            let x = h
                .iter()
                .map(|r| r.iter().map(|p| p[k].clone()).collect())
                .collect::<Tensor>();
            d.push(difference(
                &product(&derivative(&x, axis), &w, work)?,
                &product(&x, &derivative(&w, axis), work)?,
            ));
        }
        ds.push(d);
    }
    let mut determinant = None::<Tensor>;
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        let v = difference(
            &product(&ds[0][i], &ds[1][j], work)?,
            &product(&ds[0][j], &ds[1][i], work)?,
        );
        let out = determinant.get_or_insert_with(|| vec![vec![Q::zero(); v[0].len()]; v.len()]);
        for (r, s) in out.iter_mut().zip(v) {
            for (x, y) in r.iter_mut().zip(s) {
                *x += y * &normal[k];
            }
        }
    }
    let d = determinant.unwrap();
    if d.iter().flatten().any(|q| *q < Q::zero()) || !d.iter().flatten().any(|q| *q > Q::zero()) {
        return Ok(None);
    }
    // All tensor basis functions are positive in the open square. Thus the
    // nonnegative, nonzero determinant is strictly positive there. The boundary
    // is one oriented triangle with only v=0 collapsed. Planar degree is one:
    // every interior point has exactly one preimage, exterior points none.
    let point = |p: P| [p[0].clone(), p[1].clone(), p[2].clone(), Q::one()];
    let out = vec![vec![point(a), point(b)], vec![point(c)]];
    transport::check(&out)?;
    Ok(Some(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonmonomial_rational_planar_pole_keeps_the_complete_triangle() {
        // Angular clock u²/(2-u), radial clock v²/(2-v).
        // S=(angular*radial,radial,radial/8). Source W depends on both axes.
        let clock = [
            [integer(0), integer(2)],
            [integer(0), Q::new(3.into(), 2.into())],
            [integer(1), integer(1)],
        ];
        let h = clock
            .iter()
            .map(|u| {
                clock
                    .iter()
                    .map(|v| {
                        [
                            &u[0] * &v[0],
                            &u[1] * &v[0],
                            &u[1] * &v[0] / integer(8),
                            &u[1] * &v[1],
                        ]
                    })
                    .collect()
            })
            .collect::<sb::Net>();
        assert!(
            transport::pole_base(&h, &mut Work::default())
                .unwrap()
                .is_none()
        );
        let out = chart(&h, &mut Work::default()).unwrap().unwrap();
        assert_eq!(out.len(), 2);
        let mut bad = h.clone();
        bad[1][1][2] += Q::new(1.into(), 32.into());
        assert!(chart(&bad, &mut Work::default()).unwrap().is_none());
        // The same oriented boundary cannot authorize a folded interior.
        let mut folded = h;
        folded[1][1][0] = integer(20);
        assert!(chart(&folded, &mut Work::default()).unwrap().is_none());
    }
}
