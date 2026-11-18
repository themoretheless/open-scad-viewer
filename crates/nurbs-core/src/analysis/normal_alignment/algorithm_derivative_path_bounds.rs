//! normal alignment algorithm: derivative_path_bounds.
use super::*;
/// Cover every intersected nonempty knot rectangle, including both one-sided
/// derivatives at knots. Reject jumps across the integrated coordinate.
pub fn derivative_path_bounds<'a>(s: &'a Surface, domain: [[f64; 2]; 2],
    axis: usize, max_spans: usize) -> Result<(Option<OriginalDerivativeBounds<'a>>, usize)> {
    check(axis < 2, "Choose an original derivative coordinate")?;
    validate_rectangle(s, domain, max_spans)?;
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if degrees[axis] == 0 { return Ok((None, 0)); }
    let mut i = degrees[axis] + 1;
    while i < counts[axis] {
        let knot = knots[axis][i];
        let mut end = i + 1;
        while end < knots[axis].len() && knots[axis][end] == knot { end += 1; }
        if domain[axis][0] <= knot && knot <= domain[axis][1] && end-i > degrees[axis] {
            return Ok((None, 0));
        }
        i = end;
    }
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spans = 0;
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] == knots[0][u+1] || knots[1][v] == knots[1][v+1] { continue; }
            let indices = [u,v];
            let section = std::array::from_fn(|a| [domain[a][0].max(knots[a][indices[a]]),
                domain[a][1].min(knots[a][indices[a]+1])]);
            if section.iter().any(|a| a[0] > a[1]) { continue; }
            if spans == max_spans { return Ok((None, spans)); }
            let j = jacobian_on(s, indices, section)?;
            spans += 1;
            for k in 0..3 {
                hull[k][0] = hull[k][0].min(j[k][axis].lo);
                hull[k][1] = hull[k][1].max(j[k][axis].hi);
            }
        }
    }
    if spans == 0 { return Ok((None, 0)); }
    Ok((Some(OriginalDerivativeBounds { surface:s, domain, axis, components:hull }), spans))
}
