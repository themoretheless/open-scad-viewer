use super::*;
pub(super) type P = Vec<Expansion>;
pub(super) fn zero() -> Expansion {
    Expansion::scalar(0.)
}
pub(super) fn trim(mut p: P) -> P {
    while p.len() > 1 && p.last().unwrap().sign() == Sign::Zero {
        p.pop();
    }
    p
}
pub(super) fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
pub(super) fn add(a: &P, b: &P, sub: bool, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = Vec::new();
    for i in 0..a.len().max(b.len()) {
        let z = zero();
        let a = a.get(i).unwrap_or(&z);
        let b = b.get(i).unwrap_or(&z);
        p.push(if sub { a.sub(b, c)? } else { a.add(b, c)? });
    }
    Ok(trim(p))
}
pub(super) fn mul(a: &P, b: &P, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    if a.len() + b.len() > 258 {
        return Err(Reason::MissingProof);
    }
    let mut p = vec![zero(); a.len() + b.len() - 1];
    for (i, a) in a.iter().enumerate() {
        if a.sign() == Sign::Zero {
            c.charge(1)?;
            continue;
        }
        for (j, b) in b.iter().enumerate() {
            if b.sign() == Sign::Zero {
                c.charge(1)?;
                continue;
            }
            p[i + j] = p[i + j].add(&a.mul(b, c)?, c)?;
        }
    }
    Ok(trim(p))
}
pub(super) fn times(a: &P, n: u64, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let n = Expansion::integer(n, c)?;
    Ok(trim(
        a.iter()
            .map(|a| a.mul(&n, c))
            .collect::<Result<P, Reason>>()?,
    ))
}
pub(super) fn derivative(p: &P, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    if p.len() == 1 {
        return Ok(vec![zero()]);
    }
    Ok(trim(
        p.iter()
            .enumerate()
            .skip(1)
            .map(|(i, a)| a.mul(&Expansion::integer(i as u64, c)?, c))
            .collect::<Result<P, Reason>>()?,
    ))
}
pub(super) fn power(b: &P, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let n = b.len() - 1;
    let mut p = vec![zero(); b.len()];
    for (i, a) in b.iter().enumerate() {
        for k in i..=n {
            let term = a.mul(
                &Expansion::integer(choose(n, i) * choose(n - i, k - i), c)?,
                c,
            )?;
            p[k] = if (k - i) % 2 == 0 {
                p[k].add(&term, c)?
            } else {
                p[k].sub(&term, c)?
            };
        }
    }
    Ok(trim(p))
}
pub(super) fn minor(a: &P, b: &P, d: &P, e: &P, c: &mut PredicateContext<'_>) -> Result<P, Reason> {
    add(&mul(a, b, c)?, &mul(d, e, c)?, true, c)
}
pub(super) struct Fraction {
    pub n: P,
    pub d: P,
}
impl Fraction {
    pub fn new(mut n: P, mut d: P, c: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        if d.iter().all(|v| v.sign() == Sign::Zero) {
            return Err(Reason::MissingProof);
        }
        if n.iter().all(|v| v.sign() == Sign::Zero) {
            c.charge(1)?;
            return Ok(Self {
                n: vec![zero()],
                d: vec![Expansion::scalar(1.)],
            });
        }
        let a = n.iter().take_while(|v| v.sign() == Sign::Zero).count();
        let b = d.iter().take_while(|v| v.sign() == Sign::Zero).count();
        let remove = a.min(b);
        n.drain(..remove);
        d.drain(..remove);
        let size = n.len();
        let mut all = n;
        all.extend(d);
        Expansion::reduce_projective(&mut all, c)?;
        Expansion::normalize_common(&mut all, c)?;
        let d = all.split_off(size);
        Ok(Self { n: all, d })
    }
    pub fn equal(&self, b: &Self, c: &mut PredicateContext<'_>) -> Result<bool, Reason> {
        Ok(minor(&self.n, &b.d, &b.n, &self.d, c)?
            .iter()
            .all(|v| v.sign() == Sign::Zero))
    }
}
