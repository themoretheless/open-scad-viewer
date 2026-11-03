//! Geometric squared curvature with original-source regularity admission.
use super::*;
use num_traits::Signed;
pub(super) fn target(h: &[Poly], width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let velocity = super::target(h, width, w)?;
    let acceleration = higher::target(h, width, 2, w)?;
    let dim = h.len() - 1;
    let mut norm = Vec::new();
    for n in &velocity[..dim] {
        norm = add(&norm, &poly::multiply(n, n, w)?, false, w)?;
    }
    // Check the uncancelled source speed, not a reduced curvature quotient.
    // Keep the shared exact Bernstein conversion within its degree ceiling.
    let degree = norm.len().saturating_sub(1);
    if degree > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let bernstein = poly::bernstein(&norm, degree, w)?;
    if !bernstein.iter().all(Signed::is_positive) {
        return Err(Refusal::UnprovedDenominator);
    }
    let pairs: &[(usize, usize)] = if dim == 2 {
        &[(0, 1)]
    } else {
        &[(1, 2), (2, 0), (0, 1)]
    };
    let mut cross_norm = Vec::new();
    for &(j, k) in pairs {
        let left = poly::multiply(&velocity[j], &acceleration[k], w)?;
        let right = poly::multiply(&velocity[k], &acceleration[j], w)?;
        let n = add(&left, &right, true, w)?;
        cross_norm = add(&cross_norm, &poly::multiply(&n, &n, w)?, false, w)?;
    }
    let w2 = poly::multiply(h.last().unwrap(), h.last().unwrap(), w)?;
    let mut out = vec![Vec::new(); dim];
    out[0] = poly::multiply(&cross_norm, &w2, w)?;
    let norm2 = poly::multiply(&norm, &norm, w)?;
    out.push(poly::multiply(&norm2, &norm, w)?);
    Ok(out)
}
