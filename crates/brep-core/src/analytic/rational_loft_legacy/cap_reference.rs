//! An orientation seed inside an original positive knot span. This is not a
//! regularity or global normal proof: complete retained/source owners do those.
use super::*;
pub(super) fn parameter(knots: &[f64], degree: usize, count: usize) -> Result<f64> {
    let center = knots[degree] * 0.5 + knots[count] * 0.5;
    let spans = || knots[degree..=count].windows(2).filter(|p| p[0] < p[1]);
    if spans().any(|p| p[0] < center && center < p[1]) {
        return Ok(center);
    }
    spans()
        .find_map(|p| {
            let t = p[0] * 0.5 + p[1] * 0.5;
            (p[0] < t && t < p[1]).then_some(t)
        })
        .ok_or_else(|| err("No representable interior cap reference parameter"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_avoids_original_knot_lines_and_never_claims_an_endpoint_derivative() {
        assert_eq!(parameter(&[0., 0., 0.5, 1., 1.], 1, 3).unwrap(), 0.25);
        assert_eq!(parameter(&[-3., -3., 1., 5., 5.], 1, 3).unwrap(), -1.);
        assert_eq!(parameter(&[0., 0., 1., 1.], 1, 2).unwrap(), 0.5);
        let next = f64::from_bits(1_f64.to_bits() + 1);
        assert!(parameter(&[1., 1., next, next], 1, 2).is_err());
    }
}
