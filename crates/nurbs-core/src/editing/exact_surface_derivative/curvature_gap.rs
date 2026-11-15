//! Squared principal curvature separation from original rational forms.
use super::*;
use curvature::{determinant_numerator, dot, require_regular, second_form, trace_numerator};

pub(super) fn target(h: &[Poly], widths: &[Q], work: &mut Work) -> PResult<Vec<Poly>> {
    let normal = area_target(h, widths, work)?;
    let norm_squared = dot(&normal, &normal, work)?;
    require_regular(&norm_squared, work)?;

    let metric = metric_target(h, widths, work)?;
    let second = second_form(h, widths, &normal, work)?;
    let trace = trace_numerator(&metric, &second, work)?;
    let determinant = determinant_numerator(&second, work)?;

    // (k1-k2)^2 = 4(H^2-K). The trace T is over W^11, the second
    // determinant D over W^14, and the squared-normal numerator is A.
    // Both T^2 and 4 D A therefore have denominator W^22.
    let trace_squared = polynomial::multiply(&trace, &trace, work)?;
    let mut correction = polynomial::multiply(&determinant, &norm_squared, work)?;
    let four = Q::from_integer(4.into());
    for value in correction.values_mut() {
        *value = work.op(value, &four, |a, b| a * b)?;
    }
    let gap = polynomial::subtract(trace_squared, &correction, work)?;
    let weight_squared = polynomial::multiply(&h[3], &h[3], work)?;
    let numerator = polynomial::multiply(&gap, &weight_squared, work)?;
    let norm_fourth = polynomial::multiply(&norm_squared, &norm_squared, work)?;
    let denominator = polynomial::multiply(&norm_fourth, &norm_squared, work)?;
    Ok(vec![numerator, Poly::new(), Poly::new(), denominator])
}
