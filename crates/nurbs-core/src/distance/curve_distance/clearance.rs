//! Threshold-driven clearance over the complete original curve product domain.
use super::{Curve, Result, check, enclosure, resource, spans};
use crate::distance_bounds::{Interval as I, box_distance};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearanceStatus {
    AtLeast,
    Below,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Clearance {
    pub status: ClearanceStatus,
    pub distance_bounds: [f64; 2],
    pub cells: usize,
    /// The full original product remains an obligation if work/precision ends.
    pub unresolved_domain: Option<[[f64; 2]; 2]>,
}
struct Cell {
    spans: [usize; 2],
    domain: [[f64; 2]; 2],
    boxes: [Vec<I>; 2],
    bounds: [f64; 2],
}
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cell {
    fn cmp(&self, other: &Self) -> Ordering {
        other.bounds[0]
            .total_cmp(&self.bounds[0])
            .then_with(|| other.bounds[1].total_cmp(&self.bounds[1]))
    }
}
fn cell(a: &Curve, b: &Curve, spans: [usize; 2], domain: [[f64; 2]; 2]) -> Result<Cell> {
    let boxes = [
        enclosure(a, spans[0], I::new(domain[0][0], domain[0][1])?)?,
        enclosure(b, spans[1], I::new(domain[1][0], domain[1][1])?)?,
    ];
    let bounds = box_distance(&boxes[0], &boxes[1])?;
    Ok(Cell {
        spans,
        domain,
        boxes,
        bounds: [bounds.0, bounds.1],
    })
}
/// Prove minimum distance >= required, prove a pair lies closer, or retain a
/// full-domain unresolved obligation. Original outward rational enclosures
/// cover every knot-span pair. A cell is removed only by a sufficient hull
/// lower bound. A hull upper bound below required proves existence of a closer
/// pair; no seed convergence is involved. This does not compute a finely
/// converged global minimum or claim any intersection topology. Cells count
/// all created parent/child pairs; at most 100000 are admitted per query.
pub fn prove_clearance(a: &Curve, b: &Curve, required: f64, max_cells: usize) -> Result<Clearance> {
    a.validate()?;
    b.validate()?;
    check(
        a.control_points[0].len() == b.control_points[0].len()
            && required.is_finite()
            && required >= 0.
            && (1..=100000).contains(&max_cells),
        "Clearance needs equal curve dimensions, finite nonnegative threshold and 1..100000 cells",
    )?;
    let sa = spans(a);
    let sb = spans(b);
    if sa.len().saturating_mul(sb.len()) > max_cells {
        return Err(resource(
            "Initial clearance span cover exceeds the work budget",
        ));
    }
    let mut pending = BinaryHeap::new();
    let mut upper = f64::INFINITY;
    for &i in &sa {
        for &j in &sb {
            let next = cell(
                a,
                b,
                [i, j],
                [[a.knots[i], a.knots[i + 1]], [b.knots[j], b.knots[j + 1]]],
            )?;
            upper = upper.min(next.bounds[1]);
            pending.push(next);
        }
    }
    let mut cells = pending.len();
    let mut accepted = f64::INFINITY;
    while let Some(c) = pending.pop() {
        if c.bounds[0] >= required {
            accepted = accepted.min(c.bounds[0]);
            continue;
        }
        let report = |status, upper| Clearance {
            status,
            distance_bounds: [
                pending
                    .iter()
                    .map(|x| x.bounds[0])
                    .fold(accepted.min(c.bounds[0]), f64::min),
                upper,
            ],
            cells,
            unresolved_domain: if status == ClearanceStatus::Unresolved {
                Some([a.domain(), b.domain()])
            } else {
                None
            },
        };
        // One enclosed original point pair is sufficient to prove a violation.
        // Without it, DFS can spend its budget on threshold-equality cells
        // while a closer pair exists elsewhere in the product cover.
        let point = |curve: &Curve, axis: usize| {
            let d = c.domain[axis];
            let t = (d[0] * 0.5 + d[1] * 0.5).clamp(d[0], d[1]);
            enclosure(curve, c.spans[axis], I::point(t))
        };
        let witness = box_distance(&point(a, 0)?, &point(b, 1)?)?.1;
        upper = upper.min(witness);
        if upper < required {
            return Ok(report(ClearanceStatus::Below, upper));
        }
        if cells + 2 > max_cells {
            return Ok(report(ClearanceStatus::Unresolved, upper));
        }
        let width = |axis: usize| c.boxes[axis].iter().map(|x| x.width()).fold(0., f64::max);
        let preferred = usize::from(width(1) > width(0));
        let split = [preferred, 1 - preferred].into_iter().find_map(|axis| {
            let d = c.domain[axis];
            let mid = d[0] * 0.5 + d[1] * 0.5;
            (d[0] < mid && mid < d[1]).then_some((axis, mid))
        });
        let Some((axis, mid)) = split else {
            return Ok(report(ClearanceStatus::Unresolved, upper));
        };
        for range in [[c.domain[axis][0], mid], [mid, c.domain[axis][1]]] {
            let mut domain = c.domain;
            domain[axis] = range;
            let next = cell(a, b, c.spans, domain)?;
            upper = upper.min(next.bounds[1]);
            pending.push(next);
            cells += 1;
        }
    }
    Ok(Clearance {
        status: ClearanceStatus::AtLeast,
        distance_bounds: [accepted, upper],
        cells,
        unresolved_domain: None,
    })
}
