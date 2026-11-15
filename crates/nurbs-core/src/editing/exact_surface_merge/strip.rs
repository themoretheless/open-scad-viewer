//! Exact coalescence of an ordered strip, with seam cleanup along its axis.
use super::*;
use crate::{
    check,
    core::bounded_rational::{Error as ArithmeticError, Work},
    editing::exact_surface_knot_removal,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StripRefusal {
    ResourceLimit,
    Join { patch: usize, reason: Refusal },
}

#[derive(Clone, Debug)]
pub struct StripMerge {
    /// Complete merged strip only. Late refusal never publishes a prefix.
    pub surface: Option<Surface>,
    pub removed_knots: usize,
    pub work_used: u64,
    pub refusal: Option<StripRefusal>,
}

/// Merge an ordered sequence of compatible tensor charts on one axis.
/// After each join, exact knot simplification removes redundant seams
/// without changing degree or the transverse basis. Genuine creases are
/// retained. Both structural copies and proofs share the work budget.
pub fn merge_surface_strip(patches: &[Surface], axis: Axis, max_work: u64) -> Result<StripMerge> {
    check(
        patches.len() >= 2,
        "An exact strip requires at least two patches",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact strip merge work must be in [1,1000000]",
    )?;
    let mut report = StripMerge {
        surface: None,
        removed_knots: 0,
        work_used: 0,
        refusal: None,
    };
    let mut merged = None;
    for (index, right) in patches.iter().enumerate().skip(1) {
        let left = merged.as_ref().unwrap_or(&patches[0]);
        left.validate()?;
        right.validate()?;
        let controls = |s: &Surface| (s.control_points.len() * s.control_points[0].len()) as u64;
        let charge = controls(left) + controls(right);
        if charge >= max_work - report.work_used {
            report.refusal = Some(StripRefusal::ResourceLimit);
            return Ok(report);
        }
        report.work_used += charge;
        let joined = merge(left, right, axis)?;
        let Some(mut joined) = joined.surface else {
            report.refusal = Some(StripRefusal::Join {
                patch: index,
                reason: joined.refusal.unwrap_or(Refusal::InvalidOutput),
            });
            return Ok(report);
        };
        let mut ledger = Work {
            used: report.work_used,
            limit: max_work,
        };
        let linear = remove_linear_join(&mut joined, axis, &mut ledger);
        report.work_used = ledger.used;
        match linear {
            Ok(true) => {
                report.removed_knots += 1;
                merged = Some(joined);
                continue;
            }
            Err(_) => {
                report.refusal = Some(StripRefusal::ResourceLimit);
                return Ok(report);
            }
            Ok(false) => {}
        }
        if report.work_used == max_work {
            report.refusal = Some(StripRefusal::ResourceLimit);
            return Ok(report);
        }
        let compact =
            exact_surface_knot_removal::simplify(&joined, axis, max_work - report.work_used)?;
        report.work_used += compact.work_used;
        if !compact.completed_pass {
            report.refusal = Some(StripRefusal::ResourceLimit);
            return Ok(report);
        }
        report.removed_knots += compact.removed;
        merged = Some(compact.surface);
    }
    report.surface = merged;
    Ok(report)
}

// A structurally merged degree-one pair has three controls along the
// selected axis. The seam is removable exactly when all four homogeneous
// components have equal slopes on both sides, for every transverse row.
fn remove_linear_join(
    joined: &mut Surface,
    axis: Axis,
    work: &mut Work,
) -> std::result::Result<bool, ArithmeticError> {
    let (degree, knots, count, _) = basis(joined, axis);
    if degree != 1 || count != 3 {
        return Ok(false);
    }
    let [a, seam, b] = [
        work.lift(knots[1])?,
        work.lift(knots[2])?,
        work.lift(knots[3])?,
    ];
    let left = work.op(&seam, &a, |a, b| a - b)?;
    let right = work.op(&b, &seam, |a, b| a - b)?;
    let rows = basis(joined, opposite(axis)).2;
    for row in 0..rows {
        let index = |i| match axis {
            Axis::U => (i, row),
            Axis::V => (row, i),
        };
        let (i0, j0) = index(0);
        let (i1, j1) = index(1);
        let (i2, j2) = index(2);
        let weights = [
            work.lift(joined.weights[i0][j0])?,
            work.lift(joined.weights[i1][j1])?,
            work.lift(joined.weights[i2][j2])?,
        ];
        for component in 0..4 {
            let homogeneous = if component == 3 {
                weights.clone()
            } else {
                let mut values = weights.clone();
                for (i, value) in values.iter_mut().enumerate() {
                    let (u, v) = index(i);
                    let point = work.lift(joined.control_points[u][v][component])?;
                    *value = work.op(value, &point, |a, b| a * b)?;
                }
                values
            };
            let dl = work.op(&homogeneous[1], &homogeneous[0], |a, b| a - b)?;
            let dr = work.op(&homogeneous[2], &homogeneous[1], |a, b| a - b)?;
            if work.op(&dl, &right, |a, b| a * b)? != work.op(&dr, &left, |a, b| a * b)? {
                return Ok(false);
            }
        }
    }
    match axis {
        Axis::U => {
            joined.knots_u.remove(2);
            joined.control_points.remove(1);
            joined.weights.remove(1);
        }
        Axis::V => {
            joined.knots_v.remove(2);
            for (points, weights) in joined.control_points.iter_mut().zip(&mut joined.weights) {
                points.remove(1);
                weights.remove(1);
            }
        }
    }
    Ok(true)
}
