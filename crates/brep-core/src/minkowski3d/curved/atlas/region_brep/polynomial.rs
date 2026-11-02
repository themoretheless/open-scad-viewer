//! Exact bounded bivariate operations. Coefficients are [v power][u power].
use super::*;
pub(super) type Bi = Vec<Vec<Q>>;
pub(super) fn trim(mut p: Bi) -> Bi {
    for r in &mut p {
        while r.last().is_some_and(Q::is_zero) {
            r.pop();
        }
    }
    while p.last().is_some_and(Vec::is_empty) {
        p.pop();
    }
    p
}
pub(super) fn guard_poly(p: &Bi) -> Result<()> {
    if p.len() > 17
        || p.iter().any(|r| r.len() > 17)
        || p.iter()
            .flatten()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(fail("algebraic B-rep polynomial budget exceeded"));
    }
    Ok(())
}
pub(super) fn encode(p: &Bi) -> domains::DomainPolynomial {
    p.iter()
        .map(|r| r.iter().map(roots::ratio).collect())
        .collect()
}
pub(super) fn decode(p: &domains::DomainPolynomial) -> Result<Bi> {
    if p.len() > 9 || p.iter().any(|r| r.len() > 9) {
        return Err(fail("algebraic B-rep input polynomial degree exceeded"));
    }
    let p = p
        .iter()
        .map(|r| r.iter().map(roots::decode).collect::<Result<Vec<_>>>())
        .collect::<Result<Bi>>()?;
    let t = trim(p.clone());
    if t != p {
        return Err(fail("algebraic B-rep polynomial is not canonical"));
    }
    Ok(t)
}
pub(super) fn scale(p: &Bi, k: &Q) -> Bi {
    trim(
        p.iter()
            .map(|r| r.iter().map(|q| q * k).collect())
            .collect(),
    )
}
pub(super) fn sum(a: &Bi, b: &Bi) -> Result<Bi> {
    let n = a.len().max(b.len());
    let m = a.iter().chain(b).map(Vec::len).max().unwrap_or(0);
    let mut out = vec![vec![Q::zero(); m]; n];
    for p in [a, b] {
        for (j, r) in p.iter().enumerate() {
            for (i, q) in r.iter().enumerate() {
                out[j][i] += q;
            }
        }
    }
    let out = trim(out);
    guard_poly(&out)?;
    Ok(out)
}
pub(super) fn product(a: &Bi, b: &Bi, work: &mut Work) -> Result<Bi> {
    if a.is_empty() || b.is_empty() {
        return Ok(vec![]);
    }
    let n = a.len() + b.len() - 1;
    let m = a.iter().map(Vec::len).max().unwrap() + b.iter().map(Vec::len).max().unwrap() - 1;
    if n > 17 || m > 17 {
        return Err(fail("algebraic B-rep product degree exceeded"));
    }
    let mut out = vec![vec![Q::zero(); m]; n];
    for (j, r) in a.iter().enumerate() {
        for (i, x) in r.iter().enumerate() {
            for (l, s) in b.iter().enumerate() {
                for (k, y) in s.iter().enumerate() {
                    work.charge(1)?;
                    out[j + l][i + k] += x * y;
                }
            }
        }
    }
    let out = trim(out);
    guard_poly(&out)?;
    Ok(out)
}
pub(super) fn derivative(p: &Bi, axis: usize) -> Bi {
    if axis == 0 {
        trim(
            p.iter()
                .map(|r| {
                    r.iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, q)| q * integer(i as i64))
                        .collect()
                })
                .collect(),
        )
    } else {
        trim(
            p.iter()
                .enumerate()
                .skip(1)
                .map(|(j, r)| r.iter().map(|q| q * integer(j as i64)).collect())
                .collect(),
        )
    }
}
pub(super) fn at(p: &Bi, u: &Q, v: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |a, r| {
        a * v + r.iter().rev().fold(Q::zero(), |b, q| b * u + q)
    })
}
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |a, i| {
        a * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn split(p: &[Q]) -> (Vec<Q>, Vec<Q>) {
    let mut row = p.to_vec();
    let mut left = vec![row[0].clone()];
    let mut right = vec![row.last().unwrap().clone()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|a| (&a[0] + &a[1]) / integer(2))
            .collect();
        left.push(row[0].clone());
        right.push(row.last().unwrap().clone());
    }
    right.reverse();
    (left, right)
}
fn positive_net(p: &Bi, depth: usize, work: &mut Work) -> Result<bool> {
    work.charge(p.len() * p[0].len())?;
    if p.iter().flatten().all(Q::is_positive) {
        return Ok(true);
    }
    let n = p.len() - 1;
    let m = p[0].len() - 1;
    if [&p[0][0], &p[n][0], &p[0][m], &p[n][m]]
        .iter()
        .any(|q| !q.is_positive())
        || depth == 12
    {
        return Ok(false);
    }
    let axis = if m == 0 {
        1
    } else if n == 0 {
        0
    } else {
        depth % 2
    };
    let (a, b) = if axis == 0 {
        let parts = p.iter().map(|r| split(r)).collect::<Vec<_>>();
        (
            parts.iter().map(|(a, _)| a.clone()).collect(),
            parts.into_iter().map(|(_, b)| b).collect(),
        )
    } else {
        let mut a = p.clone();
        let mut b = p.clone();
        for i in 0..=m {
            let column = p.iter().map(|r| r[i].clone()).collect::<Vec<_>>();
            let (l, r) = split(&column);
            for j in 0..=n {
                a[j][i] = l[j].clone();
                b[j][i] = r[j].clone();
            }
        }
        (a, b)
    };
    Ok(positive_net(&a, depth + 1, work)? && positive_net(&b, depth + 1, work)?)
}
/// Sufficient exact positivity over the whole unit square (hence triangle).
pub(super) fn bernstein(p: &Bi, work: &mut Work) -> Result<Bi> {
    if p.is_empty() {
        return Ok(vec![vec![Q::zero()]]);
    }
    let n = p.len() - 1;
    let m = p.iter().map(Vec::len).max().unwrap() - 1;
    let mut net = vec![vec![Q::zero(); m + 1]; n + 1];
    for j in 0..=n {
        for i in 0..=m {
            for b in 0..=j {
                for a in 0..=i {
                    work.charge(1)?;
                    if let Some(q) = p.get(b).and_then(|r| r.get(a)) {
                        net[j][i] += q * choose(i, a) / choose(m, a) * choose(j, b) / choose(n, b);
                    }
                }
            }
        }
    }
    guard_poly(&net)?;
    Ok(net)
}
pub(super) fn positive(p: &Bi, work: &mut Work) -> Result<bool> {
    if p.is_empty() {
        return Ok(false);
    }
    positive_net(&bernstein(p, work)?, 0, work)
}

