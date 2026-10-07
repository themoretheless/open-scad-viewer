//! Native exact edit and affine parameter map reports.
use super::{Curve, Result, ToleranceContext, check, context};
#[cfg(feature = "codec")]
pub mod serialization;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactEditOperation {
    Insert,
    Elevate,
}
pub struct ExactCurveEdit {
    pub curve: Curve,
    pub operation: ExactEditOperation,
    pub period_preserved: bool,
    pub tolerance: ToleranceContext,
}
pub struct AffineReparameterization {
    pub curve: Curve,
    pub old_domain: [f64; 2],
    pub new_domain: [f64; 2],
    pub scale: f64,
    pub period_preserved: bool,
    pub tolerance: ToleranceContext,
}
/// Affine monotone reparameterization, including periodic exterior knots.
pub fn reparameterize_curve_report(
    curve: &Curve,
    domain: [f64; 2],
    tolerance: Option<ToleranceContext>,
) -> Result<AffineReparameterization> {
    curve.validate()?;
    check(
        domain.iter().all(|value| value.is_finite()) && domain[0] < domain[1],
        "Reparameterization domain must be finite and increasing",
    )?;
    let old = curve.domain();
    let scale = (domain[1] - domain[0]) / (old[1] - old[0]);
    check(
        scale.is_finite() && scale > 0.,
        "Reparameterization scale must be finite and positive",
    )?;
    let mut output = curve.clone();
    output.knots = curve
        .knots
        .iter()
        .map(|knot| domain[0] + (knot - old[0]) * scale)
        .collect();
    // Knot validation alone permits distinct interior knots to merge into a
    // legal multiplicity. Such a merge changes the parameterization/geometry.
    check(
        curve
            .knots
            .windows(2)
            .zip(output.knots.windows(2))
            .all(|(a, b)| a[0] == a[1] || b[0] < b[1]),
        "Reparameterization collapsed distinct knots at coordinate precision",
    )?;
    output.validate()?;
    let tolerance = context(tolerance);
    Ok(AffineReparameterization {
        curve: output,
        old_domain: old,
        new_domain: domain,
        scale,
        period_preserved: curve.periodic,
        tolerance,
    })
}
pub fn certify_exact_edit_report(
    source: &Curve,
    operation: ExactEditOperation,
    parameter: f64,
    count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<ExactCurveEdit> {
    source.validate()?;
    let output = match operation {
        ExactEditOperation::Insert => source.insert(parameter, count)?,
        ExactEditOperation::Elevate => source.elevate(count)?,
    };
    let tolerance = context(tolerance);
    let period_preserved = source.periodic == output.periodic;
    Ok(ExactCurveEdit {
        curve: output,
        operation,
        period_preserved,
        tolerance,
    })
}
