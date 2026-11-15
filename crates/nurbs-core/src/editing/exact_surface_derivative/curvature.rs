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

pub(super) fn second_form(
    h: &[Poly],
    widths: &[Q],
    normal: &[Poly],
    work: &mut Work,
) -> PResult<[Poly; 3]> {
    Ok([
        dot(normal, &mixed::target(h, [2, 0], widths, work)?, work)?,
        dot(normal, &mixed::target(h, [1, 1], widths, work)?, work)?,
        dot(normal, &mixed::target(h, [0, 2], widths, work)?, work)?,
    ])
}

pub(super) fn trace_numerator(
    metric: &[Poly],
    [e, f, g]: &[Poly; 3],
    work: &mut Work,
) -> PResult<Poly> {
    let e_g = polynomial::multiply(e, &metric[2], work)?;
    let g_e = polynomial::multiply(g, &metric[0], work)?;
    let f_f = polynomial::multiply(f, &metric[1], work)?;
    let mut trace = polynomial::subtract(polynomial::subtract(e_g, &f_f, work)?, &f_f, work)?;
    for (key, value) in g_e {
        polynomial::accumulate(&mut trace, key, &value, work)?;
    }
    Ok(trace)
}

pub(super) fn determinant_numerator([e, f, g]: &[Poly; 3], work: &mut Work) -> PResult<Poly> {
    let product = polynomial::multiply(e, g, work)?;
    let square = polynomial::multiply(f, f, work)?;
    polynomial::subtract(product, &square, work)
}
