use super::*;
// Zero controls off the owned boundary need not be reachable surface points.
// Positive single-Bezier bases restrict support-plane contact to complete
// zero natural edges and zero corners. Extra collapsed edges are allowed only
// when all their controls equal an endpoint of the owned boundary curve.
pub(super) fn bezier_plane_edge_only(
    s: &Surface,
    plane: [&[f64]; 3],
    boundary: (usize, usize),
) -> bool {
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if (0..2).any(|k| {
        degrees[k] == 0 || sizes[k] != degrees[k] + 1 || !clamped(knots[k], degrees[k], sizes[k])
    }) {
        return false;
    }
    let at = |fixed: usize, free: usize| {
        if boundary.0 == 0 {
            &s.control_points[fixed][free]
        } else {
            &s.control_points[free][fixed]
        }
    };
    let endpoints = [at(boundary.1, 0), at(boundary.1, sizes[1 - boundary.0] - 1)];
    let zero = |p: &[f64]| orient(&[plane[0], plane[1], plane[2], p], None) == Some(Sign::Zero);
    for u in [0, sizes[0] - 1] {
        for v in [0, sizes[1] - 1] {
            if [u, v][boundary.0] != boundary.1
                && zero(&s.control_points[u][v])
                && !endpoints.contains(&&s.control_points[u][v])
            {
                return false;
            }
        }
    }
    for axis in 0..2 {
        for fixed in [0, sizes[axis] - 1] {
            if axis == boundary.0 && fixed == boundary.1 {
                continue;
            }
            let controls = (0..sizes[1 - axis])
                .map(|free| {
                    if axis == 0 {
                        &s.control_points[fixed][free]
                    } else {
                        &s.control_points[free][fixed]
                    }
                })
                .collect::<Vec<_>>();
            if controls.iter().all(|p| zero(p))
                && !endpoints
                    .iter()
                    .any(|endpoint| controls.iter().all(|p| *p == *endpoint))
            {
                return false;
            }
        }
    }
    true
}
