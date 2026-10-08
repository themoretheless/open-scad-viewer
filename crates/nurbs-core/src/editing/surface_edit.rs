//! Native tensor-product edits. No fitting or snapping; real-arithmetic
//! geometric preservation does not constitute a binary64 error certificate.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Axis, Surface},
};

pub fn insert(source: &Surface, axis: Axis, parameter: f64, count: usize) -> Result<Surface> {
    source.edit_axis(axis, |c| c.insert(parameter, count))
}
pub fn elevate(source: &Surface, axis: Axis, degree: usize) -> Result<Surface> {
    source.edit_axis(axis, |c| c.elevate(degree))
}
/// Both outputs retain their original parameter coordinates and meet at the
/// requested isoparametric seam. Split must be strictly within the domain.
pub fn split(source: &Surface, axis: Axis, parameter: f64) -> Result<[Surface; 2]> {
    source.validate()?;
    let (degree, knots, count) = match axis {
        Axis::U => (
            source.degree_u,
            &source.knots_u,
            source.control_points.len(),
        ),
        Axis::V => (
            source.degree_v,
            &source.knots_v,
            source.control_points[0].len(),
        ),
    };
    let a = knots[degree];
    let b = knots[count];
    check(
        parameter.is_finite() && a < parameter && parameter < b,
        "Surface split must lie strictly inside the active domain",
    )?;
    Ok([
        source.edit_axis(axis, |c| c.trim(a, parameter))?,
        source.edit_axis(axis, |c| c.trim(parameter, b))?,
    ])
}
/// New S(u,v) = old S(v,u). Transposition reverses normal orientation.
pub fn transpose(source: &Surface) -> Result<Surface> {
    source.validate()?;
    let mut result = Surface {
        degree_u: source.degree_v,
        degree_v: source.degree_u,
        knots_u: source.knots_v.clone(),
        knots_v: source.knots_u.clone(),
        periodic_u: source.periodic_v,
        periodic_v: source.periodic_u,
        control_points: vec![],
        weights: vec![],
    };
    for v in 0..source.control_points[0].len() {
        result
            .control_points
            .push(source.control_points.iter().map(|r| r[v].clone()).collect());
        result
            .weights
            .push(source.weights.iter().map(|r| r[v]).collect());
    }
    result.validate()?;
    Ok(result)
}
/// Reverse one parameter within its existing active interval; also reverses
/// normal orientation. Periodic data follow the native curve reverse contract.
pub fn reverse(source: &Surface, axis: Axis) -> Result<Surface> {
    source.edit_axis(axis, Curve::reverse)
}
/// Split at every active knot into clamped rational Bezier patches. Retains
/// original U/V intervals. Output order is U-major; maximum 1024 patches.
pub fn decompose(source: &Surface) -> Result<Vec<Surface>> {
    source.validate()?;
    let breaks = |knots: &[f64], degree: usize, count: usize| {
        let mut out = knots
            .iter()
            .copied()
            .filter(|&k| k >= knots[degree] && k <= knots[count])
            .collect::<Vec<_>>();
        out.dedup();
        out
    };
    let u = breaks(
        &source.knots_u,
        source.degree_u,
        source.control_points.len(),
    );
    let v = breaks(
        &source.knots_v,
        source.degree_v,
        source.control_points[0].len(),
    );
    check(
        (u.len() - 1) * (v.len() - 1) <= 1024,
        "Bezier decomposition exceeds patch budget",
    )?;
    let mut out = Vec::new();
    for a in u.windows(2) {
        for b in v.windows(2) {
            out.push(source.trim([a[0], a[1], b[0], b[1]])?);
        }
    }
    Ok(out)
}
