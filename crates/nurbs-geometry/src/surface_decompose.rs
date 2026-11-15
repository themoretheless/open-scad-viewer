//! Bezier decomposition of a tensor-product surface.
use crate::{Result, check, surface::Surface};

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
