//! Original rational-curve point isolation by bounded convex hulls.
use super::*;

/// Isolate a point inverse with positive-weight rational convex-hull
/// exclusion.  Every retained interval is produced by exact knot insertion
/// (`Curve::trim`); therefore its Euclidean control hull encloses the complete
/// rational image.  Ambiguous, non-isolated and non-representable roots refuse.
pub(super) fn isolate_curve_point(base: &Curve, target: &[f64]) -> Result<f64> {
    const MAX_CELLS: usize = 16_384;
    const PARAMETER_WIDTH: f64 = 1e-12;
    const GEOMETRIC_WIDTH: f64 = 1e-9;
    fn contains(curve: &Curve, target: &[f64]) -> bool {
        (0..target.len()).all(|axis| {
            let lo = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            target[axis] >= lo - 1e-12 && target[axis] <= hi + 1e-12
        })
    }
    let domain = base.domain();
    let mut breaks = base
        .knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    let mut pending = breaks
        .array_windows()
        .map(|[a, b]| base.trim(*a, *b))
        .collect::<Result<Vec<_>>>()?;
    let mut isolated = Vec::<[f64; 2]>::new();
    let mut visited = 0usize;
    while let Some(cell) = pending.pop() {
        visited += 1;
        if visited > MAX_CELLS {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exceeds certified isolation budget",
            ));
        }
        if !contains(&cell, target) {
            continue;
        }
        let domain = cell.domain();
        let hull_width = (0..target.len())
            .map(|axis| {
                let lo = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                hi - lo
            })
            .fold(0., f64::max);
        if domain[1] - domain[0] <= PARAMETER_WIDTH || hull_width <= GEOMETRIC_WIDTH {
            isolated.push(domain);
            continue;
        }
        let mid = (domain[0] + domain[1]) / 2.;
        if mid == domain[0] || mid == domain[1] {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exhausted parameter precision",
            ));
        }
        let [left, right] = cell.split(mid)?;
        pending.push(right);
        pending.push(left);
    }
    isolated.sort_by(|left, right| left[0].total_cmp(&right[0]));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in isolated {
        if let Some(last) = merged.last_mut()
            && interval[0] <= last[1] + PARAMETER_WIDTH
        {
            last[1] = last[1].max(interval[1]);
            continue;
        }
        merged.push(interval)
    }
    if merged.len() != 1 {
        return Err(refuse(if merged.is_empty() {
            "TRIMMED_CURVE point selector is not on the basis curve"
        } else {
            "TRIMMED_CURVE point selector inverse is not unique"
        }));
    }
    let interval = merged[0];
    let mut parameter = (interval[0] + interval[1]) / 2.;
    for _ in 0..12 {
        let evaluation = base.evaluate(parameter)?;
        let Some(derivative) = evaluation.d1 else {
            break;
        };
        let denominator = derivative.iter().map(|value| value * value).sum::<f64>();
        if denominator <= 1e-24 {
            break;
        }
        let correction = evaluation
            .point
            .iter()
            .zip(target)
            .zip(&derivative)
            .map(|((point, wanted), tangent)| (point - wanted) * tangent)
            .sum::<f64>()
            / denominator;
        let next = (parameter - correction).clamp(interval[0], interval[1]);
        if next == parameter {
            break;
        }
        parameter = next;
    }
    let residual = base
        .evaluate(parameter)?
        .point
        .iter()
        .zip(target)
        .map(|(point, wanted)| (point - wanted) * (point - wanted))
        .sum::<f64>()
        .sqrt();
    if !residual.is_finite() || residual > GEOMETRIC_WIDTH {
        return Err(refuse(
            "TRIMMED_CURVE isolated point inverse failed certified residual bound",
        ));
    }
    Ok(parameter)
}
