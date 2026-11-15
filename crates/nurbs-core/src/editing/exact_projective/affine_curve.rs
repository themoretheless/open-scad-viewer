//! Exact affine change of a rational curve's absolute clock.
use super::{Curve, PResult, Refusal, Report, Work, finish, mobius::validate_clock};
use crate::{Result, check};

/// Place a rational curve's original start and end at the given parameter
/// values. Descending endpoints reverse the traversal. All knots,
/// including exterior knots of unclamped and periodic curves, are
/// transformed by the same exact affine map. The output uses the original
/// Cartesian poles and weights, reversing their order when necessary.
/// A knot that cannot be stored exactly with the new clock causes refusal.
pub fn affine_curve_clock(
    source: &Curve,
    endpoints: [f64; 2],
    max_work: u64,
) -> Result<Report<Curve>> {
    source.validate()?;
    validate_endpoints(endpoints)?;
    validate_clock(&[], max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let out = transform(source, endpoints, &mut work);
    Ok(finish(out, work))
}

fn transform(source: &Curve, [start, end]: [f64; 2], work: &mut Work) -> PResult<Curve> {
    let knots = map_knots(&source.knots, source.domain(), [start, end], work)?;
    let mut out = source.clone();
    if start > end {
        out.control_points.reverse();
        out.weights.reverse();
    }
    out.knots = knots;
    out.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(out)
}

pub(super) fn validate_endpoints(endpoints: [f64; 2]) -> Result<()> {
    check(
        endpoints.iter().all(|t| t.is_finite() && t.abs() <= 1e9) && endpoints[0] != endpoints[1],
        "An affine clock needs two distinct finite endpoints within knot limits",
    )
}

pub(super) fn map_knots(
    source: &[f64],
    [a, b]: [f64; 2],
    [start, end]: [f64; 2],
    work: &mut Work,
) -> PResult<Vec<f64>> {
    let old_start = work.lift(a)?;
    let old_end = work.lift(b)?;
    let new_start = work.lift(start)?;
    let new_end = work.lift(end)?;
    let original_width = work.op(&old_end, &old_start, |a, b| a - b)?;
    let output_width = work.op(&new_end, &new_start, |a, b| a - b)?;
    let scale = work.op(&output_width, &original_width, |a, b| a / b)?;
    let mut knots = Vec::with_capacity(source.len());
    for &knot in source {
        let k = work.lift(knot)?;
        let offset = work.op(&k, &old_start, |a, b| a - b)?;
        let delta = work.op(&offset, &scale, |a, b| a * b)?;
        let absolute = work.op(&new_start, &delta, |a, b| a + b)?;
        knots.push(work.float(&absolute)?);
    }
    if start > end {
        knots.reverse();
    }
    Ok(knots)
}
