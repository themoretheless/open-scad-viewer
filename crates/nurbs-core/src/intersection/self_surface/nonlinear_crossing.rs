//! Bounded original-source planar contact search on exact rational iso-curves.
use super::*;
use crate::{curve::Curve, distance_bounds::Interval as I};
fn exact_f64(q: &BigRational) -> Option<f64> {
    let x = q.to_f64()?;
    (BigRational::from_f64(x).as_ref() == Some(q)).then_some(x)
}
fn original_iso(s: &Surface, axis: usize, homogeneous: &[[BigRational; 4]]) -> Option<Curve> {
    let mut controls = Vec::new();
    let mut weights = Vec::new();
    for h in homogeneous {
        let w = exact_f64(&h[3])?;
        let mut p = Vec::new();
        for c in &h[..3] {
            p.push(exact_f64(&(c / &h[3]))?);
        }
        controls.push(p);
        weights.push(w);
    }
    let (degree, knots) = if axis == 0 {
        (s.degree_u, s.knots_u.clone())
    } else {
        (s.degree_v, s.knots_v.clone())
    };
    Some(Curve {
        degree,
        knots,
        control_points: controls,
        weights,
        periodic: false,
    })
}
// Dyadic probes only steer existence proofs; their exhaustion proves nothing
// about unqueried fibers. All admissions and solver cells share one budget.
const PARAMETERS: [f64; 15] = [
    0.5, 0.25, 0.75, 0.125, 0.375, 0.625, 0.875, 0.0625, 0.1875, 0.3125, 0.4375, 0.5625, 0.6875,
    0.8125, 0.9375,
];
pub(super) fn find(
    s: &Surface,
    max_cells: usize,
) -> Result<(Option<CertifiedCrossing>, ProbeSearch)> {
    let mut search = ProbeSearch::default();
    for parameter in PARAMETERS {
        for axis in 0..2 {
            let degree = if axis == 0 { s.degree_u } else { s.degree_v };
            if !(1..=16).contains(&degree) {
                continue;
            }
            if search.cells == max_cells {
                search.exhausted = true;
                return Ok((None, search));
            }
            search.cells += 1;
            let mut attempt = ProbeAttempt {
                axis,
                parameter,
                cells: 1,
                iso_coverage_complete: false,
                refusal: None,
            };
            if search.cells == max_cells {
                attempt.refusal = Some("probe budget".into());
                search.attempts.push(attempt);
                search.exhausted = true;
                return Ok((None, search));
            }
            let Some(homogeneous) = super::linear_crossing::iso_at(s, axis, parameter) else {
                attempt.refusal = Some("unsupported transverse basis".into());
                search.attempts.push(attempt);
                continue;
            };
            let domain = if axis == 0 {
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]]
            } else {
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
            };
            let [start, end] = domain;
            let width = I::point(end).sub(I::point(start))?;
            let result = if let Some(c) = original_iso(s, axis, &homogeneous) {
                crate::intersection::intersect_self_curve_certified_report(
                    &c,
                    1e-7,
                    max_cells - search.cells,
                )
            } else {
                crate::intersection::intersect_surface_iso_at_certified_report(
                    s,
                    axis,
                    parameter,
                    1e-7,
                    max_cells - search.cells,
                )
            };
            let report = match result {
                Ok(r) => r,
                Err(e) => {
                    attempt.cells += max_cells - search.cells;
                    attempt.refusal = Some(
                        format!("{}: {}", e.code, e.message)
                            .chars()
                            .take(1024)
                            .collect(),
                    );
                    search.cells = max_cells;
                    search.exhausted = true;
                    search.attempts.push(attempt);
                    return Ok((None, search));
                }
            };
            search.cells += report.cells;
            attempt.cells += report.cells;
            attempt.iso_coverage_complete = report.complete;
            search.attempts.push(attempt);
            for crossing in report.crossings {
                let mut boxes = [[[parameter, parameter]; 2]; 2];
                let mut interior = true;
                for (j, range) in crossing.parameters.into_iter().enumerate() {
                    let t = I::new(range[0], range[1])?
                        .sub(I::point(start))?
                        .div(width)?;
                    if t.lo <= 0. || t.hi >= 1. {
                        interior = false;
                        break;
                    }
                    boxes[j][axis] = [t.lo, t.hi];
                }
                if interior
                    && (boxes[0][axis][1] < boxes[1][axis][0]
                        || boxes[1][axis][1] < boxes[0][axis][0])
                {
                    let crossing = CertifiedCrossing {
                        parameter_boxes: boxes,
                        position_bounds: crossing.position_bounds,
                        cells: search.cells,
                    };
                    return Ok((Some(crossing), search));
                }
            }
        }
    }
    Ok((None, search))
}
