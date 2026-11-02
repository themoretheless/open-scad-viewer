//! Original same-clock/common-basis G2 perturbation bounds without proxies.
use super::*;
use crate::surface_measure::jets;
pub(super) fn eligible(p: [&Curve; 2], s: [&Surface; 2], reverse: [bool; 2]) -> bool {
    p[0] == p[1]
        && reverse[0] == reverse[1]
        && s[0].degree_u == s[1].degree_u
        && s[0].degree_v == s[1].degree_v
        && s[0].knots_u == s[1].knots_u
        && s[0].knots_v == s[1].knots_v
        && s[0].weights == s[1].weights
        && !s[0].periodic_u
        && !s[0].periodic_v
        && !s[1].periodic_u
        && !s[1].periodic_v
        && s[0].weights.iter().flatten().all(|w| *w > 0.)
}
fn c2(s: &Surface, uv: [[f64; 2]; 2]) -> bool {
    for (a, (p, k, n)) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ]
    .into_iter()
    .enumerate()
    {
        let mut i = p + 1;
        while i < n {
            let mut j = i + 1;
            while j < n && k[j] == k[i] {
                j += 1;
            }
            if uv[a][0] <= k[i] && k[i] <= uv[a][1] && p < j - i + 2 {
                return false;
            }
            i = j;
        }
    }
    true
}
pub(super) fn upper(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<[f64; 2]>> {
    if !eligible(p, s, reverse) {
        return Ok(None);
    }
    let Some(uv) = uv_box(p[0], t, reverse[0], work, max)? else {
        return Ok(None);
    };
    if !c2(s[0], uv) {
        return Ok(None);
    }
    let mut upper = [0f64; 2];
    let mut covered = false;
    for u in s[0].degree_u..s[0].control_points.len() {
        for v in s[0].degree_v..s[0].control_points[0].len() {
            let full = [
                [s[0].knots_u[u], s[0].knots_u[u + 1]],
                [s[0].knots_v[v], s[0].knots_v[v + 1]],
            ];
            let query =
                std::array::from_fn(|a| [uv[a][0].max(full[a][0]), uv[a][1].min(full[a][1])]);
            if (0..2).any(|a| full[a][0] >= full[a][1] || query[a][0] > query[a][1]) {
                continue;
            }
            if max.saturating_sub(*work) < 3 {
                return Ok(None);
            }
            *work += 3;
            let base = jets::calculate_partial_stable_up_to(s[0], [u, v], query, [None; 2], 2)?;
            let delta = jets::calculate_partial_stable_difference(s[1], s[0], [u, v], query, 2)?;
            let (bound, _) = crate::surface_collar::world_difference_bounds(&base, &delta, 10000)?;
            let Some(bound) = bound else {
                return Ok(None);
            };
            for k in 0..2 {
                upper[k] = upper[k].max(bound[k]);
            }
            covered = true;
        }
    }
    Ok(covered.then_some(upper))
}
