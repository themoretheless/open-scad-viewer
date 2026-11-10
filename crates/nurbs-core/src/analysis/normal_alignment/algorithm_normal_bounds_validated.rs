//! normal alignment algorithm: normal_bounds_validated.
use super::*;
pub fn normal_bounds_validated(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let mut spans = 0;
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            let indices = [u, v];
            let mut section = [[0.; 2]; 2];
            let mut outside = false;
            for axis in 0..2 {
                let i = indices[axis];
                let lo = knots[axis][i];
                let hi = knots[axis][i + 1];
                if lo == hi || domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok((None, spans));
            }
            spans += 1;
            // Differentiate the original span first, then restrict the derivative
            // polynomial. Narrow/point rectangles never divide by their own width.
            let j = jacobian_on(s, indices, section)?;
            for k in 0..3 {
                let a = (k + 1) % 3;
                let b = (k + 2) % 3;
                let n = j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?)?;
                hull[k][0] = hull[k][0].min(n.lo);
                hull[k][1] = hull[k][1].max(n.hi);
            }
        }
    }
    check(spans > 0, "Normal rectangle has no nonempty knot spans")?;
    Ok((Some(hull), spans))
}
