use super::*;
pub(super) type Poly = Vec<Q>;
pub(super) struct Work {
    pub used: u64,
    pub limit: u64,
}
const BITS: u64 = 4096;
impl Work {
    pub fn charge(&mut self, n: u64) -> PResult<()> {
        if n > self.limit - self.used {
            return Err(Refusal::ResourceLimit);
        }
        self.used += n;
        Ok(())
    }
    fn guard(q: &Q) -> PResult<()> {
        if q.numer().bits() > BITS || q.denom().bits() > BITS {
            Err(Refusal::ResourceLimit)
        } else {
            Ok(())
        }
    }
    pub fn lift(&mut self, v: f64) -> PResult<Q> {
        self.charge(1)?;
        let q = Q::from_float(v).ok_or(Refusal::NotRepresentable)?;
        Self::guard(&q)?;
        Ok(q)
    }
    pub fn op(&mut self, a: &Q, b: &Q, f: impl FnOnce(&Q, &Q) -> Q) -> PResult<Q> {
        self.charge(
            1 + (a.numer().bits() + a.denom().bits() + b.numer().bits() + b.denom().bits()) / 64,
        )?;
        let q = f(a, b);
        Self::guard(&q)?;
        Ok(q)
    }
    pub fn int(
        &mut self,
        a: &BigInt,
        b: &BigInt,
        f: impl FnOnce(&BigInt, &BigInt) -> BigInt,
    ) -> PResult<BigInt> {
        self.charge(1 + (a.bits() + b.bits()) / 64)?;
        let q = f(a, b);
        if q.bits() > BITS {
            Err(Refusal::ResourceLimit)
        } else {
            Ok(q)
        }
    }
    pub fn float(&mut self, q: &Q) -> PResult<f64> {
        self.charge(1 + (q.numer().bits() + q.denom().bits()) / 64)?;
        let v = q
            .to_f64()
            .filter(|v| v.is_finite())
            .ok_or(Refusal::NotRepresentable)?;
        if Q::from_float(v).as_ref() != Some(q) {
            return Err(Refusal::NotRepresentable);
        }
        Ok(v)
    }
}
pub(super) fn trim(p: &mut Poly) {
    while p.last().is_some_and(Zero::is_zero) {
        p.pop();
    }
}
fn choose(n: usize, k: usize) -> u64 {
    let mut r = 1;
    for i in 0..k {
        r = r * (n - i) as u64 / (i + 1) as u64;
    }
    r
}
pub(super) fn power(b: &[Q], w: &mut Work) -> PResult<Poly> {
    let p = b.len() - 1;
    let mut a = vec![Q::zero(); p + 1];
    for i in 0..=p {
        for j in 0..=p - i {
            let n = choose(p, i) * choose(p - i, j);
            let c = Q::from_integer(BigInt::from(n) * if j % 2 == 0 { 1 } else { -1 });
            let t = w.op(&b[i], &c, |a, b| a * b)?;
            a[i + j] = w.op(&a[i + j], &t, |a, b| a + b)?;
        }
    }
    trim(&mut a);
    Ok(a)
}
pub(super) fn bernstein(a: &Poly, p: usize, w: &mut Work) -> PResult<Vec<Q>> {
    let mut b = vec![Q::zero(); p + 1];
    for i in 0..=p {
        for k in 0..a.len().min(i + 1) {
            let c = Q::new(choose(i, k).into(), choose(p, k).into());
            let v = w.op(&a[k], &c, |a, b| a * b)?;
            b[i] = w.op(&b[i], &v, |a, b| a + b)?;
        }
    }
    Ok(b)
}
fn monic(mut a: Poly, w: &mut Work) -> PResult<Poly> {
    if let Some(lead) = a.last().cloned() {
        for x in &mut a {
            *x = w.op(x, &lead, |a, b| a / b)?;
        }
    }
    Ok(a)
}
pub(super) fn divide(a: &Poly, b: &Poly, w: &mut Work) -> PResult<(Poly, Poly)> {
    if b.is_empty() {
        return Err(Refusal::UnprovedFactor);
    }
    let mut r = a.clone();
    let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
    while !r.is_empty() && r.len() >= b.len() {
        let shift = r.len() - b.len();
        let c = w.op(r.last().unwrap(), b.last().unwrap(), |a, b| a / b)?;
        q[shift] = c.clone();
        for (j, v) in b.iter().enumerate() {
            let t = w.op(&c, v, |a, b| a * b)?;
            r[shift + j] = w.op(&r[shift + j], &t, |a, b| a - b)?;
        }
        trim(&mut r);
    }
    trim(&mut q);
    Ok((q, r))
}
pub(super) fn gcd(mut a: Poly, mut b: Poly, w: &mut Work) -> PResult<Poly> {
    while !b.is_empty() {
        let (_, r) = divide(&a, &b, w)?;
        a = b;
        b = r;
    }
    monic(a, w)
}
pub(super) fn multiply(a: &Poly, b: &Poly, w: &mut Work) -> PResult<Poly> {
    if a.is_empty() || b.is_empty() {
        return Ok(vec![]);
    }
    let mut r = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            let t = w.op(x, y, |a, b| a * b)?;
            r[i + j] = w.op(&r[i + j], &t, |a, b| a + b)?;
        }
    }
    trim(&mut r);
    Ok(r)
}
pub(super) fn integer_gcd(mut a: BigInt, mut b: BigInt, w: &mut Work) -> PResult<BigInt> {
    while !b.is_zero() {
        let r = w.int(&a, &b, |a, b| a % b)?;
        a = b;
        b = r;
    }
    Ok(a)
}
