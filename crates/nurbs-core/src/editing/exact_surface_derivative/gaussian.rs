//! Gaussian curvature using the original rational jets.
use super::*;
use curvature::{dot, require_regular};

pub(super) fn target(h: &[Poly], widths: &[Q], work: &mut Work) -> PResult<Vec<Poly>> {
    let normal = area_target(h, widths, work)?;
    let norm_squared = dot(&normal, &normal, work)?;
    require_regular(&norm_squared, work)?;

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
