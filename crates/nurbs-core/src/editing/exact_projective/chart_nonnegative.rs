//! Exact nonnegativity on the closed unit interval, including interior tangency.
use crate::{
    core::bounded_rational::Work,
    editing::{
        exact_derivative::{PResult, Refusal},
        rational_factor::polynomial::{self as poly, Poly},
    },
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

pub(super) fn nonnegative(power: &Poly, degree: usize, work: &mut Work) -> PResult<bool> {
    if poly::bernstein(power, degree, work)?
        .iter()
        .all(|q| !q.is_negative())
    {
        return Ok(true);
    }
    // The first nonzero power coefficient determines the sign just to the
    // right of zero. Only an odd root inside the interval can change it.
    if power
        .iter()
        .find(|q| !q.is_zero())
        .is_some_and(Signed::is_negative)
    {
        return Ok(false);
    }
    let mut odd = odd_roots(power, work)?;
    if odd.first().is_some_and(Zero::is_zero) {
        odd = quotient(&odd, &vec![Q::zero(), Q::one()], work)?;
    }
    if sum(&odd, work)?.is_zero() {
        odd = quotient(&odd, &vec![-Q::one(), Q::one()], work)?;
    }
    Ok(no_interior_roots(odd, work)?)
}

fn derivative(power: &Poly, work: &mut Work) -> PResult<Poly> {
    power
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, q)| {
            let degree = Q::from_integer(i.into());
            work.op(q, &degree, |a, b| a * b).map_err(Into::into)
        })
        .collect()
}

fn quotient(a: &Poly, b: &Poly, work: &mut Work) -> PResult<Poly> {
    let (q, r) = poly::divide(a, b, work)?;
    if !r.is_empty() {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok(q)
}

fn odd_roots(power: &Poly, work: &mut Work) -> PResult<Poly> {
    if power.len() <= 1 {
        return Ok(vec![Q::one()]);
    }
    let common = poly::gcd(power.clone(), derivative(power, work)?, work)?;
    let distinct = quotient(power, &common, work)?;
    // A multiple root has opposite multiplicity parity in gcd(p,p').
    // Remove its odd roots from the distinct roots of p.
    let remove = odd_roots(&common, work)?;
    quotient(&distinct, &remove, work)
}

fn sum(power: &Poly, work: &mut Work) -> PResult<Q> {
    power.iter().try_fold(Q::zero(), |acc, q| {
        work.op(&acc, q, |a, b| a + b).map_err(Into::into)
    })
}

#[derive(Default)]
struct Signs {
    previous: Option<bool>,
    variations: usize,
}

impl Signs {
    fn append(&mut self, q: &Q) {
        if !q.is_zero() {
            let negative = q.is_negative();
            self.variations += usize::from(self.previous.is_some_and(|last| last != negative));
            self.previous = Some(negative);
        }
    }
}

fn no_interior_roots(mut a: Poly, work: &mut Work) -> PResult<bool> {
    if a.len() <= 1 {
        return Ok(true);
    }
    let mut at_zero = Signs::default();
    let mut at_one = Signs::default();
    let mut b = derivative(&a, work)?;
    at_zero.append(&a[0]);
    at_one.append(&sum(&a, work)?);
    // Exact Sturm variations, with the endpoint roots already removed.
    while !b.is_empty() {
        at_zero.append(&b[0]);
        at_one.append(&sum(&b, work)?);
        let (_, remainder) = poly::divide(&a, &b, work)?;
        a = b;
        b = remainder.into_iter().map(|q| -q).collect();
    }
    Ok(at_zero.variations == at_one.variations)
}
