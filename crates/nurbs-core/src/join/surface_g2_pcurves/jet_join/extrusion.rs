//! Correlated rational profile comparison after exact original axial 2-jet recognition.
use super::*;
use crate::surface_shape_operator::FlatPoleCertificate;
#[path = "extrusion/flat.rs"]
mod flat;
#[path = "extrusion/polynomial.rs"]
mod polynomial;
#[path = "extrusion/prepared.rs"]
mod prepared;
#[path = "extrusion/source.rs"]
mod source;
pub use flat::inspect_planar_boundary;
pub struct ExtrusionBoundaryReport {
    pub positions: Positions,
    pub decision: Decision,
    pub orientation: Orientation,
    pub normal: [f64; 2],
    pub tensor: [f64; 2],
    /// Every covered one-sided original cell is regular. Continuity between
    /// knots of either surface remains a separate internal G2 obligation.
    pub regularity_proven: bool,
    pub cells: usize,
    pub pending: usize,
    /// Query failures, including superseded subdivisions; not uncovered area.
    pub profile_unresolved_queries: usize,
    pub source_unresolved_queries: usize,
}
struct Cell {
    segment: usize,
    t: [f64; 2],
    bound: Option<[[f64; 2]; 2]>,
}
/// The actual original source is compared with its exact trace cylinder;
/// no nearly extruded source coefficient is snapped. The candidate first
/// three V rows must have the same perpendicular coordinates and weights.
/// Hence its boundary normal and world shape tensor equal those of its
/// projected profile extruded along the axis, despite axial shear or scale.
/// A collapsed endpoint requires a certificate borrowing this exact source.
/// This proves a local seam, not complete surface embedding or a sewn solid.
/// Multispan sources retain every intersected original cell and its limiting
/// jets. This does not grant internal continuity to a piecewise source or
/// candidate merely because their corresponding one-sided fields agree.
pub fn inspect_extrusion_boundary(
    curve: &Curve,
    pc: [&Curve; 2],
    surface: [&Surface; 2],
    axis: usize,
    poles: [Option<&FlatPoleCertificate<'_>>; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<ExtrusionBoundaryReport> {
    check(
        tolerances.iter().all(|x| x.is_finite() && *x >= 0.)
            && tolerances[1] <= 2.
            && max_cells <= 100000
            && max_exact <= 1000000,
        "Invalid original extrusion seam limits",
    )?;
    let prepared = prepared::Prepared::new(surface, pc, axis, poles,max_cells)?;
    let positions = verify_positions(
        curve,
        pc,
        surface,
        [false; 2],
        tolerances[0],
        max_cells / 4,
        max_exact,
    )?;
    let mut work = positions.cells;
    let mut lower = [0_f64; 2];
    let mut source_sign = None;
    let profile_unresolved = std::cell::Cell::new(0usize);
    let source_unresolved = std::cell::Cell::new(0usize);
    let query = |segment: usize,
                 t: [f64; 2],
                 source_sign: &mut Option<i8>|
     -> Result<Option<[[f64; 2]; 2]>> {
        let part = &prepared.segments[segment];
        let original = &prepared.sources[part.source];
        if original.outside(part.ends, t)? {
            return Ok(Some([[0.; 2]; 2]));
        }
        let Some(profile) = part.pair.bounds(t, axis)? else {
            profile_unresolved.set(profile_unresolved.get() + 1);
            return Ok(None);
        };
        let Some((source, sign)) = original.bounds(part.ends, t, axis)? else {
            source_unresolved.set(source_unresolved.get() + 1);
            return Ok(None);
        };
        check(
            source_sign.is_none_or(|s| s == sign),
            "Original seam orientation changes between source cells",
        )?;
        *source_sign = Some(sign);
        let mut bound = [[0.; 2]; 2];
        for k in 0..2 {
            bound[k][0] = (profile[k][0] - source[k][1])
                .max(source[k][0] - profile[k][1])
                .max(0.)
                .next_down()
                .max(0.);
            bound[k][1] = I::point(profile[k][1]).add(I::point(source[k][1]))?.hi;
        }
        Ok(Some(bound))
    };
    let mut cells = Vec::new();
    for (i, segment) in prepared.segments.iter().enumerate() {
        let bound = if max_cells - work >= 3 {
            work += 3;
            query(i, segment.range, &mut source_sign)?
        } else {
            None
        };
        cells.push(Cell {
            segment: i,
            t: segment.range,
            bound,
        });
    }
    loop {
        let upper = cells.iter().try_fold([0_f64; 2], |mut sum, c| {
            let b = c.bound?;
            for k in 0..2 {
                sum[k] = sum[k].max(b[k][1]);
            }
            Some(sum)
        });
        let good = |b: [[f64; 2]; 2]| b[0][1] <= tolerances[1] && b[1][1] <= tolerances[2];
        let done = source_sign.is_some()
            && positions.proven
            && upper.is_some_and(|x| x[0] <= tolerances[1] && x[1] <= tolerances[2]);
        let bad = lower[0] > tolerances[1] || lower[1] > tolerances[2];
        let finish = |decision, source_sign: Option<i8>| ExtrusionBoundaryReport {
            positions: positions.clone(),
            decision,
            orientation: if source_sign == Some(prepared.candidate_sign) {
                Orientation::Same
            } else {
                Orientation::Opposite
            },
            normal: [lower[0], upper.map_or(f64::INFINITY, |x| x[0])],
            tensor: [lower[1], upper.map_or(f64::INFINITY, |x| x[1])],
            regularity_proven: upper.is_some() && source_sign.is_some(),
            cells: work,
            pending: cells.iter().filter(|c| !c.bound.is_some_and(good)).count(),
            profile_unresolved_queries: profile_unresolved.get(),
            source_unresolved_queries: source_unresolved.get(),
        };
        if done {
            return Ok(finish(Decision::WithinTolerance, source_sign));
        }
        if bad {
            return Ok(finish(
                Decision::CorrespondenceExceedsTolerance,
                source_sign,
            ));
        }
        if max_cells - work < 3 || !positions.proven {
            return Ok(finish(Decision::Unresolved, source_sign));
        }
        let Some((at, _)) = cells
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.bound.is_some_and(good))
            .max_by(|(_, a), (_, b)| (a.t[1] - a.t[0]).total_cmp(&(b.t[1] - b.t[0])))
        else {
            return Ok(finish(Decision::Unresolved, source_sign));
        };
        let old = &cells[at];
        let t = old.t;
        let segment = old.segment;
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            return Ok(finish(Decision::Unresolved, source_sign));
        }
        let mut children = Vec::new();
        for t in [[t[0], mid], [mid, t[1]]] {
            if max_cells - work < 3 {
                break;
            }
            work += 3;
            let bound = query(segment, t, &mut source_sign)?;
            children.push(Cell { segment, t, bound });
        }
        if children.len() == 2 {
            cells.swap_remove(at);
            cells.extend(children);
        }
        let part = &prepared.segments[segment];
        if max_cells - work >= 3 && prepared.sources[part.source].contains_point(part.ends, mid)? {
            work += 3;
            if let Some(b) = query(segment, [mid, mid], &mut source_sign)? {
                for k in 0..2 {
                    lower[k] = lower[k].max(b[k][0]);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "extrusion/tests.rs"]
mod tests;
