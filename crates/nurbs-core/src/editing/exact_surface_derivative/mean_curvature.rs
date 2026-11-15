//! Rational squared mean curvature using the original surface jets.
use super::*;
use curvature::{dot, require_regular};

pub(super) fn target(h: &[Poly], widths: &[Q], work: &mut Work) -> PResult<Vec<Poly>> {
    let normal = area_target(h, widths, work)?;
    let norm_squared = dot(&normal, &normal, work)?;
    require_regular(&norm_squared, work)?;

    let metric = metric_target(h, widths, work)?;
    let e = dot(&normal, &mixed::target(h, [2, 0], widths, work)?, work)?;
    let f = dot(&normal, &mixed::target(h, [1, 1], widths, work)?, work)?;
    let g = dot(&normal, &mixed::target(h, [0, 2], widths, work)?, work)?;
    let eg = polynomial::multiply(&e, &metric[2], work)?;
    let fe = polynomial::multiply(&g, &metric[0], work)?;
    let cross_term = polynomial::multiply(&f, &metric[1], work)?;
    let mut trace = polynomial::subtract(
        polynomial::subtract(eg, &cross_term, work)?,
        &cross_term,
        work,
    )?;
    for (key, value) in fe {
        polynomial::accumulate(&mut trace, key, &value, work)?;
    }

    // The unnormalized second form is over W^7, the first form over W^4.
    // Squaring their trace gives W^22, while |Su × Sv|^6 gives W^24.
    let trace_squared = polynomial::multiply(&trace, &trace, work)?;
    let w_squared = polynomial::multiply(&h[3], &h[3], work)?;
    let numerator = polynomial::multiply(&trace_squared, &w_squared, work)?;
    let norm_fourth = polynomial::multiply(&norm_squared, &norm_squared, work)?;
    let mut denominator = polynomial::multiply(&norm_fourth, &norm_squared, work)?;
    let four = Q::from_integer(4.into());
    for coefficient in denominator.values_mut() {
        *coefficient = work.op(coefficient, &four, |a, b| a * b)?;
    }
    Ok(vec![numerator, Poly::new(), Poly::new(), denominator])
}
