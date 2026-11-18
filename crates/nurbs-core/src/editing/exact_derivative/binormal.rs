//! Unnormalized binormal from original source jets, before storage.
use super::*;
pub(super) fn target(h: &[Poly], width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let velocity = super::target(h, width, w)?;
    let acceleration = higher::target(h, width, 2, w)?;
    let mut out = Vec::with_capacity(4);
    for (j, k) in [(1, 2), (2, 0), (0, 1)] {
        let left = poly::multiply(&velocity[j], &acceleration[k], w)?;
        let right = poly::multiply(&velocity[k], &acceleration[j], w)?;
        out.push(add(&left, &right, true, w)?);
    }
    out.push(poly::multiply(&velocity[3], &acceleration[3], w)?);
    Ok(out)
}
