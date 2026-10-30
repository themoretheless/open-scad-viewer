//! Narrow crate-internal access to the existing exact algebraic root verifier.
use super::*;
fn bounded(p: &[Q]) -> Result<()> {
    if p.len() > 65
        || p.iter()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
    {
        return Err(refused("Atlas algebraic polynomial budget exceeded"));
    }
    Ok(())
}
pub(crate) fn gcd(a: &[Q], b: &[Q]) -> Result<Vec<Q>> {
    bounded(a)?;
    bounded(b)?;
    let trim = |p: &[Q]| {
        p.iter()
            .cloned()
            .rev()
            .skip_while(Zero::is_zero)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    };
    let p = arithmetic::gcd(&trim(a), &trim(b))?;
    bounded(&p)?;
    Ok(p)
}
pub(crate) fn roots(p: &[Q]) -> Result<Vec<[Q; 2]>> {
    bounded(p)?;
    arithmetic::algebraic_events::isolates(
        &p.to_vec(),
        Q::zero(),
        Q::one(),
        &Q::new(1.into(), 65536.into()),
    )
}
pub(crate) fn count(p: &[Q], range: &[Q; 2]) -> Result<usize> {
    bounded(p)?;
    arithmetic::algebraic_events::count(&p.to_vec(), &range[0], &range[1])
}
pub(crate) fn zero(p: &[Q], f: &[Q], range: &[Q; 2]) -> Result<bool> {
    bounded(p)?;
    bounded(f)?;
    arithmetic::event_coordinates::zero_at(&p.to_vec(), &f.to_vec(), range)
}
pub(crate) fn sign(p: &[Q], f: &[Q], range: &[Q; 2]) -> Result<i8> {
    bounded(p)?;
    bounded(f)?;
    arithmetic::event_coordinates::sign_at(&p.to_vec(), &f.to_vec(), range, 192)
}
pub(crate) fn ratio(q: &Q) -> ExactRatio {
    algebraic_intersections::ratio(q)
}
pub(crate) fn decode(q: &ExactRatio) -> Result<Q> {
    let q = algebraic_intersections::decode_ratio(q)?;
    if q.numer().bits() > 8192 || q.denom().bits() > 8192 {
        return Err(refused("Atlas exact ratio budget exceeded"));
    }
    Ok(q)
}
pub(crate) fn polynomial(p: &[Q]) -> ExactPolynomial {
    algebraic_intersections::polynomial(p)
}
pub(crate) fn decode_polynomial(p: &ExactPolynomial) -> Result<Vec<Q>> {
    let p = algebraic_intersections::decode_polynomial(p)?;
    bounded(&p)?;
    Ok(p)
}

pub(crate) fn branches(p: &[Vec<Q>]) -> Result<ParameterBranchDecomposition> {
    if p.len() > 9 || p.iter().any(|r| r.len() > 9) {
        return Err(refused("Atlas implicit tensor degree budget exceeded"));
    }
    for row in p {
        bounded(row)?;
    }
    super::branches::decompose(
        &p.to_vec(),
        Some([Q::one(), -Q::one()]),
        &Q::new(1.into(), 65536.into()),
    )
}

pub(crate) fn arrangement_branches(p: &[Vec<Q>]) -> Result<ParameterBranchDecomposition> {
    if p.len() > 17 || p.iter().any(|r| r.len() > 17) {
        return Err(refused("Atlas joint arrangement degree budget exceeded"));
    }
    for row in p {
        bounded(row)?;
    }
    super::branches::decompose(
        &p.to_vec(),
        Some([Q::one(), -Q::one()]),
        &Q::new(1.into(), 65536.into()),
    )
}