pub(super) fn negative_boundary(p: &Bi, work: &mut Work) -> Result<bool> {
    let edge0 = trim(
        p.iter()
            .map(|r| vec![r.first().cloned().unwrap_or_default()])
            .collect(),
    );
    let edge1 = trim(vec![p.first().cloned().unwrap_or_default()]);
    let mut diagonal = vec![Q::zero(); 33];
    for (j, r) in p.iter().enumerate() {
        for (i, q) in r.iter().enumerate() {
            for k in 0..=j {
                work.charge(1)?;
                let c = q * choose(j, k);
                diagonal[i + k] += if k % 2 == 0 { c } else { -c };
            }
        }
    }
    while diagonal.last().is_some_and(Q::is_zero) {
        diagonal.pop();
    }
    let diagonal = trim(vec![diagonal]);
    Ok(positive(&scale(&edge0, &-Q::one()), work)?
        && positive(&scale(&edge1, &-Q::one()), work)?
        && positive(&scale(&diagonal, &-Q::one()), work)?)
}

pub(super) fn compose(p: &Bi, map: &[[Q; 3]; 2], work: &mut Work) -> Result<Bi> {
    let linear = map
        .each_ref()
        .map(|a| trim(vec![vec![a[0].clone(), a[1].clone()], vec![a[2].clone()]]));
    let degrees = [
        p.iter().map(Vec::len).max().unwrap_or(1) - 1,
        p.len().saturating_sub(1),
    ];
    let mut powers = [vec![vec![vec![Q::one()]]], vec![vec![vec![Q::one()]]]];
    for axis in 0..2 {
        for i in 1..=degrees[axis] {
            let next = product(&powers[axis][i - 1], &linear[axis], work)?;
            powers[axis].push(next);
        }
    }
    let mut out = vec![];
    for (j, row) in p.iter().enumerate() {
        for (i, q) in row.iter().enumerate() {
            if !q.is_zero() {
                out = sum(
                    &out,
                    &scale(&product(&powers[0][i], &powers[1][j], work)?, q),
                )?;
            }
        }
    }
    Ok(out)
}
