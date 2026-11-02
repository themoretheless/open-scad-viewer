//! Central original iso versus a disjoint parameter window of its own support.
use super::*;
use crate::distance_bounds::Interval as I;
pub(super) fn find(
    s: &Surface,
    max_cells: usize,
    search: &mut ProbeSearch,
) -> Result<Option<CertifiedCrossing>> {
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for axis in 0..2 {
        for reverse in [false, true] {
            if search.cells == max_cells {
                search.exhausted = true;
                return Ok(None);
            }
            search.cells += 1;
            let mut attempt = ProbeAttempt {
                kind: if reverse {
                    "curve-surface-reverse"
                } else {
                    "curve-surface-forward"
                },
                axis,
                parameter: 0.5,
                cells: 1,
                iso_coverage_complete: false,
                refusal: None,
            };
            if search.cells == max_cells {
                attempt.refusal = Some("probe budget".into());
                search.attempts.push(attempt);
                search.exhausted = true;
                return Ok(None);
            }
            let curve = linear_crossing::iso(s, axis)
                .and_then(|h| nonlinear_crossing::original_iso(s, axis, &h));
            let Some(curve) = curve else {
                attempt.refusal =
                    Some("3D source iso is not exactly binary64-representable".into());
                search.attempts.push(attempt);
                continue;
            };
            let [a, b] = domains[axis];
            let cut = a + (b - a) * 0.4375;
            if cut <= a || cut >= b {
                attempt.refusal = Some("no interior source split".into());
                search.attempts.push(attempt);
                continue;
            }
            let (left, right) = ([a, cut], [cut, b]);
            let (from, to) = if reverse {
                (right, left)
            } else {
                (left, right)
            };
            let mut window = [from, domains[0], domains[1]];
            window[axis + 1] = to;
            // Leave work for the reverse orientation and the other axis; a
            // diagonal boundary in one window must not consume every proposal.
            let slots_left = 4 - (axis * 2 + usize::from(reverse));
            let quota = ((max_cells - search.cells) / slots_left).max(1);
            let report = match crate::intersection::certified_curve_surface::first_in_window(
                &curve, s, window, quota,
            ) {
                Ok(r) => r,
                Err(e) => {
                    attempt.cells += quota;
                    attempt.refusal = Some(
                        format!("{}: {}", e.code, e.message)
                            .chars()
                            .take(1024)
                            .collect(),
                    );
                    search.cells += quota;
                    search.attempts.push(attempt);
                    continue;
                }
            };
            search.cells += report.cells;
            attempt.cells += report.cells;
            search.attempts.push(attempt);
            for root in report.roots {
                let mut uv = [[[0.5, 0.5]; 2]; 2];
                let mut valid = true;
                for (i, range) in root.parameter_box.into_iter().enumerate() {
                    let coordinate = if i == 0 { axis } else { i - 1 };
                    let d = domains[coordinate];
                    let t = I::new(range[0], range[1])?
                        .sub(I::point(d[0]))?
                        .div(I::point(d[1]).sub(I::point(d[0]))?)?;
                    if t.lo <= 0. || t.hi >= 1. {
                        valid = false;
                        break;
                    }
                    uv[usize::from(i > 0)][coordinate] = [t.lo, t.hi];
                }
                if valid && (uv[0][axis][1] < uv[1][axis][0] || uv[1][axis][1] < uv[0][axis][0]) {
                    return Ok(Some(CertifiedCrossing {
                        proof: CrossingProof::CurveSurface,
                        parameter_boxes: uv,
                        position_bounds: root.position_bounds,
                        cells: search.cells,
                    }));
                }
            }
        }
    }
    Ok(None)
}
