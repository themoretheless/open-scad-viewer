//! Exact homogeneous knot restriction and Bernstein homotopy bounds.
use super::*;
use nurbs_core::surface::Surface;
mod composition;
mod trace;
pub(super) use composition::{affine, affine_curve_error, fan};
pub(super) use trace::{Trace, continuous, same_trace, validate_uv};
pub(super) type H = [Q; 4];
pub(super) type Net = Vec<Vec<H>>;
fn choose(n: usize, k: usize) -> Q {
    let mut v = Q::one();
    for i in 0..k {
        v = v * integer((n - i) as i64) / integer((i + 1) as i64)
    }
    v
}
fn restrict(h: &[H], knots: &[f64], degree: usize, a: &Q, b: &Q) -> Result<Vec<H>> {
    let knots = knots
        .iter()
        .map(|x| Q::from_float(*x).unwrap())
        .collect::<Vec<_>>();
    let span = (degree..h.len())
        .find(|i| knots[*i] <= *a && *b <= knots[*i + 1] && a <= b && knots[*i] < knots[*i + 1])
        .ok_or_else(|| fail("surface restriction crosses an unpartitioned knot"))?;
    let binom: Vec<Q> = (0..=degree).map(|j| choose(degree, j)).collect();
    let mut rows = Vec::with_capacity(degree + 1);
    for i in 0..=degree {
        let u = Q::new(i.into(), degree.into());
        let one_minus_u = Q::one() - &u;
        let t = a + (b - a) * &u;
        let mut d = h[span - degree..=span].to_vec();
        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let k = span - degree + j;
                let den = &knots[k + degree - r + 1] - &knots[k];
                if den <= Q::zero() {
                    return Err(fail("empty exact surface restriction denominator"));
                }
                let alpha = (&t - &knots[k]) / den;
                let one_minus_alpha = Q::one() - &alpha;
                d[j] = std::array::from_fn(|k| {
                    &one_minus_alpha * &d[j - 1][k] + &alpha * &d[j][k]
                });
            }
        }
        let mut row = Vec::with_capacity(degree + 5);
        for (j, b_j) in binom.iter().enumerate() {
            row.push(b_j * u.pow(j as i32) * one_minus_u.pow((degree - j) as i32));
        }
        row.extend(d.pop().unwrap());
        rows.push(row);
    }
    for c in 0..=degree {
        let pivot = (c..=degree)
            .find(|i| !rows[*i][c].is_zero())
            .ok_or_else(|| fail("singular surface Bernstein restriction"))?;
        rows.swap(c, pivot);
        let scale = rows[c][c].clone();
        for x in &mut rows[c][c..] {
            *x /= &scale
        }
        for i in 0..=degree {
            if i == c {
                continue;
            }
            let scale = rows[i][c].clone();
            for j in c..degree + 5 {
                let q = &scale * &rows[c][j];
                rows[i][j] -= q
            }
        }
    }
    let net = rows
        .into_iter()
        .map(|r| std::array::from_fn(|k| r[degree + 1 + k].clone()))
        .collect::<Vec<H>>();
    if net.iter().any(|p| {
        p[3] <= Q::zero()
            || p.iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    }) {
        return Err(fail(
            "nonpositive or oversized exact restricted surface coefficient",
        ));
    }
    Ok(net)
}
pub(super) fn cell(s: &Surface, u: [Q; 2], v: [Q; 2]) -> Result<Net> {
    let h = s
        .control_points
        .iter()
        .zip(&s.weights)
        .map(|(row, w)| {
            row.iter()
                .zip(w)
                .map(|(p, w)| {
                    let w = Q::from_float(*w).unwrap();
                    [
                        Q::from_float(p[0]).unwrap() * &w,
                        Q::from_float(p[1]).unwrap() * &w,
                        Q::from_float(p[2]).unwrap() * &w,
                        w,
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut first = Vec::with_capacity(h[0].len());
    for j in 0..h[0].len() {
        first.push(restrict(
            &h.iter().map(|r| r[j].clone()).collect::<Vec<_>>(),
            &s.knots_u,
            s.degree_u,
            &u[0],
            &u[1],
        )?)
    }
    (0..=s.degree_u)
        .map(|i| {
            restrict(
                &first.iter().map(|r| r[i].clone()).collect::<Vec<_>>(),
                &s.knots_v,
                s.degree_v,
                &v[0],
                &v[1],
            )
        })
        .collect()
}
pub(super) fn point(h: &H) -> P {
    std::array::from_fn(|i| &h[i] / &h[3])
}
pub(super) fn multiply(a: &[Q], b: &[Q]) -> Vec<Q> {
    let (n, m) = (a.len() - 1, b.len() - 1);
    let ca = (0..=n).map(|i| choose(n, i)).collect::<Vec<_>>();
    let cb = (0..=m).map(|j| choose(m, j)).collect::<Vec<_>>();
    let cc = (0..=n + m).map(|k| choose(n + m, k)).collect::<Vec<_>>();
    let mut c = vec![Q::zero(); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            if !a[i].is_zero() && !b[j].is_zero() {
                c[i + j] += &a[i] * &b[j] * &ca[i] * &cb[j] / &cc[i + j];
            }
        }
    }
    c
}
pub(super) fn same(a: &[H], b: &[H]) -> bool {
    let aw = a.iter().map(|p| p[3].clone()).collect::<Vec<_>>();
    let bw = b.iter().map(|p| p[3].clone()).collect::<Vec<_>>();
    (0..3).all(|k| {
        multiply(&a.iter().map(|p| p[k].clone()).collect::<Vec<_>>(), &bw)
            == multiply(&b.iter().map(|p| p[k].clone()).collect::<Vec<_>>(), &aw)
    })
}
pub(super) fn edges(h: &Net) -> [Vec<H>; 4] {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    [
        (0..=n).map(|i| h[i][0].clone()).collect(),
        h[n].clone(),
        (0..=n).rev().map(|i| h[i][m].clone()).collect(),
        h[0].iter().rev().cloned().collect(),
    ]
}
pub(super) fn corners(h: &Net) -> [P; 4] {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    [
        point(&h[0][0]),
        point(&h[n][0]),
        point(&h[n][m]),
        point(&h[0][m]),
    ]
}
pub(super) fn planar(h: &Net) -> bool {
    let points = h.iter().flatten().map(point).collect::<Vec<_>>();
    let a = &points[0];
    for b in &points[1..] {
        for c in &points[1..] {
            let n = cross(&sub(b, a), &sub(c, a));
            if !zero(&n) {
                return points.iter().all(|p| dot(&n, &sub(p, a)).is_zero());
            }
        }
    }
    true
}
/// Bound S-L on both triangles of a parameter cell. N-W*B is elevated
/// exactly to bidegree (p+1,q+1), then |B-L| <= |mixed corner difference|/4.
pub(super) fn error(h: &Net) -> P {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let c = corners(h);
    let b = [[c[0].clone(), c[3].clone()], [c[1].clone(), c[2].clone()]];
    let wmin = h.iter().flatten().map(|p| &p[3]).min().unwrap();
    let cn: Vec<Q> = (0..=n).map(|i| choose(n, i)).collect();
    let cn1: Vec<Q> = (0..=n + 1).map(|i| choose(n + 1, i)).collect();
    let cm: Vec<Q> = (0..=m).map(|j| choose(m, j)).collect();
    let cm1: Vec<Q> = (0..=m + 1).map(|j| choose(m + 1, j)).collect();
    let four = integer(4);
    std::array::from_fn(|axis| {
        let mut maximum = Q::zero();
        for i in 0..=n + 1 {
            for j in 0..=m + 1 {
                let mut value = Q::zero();
                for x in 0..=1 {
                    for y in 0..=1 {
                        if i < x || j < y || i - x > n || j - y > m {
                            continue;
                        }
                        let p = &h[i - x][j - y];
                        let weight = (&cn[i - x] / &cn1[i]) * (&cm[j - y] / &cm1[j]);
                        value += weight * (&p[axis] - &p[3] * &b[x][y][axis]);
                    }
                }
                maximum = maximum.max(value.abs());
            }
        }
        maximum / wmin + (&c[0][axis] + &c[2][axis] - &c[1][axis] - &c[3][axis]).abs() / &four
    })
}
pub(super) fn curve(c: &crate::Curve, a: &Q, b: &Q) -> Result<Vec<H>> {
    c.validate()?;
    if !(1..=8).contains(&c.degree) || ![2, 3].contains(&c.control_points[0].len()) {
        return Err(fail("trim curve dimension or degree budget"));
    }
    let net = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(p, w)| {
            let w = Q::from_float(*w).unwrap();
            [
                Q::from_float(p[0]).unwrap() * &w,
                Q::from_float(p[1]).unwrap() * &w,
                if p.len() == 3 {
                    Q::from_float(p[2]).unwrap() * &w
                } else {
                    Q::zero()
                },
                w,
            ]
        })
        .collect::<Vec<_>>();
    restrict(&net, &c.knots, c.degree, a, b)
}
pub(super) fn at(s: &Surface, p: &[Q; 2]) -> Result<P> {
    Ok(point(
        &cell(
            s,
            [p[0].clone(), p[0].clone()],
            [p[1].clone(), p[1].clone()],
        )?[0][0],
    ))
}
/// All exact restricted controls over a closed UV box, including both sides of
/// internal knots. A zero-width interval remains a true surface restriction.
pub(super) fn controls(s: &Surface, lo: &[Q; 2], hi: &[Q; 2]) -> Result<Vec<P>> {
    let intervals = |k: &[f64], degree: usize, count: usize, axis: usize| -> Result<Vec<[Q; 2]>> {
        if lo[axis] > hi[axis]
            || lo[axis] < Q::from_float(k[degree]).unwrap()
            || hi[axis] > Q::from_float(k[count]).unwrap()
        {
            return Err(fail("trim fan leaves its authored surface domain"));
        }
        if lo[axis] == hi[axis] {
            return Ok(vec![[lo[axis].clone(), hi[axis].clone()]]);
        }
        let mut values = vec![lo[axis].clone(), hi[axis].clone()];
        values.extend(
            k.iter()
                .map(|x| Q::from_float(*x).unwrap())
                .filter(|x| lo[axis] < *x && *x < hi[axis]),
        );
        values.sort();
        values.dedup();
        Ok(values
            .windows(2)
            .map(|i| [i[0].clone(), i[1].clone()])
            .collect())
    };
    let u = intervals(&s.knots_u, s.degree_u, s.control_points.len(), 0)?;
    let v = intervals(&s.knots_v, s.degree_v, s.control_points[0].len(), 1)?;
    if u.len() * v.len() > 256 {
        return Err(fail("trim fan knot coverage budget"));
    }
    let mut out = Vec::new();
    for a in u {
        for b in &v {
            out.extend(cell(s, a.clone(), b.clone())?.iter().flatten().map(point))
        }
    }
    Ok(out)
}
/// Exact rational composition of one pcurve cell with its original tensor
/// chart. Bernstein products retain every source bit and positive denominator.
fn lift_chart(s: &Surface, p: &[H], probe: Option<&[Q; 2]>, work: &mut Work) -> Result<Vec<H>> {
    let choose_span = |knots: &[f64], degree: usize, count: usize, axis: usize| -> Result<[Q; 2]> {
        let values = if let Some(probe) = probe {
            vec![probe[axis].clone()]
        } else {
            p.iter().map(|p| &p[axis] / &p[3]).collect::<Vec<_>>()
        };
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
            .ok_or_else(|| fail("trim seam crosses an unresolved original surface knot"))
    };
    let du = choose_span(&s.knots_u, s.degree_u, s.control_points.len(), 0)?;
    let dv = choose_span(&s.knots_v, s.degree_v, s.control_points[0].len(), 1)?;
    let h = cell(s, du.clone(), dv.clone())?;
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let basis = |axis: usize, d: &[Q; 2], degree: usize| -> Vec<Vec<Q>> {
        let u = p
            .iter()
            .map(|p| (&p[axis] - &d[0] * &p[3]) / (&d[1] - &d[0]))
            .collect::<Vec<_>>();
        let v = p.iter().zip(&u).map(|(p, u)| &p[3] - u).collect::<Vec<_>>();
        let power = |p: &Vec<Q>, n: usize| (0..n).fold(vec![Q::one()], |a, _| multiply(&a, p));
        (0..=degree)
            .map(|i| {
                multiply(&power(&u, i), &power(&v, degree - i))
                    .iter()
                    .map(|x| x * choose(degree, i))
                    .collect()
            })
            .collect()
    };
    let u = basis(0, &du, n);
    let v = basis(1, &dv, m);
    let degree = (p.len() - 1) * (n + m);
    if degree > 128 {
        return Err(fail("trim composition degree exceeds 128"));
    }
    let mut out = vec![std::array::from_fn(|_| Q::zero()); degree + 1];
    for i in 0..=n {
        for j in 0..=m {
            work.charge((u[i].len() * v[j].len() * 4) as usize)?;
            let b = multiply(&u[i], &v[j]);
            for (k, b) in b.iter().enumerate() {
                for a in 0..4 {
                    out[k][a] += &h[i][j][a] * b
                }
            }
        }
    }
    if out.iter().any(|p| {
        probe.is_none() && p[3] <= Q::zero()
            || p.iter()
                .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    }) {
        return Err(fail("trim composition weight or bit budget"));
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn evaluate(h: &Net, u: &Q, v: &Q) -> P {
        let bezier = |mut p: Vec<H>, t: &Q| {
            while p.len() > 1 {
                p = p
                    .windows(2)
                    .map(|x| std::array::from_fn(|i| (&Q::one() - t) * &x[0][i] + t * &x[1][i]))
                    .collect()
            }
            p.pop().unwrap()
        };
        point(&bezier(h.iter().map(|r| bezier(r.clone(), v)).collect(), u))
    }
    #[test]
    fn continuous_bounds_cover_both_quad_diagonals_with_rational_weights() {
        for degree in [2usize, 4, 8] {
            let h = (0..=degree)
                .map(|i| {
                    (0..=degree)
                        .map(|j| {
                            let w = integer((1 + (i + j) % 3) as i64);
                            [
                                integer(i as i64) * &w,
                                integer(j as i64) * &w,
                                integer(((i * j + 2 * i + j) % 5) as i64) * &w,
                                w,
                            ]
                        })
                        .collect()
                })
                .collect();
            let e = error(&h);
            let c = corners(&h);
            for i in 0..=7 {
                for j in 0..=7 {
                    let u = Q::new(i.into(), 7.into());
                    let v = Q::new(j.into(), 7.into());
                    let s = evaluate(&h, &u, &v);
                    let first = if u >= v {
                        [(&Q::one() - &u, 0), (&u - &v, 1), (v.clone(), 2)]
                    } else {
                        [(&Q::one() - &v, 0), (u.clone(), 2), (&v - &u, 3)]
                    };
                    let second = if &u + &v <= Q::one() {
                        [(&Q::one() - &u - &v, 0), (u.clone(), 1), (v.clone(), 3)]
                    } else {
                        [
                            (&Q::one() - &v, 1),
                            (&u + &v - Q::one(), 2),
                            (&Q::one() - &u, 3),
                        ]
                    };
                    for bary in [first, second] {
                        for k in 0..3 {
                            let p = bary.iter().map(|(w, i)| w * &c[*i][k]).sum::<Q>();
                            assert!((&s[k] - p).abs() <= e[k]);
                        }
                    }
                }
            }
        }
    }
}
