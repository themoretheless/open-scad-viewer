//! Scalar-law validation and derivative enclosures for the progressive miter sweep.
use super::scalar_certificate;
use crate::{check, curve::Curve, Result};
pub(super) fn law(c: &Curve, f: f64) -> Result<f64> {
    let [a, b] = c.domain();
    Ok(c.evaluate(a + (b - a) * f)?.point[0])
}
pub(super) fn validate_law(c: &Curve, positive: bool) -> Result<()> {
    c.validate()?;
    let [a, b] = c.domain();
    let mut index = c.degree + 1;
    while index < c.knots.len() - c.degree - 1 {
        let value = c.knots[index];
        let mut end = index + 1;
        while end < c.knots.len() && c.knots[end] == value {
            end += 1;
        }
        check(
            end - index <= c.degree,
            "Miter laws must be continuous at internal knots",
        )?;
        index = end;
    }
    check(
        (b - a).is_finite(),
        "Miter law domain span is not representable",
    )?;
    check(
        c.control_points
            .iter()
            .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0. && (!positive || p[0] > 0.)),
        "Miter scalar laws require [value,0,0]; scale must be positive",
    )
}
/// Outward scalar-law jets scaled to the normalized local interval.
/// Sweep frames, miter shear and retained endpoints still need their own
/// enclosures before the complete interpolation report can be certified.
pub(super) fn scalar_bounds(c: &Curve, lo: f64, hi: f64) -> Result<(f64, f64, f64, bool)> {
    use crate::distance_bounds::Interval;
    let [a, b] = c.domain();
    let span = Interval::point(b).sub(Interval::point(a))?;
    let parameter = Interval::point(a)
        .add(span.mul(Interval::new(lo, hi)?)?)?
        .intersect(a, b)?;
    let report =
        scalar_certificate::certify(c, [parameter.lo, parameter.hi], c.control_points.len())?;
    check(
        report.status == scalar_certificate::Status::Certified,
        "Miter scalar derivative enclosure is unresolved",
    )?;
    let magnitude = |range: [f64; 2]| range[0].abs().max(range[1].abs());
    let factor = span.mul(Interval::point(hi).sub(Interval::point(lo))?)?;
    let first = Interval::point(magnitude(report.first.unwrap())).mul(factor)?;
    let second = Interval::point(magnitude(report.second.unwrap())).mul(factor.mul(factor)?)?;
    Ok((
        magnitude(report.value.unwrap()),
        first.hi,
        second.hi,
        report.single_span,
    ))
}
