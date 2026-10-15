//! Independent exact rational derivative sign check (degree at most five).
//! In particular an even stationary zero is not rejected or sampled away.
use super::*;
use num_traits::Signed;
type Poly = Vec<Q>;
mod algebraic;
pub(super) use algebraic::endpoint_only;
fn trim(mut p: Poly) -> Poly {
    while p.last().is_some_and(Zero::is_zero) {
        p.pop();
    }
    p
}
fn choose(n: usize, k: usize) -> i64 {
    let mut v = 1;
    for i in 0..k {
        v = v * (n - i) as i64 / (i + 1) as i64;
    }
    v
}
fn power(net: &[[Q; 3]], axis: usize) -> Poly {
    let n = net.len() - 1;
    let mut p = vec![Q::zero(); n + 1];
    for i in 0..=n {
        for (k, value) in p.iter_mut().enumerate().skip(i) {
            let sign = if (k - i) % 2 == 0 { 1 } else { -1 };
            *value += &net[i][axis]
                * Q::from_integer((sign * choose(n, i) * choose(n - i, k - i)).into());
        }
    }
    trim(p)
}
fn derivative(p: &[Q]) -> Poly {
    trim(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(i, x)| x * Q::from_integer(i.into()))
            .collect(),
    )
}
fn product(a: &[Q], b: &[Q]) -> Poly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    trim(out)
}
fn sub(a: &[Q], b: &[Q]) -> Poly {
    let mut out = a.to_vec();
    out.resize(a.len().max(b.len()), Q::zero());
    for (i, x) in b.iter().enumerate() {
        out[i] -= x;
    }
    trim(out)
}
fn divide(a: &[Q], b: &[Q]) -> Result<(Poly, Poly)> {
    if b.is_empty() {
        return Err(refused("Exact certificate polynomial division by zero"));
    }
    let mut r = trim(a.to_vec());
    let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
    while r.len() >= b.len() {
        let k = r.len() - b.len();
        let x = r.last().unwrap() / b.last().unwrap();
        q[k] = x.clone();
        for (i, y) in b.iter().enumerate() {
            r[k + i] -= &x * y;
        }
        r = trim(r);
        if r.iter()
            .any(|x| x.numer().bits() > 131072 || x.denom().bits() > 131072)
        {
            return Err(refused(
                "Exact certificate polynomial arithmetic budget exhausted",
            ));
        }
    }
    Ok((trim(q), r))
}
fn quotient(a: &[Q], b: &[Q]) -> Result<Poly> {
    let (q, r) = divide(a, b)?;
    if !r.is_empty() {
        return Err(refused("Nonexact certificate polynomial quotient"));
    }
    Ok(q)
}
fn gcd(a: &[Q], b: &[Q]) -> Result<Poly> {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    while !b.is_empty() {
        let r = divide(&a, &b)?.1;
        a = b;
        b = r;
    }
    if let Some(k) = a.last().cloned() {
        for x in &mut a {
            *x /= &k;
        }
    }
    Ok(a)
}
fn at(p: &[Q], x: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |v, c| v * x + c)
}
fn variation(chain: &[Poly], x: &Q) -> usize {
    let mut previous = 0;
    let mut changes = 0;
    for p in chain {
        let v = at(p, x);
        let s = if v > Q::zero() {
            1
        } else if v < Q::zero() {
            -1
        } else {
            0
        };
        if s != 0 {
            changes += usize::from(previous != 0 && s != previous);
            previous = s;
        }
    }
    changes
}
fn interior_roots(p: &[Q]) -> Result<usize> {
    let mut p = p.to_vec();
    while p.len() > 1 && p[0].is_zero() {
        p.remove(0);
    }
    while p.len() > 1 && at(&p, &Q::one()).is_zero() {
        p = quotient(&p, &[-Q::one(), Q::one()])?;
    }
    if p.len() < 2 {
        return Ok(0);
    }
    let mut chain = vec![p.clone(), derivative(&p)];
    loop {
        let n = chain.len();
        let r = divide(&chain[n - 2], &chain[n - 1])?.1;
        if r.is_empty() {
            break;
        }
        let scale = r.last().unwrap().abs();
        chain.push(r.into_iter().map(|x| -x / &scale).collect());
    }
    variation(&chain, &Q::zero())
        .checked_sub(variation(&chain, &Q::one()))
        .ok_or_else(|| refused("Inconsistent exact certificate Sturm census"))
}

/// Exact Green integral for polynomial spans, including all translated bits.
pub(super) fn polynomial_area(net: &[[Q; 3]], origin: &[Q; 2]) -> Q {
    let mut net = net.to_vec();
    for h in &mut net {
        h[0] = &h[0] / &h[2] - &origin[0];
        h[1] = &h[1] / &h[2] - &origin[1];
        h[2] = Q::one();
    }
    let x = power(&net, 0);
    let y = power(&net, 1);
    let f = sub(&product(&x, &derivative(&y)), &product(&y, &derivative(&x)));
    f.iter().enumerate().fold(Q::zero(), |s, (i, c)| {
        s + c / Q::from_integer((2 * (i + 1)).into())
    })
}

pub(super) fn monotone(net: &[[Q; 3]], axis: usize) -> Result<bool> {
    let x = power(net, axis);
    let w = power(net, 2);
    let p = sub(&product(&derivative(&x), &w), &product(&x, &derivative(&w)));
    let Some(first) = p.iter().find(|c| !c.is_zero()) else {
        return Ok(false);
    };
    let delta = &net.last().unwrap()[axis] / &net.last().unwrap()[2] - &net[0][axis] / &net[0][2];
    if delta.is_zero() || (delta > Q::zero()) != (first > &Q::zero()) {
        return Ok(false);
    }
    // Yun square-free decomposition: only odd-multiplicity interior roots
    // change sign. Endpoint roots and even stationary roots preserve it.
    let mut g = gcd(&p, &derivative(&p))?;
    let mut w = quotient(&p, &g)?;
    let mut multiplicity = 1;
    while w.len() > 1 {
        let y = gcd(&w, &g)?;
        let factor = quotient(&w, &y)?;
        if multiplicity % 2 == 1 && interior_roots(&factor)? > 0 {
            return Ok(false);
        }
        w = y;
        g = quotient(&g, &w)?;
        multiplicity += 1;
        if multiplicity > 7 {
            return Err(refused("Exact certificate multiplicity budget exhausted"));
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn net(x: &[f64]) -> Vec<[Q; 3]> {
        x.iter()
            .map(|x| [Q::from_float(*x).unwrap(), Q::zero(), Q::one()])
            .collect()
    }
    #[test]
    fn even_stationary_zero_and_endpoint_cusps_are_monotone() {
        // x=(2t-1)^3: unordered controls, a double interior derivative zero.
        assert!(monotone(&net(&[-1., 1., -1., 1.]), 0).unwrap());
        assert!(monotone(&net(&[0., 0., 0., 1.]), 0).unwrap());
        assert!(monotone(&net(&[1., 0., 0., 0.]), 0).unwrap());
        assert!(!monotone(&net(&[0., 2., -1., 1.]), 0).unwrap());
        assert!(!monotone(&net(&[0., 0., 0., 0.]), 0).unwrap());
    }
}
