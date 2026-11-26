//! Exact nonnegativity on the closed unit interval, including interior tangency.
use crate::{
    core::bounded_rational::Work,
    exact_geometry::{
        exact_derivative::{PResult, Refusal},
        rational_factor::polynomial::{self as poly, Poly},
    },
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

pub(super) fn nonnegative(power: &Poly, degree: usize, work: &mut Work) -> PResult<bool> {
    // The fast basis conversion uses integer binomials at curve degree.
    // Composed distance polynomials can exceed that range.
    if degree <= 25
        && poly::bernstein(power, degree, work)?
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
    Ok(interior_root_count(odd_roots(power, work)?, work)? == 0)
}

/// Count distinct interior contacts and the subset with odd multiplicity.
pub(super) fn contact_counts(power: &Poly, work: &mut Work) -> PResult<[usize; 2]> {
    if power.len() <= 1 {
        return Ok([0, 0]);
    }
    let [distinct, odd] = interior_contact_factors(power, work)?;
    Ok([count_roots(distinct, work)?, count_roots(odd, work)?])
}

pub(super) fn interior_contact_factors(power: &Poly, work: &mut Work) -> PResult<[Poly; 2]> {
    let (distinct, common) = square_free_layer(power, work)?;
    let odd = quotient(&distinct, &odd_roots(&common, work)?, work)?;
    Ok([
        without_endpoints(distinct, work)?,
        without_endpoints(odd, work)?,
    ])
}

pub(super) fn derivative(power: &Poly, work: &mut Work) -> PResult<Poly> {
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
    let (distinct, common) = square_free_layer(power, work)?;
    // A multiple root has opposite multiplicity parity in gcd(p,p').
    // Remove its odd roots from the distinct roots of p.
    let remove = odd_roots(&common, work)?;
    quotient(&distinct, &remove, work)
}

fn square_free_layer(power: &Poly, work: &mut Work) -> PResult<(Poly, Poly)> {
    let common = poly::gcd(power.clone(), derivative(power, work)?, work)?;
    Ok((quotient(power, &common, work)?, common))
}

pub(super) fn sum(power: &Poly, work: &mut Work) -> PResult<Q> {
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

fn without_endpoints(mut a: Poly, work: &mut Work) -> PResult<Poly> {
    if a.len() <= 1 {
        return Ok(a);
    }
    // The input is square-free, so each endpoint factor occurs at most once.
    if a[0].is_zero() {
        a = quotient(&a, &vec![Q::zero(), Q::one()], work)?;
    }
    if sum(&a, work)?.is_zero() {
        a = quotient(&a, &vec![-Q::one(), Q::one()], work)?;
    }
    Ok(a)
}

fn interior_root_count(a: Poly, work: &mut Work) -> PResult<usize> {
    count_roots(without_endpoints(a, work)?, work)
}

fn count_roots(a: Poly, work: &mut Work) -> PResult<usize> {
    if a.len() <= 1 {
        return Ok(0);
    }
    let chain = SturmChain::new(a, work)?;
    let (start, _) = chain.at(&Q::zero(), work)?;
    let (end, _) = chain.at(&Q::one(), work)?;
    start.checked_sub(end).ok_or(Refusal::UnprovedIdentity)
}

pub(super) struct SturmChain(Vec<Poly>);

impl SturmChain {
    /// A square-free, nonzero polynomial gives variations which count roots
    /// in (left, right], including when a bisection hits an exact root.
    pub(super) fn new(a: Poly, work: &mut Work) -> PResult<Self> {
        let mut b = derivative(&a, work)?;
        let mut chain = vec![a];
        while !b.is_empty() {
            let (_, remainder) = poly::divide(chain.last().unwrap(), &b, work)?;
            chain.push(b);
            b = remainder.into_iter().map(|q| -q).collect();
        }
        Ok(Self(chain))
    }

    pub(super) fn at(&self, x: &Q, work: &mut Work) -> PResult<(usize, bool)> {
        let mut signs = Signs::default();
        let mut root = false;
        for (i, p) in self.0.iter().enumerate() {
            let value = evaluate(p, x, work)?;
            if i == 0 {
                root = value.is_zero();
            }
            signs.append(&value);
        }
        Ok((signs.variations, root))
    }
}

pub(super) fn evaluate(p: &Poly, x: &Q, work: &mut Work) -> PResult<Q> {
    if x.is_zero() {
        return Ok(p.first().cloned().unwrap_or_default());
    }
    if x.is_one() {
        return sum(p, work);
    }
    let mut it = p.iter().rev();
    let start = it.next().cloned().unwrap_or_default();
    it.try_fold(start, |acc, q| {
        let product = work.op(&acc, x, |a, b| a * b)?;
        Ok(work.op(&product, q, |a, b| a + b)?)
    })
}

pub(super) fn midpoint_coordinate(coordinate: &Poly, weight: &Poly, work: &mut Work) -> PResult<Q> {
    let half = Q::new(1.into(), 2.into());
    let numerator = evaluate(coordinate, &half, work)?;
    let denominator = evaluate(weight, &half, work)?;
    if !denominator.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    Ok(work.op(&numerator, &denominator, |a, b| a / b)?)
}
