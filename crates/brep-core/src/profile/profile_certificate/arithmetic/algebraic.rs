//! Independent algebraic endpoint-only census for overlapping cusp hulls.
//! Exact rational moving-line identities construct an annihilating polynomial.
//! Sturm checks its entire open parameter interval, including multiple roots.
//! Extra eliminant roots cause refusal, never a false negative intersection.
use super::*;
fn dot(line: &[Q], h: &[Poly; 3]) -> Poly {
    let mut out = Vec::new();
    for k in 0..3 {
        out = sub(&out, &h[k].iter().map(|x| -x * &line[k]).collect::<Poly>());
    }
    out
}
fn homogeneous(net: &[[Q; 3]]) -> Result<[Poly; 3]> {
    let mut h = std::array::from_fn(|k| power(net, k));
    let mut g = h[2].clone();
    for p in &h[..2] {
        g = gcd(&g, p)?;
    }
    if g.len() > 1 {
        for p in &mut h {
            *p = quotient(p, &g)?;
        }
    }
    // The source denominator is positive. Its exact common factor has no
    // zero on [0,1], so cancellation changes neither image nor finite domain.
    Ok(h)
}
fn kernel(mut rows: Vec<Vec<Q>>) -> Vec<Vec<Q>> {
    let mut pivots = Vec::new();
    for col in 0..6 {
        let k = pivots.len();
        let Some(i) = (k..rows.len()).find(|i| !rows[*i][col].is_zero()) else {
            continue;
        };
        rows.swap(k, i);
        let scale = rows[k][col].clone();
        for v in &mut rows[k] {
            *v /= &scale;
        }
        for i in 0..rows.len() {
            if i == k {
                continue;
            }
            let scale = rows[i][col].clone();
            for j in col..6 {
                let v = &rows[k][j] * &scale;
                rows[i][j] -= v;
            }
        }
        pivots.push(col);
    }
    let mut out = Vec::new();
    for free in (0..6).filter(|i| !pivots.contains(i)) {
        let mut v = vec![Q::zero(); 6];
        v[free] = Q::one();
        for (i, col) in pivots.iter().enumerate() {
            v[*col] = -&rows[i][free];
        }
        out.push(v);
    }
    out
}
fn pow(p: &[Q], n: usize) -> Poly {
    let mut out = vec![Q::one()];
    for _ in 0..n {
        out = product(&out, p);
    }
    out
}
fn form(p: &[Q], degree: usize, x: &[Q], y: &[Q]) -> Poly {
    let mut out = Vec::new();
    for (i, c) in p.iter().enumerate() {
        let v = product(&pow(x, i), &pow(y, degree - i))
            .into_iter()
            .map(|x| -x * c)
            .collect::<Poly>();
        out = sub(&out, &v);
    }
    out
}
fn endpoint_match(a: &[[Q; 3]], b: &[Q; 3]) -> bool {
    [a.first().unwrap(), a.last().unwrap()]
        .iter()
        .any(|p| &p[0] * &b[2] == &b[0] * &p[2] && &p[1] * &b[2] == &b[1] * &p[2])
}
fn census(f: &[Q], a: &[[Q; 3]], b: &[[Q; 3]], adjacent: bool) -> Result<bool> {
    if f.is_empty() || interior_roots(f)? > 0 {
        return Ok(false);
    }
    for (t, point) in [
        (Q::zero(), b.first().unwrap()),
        (Q::one(), b.last().unwrap()),
    ] {
        if at(f, &t).is_zero() && (!adjacent || !endpoint_match(a, point)) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn one_way(a: &[[Q; 3]], b: &[[Q; 3]], adjacent: bool) -> Result<bool> {
    let h = homogeneous(a)?;
    let other = homogeneous(b)?;
    let degree = h.iter().map(|p| p.len().saturating_sub(1)).max().unwrap();
    if degree == 1 {
        let p = std::array::from_fn::<_, 3, _>(|k| h[k].first().cloned().unwrap_or_else(Q::zero));
        let q = std::array::from_fn::<_, 3, _>(|k| h[k].get(1).cloned().unwrap_or_else(Q::zero));
        let line = std::array::from_fn::<_, 3, _>(|k| {
            &p[(k + 1) % 3] * &q[(k + 2) % 3] - &p[(k + 2) % 3] * &q[(k + 1) % 3]
        });
        if !dot(&line, &h).is_empty() {
            return Err(refused("Invalid exact line annihilator"));
        }
        return census(&dot(&line, &other), a, b, adjacent);
    }
    if !(2..=3).contains(&degree) {
        return Ok(false);
    }
    let matrix: Vec<Vec<Q>> = (0..degree + 2)
        .map(|i| {
            (0..6)
                .map(|j| {
                    if i < j / 3 {
                        Q::zero()
                    } else {
                        h[j % 3].get(i - j / 3).cloned().unwrap_or_else(Q::zero)
                    }
                })
                .collect()
        })
        .collect();
    for line in kernel(matrix.clone()) {
        if matrix
            .iter()
            .any(|row| row.iter().zip(&line).fold(Q::zero(), |s, (x, y)| s + x * y) != Q::zero())
        {
            return Err(refused("Invalid exact moving-line witness"));
        }
        let cross: Vec<_> = (0..3)
            .map(|k| {
                let (i, j) = ((k + 1) % 3, (k + 2) % 3);
                &line[i] * &line[3 + j] - &line[j] * &line[3 + i]
            })
            .collect();
        let Some(axis) = cross.iter().position(|v| !v.is_zero()) else {
            continue;
        };
        let g = dot(&line[3..], &h);
        if g.is_empty() || g.len() > degree || gcd(&g, &h[axis])?.len() != 1 {
            continue;
        }
        let evaluate = |b: &[Poly; 3]| {
            let x = dot(&line[..3], b).into_iter().map(|x| -x).collect::<Poly>();
            let y = dot(&line[3..], b);
            sub(
                &product(&b[axis], &form(&g, degree - 1, &x, &y)),
                &form(&h[axis], degree, &x, &y),
            )
        };
        if !evaluate(&h).is_empty() {
            return Err(refused("Exact implicit identity failed"));
        }
        if census(&evaluate(&other), a, b, adjacent)? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(in super::super) fn endpoint_only(a: &[[Q; 3]], b: &[[Q; 3]], adjacent: bool) -> Result<bool> {
    Ok(one_way(a, b, adjacent)? || one_way(b, a, adjacent)?)
}
