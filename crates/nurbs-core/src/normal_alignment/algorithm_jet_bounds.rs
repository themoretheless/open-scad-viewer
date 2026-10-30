//! normal alignment algorithm: jet_bounds.
use super::*;
/// All incident span-side jets over the rectangle. Their union does not prove
/// source continuity across repeated knots; root inclusion must check that separately.
pub fn jet_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<JetBounds>, usize)> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)?;
    let mut out: Option<JetBounds> = None;
    let mut spans = 0;
    let union = |a: &mut I, b: I| {
        a.lo = a.lo.min(b.lo);
        a.hi = a.hi.max(b.hi);
    };
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let mut section = [[0.; 2]; 2];
            let indices = [u, v];
            let knots = [&s.knots_u, &s.knots_v];
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
            let next = jet_on(s, indices, section)?;
            if let Some(current) = &mut out {
                for k in 0..3 {
                    union(&mut current.point[k], next.point[k]);
                    for axis in 0..2 {
                        union(&mut current.first[axis][k], next.first[axis][k]);
                    }
                    for slot in 0..3 {
                        union(&mut current.second[slot][k], next.second[slot][k]);
                    }
                }
            } else {
                out = Some(next);
            }
        }
    }
    check(spans > 0, "Jet rectangle has no nonempty knot spans")?;
    Ok((out, spans))
}
