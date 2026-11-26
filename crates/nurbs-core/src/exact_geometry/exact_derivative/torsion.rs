//! Original-source torsion, with binormal regularity before any cancellation.
use super::*;
use num_traits::Signed;
pub(super) fn target(h: &[Poly], width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let v = super::target(h, width, w)?;
    let a = higher::target(h, width, 2, w)?;
    let mut cross = Vec::new();
    let mut norm = Vec::new();
    for (j, k) in [(1, 2), (2, 0), (0, 1)] {
        let left = poly::multiply(&v[j], &a[k], w)?;
        let right = poly::multiply(&v[k], &a[j], w)?;
        let n = add(&left, &right, true, w)?;
        norm = add(&norm, &poly::multiply(&n, &n, w)?, false, w)?;
        cross.push(n);
    }
    let degree = norm.len().saturating_sub(1);
    if degree > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    if !poly::bernstein(&norm, degree, w)?
        .iter()
        .all(Signed::is_positive)
    {
        return Err(Refusal::UnprovedDenominator);
    }
    let jerk = higher::target(h, width, 3, w)?;
    let mut determinant = Vec::new();
    for i in 0..3 {
        determinant = add(
            &determinant,
            &poly::multiply(&cross[i], &jerk[i], w)?,
            false,
            w,
        )?;
    }
    let numerator = poly::multiply(&determinant, h.last().unwrap(), w)?;
    Ok(vec![numerator, Vec::new(), Vec::new(), norm])
}
