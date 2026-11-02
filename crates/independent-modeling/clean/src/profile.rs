//! Explicitly sampled planar profiles. No general offset/self-intersection solver.
use crate::{Result, require};
pub fn regular_polygon(radius: f64, sides: usize) -> Result<Vec<[f64; 2]>> {
    require(
        radius.is_finite() && radius > 0. && (3..=4096).contains(&sides),
        "invalid regular polygon",
    )?;
    Ok((0..sides)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / sides as f64;
            [radius * a.cos(), radius * a.sin()]
        })
        .collect())
}
/// A rounded rectangle centered on XY. Radius strictly below each half extent;
/// each corner is a sampled quarter-circle, not an analytic solid fillet.
pub fn rounded_rectangle(
    size: [f64; 2],
    radius: f64,
    corner_segments: usize,
) -> Result<Vec<[f64; 2]>> {
    require(
        size.iter().all(|x| x.is_finite() && *x > 0.)
            && radius.is_finite()
            && radius > 0.
            && radius < size[0] / 2.
            && radius < size[1] / 2.
            && (1..=512).contains(&corner_segments),
        "invalid rounded rectangle",
    )?;
    let mut result = Vec::new();
    for (corner, [sx, sy]) in [[1., 1.], [-1., 1.], [-1., -1.], [1., -1.]]
        .into_iter()
        .enumerate()
    {
        let center = [sx * (size[0] / 2. - radius), sy * (size[1] / 2. - radius)];
        for i in 0..=corner_segments {
            let a = corner as f64 * std::f64::consts::FRAC_PI_2
                + i as f64 / corner_segments as f64 * std::f64::consts::FRAC_PI_2;
            result.push([center[0] + radius * a.cos(), center[1] + radius * a.sin()]);
        }
    }
    Ok(result)
}
/// Straight corner cuts on a rectangle, not general edge chamfering.
pub fn chamfered_rectangle(size: [f64; 2], cut: f64) -> Result<Vec<[f64; 2]>> {
    require(
        size.iter().all(|x| x.is_finite() && *x > 0.)
            && cut.is_finite()
            && cut > 0.
            && cut < size[0] / 2.
            && cut < size[1] / 2.,
        "invalid rectangle corner cut",
    )?;
    let [x, y] = size.map(|v| v / 2.);
    Ok(vec![
        [-x + cut, -y],
        [x - cut, -y],
        [x, -y + cut],
        [x, y - cut],
        [x - cut, y],
        [-x + cut, y],
        [-x, y - cut],
        [-x, -y + cut],
    ])
}
