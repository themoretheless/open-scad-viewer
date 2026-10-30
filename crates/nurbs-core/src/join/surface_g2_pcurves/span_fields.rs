//! Continuous coverage of original one-sided shape fields, not joined G2 admission.
use super::*;
#[path = "span_fields/field.rs"]
mod field;
#[path = "span_fields/mean_value.rs"]
mod mean_value;
/// Every original one-sided field is enclosed. No C2 chart join or pole
/// quotient follows; callers must prove those independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldDecision {
    Enclosed,
    ExceedsTolerance,
    Unresolved,
}
pub struct FieldCoverage {
    pub positions: Positions,
    pub decision: FieldDecision,
    pub branches: [BranchBounds; 2],
    pub orientation: Option<Orientation>,
    pub one_sided_regular: bool,
    pub cells: usize,
    /// Cells still missing a bound in any viable orientation. Zero alone
    /// does not establish one coherent whole-curve orientation.
    pub pending: usize,
}
struct Cell {
    t: [f64; 2],
    upper: Option<[[f64; 2]; 2]>,
}
fn cell(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    tolerances: [f64; 2],
    witnesses: &mut [[f64; 2]; 2],
    work: &mut usize,
    max: usize,
) -> Result<Cell> {
    if t[1] - t[0] <= 1. / 16. && *work < max {
        if let Some((upper, lower)) = mean_value::bounds(p, s, reverse, t, work, max)? {
            for k in 0..2 {
                for j in 0..2 {
                    witnesses[k][j] = witnesses[k][j].max(lower[k][j]);
                }
            }
            return Ok(Cell {
                t,
                upper: Some(upper),
            });
        }
    }
    let upper = field::pair(p, s, reverse, t, work, max)?.map(|x| x.0);
    if *work < max
        && !upper.is_some_and(|u| {
            u.iter()
                .any(|b| b[0] <= tolerances[0] && b[1] <= tolerances[1])
        })
    {
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if let Some((_, lower)) = field::pair(p, s, reverse, [mid, mid], work, max)? {
            for k in 0..2 {
                for j in 0..2 {
                    witnesses[k][j] = witnesses[k][j].max(lower[k][j]);
                }
            }
        }
    }
    Ok(Cell { t, upper })
}
/// Bound the complete original pcurve images, including every intersected
/// original span's one-sided shape. Positive source weights justify hull
/// clipping only; no cropped surface or sampled geometry is substituted.
/// `Enclosed` means field coverage within tolerances and fresh positions.
/// It deliberately does NOT certify chart joins, pole regularity or a solid.
pub fn inspect_span_fields(
    c: &Curve,
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    position: f64,
    normal: f64,
    curvature: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FieldCoverage> {
    check(
        normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && curvature.is_finite()
            && curvature >= 0.,
        "Invalid span field shape tolerances",
    )?;
    let positions = verify_positions(c, p, s, reverse, position, max_cells, max_exact)?;
    let mut work = positions.cells;
    let mut witnesses = [[0_f64; 2]; 2];
    let mut cells = vec![Cell {
        t: [0., 1.],
        upper: None,
    }];
    if positions.proven
        && work < max_cells
        && !s.iter().any(|s| s.periodic_u || s.periodic_v)
        && !p.iter().any(|p| p.periodic)
    {
        cells[0] = cell(
            p,
            s,
            reverse,
            [0., 1.],
            [normal, curvature],
            &mut witnesses,
            &mut work,
            max_cells,
        )?;
    }
    loop {
        let upper = cells.iter().try_fold([[0_f64; 2]; 2], |mut u, c| {
            let b = c.upper?;
            for k in 0..2 {
                for j in 0..2 {
                    u[k][j] = u[k][j].max(b[k][j]);
                }
            }
            Some(u)
        });
        let orientation = upper
            .and_then(|u| (0..2).find(|&k| u[k][0] <= normal && u[k][1] <= curvature))
            .map(|k| {
                if k == 0 {
                    Orientation::Same
                } else {
                    Orientation::Opposite
                }
            });
        let failed = witnesses.map(|b| b[0] > normal || b[1] > curvature);
        let decision = if failed[0] && failed[1] {
            Some(FieldDecision::ExceedsTolerance)
        } else if positions.proven && orientation.is_some() {
            Some(FieldDecision::Enclosed)
        } else if work == max_cells
            || !positions.proven
            || s.iter().any(|s| s.periodic_u || s.periodic_v)
            || p.iter().any(|p| p.periodic)
        {
            Some(FieldDecision::Unresolved)
        } else {
            None
        };
        let result = |decision| FieldCoverage {
            positions: positions.clone(),
            decision,
            branches: std::array::from_fn(|k| BranchBounds {
                normal_lower: witnesses[k][0],
                tensor_lower: witnesses[k][1],
                normal_upper: upper.map(|u| u[k][0]),
                tensor_upper: upper.map(|u| u[k][1]),
            }),
            orientation: if decision == FieldDecision::Enclosed {
                orientation
            } else {
                None
            },
            one_sided_regular: upper.is_some(),
            cells: work,
            pending: cells
                .iter()
                .filter(|c| {
                    c.upper.is_none_or(|u| {
                        !(0..2).any(|k| !failed[k] && u[k][0] <= normal && u[k][1] <= curvature)
                    })
                })
                .count(),
        };
        if let Some(decision) = decision {
            return Ok(result(decision));
        }
        let index = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.upper.is_none_or(|u| {
                    !(0..2).any(|k| !failed[k] && u[k][0] <= normal && u[k][1] <= curvature)
                })
            })
            .max_by(|(_, a), (_, b)| (a.t[1] - a.t[0]).total_cmp(&(b.t[1] - b.t[0])))
            .map(|(i, _)| i);
        // Cells may each admit a different orientation. Then refine the
        // longest cell until one coherent whole-trace sign is bounded.
        let index = index.unwrap_or_else(|| {
            cells
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| (a.t[1] - a.t[0]).total_cmp(&(b.t[1] - b.t[0])))
                .unwrap()
                .0
        });
        let old = &cells[index];
        let mid = old.t[0] * 0.5 + old.t[1] * 0.5;
        if !(old.t[0] < mid && mid < old.t[1]) {
            return Ok(result(FieldDecision::Unresolved));
        }
        let range = old.t;
        let left = cell(
            p,
            s,
            reverse,
            [range[0], mid],
            [normal, curvature],
            &mut witnesses,
            &mut work,
            max_cells,
        )?;
        if work == max_cells && left.upper.is_none() {
            continue;
        }
        let right = cell(
            p,
            s,
            reverse,
            [mid, range[1]],
            [normal, curvature],
            &mut witnesses,
            &mut work,
            max_cells,
        )?;
        if work == max_cells && right.upper.is_none() {
            continue;
        }
        cells.swap_remove(index);
        cells.push(left);
        cells.push(right);
    }
}
#[cfg(test)]
#[path = "span_fields/tests.rs"]
mod tests;
