use super::{Chain, Curve, Limits, lift, surface_offset};
use crate::{Result, check};
fn endpoint(c: &Curve, end: bool) -> [f64; 3] {
    let p = if end {
        c.control_points.last().unwrap()
    } else {
        &c.control_points[0]
    };
    [p[0], p[1], p[2]]
}
pub(super) fn corner(
    source: &crate::surface::Surface,
    d: f64,
    p: [f64; 2],
    spans: usize,
) -> Result<[f64; 3]> {
    let b = surface_offset::bounds(source, [[p[0]; 2], [p[1]; 2]], d, spans)?;
    let image = b
        .image
        .ok_or_else(|| crate::input("Original offset rail corner remains unresolved"))?;
    Ok(image.map(|r| r[0] * 0.5 + r[1] * 0.5))
}
pub(super) fn exact_world_joins(curves: &[Curve]) -> Result<()> {
    for (i, c) in curves.iter().enumerate() {
        c.validate()?;
        let n = c.control_points.len();
        let d = c.domain();
        check(
            c.control_points[0].len() == 3
                && !c.periodic
                && c.knots[..=c.degree].iter().all(|t| *t == d[0])
                && c.knots[n..].iter().all(|t| *t == d[1])
                && c.control_points.last()
                    == Some(&curves[(i + 1) % curves.len()].control_points[0]),
            "Actual world loop lacks literal clamped endpoint closure",
        )?;
    }
    Ok(())
}
pub(super) fn fit(
    chain: &Chain,
    side: usize,
    closure: &[Curve],
    world: &mut Vec<Curve>,
    limits: Limits,
) -> Result<Vec<lift::FitReport>> {
    if closure.is_empty() {
        return Ok(vec![]);
    }
    let source = &chain.sources[side];
    let d = chain.distances[side];
    let mut vertices = vec![endpoint(world.last().unwrap(), true)];
    for curve in closure.iter().take(closure.len() - 1) {
        let p = curve.control_points.last().unwrap();
        vertices.push(corner(source, d, [p[0], p[1]], limits.closing_fit.spans)?);
    }
    vertices.push(endpoint(&world[0], false));
    let mut reports = vec![];
    for (i, uv) in closure.iter().enumerate() {
        let r = lift::fit(
            uv,
            source,
            d,
            chain.tolerance_mm,
            Some([vertices[i], vertices[i + 1]]),
            limits.closing_fit,
        )?;
        world.push(
            r.curve
                .as_ref()
                .ok_or_else(|| crate::input("Original closing rail lift remains unqualified"))?
                .clone(),
        );
        reports.push(r);
    }
    Ok(reports)
}
