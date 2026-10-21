//! Complete original-span scalar roots into original-parameter coverage cells.
use super::{Cell, Classification, ExactEndpoints, Proof, StopReason};
use crate::{Result, curve::Curve, distance_bounds::Interval as I};
use cad_predicates::OriginalPlaneRootsOutcome;
fn classify(order: usize, start: bool, end: bool) -> Classification {
    match (order, start, end) {
        (1, true, _) => Classification::RootAtStart,
        (1, _, true) => Classification::RootAtEnd,
        (2, true, _) => Classification::TangencyAtStart,
        (2, _, true) => Classification::TangencyAtEnd,
        (_, true, _) => Classification::HigherOrderContactAtStart,
        (_, _, true) => Classification::HigherOrderContactAtEnd,
        (1, _, _) => Classification::UniqueRoot,
        (2, _, _) => Classification::TangencyInInterval,
        _ => Classification::HigherOrderContactInInterval,
    }
}
pub(super) fn cover(
    c: &Curve,
    span: usize,
    residual: [f64; 2],
    tolerance: f64,
    available: usize,
    exact: &mut ExactEndpoints<'_>,
) -> Option<Vec<Cell>> {
    let domain = [c.knots[span], c.knots[span + 1]];
    let width = I::point(domain[1]).sub(I::point(domain[0])).ok()?;
    let ratio = I::point(tolerance).div(width).ok()?.lo;
    let mut target = (ratio * 0.125).min(0.25);
    while target.is_finite() && target > 0. {
        let outcome = exact.roots(span, target)?;
        let roots = match outcome {
            OriginalPlaneRootsOutcome::Coincident => {
                return Some(vec![Cell {
                    domain,
                    span,
                    residual_bounds: [0., 0.],
                    classification: Classification::Coincident,
                    stop_reason: StopReason::ExactCoincidence,
                    root_multiplicity: None,
                    isolated_root_bounds: None,
                    proof: Some(Proof::OriginalSturmRootIsolation),
                }]);
            }
            OriginalPlaneRootsOutcome::Roots(roots) => roots,
            OriginalPlaneRootsOutcome::Indeterminate(_) => return None,
        };
        if roots.is_empty() {
            return Some(vec![Cell {
                domain,
                span,
                residual_bounds: residual,
                classification: Classification::Excluded,
                stop_reason: StopReason::ExactSturmIsolation,
                root_multiplicity: None,
                isolated_root_bounds: None,
                proof: Some(Proof::OriginalSturmRootIsolation),
            }]);
        }
        if roots.len() > 1 && roots.len() > available {
            return None;
        }
        let mapped = (|| -> Result<Vec<[f64; 2]>> {
            roots
                .iter()
                .map(|root| {
                    let [a, b] = root.interval;
                    if a == 0. && b == 0. {
                        return Ok([domain[0]; 2]);
                    }
                    if a == 1. && b == 1. {
                        return Ok([domain[1]; 2]);
                    }
                    if a == b {
                        let point = crate::exact_curve_segments::exact_add(domain[1], -domain[0])
                            .and_then(|w| crate::exact_curve_segments::exact_mul(w, a))
                            .and_then(|v| crate::exact_curve_segments::exact_add(domain[0], v));
                        if let Some(point) = point {
                            return Ok([point; 2]);
                        }
                    }
                    let t = I::point(domain[0])
                        .add(width.mul(I::new(a, b)?)?)?
                        .intersect(domain[0], domain[1])?;
                    Ok([t.lo, t.hi])
                })
                .collect()
        })()
        .ok()?;
        let narrow = mapped
            .iter()
            .all(|b| b[0] == b[1] || (b[1] - b[0]).next_up() <= tolerance);
        let separated = mapped.windows(2).all(|b| b[0][1] < b[1][0]);
        let mut separators = Vec::new();
        if narrow && separated {
            for b in mapped.windows(2) {
                let mid = b[0][1] * 0.5 + b[1][0] * 0.5;
                if !(b[0][1] < mid && mid < b[1][0]) {
                    break;
                }
                separators.push(mid);
            }
            if separators.len() + 1 == roots.len() {
                let cells = roots
                    .iter()
                    .enumerate()
                    .map(|(i, root)| Cell {
                        domain: [
                            if i == 0 { domain[0] } else { separators[i - 1] },
                            if i + 1 == roots.len() {
                                domain[1]
                            } else {
                                separators[i]
                            },
                        ],
                        span,
                        residual_bounds: residual,
                        classification: classify(
                            root.multiplicity,
                            root.interval == [0., 0.],
                            root.interval == [1., 1.],
                        ),
                        stop_reason: StopReason::ExactSturmIsolation,
                        root_multiplicity: Some(root.multiplicity),
                        isolated_root_bounds: Some(mapped[i]),
                        proof: Some(Proof::OriginalSturmRootIsolation),
                    })
                    .collect();
                return Some(cells);
            }
        }
        let smaller = target * 0.5;
        if smaller == target {
            return None;
        }
        target = smaller;
    }
    None
}
