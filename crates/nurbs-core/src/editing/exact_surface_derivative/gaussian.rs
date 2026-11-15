//! Gaussian curvature using the original rational jets.
use super::*;
use curvature::{determinant_numerator, dot, require_regular, second_form};

pub(super) fn target(h: &[Poly], widths: &[Q], work: &mut Work) -> PResult<Vec<Poly>> {
    let normal = area_target(h, widths, work)?;
    let norm_squared = dot(&normal, &normal, work)?;
    require_regular(&norm_squared, work)?;

    let second = second_form(h, widths, &normal, work)?;
    let determinant = determinant_numerator(&second, work)?;
    // Unnormalized second fundamental numerators are over W^7.
    // The fourth power of the normal length is over W^16.
    let w_squared = polynomial::multiply(&h[3], &h[3], work)?;
    let numerator = polynomial::multiply(&determinant, &w_squared, work)?;
    let denominator = polynomial::multiply(&norm_squared, &norm_squared, work)?;
    Ok(vec![numerator, Poly::new(), Poly::new(), denominator])
}
