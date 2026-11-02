//! Bounded exact rational proposals and one global binary64 storage gauge.
use super::rational_factor::Refusal;
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
type PResult<T> = std::result::Result<T, Refusal>;
pub(super) use crate::core::bounded_rational::Work;
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
