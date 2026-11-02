//! Bounded exact arithmetic shared by construction and editing.
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    ResourceLimit,
    NotRepresentable,
}
type PResult<T> = std::result::Result<T, Error>;
pub(crate) struct Work {
    pub used: u64,
    pub limit: u64,
}
const BITS: u64 = 4096;
impl Work {
    pub fn charge(&mut self, n: u64) -> PResult<()> {
        if n > self.limit - self.used {
            return Err(Error::ResourceLimit);
        }
        self.used += n;
        Ok(())
    }
    fn guard(q: &Q) -> PResult<()> {
        if q.numer().bits() > BITS || q.denom().bits() > BITS {
            Err(Error::ResourceLimit)
        } else {
            Ok(())
        }
    }
    pub fn lift(&mut self, v: f64) -> PResult<Q> {
        self.charge(1)?;
        let q = Q::from_float(v).ok_or(Error::NotRepresentable)?;
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
            Err(Error::ResourceLimit)
        } else {
            Ok(q)
        }
    }
    pub fn float(&mut self, q: &Q) -> PResult<f64> {
        self.charge(1 + (q.numer().bits() + q.denom().bits()) / 64)?;
        let v = q
            .to_f64()
            .filter(|v| v.is_finite())
            .ok_or(Error::NotRepresentable)?;
        if Q::from_float(v).as_ref() != Some(q) {
            return Err(Error::NotRepresentable);
        }
        Ok(v)
    }
}
