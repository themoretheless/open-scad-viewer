//! Exact removal of common polynomial clock factors, including interior zeros.
//! Cancellation proposes a continuous extension; the caller still proves range,
//! monotonicity and the complete spatial composition identity.
use super::*;
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |v, i| {
        v * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn trim(p: &mut Vec<Q>) {
    while p.len() > 1 && p.last().unwrap().is_zero() {
        p.pop();
    }
}
fn guard(p: &[Q]) -> Result<()> {
    if p.iter()
        .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(fail("clock cancellation bit budget exceeded"));
    }
    Ok(())
}
pub(super) fn power(p: &[Q], work: &mut Work) -> Result<Vec<Q>> {
    let n = p.len() - 1;
    work.charge(p.len() * p.len())?;
    let mut out = vec![Q::zero(); p.len()];
    for i in 0..=n {
        for j in i..=n {
            let v = &p[i] * choose(n, i) * choose(n - i, j - i);
            if (j - i) % 2 == 0 {
                out[j] += &v;
            } else {
                out[j] -= &v;
            }
        }
    }
    guard(&out)?;
    trim(&mut out);
    Ok(out)
}
fn divide(mut a: Vec<Q>, b: &[Q], work: &mut Work) -> Result<(Vec<Q>, Vec<Q>)> {
    if b.last().is_none_or(|v| v.is_zero()) {
        return Err(fail("zero clock polynomial divisor"));
    }
    let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
    while a.len() >= b.len() && !a.iter().all(Q::is_zero) {
        work.charge(b.len())?;
        let k = a.len() - b.len();
        let v = a.last().unwrap() / b.last().unwrap();
        q[k] += &v;
        for j in 0..b.len() {
            a[k + j] -= &v * &b[j];
        }
        guard(&a)?;
        guard(&q)?;
        trim(&mut a);
    }
    trim(&mut q);
    Ok((q, a))
}
pub(super) fn clock(p: Vec<[Q; 2]>, work: &mut Work) -> Result<Vec<[Q; 2]>> {
    if p.is_empty() || p.len() > 65 {
        return Err(fail("clock cancellation degree budget exceeded"));
    }
    // Keep already regular clocks on their original fast path.
    if p.iter().all(|v| v[1] > Q::zero()) {
        return Ok(p);
    }
    if p.iter().all(|v| v[1] < Q::zero()) {
        return Ok(p
            .into_iter()
            .map(|v| [-v[0].clone(), -v[1].clone()])
            .collect());
    }
    let n = power(&p.iter().map(|v| v[0].clone()).collect::<Vec<_>>(), work)?;
    let d = power(&p.iter().map(|v| v[1].clone()).collect::<Vec<_>>(), work)?;
    if d.iter().all(Q::is_zero) {
        return Err(fail("identically zero clock denominator"));
    }
    let (mut a, mut b) = (n.clone(), d.clone());
    while !b.iter().all(Q::is_zero) {
        let (_, r) = divide(a, &b, work)?;
        a = b;
        b = r;
    }
    let lead = a.last().unwrap().clone();
    for v in &mut a {
        *v /= &lead;
    }
    if a.len() == 1 {
        return Ok(p);
    }
    let (n, rn) = divide(n, &a, work)?;
    let (d, rd) = divide(d, &a, work)?;
    if !rn.iter().all(Q::is_zero) || !rd.iter().all(Q::is_zero) {
        return Err(fail("inexact clock cancellation"));
    }
    let degree = n.len().max(d.len()) - 1;
    work.charge((degree + 1) * (degree + 1) * 2)?;
    let mut out = (0..=degree)
        .map(|i| {
            std::array::from_fn(|k| {
                let c = if k == 0 { &n } else { &d };
                (0..=i.min(c.len() - 1))
                    .map(|j| &c[j] * choose(i, j) / choose(degree, j))
                    .sum()
            })
        })
        .collect::<Vec<[Q; 2]>>();
    if out.iter().all(|v| v[1] < Q::zero()) {
        for v in &mut out {
            for q in v {
                *q = -q.clone();
            }
        }
    }
    if !out[0][1].is_zero() {
        let gauge = out[0][1].abs();
        for v in &mut out {
            for q in v {
                *q /= &gauge;
            }
        }
    }
    for v in &out {
        guard(v)?;
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removable_interior_zero_is_exact_but_a_true_pole_remains() {
        // N=t(2t-1), D=2t-1, with an interior common zero.
        let p = vec![
            [integer(0), integer(-1)],
            [-Q::new(1.into(), 2.into()), integer(0)],
            [integer(1), integer(1)],
        ];
        assert_eq!(
            clock(p, &mut Work::default()).unwrap(),
            vec![[integer(0), integer(1)], [integer(1), integer(1)]]
        );
        let pole = vec![[integer(1), integer(-1)], [integer(1), integer(1)]];
        assert_eq!(clock(pole.clone(), &mut Work::default()).unwrap(), pole);
        assert!(clock(vec![[integer(0), integer(0)]], &mut Work::default()).is_err());
    }
}

// Recover a unique rational parameter of a known spatial vertex. The common
// polynomial factor binds all three coordinates; no nearest-point guess.
pub(super) fn parameter(h: &[sb::H], point: &P, work: &mut Work) -> Result<Option<Q>> {
    if h.len() < 2 || h.len() > 65 {
        return Err(fail("seam point degree budget exceeded"));
    }
    let gcd = |mut a: Vec<Q>, mut b: Vec<Q>, work: &mut Work| -> Result<Vec<Q>> {
        while !b.iter().all(Q::is_zero) {
            let (_, r) = divide(a, &b, work)?;
            a = b;
            b = r;
        }
        if !a.iter().all(Q::is_zero) {
            let lead = a.last().unwrap().clone();
            for v in &mut a {
                *v /= &lead;
            }
        }
        Ok(a)
    };
    let mut common = vec![Q::zero()];
    for axis in 0..3 {
        let polynomial = power(
            &h.iter()
                .map(|p| &p[axis] - &point[axis] * &p[3])
                .collect::<Vec<_>>(),
            work,
        )?;
        common = gcd(common, polynomial, work)?;
        if common.len() == 1 && !common[0].is_zero() {
            return Ok(None);
        }
    }
    if common.len() < 2 {
        return Ok(None);
    }
    // Homogeneous base factors (including nonreal roots) are not point
    // parameters. They otherwise obscure a unique rational spatial preimage.
    let weight = power(&h.iter().map(|p| p[3].clone()).collect::<Vec<_>>(), work)?;
    let base = gcd(common.clone(), weight, work)?;
    if base.len() > 1 {
        let (reduced, remainder) = divide(common, &base, work)?;
        if !remainder.iter().all(Q::is_zero) {
            return Err(fail("seam base-factor quotient is inexact"));
        }
        common = reduced;
        if common.len() < 2 {
            return Ok(None);
        }
    }
    let derivative = (1..common.len())
        .map(|i| integer(i as i64) * &common[i])
        .collect::<Vec<_>>();
    let repeated = gcd(common.clone(), derivative, work)?;
    let (root, remainder) = divide(common, &repeated, work)?;
    if root.len() != 2 || !remainder.iter().all(Q::is_zero) {
        return Ok(None);
    }
    let t = -&root[0] / &root[1];
    if t <= Q::zero() || t >= Q::one() {
        return Ok(None);
    }
    let mut value = h.to_vec();
    while value.len() > 1 {
        work.charge(value.len() * 4)?;
        value = value
            .windows(2)
            .map(|v| std::array::from_fn(|k| (Q::one() - &t) * &v[0][k] + &t * &v[1][k]))
            .collect();
    }
    if value[0][3] <= Q::zero() || (0..3).any(|k| value[0][k] != &point[k] * &value[0][3]) {
        return Ok(None);
    }
    Ok(Some(t))
}

#[cfg(test)]
mod base_factor_tests {
    use super::*;
    #[test]
    fn nonreal_homogeneous_base_roots_do_not_hide_a_rational_seam_point() {
        let factor = vec![integer(1), integer(1), integer(2)];
        let x = sb::multiply(&[Q::zero(), Q::one()], &factor);
        let w = sb::multiply(&[Q::one(), Q::one()], &factor);
        let h = x
            .into_iter()
            .zip(w)
            .map(|(x, w)| [x, Q::zero(), Q::zero(), w])
            .collect::<Vec<_>>();
        let half = Q::new(1.into(), 2.into());
        assert_eq!(
            parameter(
                &h,
                &[half.clone(), Q::zero(), Q::zero()],
                &mut Work::default()
            )
            .unwrap(),
            Some(half)
        );
        assert!(parameter(
            &h,
            &[Q::new(1.into(), 2.into()), Q::one(), Q::zero()],
            &mut Work::default()
        )
        .unwrap()
        .is_none());
    }
}
