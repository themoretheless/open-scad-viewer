//! Higher derivatives from the original homogeneous source, before storage.
use super::*;
fn physical(a: &Poly, width: &Q, w: &mut Work) -> PResult<Poly> {
    let mut out = differentiated(a, w)?;
    for x in &mut out {
        *x = w.op(x, width, |x, width| x / width)?;
    }
    Ok(out)
}
pub(super) fn target(h: &[Poly], width: &Q, order: usize, w: &mut Work) -> PResult<Vec<Poly>> {
    let denominator = h.last().unwrap();
    let dw = physical(denominator, width, w)?;
    let mut numerators = h[..h.len() - 1].to_vec();
    for exponent in 1..=order {
        let mut next = Vec::new();
        for n in &numerators {
            let dn = physical(n, width, w)?;
            let left = poly::multiply(&dn, denominator, w)?;
            let mut right = poly::multiply(n, &dw, w)?;
            for x in &mut right {
                *x = w.op(x, &Q::from_integer(exponent.into()), |x, k| x * k)?;
            }
            next.push(add(&left, &right, true, w)?);
        }
        numerators = next;
    }
    let mut weight = vec![Q::one()];
    for _ in 0..=order {
        weight = poly::multiply(&weight, denominator, w)?;
    }
    numerators.push(weight);
    Ok(numerators)
}
