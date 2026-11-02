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
