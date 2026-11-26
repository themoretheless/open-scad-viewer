//! Squared speed from the original homogeneous derivative, before storage.
use super::*;
pub(super) fn target(h: &[Poly], width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let velocity = super::target(h, width, w)?;
    let dim = velocity.len() - 1;
    let mut sum = Vec::new();
    for n in &velocity[..dim] {
        let square = poly::multiply(n, n, w)?;
        sum = add(&sum, &square, false, w)?;
    }
    let mut out = vec![Vec::new(); dim];
    out[0] = sum;
    out.push(poly::multiply(&velocity[dim], &velocity[dim], w)?);
    Ok(out)
}
