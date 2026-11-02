//! Bounded exact rational proposals and one global binary64 storage gauge.
use super::rational_factor::Refusal;
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
type PResult<T> = std::result::Result<T, Refusal>;
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
pub(super) fn integer_gcd(mut a: BigInt, mut b: BigInt, w: &mut Work) -> PResult<BigInt> {
    while !b.is_zero() {
        let r = w.int(&a, &b, |a, b| a % b)?;
        a = b;
        b = r;
    }
    Ok(a)
}

pub(super) fn store(b: &[Vec<Q>], w: &mut Work) -> PResult<(Vec<Vec<f64>>, Vec<f64>)> {
    let dim = b.len() - 1;
    let weights = &b[dim];
    if !(weights.iter().all(|x| x.is_positive()) || weights.iter().all(|x| x.is_negative())) {
        return Err(Refusal::NonPositiveWeights);
    }
    // A single global rational gauge: clear weight denominators, remove the
    // common integer gcd, then choose ONE dyadic scale for the complete curve.
    let mut lcm = BigInt::one();
    for x in weights {
        let g = integer_gcd(lcm.clone(), x.denom().clone(), w)?;
        let a = w.int(&lcm, &g, |a, b| a / b)?;
        lcm = w.int(&a, x.denom(), |a, b| a * b)?;
    }
    let mut integers = Vec::new();
    let mut common = BigInt::zero();
    for x in weights {
        let ratio = w.int(&lcm, x.denom(), |a, b| a / b)?;
        let a = w.int(&x.numer().abs(), &ratio, |a, b| a * b)?;
        common = integer_gcd(common, a.clone(), w)?;
        integers.push(a);
    }
    for a in &mut integers {
        *a = w.int(a, &common, |a, b| a / b)?;
    }
    let shift = integers
        .iter()
        .map(BigInt::bits)
        .max()
        .ok_or(Refusal::InvalidOutput)?
        .saturating_sub(1);
    w.charge(1 + shift / 64)?;
    let denominator = Q::from_integer(BigInt::one() << shift as usize);
    let mut controls = Vec::new();
    let mut output_weights = Vec::new();
    for (i, integer) in integers.into_iter().enumerate() {
        let mut cp = Vec::new();
        for row in &b[..dim] {
            let x = w.op(&row[i], &weights[i], |a, b| a / b)?;
            cp.push(w.float(&x)?);
        }
        controls.push(cp);
        let normalized = w.op(&Q::from_integer(integer), &denominator, |a, b| a / b)?;
        output_weights.push(w.float(&normalized)?);
    }
    Ok((controls, output_weights))
}
