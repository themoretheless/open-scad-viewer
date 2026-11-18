use super::*;
pub(super) fn representative(
    band: [f64; 2],
    domain: [f64; 2],
    used: &mut usize,
    max: usize,
) -> Result<f64> {
    spend(used, max, 2100)?;
    if band.iter().any(|x| !x.is_finite()) || band[0] > band[1] {
        return Err(unproven("Original cut clock bands do not intersect"));
    }
    let t = crate::edge_partition::root_representative(band, domain);
    if !t.is_finite() || t < band[0] || t > band[1] || t <= domain[0] || t >= domain[1] {
        return Err(unproven(
            "No interior binary64 representative in the original cut band",
        ));
    }
    Ok(t)
}
pub(super) fn evaluate(c: &Curve, t: f64, used: &mut usize, max: usize) -> Result<Vec<I>> {
    let spans = c.knots.windows(2).filter(|w| w[0] < w[1]).count();
    spend(used, max, spans)?;
    interval_eval::evaluate_interval(c, I::point(t))
}
fn upper_norm(parts: impl Iterator<Item = Result<I>>) -> Result<f64> {
    let mut sum = I::point(0.);
    for d in parts {
        let d = d?;
        let a = d.lo.abs().max(d.hi.abs());
        sum = sum.add(I::point(a).mul(I::point(a))?)?;
    }
    Ok(sum.hi.max(0.).sqrt().next_up())
}
pub(super) fn distance(point: &[f64], bounds: &[[f64; 2]]) -> Result<f64> {
    if point.len() != bounds.len() {
        return Err(invalid("Cut enclosure dimension mismatch"));
    }
    upper_norm(
        point
            .iter()
            .zip(bounds)
            .map(|(&p, b)| I::point(p).sub(I::new(b[0], b[1])?)),
    )
}
pub(super) fn box_distance(a: &[I], b: &[[f64; 2]]) -> Result<f64> {
    if a.len() != b.len() {
        return Err(invalid("Cut UV dimension mismatch"));
    }
    upper_norm(a.iter().zip(b).map(|(&a, b)| a.sub(I::new(b[0], b[1])?)))
}
