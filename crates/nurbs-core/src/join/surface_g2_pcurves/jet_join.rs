//! Correlated source-jet jumps across a literal original C0 shared column.
use super::*;
use crate::surface_measure::jets;
#[path = "jet_join/paired.rs"]
mod paired;
#[path = "jet_join/polynomial.rs"]
mod polynomial;
pub struct SpanJoinReport {
    pub decision: Decision,
    pub normal: [f64; 2],
    pub tensor: [f64; 2],
    pub regularity_proven: bool,
    pub cells: usize,
    pub pending: usize,
}
fn query(
    s: &Surface,
    p: &polynomial::Prepared,
    knot: f64,
    v: [f64; 2],
) -> Result<Option<[[f64; 2]; 2]>> {
    let mut a = jets::calculate_partial_stable_up_to(
        s,
        [p.left, s.degree_v],
        [[knot, knot], v],
        [None; 2],
        2,
    )?;
    let mut b = jets::calculate_partial_stable_up_to(
        s,
        [p.right, s.degree_v],
        [[knot, knot], v],
        [None; 2],
        2,
    )?;
    let dv = I::point(p.v[1]).sub(I::point(p.v[0]))?;
    for (j, span) in [(&mut a, p.left), (&mut b, p.right)] {
        let du = I::point(s.knots_u[span + 1]).sub(I::point(s.knots_u[span]))?;
        for order in 1..=2 {
            for u in 0..=order {
                let v = order - u;
                let mut scale = I::point(1.);
                for _ in 0..u {
                    scale = scale.mul(du)?;
                }
                for _ in 0..v {
                    scale = scale.mul(dv)?;
                }
                for k in 0..3 {
                    j[u][v][k] = j[u][v][k].div(scale)?;
                }
            }
        }
    }
    paired::bounds(&a, &b, p.jumps(v)?)
}
struct Cell {
    v: [f64; 2],
    bound: Option<[[f64; 2]; 2]>,
}
/// The exact common original control column grants G0. Both original sides
/// must be regular, and correlated complete normal/tensor jumps within the
/// requested tolerances. This is a local join proof, not embedding or volume.
pub fn inspect_original_span_join(
    s: &Surface,
    knot_u: f64,
    normal: f64,
    curvature: f64,
    max_cells: usize,
) -> Result<SpanJoinReport> {
    s.validate()?;
    check(
        knot_u.is_finite()
            && normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && curvature.is_finite()
            && curvature >= 0.
            && max_cells <= 100000,
        "Invalid original join limits",
    )?;
    let prepared = polynomial::Prepared::new(s, knot_u)?;
    let mut work = 0;
    let mut lower = [0_f64; 2];
    let mut cell = |v: [f64; 2]| -> Result<Cell> {
        if max_cells - work < 2 {
            return Ok(Cell { v, bound: None });
        }
        work += 2;
        let bound = query(s, &prepared, knot_u, v)?;
        if work + 2 <= max_cells
            && !bound.is_some_and(|b| b[0][1] <= normal && b[1][1] <= curvature)
        {
            let m = v[0] * 0.5 + v[1] * 0.5;
            work += 2;
            if let Some(b) = query(s, &prepared, knot_u, [m, m])? {
                for k in 0..2 {
                    lower[k] = lower[k].max(b[k][0]);
                }
            }
        }
        Ok(Cell { v, bound })
    };
    let first = cell(prepared.v)?;
    drop(cell);
    let mut cells = vec![first];
    loop {
        let upper = cells.iter().try_fold([0_f64; 2], |mut a, c| {
            let b = c.bound?;
            for k in 0..2 {
                a[k] = a[k].max(b[k][1]);
            }
            Some(a)
        });
        let done = upper.is_some_and(|u| u[0] <= normal && u[1] <= curvature);
        let bad = lower[0] > normal || lower[1] > curvature;
        let finish = |decision| SpanJoinReport {
            decision,
            normal: [lower[0], upper.map_or(f64::INFINITY, |u| u[0])],
            tensor: [lower[1], upper.map_or(f64::INFINITY, |u| u[1])],
            regularity_proven: upper.is_some(),
            cells: work,
            pending: cells
                .iter()
                .filter(|c| {
                    !c.bound
                        .is_some_and(|b| b[0][1] <= normal && b[1][1] <= curvature)
                })
                .count(),
        };
        if bad {
            return Ok(finish(Decision::CorrespondenceExceedsTolerance));
        }
        if done {
            return Ok(finish(Decision::WithinTolerance));
        }
        if max_cells - work < 2 {
            return Ok(finish(Decision::Unresolved));
        }
        let at = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                !c.bound
                    .is_some_and(|b| b[0][1] <= normal && b[1][1] <= curvature)
            })
            .max_by(|(_, a), (_, b)| (a.v[1] - a.v[0]).total_cmp(&(b.v[1] - b.v[0])))
            .unwrap()
            .0;
        let range = cells[at].v;
        let mid = range[0] * 0.5 + range[1] * 0.5;
        if !(range[0] < mid && mid < range[1]) {
            return Ok(finish(Decision::Unresolved));
        }
        let mut children = Vec::new();
        for v in [[range[0], mid], [mid, range[1]]] {
            if max_cells - work < 2 {
                break;
            }
            work += 2;
            let bound = query(s, &prepared, knot_u, v)?;
            if work + 2 <= max_cells
                && !bound.is_some_and(|b| b[0][1] <= normal && b[1][1] <= curvature)
            {
                let m = v[0] * 0.5 + v[1] * 0.5;
                work += 2;
                if let Some(b) = query(s, &prepared, knot_u, [m, m])? {
                    for k in 0..2 {
                        lower[k] = lower[k].max(b[k][0]);
                    }
                }
            }
            children.push(Cell { v, bound });
        }
        if children.len() == 2
            && !(max_cells - work < 2 && children.iter().any(|c| c.bound.is_none()))
        {
            cells.swap_remove(at);
            cells.extend(children);
        }
    }
}
#[cfg(test)]
#[path = "jet_join/tests.rs"]
mod tests;

#[path="jet_join/extrusion.rs"] mod extrusion;
pub use extrusion::{ExtrusionBoundaryReport,inspect_extrusion_boundary,inspect_planar_boundary};
