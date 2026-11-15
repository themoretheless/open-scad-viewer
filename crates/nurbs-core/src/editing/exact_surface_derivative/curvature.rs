//! Original normal regularity and polynomial inner products for curvature.
use super::*;
use num_traits::Signed;

pub(super) fn dot(a: &[Poly], b: &[Poly], work: &mut Work) -> PResult<Poly> {
    let mut sum = Poly::new();
    for (a, b) in a[..3].iter().zip(&b[..3]) {
        for (key, value) in polynomial::multiply(a, b, work)? {
            polynomial::accumulate(&mut sum, key, &value, work)?;
        }
    }
    Ok(sum)
}

pub(super) fn require_regular(norm_squared: &Poly, work: &mut Work) -> PResult<()> {
    let (pu, pv) = storage::degrees(std::slice::from_ref(norm_squared));
    if pu > 25 || pv > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    // Prove regularity on the original chart before any field cancellation.
    if !storage::bernstein(norm_squared, pu, pv, work)?
        .iter()
        .all(Signed::is_positive)
    {
        return Err(Refusal::UnprovedDenominator);
    }

    Ok(())
}
