//! Gaussian curvature using the original rational jets.
use super::*;
use num_traits::Signed;

fn dot(a: &[Poly], b: &[Poly], work: &mut Work) -> PResult<Poly> {
    let mut sum = Poly::new();
    for (a, b) in a[..3].iter().zip(&b[..3]) {
        for (key, value) in polynomial::multiply(a, b, work)? {
            polynomial::accumulate(&mut sum, key, &value, work)?;
        }
    }
    Ok(sum)
}

pub(super) fn target(h: &[Poly], widths: &[Q], work: &mut Work) -> PResult<Vec<Poly>> {
    let normal = area_target(h, widths, work)?;
    let norm_squared = dot(&normal, &normal, work)?;
    let (pu, pv) = storage::degrees(std::slice::from_ref(&norm_squared));
    if pu > 25 || pv > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    // A zero Jacobian makes K undefined, even if the determinant vanishes.
    // Prove the original squared normal positive before building or reducing K.
    if !storage::bernstein(&norm_squared, pu, pv, work)?
        .iter()
        .all(Signed::is_positive)
    {
        return Err(Refusal::UnprovedDenominator);
    }

    let uu = dot(&normal, &mixed::target(h, [2, 0], widths, work)?, work)?;
    let uv = dot(&normal, &mixed::target(h, [1, 1], widths, work)?, work)?;
    let vv = dot(&normal, &mixed::target(h, [0, 2], widths, work)?, work)?;
    let product = polynomial::multiply(&uu, &vv, work)?;
    let mixed_squared = polynomial::multiply(&uv, &uv, work)?;
    let determinant = polynomial::subtract(product, &mixed_squared, work)?;
    // Unnormalized second fundamental numerators are over W^7.
    // The fourth power of the normal length is over W^16.
    let w_squared = polynomial::multiply(&h[3], &h[3], work)?;
    let numerator = polynomial::multiply(&determinant, &w_squared, work)?;
    let denominator = polynomial::multiply(&norm_squared, &norm_squared, work)?;
    Ok(vec![numerator, Poly::new(), Poly::new(), denominator])
}
