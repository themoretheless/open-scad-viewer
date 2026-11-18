//! Exact common-root membership in a closed dyadic interval.
use super::*;
fn closed_count(
    p: &Poly,
    interval: [f64; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<usize, Reason> {
    if zero(p) {
        return Err(Reason::MissingProof);
    }
    let (chain, _) = square_free_chain(p, ctx)?;
    // Sturm variations count (lo,hi]; explicitly include lo. This also
    // handles singleton endpoint boxes without any perturbation/sampling.
    Ok(count(&chain, interval[0], interval[1], ctx)?
        + usize::from(at(&chain[0], interval[0], ctx)? == Sign::Zero))
}
pub(crate) fn unique_common_root(
    first: &Poly,
    second: &Poly,
    interval: [f64; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    if closed_count(first, interval, ctx)? != 1 {
        return Err(Reason::MissingProof);
    }
    let mut a = first.clone();
    let mut b = second.clone();
    normalize(&mut a, ctx)?;
    normalize(&mut b, ctx)?;
    while !zero(&b) {
        ctx.charge(1)?;
        let (_, mut r, _) = pseudo_divide(&a, &b, ctx)?;
        normalize(&mut r, ctx)?;
        a = b;
        b = r;
    }
    // Every root of this exact PRS gcd is a root of the original first
    // polynomial. Its unique root in this box is therefore the SAME root.
    Ok(closed_count(&a, interval, ctx)? == 1)
}

/// Mathematical reference: https://doi.org/10.1007/s10817-017-9424-6
/// Exact sign via Sturm-Tarski: variation difference of SRemS(P,P'Q)
/// sums sign(Q) over the distinct roots of square-free P in (lo,hi).
/// After the unique closed-root check and explicit endpoint handling this
/// sum is precisely the requested sign. No separation of nearby roots or
/// binary64 approximation of the algebraic parameter is required.
pub(crate) fn unique_root_sign(
    first: &Poly,
    second: &Poly,
    interval: [f64; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Sign, Reason> {
    if unique_common_root(first, second, interval, ctx)? {
        return Ok(Sign::Zero);
    }
    for endpoint in interval {
        if at(first, endpoint, ctx)? == Sign::Zero {
            return at(second, endpoint, ctx);
        }
    }
    let (sf, _) = square_free_chain(first, ctx)?;
    let mut p = sf[0].clone();
    let mut q = mul(&derivative(&p, ctx)?, second, ctx)?;
    normalize(&mut p, ctx)?;
    normalize(&mut q, ctx)?;
    let limit = p.len().max(q.len()) + 3;
    let mut chain = vec![p];
    while !zero(&q) {
        ctx.charge(1)?;
        let (_, mut next, negative) = pseudo_divide(chain.last().unwrap(), &q, ctx)?;
        if !negative {
            next = next.iter().map(Expansion::negated).collect();
        }
        normalize(&mut next, ctx)?;
        chain.push(q);
        q = next;
        if chain.len() > limit {
            return Err(Reason::MissingProof);
        }
    }
    let lo = variations(&chain, interval[0], ctx)? as i64;
    let hi = variations(&chain, interval[1], ctx)? as i64;
    match lo - hi {
        -1 => Ok(Sign::Negative),
        1 => Ok(Sign::Positive),
        _ => Err(Reason::MissingProof),
    }
}
