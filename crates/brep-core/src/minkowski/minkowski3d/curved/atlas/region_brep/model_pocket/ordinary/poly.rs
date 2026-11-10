use super::*;
pub(super) type Poly = Vec<Q>;
pub(super) fn trim(mut p: Poly) -> Poly {
    while p.last().is_some_and(Q::is_zero) {
        p.pop();
    }
    p
}
pub(super) fn bound(p: &[Q]) -> Result<()> {
    if p.len() > 257
        || p.iter()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(fail("ordinary Model exact polynomial budget exceeded"));
    }
    Ok(())
}
pub(super) fn add(a: &[Q], b: &[Q]) -> Result<Poly> {
    let mut p = a.to_vec();
    p.resize(a.len().max(b.len()), Q::zero());
    for (i, q) in b.iter().enumerate() {
        p[i] += q;
    }
    bound(&p)?;
    Ok(trim(p))
}
pub(super) fn scale(a: &[Q], q: &Q) -> Poly {
    trim(a.iter().map(|x| x * q).collect())
}
pub(super) fn mul(a: &[Q], b: &[Q], work: &mut Work) -> Result<Poly> {
    if a.is_empty() || b.is_empty() {
        return Ok(vec![]);
    }
    if a.len() + b.len() > 258 {
        return Err(fail("ordinary Model polynomial product degree exceeded"));
    }
    let mut p = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            work.charge(1)?;
            p[i + j] += x * y;
        }
    }
    bound(&p)?;
    Ok(trim(p))
}
pub(super) fn derivative(p: &[Q]) -> Poly {
    trim(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(i, x)| x * integer(i as i64))
            .collect(),
    )
}
pub(super) fn at(p: &[Q], t: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |a, c| a * t + c)
}
pub(super) fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |p, i| {
        p * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
pub(super) fn bernstein(p: &[Q], degree: usize) -> Result<Poly> {
    if p.len() > degree + 1 {
        return Err(fail("ordinary Model Bernstein elevation lowers degree"));
    }
    let mut b = vec![Q::zero(); degree + 1];
    for i in 0..=degree {
        for k in 0..=i {
            if let Some(q) = p.get(k) {
                b[i] += q * choose(i, k) / choose(degree, k);
            }
        }
    }
    bound(&b)?;
    Ok(b)
}
pub(super) fn quotient(a: &[Q], b: &[Q]) -> Result<Poly> {
    if b.is_empty() {
        return Err(fail("ordinary Model polynomial divisor is zero"));
    }
    let mut r = trim(a.to_vec());
    let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
    while !r.is_empty() && r.len() >= b.len() {
        let k = r.len() - b.len();
        let c = r.last().unwrap() / b.last().unwrap();
        q[k] = c.clone();
        for (i, x) in b.iter().enumerate() {
            r[k + i] -= &c * x;
        }
        r = trim(r);
        bound(&r)?;
    }
    if !r.is_empty() {
        return Err(fail("ordinary Model polynomial cancellation is not exact"));
    }
    Ok(trim(q))
}
pub(super) fn power(h: &[sb::H], axis: usize) -> Poly {
    let n = h.len() - 1;
    let mut p = vec![Q::zero(); n + 1];
    for (i, x) in h.iter().enumerate() {
        for k in i..=n {
            let term = &x[axis] * choose(n, i) * choose(n - i, k - i);
            p[k] += if (k - i) % 2 == 0 { term } else { -term };
        }
    }
    trim(p)
}
pub(super) fn substitute(p: &Bi, uv: &[Poly; 3], degree: usize, work: &mut Work) -> Result<Poly> {
    let mut powers: [Vec<Poly>; 3] = std::array::from_fn(|_| vec![vec![Q::one()]]);
    for axis in 0..3 {
        for i in 1..=degree {
            let value = mul(&powers[axis][i - 1], &uv[axis], work)?;
            powers[axis].push(value);
        }
    }
    let mut result = vec![];
    for (j, row) in p.iter().enumerate() {
        for (i, q) in row.iter().enumerate() {
            if !q.is_zero() {
                let term = mul(
                    &mul(&powers[0][i], &powers[1][j], work)?,
                    &powers[2][degree - i - j],
                    work,
                )?;
                result = add(&result, &scale(&term, q))?;
            }
        }
    }
    Ok(result)
}
pub(super) fn total_degree(p: &Bi) -> usize {
    p.iter()
        .enumerate()
        .flat_map(|(j, r)| {
            r.iter()
                .enumerate()
                .filter(|(_, q)| !q.is_zero())
                .map(move |(i, _)| i + j)
        })
        .max()
        .unwrap_or(0)
}
pub(super) fn reduced(mut coordinates: Vec<Poly>) -> Result<Vec<Poly>> {
    let mut common = vec![];
    for p in &coordinates {
        common = roots::gcd(&common, p)?;
    }
    if common.len() > 1 {
        for p in &mut coordinates {
            *p = quotient(p, &common)?;
        }
    }
    Ok(coordinates)
}
pub(super) fn exact(q: &Q) -> Result<f64> {
    let value = q
        .to_f64()
        .ok_or_else(|| fail("ordinary Model exact coordinate overflow"))?;
    if !value.is_finite() || Q::from_float(value).as_ref() != Some(q) {
        return Err(fail(
            "ordinary Model needs a certified approximation for this non-binary64 exact control",
        ));
    }
    Ok(value)
}
fn gcd_int(mut a: num_bigint::BigInt, mut b: num_bigint::BigInt) -> num_bigint::BigInt {
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a.abs()
}
/// A homogeneous constant gauge can clear rational weight denominators without
/// rounding a single Euclidean control point or changing the represented map.
pub(super) fn float_net(coordinates: &[Poly], degree: usize) -> Result<(Vec<Vec<f64>>, Vec<f64>)> {
    if degree > 16 {
        return Err(fail("ordinary Model output degree exceeds 16"));
    }
    let values = coordinates
        .iter()
        .map(|p| bernstein(p, degree))
        .collect::<Result<Vec<_>>>()?;
    let mut weights = values.last().unwrap().clone();
    let sign = if weights.iter().all(Q::is_positive) {
        Q::one()
    } else if weights.iter().all(Q::is_negative) {
        -Q::one()
    } else {
        return Err(fail(
            "ordinary Model control denominator is not strictly sign-definite",
        ));
    };
    for w in &mut weights {
        *w *= &sign;
    }
    let mut gauge = num_bigint::BigInt::one();
    for w in &weights {
        gauge = &gauge / gcd_int(gauge.clone(), w.denom().clone()) * w.denom();
        if gauge.bits() > 8192 {
            return Err(fail("ordinary Model weight gauge budget exceeded"));
        }
    }
    let integers = weights
        .iter()
        .map(|w| (w * Q::from_integer(gauge.clone())).to_integer())
        .collect::<Vec<_>>();
    let common = integers
        .iter()
        .fold(num_bigint::BigInt::zero(), |a, b| gcd_int(a, b.clone()));
    let stored_weights = integers
        .iter()
        .map(|w| exact(&Q::from_integer(w / &common)))
        .collect::<Result<Vec<_>>>()?;
    let mut points = vec![];
    for i in 0..=degree {
        points.push(
            values[..values.len() - 1]
                .iter()
                .map(|p| exact(&(&p[i] / &values.last().unwrap()[i])))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok((points, stored_weights))
}
pub(super) fn curve(pieces: &[[Poly; 4]], breaks: &[f64]) -> Result<crate::Curve> {
    let mut reduced_pieces = vec![];
    for p in pieces {
        reduced_pieces.push(reduced(p.to_vec())?);
    }
    let degree = reduced_pieces
        .iter()
        .flatten()
        .map(|p| p.len().saturating_sub(1))
        .max()
        .unwrap()
        .max(1);
    let mut control = vec![];
    let mut weights = vec![];
    let mut knots = vec![breaks[0]; degree + 1];
    for (k, p) in reduced_pieces.iter().enumerate() {
        let (mut cp, mut w) = float_net(p, degree)?;
        if k > 0 {
            if control.last() != cp.first() {
                return Err(fail(
                    "ordinary Model curve pieces have unequal exact endpoints",
                ));
            }
            let gauge =
                Q::from_float(*weights.last().unwrap()).unwrap() / Q::from_float(w[0]).unwrap();
            w = w
                .iter()
                .map(|v| exact(&(Q::from_float(*v).unwrap() * &gauge)))
                .collect::<Result<_>>()?;
            cp.remove(0);
            w.remove(0);
            knots.extend(vec![breaks[k]; degree]);
        }
        control.extend(cp);
        weights.extend(w);
    }
    knots.extend(vec![*breaks.last().unwrap(); degree + 1]);
    Ok(crate::Curve {
        degree,
        knots,
        control_points: control,
        weights,
        periodic: false,
    })
}
