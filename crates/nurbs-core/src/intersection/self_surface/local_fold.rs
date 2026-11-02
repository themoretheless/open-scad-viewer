//! Exact interior fold on an original B-spline span; no rounded extraction.
use super::*;
mod exact_span;

pub(super) fn find(s: &Surface, max_spans: usize) -> Option<(usize, [[f64; 2]; 2])> {
    let q = |x| BigRational::from_f64(x).unwrap();
    let mut visited = 0;
    for axis in 0..2 {
        let (knots, degree, count) = if axis == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        // Degree-zero charts cannot supply a continuous fold witness.
        if degree == 0 {
            continue;
        }
        for span in degree..count {
            let (a, b) = (knots[span], knots[span + 1]);
            if a == b {
                continue;
            }
            if visited == max_spans {
                return None;
            }
            visited += 1;
            // Multiplicity >=degree at both ends means the active original
            // controls already ARE the Bernstein net. Do not manufacture an
            // exact proof from floating-point knot insertion or trimmed data.
            // Other spans use exact Cox-de Boor homogeneous polynomials.
            let bernstein = knots[span - degree + 1..=span].iter().all(|&t| t == a)
                && knots[span + 1..=span + degree].iter().all(|&t| t == b);
            let start = span - degree;
            let other_count = if axis == 0 {
                s.control_points[0].len()
            } else {
                s.control_points.len()
            };
            let index = |i, j| if axis == 0 { (i, j) } else { (j, i) };
            let reflected = (0..=degree).all(|i| {
                (0..other_count).all(|j| {
                    let (u, v) = index(start + i, j);
                    let (r, t) = index(start + degree - i, j);
                    s.control_points[u][v] == s.control_points[r][t]
                        && s.weights[u][v] == s.weights[r][t]
                })
            });
            if !(bernstein && reflected) && !exact_span::reflected_span(s, axis, span) {
                continue;
            }
            let domain_start = q(knots[degree]);
            let width = q(knots[count]) - &domain_start;
            let mut witness = [[0.5; 2]; 2];
            let mut representable = true;
            for (i, numerator) in [
                q(a) * BigRational::from_integer(3.into()) + q(b),
                q(a) + q(b) * BigRational::from_integer(3.into()),
            ]
            .into_iter()
            .enumerate()
            {
                let normalized =
                    (numerator / BigRational::from_integer(4.into()) - &domain_start) / &width;
                let Some(value) = normalized.to_f64() else {
                    representable = false;
                    break;
                };
                if q(value) != normalized || value <= 0. || value >= 1. {
                    representable = false;
                    break;
                }
                witness[i][axis] = value;
            }
            if representable {
                return Some((axis, witness));
            }
        }
    }
    None
}
