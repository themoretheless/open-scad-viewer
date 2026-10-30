//! span intersections algorithm: certify_span_intersections.
use super::*;
pub fn certify_span_intersections(
    a: &Curve,
    b: &Curve,
    self_pair: bool,
    w: f64,
) -> Result<SpanIntersectionCertificate> {
    let source = source(a, b, self_pair, w)?;
    let x = cells(a)?;
    let y = cells(b)?;
    let mut pairs = Vec::new();
    for i in 0..x.len() {
        for j in 0..y.len() {
            if self_pair && i > j {
                continue;
            }
            let mut pair = witness(&x[i], &y[j], i, j, self_pair && i == j, w)?;
            if matches!(pair.kind.as_str(), "overlap" | "locus" | "whole-square") {
                let h = if pair.kind == "whole-square" {
                    Vec::new()
                } else {
                    decode_locus(&pair.relation)?
                };
                pair.branches = Some(branches::decompose(
                    &h,
                    self_pair.then(|| local_diagonal(&x[i], &y[j])),
                    &width(&x[i], &y[j], w),
                )?);
            }
            pairs.push(pair);
        }
    }
    let c = SpanIntersectionCertificate {
        version: 3,
        source,
        self_pair,
        maximum_parameter_width: w,
        pairs,
    };
    verify_span_intersections(a, b, &c)?;
    Ok(c)
}
