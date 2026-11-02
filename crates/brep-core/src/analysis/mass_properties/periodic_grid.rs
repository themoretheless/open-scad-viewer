//! Numerical quadrature scheduling on the authored lifted UV chart. Original
//! knot translations are proposals only; this grants no certified volume bound.
use super::*;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
pub(super) fn lift(
    model: &Model,
    face: usize,
    mut grid: [Vec<f64>; 2],
    budget: &mut Budget,
) -> Result<[Vec<f64>; 2]> {
    let f = &model.faces[face];
    let s = &f.surface;
    for k in 0..2 {
        if ![s.periodic_u, s.periodic_v][k] {
            continue;
        }
        let mut range = [f64::INFINITY, f64::NEG_INFINITY];
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[w].coedges {
                for p in &c.pcurve.control_points {
                    range[0] = range[0].min(p[k]);
                    range[1] = range[1].max(p[k]);
                }
            }
        }
        let q = |v| Q::from_float(v).unwrap();
        let lo = grid[k][0];
        let hi = *grid[k].last().unwrap();
        let period = q(hi) - q(lo);
        let first = ((q(range[0]) - q(lo)) / &period)
            .floor()
            .to_i64()
            .ok_or_else(|| {
                Error::new("BREP_RESOURCE_LIMIT", "Periodic integration chart overflow")
            })?;
        let last = ((q(range[1]) - q(lo)) / &period)
            .floor()
            .to_i64()
            .ok_or_else(|| {
                Error::new("BREP_RESOURCE_LIMIT", "Periodic integration chart overflow")
            })?;
        if last.checked_sub(first).is_none_or(|n| n > 128) {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Periodic integration chart exceeds bounded lap schedule",
            ));
        }
        let count = (last - first + 1) as usize * grid[k].len();
        spend(budget, count)?;
        let mut lifted = vec![range[0], range[1]];
        for j in first..=last {
            for &v in &grid[k] {
                let x = (q(v) + Q::from_integer(j.into()) * &period)
                    .to_f64()
                    .ok_or_else(|| {
                        Error::new("BREP_RESOURCE_LIMIT", "Periodic grid scalar overflow")
                    })?;
                if x > range[0] && x < range[1] {
                    lifted.push(x);
                }
            }
        }
        lifted.sort_by(f64::total_cmp);
        lifted.dedup();
        grid[k] = lifted;
    }
    Ok(grid)
}
