//! Parametrized surface area with outward-rounded bounds of the original net.
//! Integrates |S_u cross S_v| over every active knot rectangle, with multiplicity.
use crate::{
    Result, check,
    distance_bounds::Interval,
    sweep_support::interval_vec3::{cross, norm},
    numeric, resource,
    surface::Surface,
};
pub(crate) mod jets;
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Tolerance,
    WorkLimit,
    PrecisionLimit,
}
#[derive(Clone, Debug)]
pub struct AreaReport {
    pub value: f64,
    pub bounds: [f64; 2],
    pub error_upper: f64,
    pub within_tolerance: bool,
    /// Includes discarded subdivision trials and replaced parent cells.
    pub cells: usize,
    pub stop_reason: StopReason,
}
struct Cell {
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    area: Interval,
    serial: usize,
}
impl Cell {
    fn gap(&self) -> f64 {
        self.area.hi - self.area.lo
    }
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.gap() == b.gap() && self.serial == b.serial
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Cell {
    fn cmp(&self, b: &Self) -> Ordering {
        self.gap()
            .total_cmp(&b.gap())
            .then(self.serial.cmp(&b.serial))
    }
}
fn add(a: [Interval; 3], b: [Interval; 3], scale: f64) -> Result<[Interval; 3]> {
    let mut out = a;
    for k in 0..3 {
        out[k] = a[k].add(b[k].mul(Interval::point(scale))?)?;
    }
    Ok(out)
}
fn cell(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2], serial: usize) -> Result<Cell> {
    let j = jets::calculate(s, span, domain, None)?;
    let normal = cross(j[1][0], j[0][1])?;
    let jacobian = norm(normal)?;
    let mut lo = jacobian.lo;
    let mut hi = jacobian.hi;
    if jacobian.lo > 0. {
        let nu = add(cross(j[2][0], j[0][1])?, cross(j[1][0], j[1][1])?, 1.)?;
        let nv = add(cross(j[1][1], j[0][1])?, cross(j[1][0], j[0][2])?, 1.)?;
        let nuu = add(
            add(cross(j[3][0], j[0][1])?, cross(j[2][0], j[1][1])?, 2.)?,
            cross(j[1][0], j[2][1])?,
            1.,
        )?;
        let nvv = add(
            add(cross(j[1][2], j[0][1])?, cross(j[1][1], j[0][2])?, 2.)?,
            cross(j[1][0], j[0][3])?,
            1.,
        )?;
        let remainder = |first, second| -> Result<Interval> {
            let first = norm(first)?;
            norm(second)?
                .add(first.mul(first)?.div(Interval::point(jacobian.lo))?)?
                .div(Interval::point(12.))
        };
        let error = remainder(nu, nuu)?.add(remainder(nv, nvv)?)?.hi;
        let mut corner_sum = Interval::point(0.);
        for u in 0..2 {
            for v in 0..2 {
                let c = jets::calculate(s, span, domain, Some([u, v]))?;
                corner_sum = corner_sum.add(norm(cross(c[1][0], c[0][1])?)?)?;
            }
        }
        let trapezoid = corner_sum.mul(Interval::point(0.25))?;
        lo = lo.max((trapezoid.lo - error).next_down().max(0.));
        hi = hi.min((trapezoid.hi + error).next_up());
    }
    Ok(Cell {
        span,
        domain,
        area: Interval::new(lo, hi)?,
        serial,
    })
}

fn sum(heap: &BinaryHeap<Cell>) -> Result<Interval> {
    let mut total = Interval::point(0.);
    for c in heap {
        total = total.add(c.area)?;
    }
    Interval::new(total.lo.max(0.), total.hi)
}
fn report(total: Interval, cells: usize, tolerance: f64, reason: StopReason) -> Result<AreaReport> {
    let value = total.lo + (total.hi - total.lo) / 2.;
    let error_upper = (value - total.lo).max(total.hi - value).next_up();
    numeric(
        value.is_finite() && error_upper.is_finite(),
        "Surface area exhausted numeric range",
    )?;
    let within_tolerance = error_upper <= tolerance;
    Ok(AreaReport {
        value,
        bounds: [total.lo, total.hi],
        error_upper,
        within_tolerance,
        cells,
        stop_reason: if within_tolerance {
            StopReason::Tolerance
        } else {
            reason
        },
    })
}
/// Absolute tolerance is in squared model units. A budget stop returns valid
/// area bounds without asserting the requested accuracy. No trim-loop removal
/// or union-of-images area is implied; folds/overlaps count with multiplicity.
pub fn area(s: &Surface, tolerance: f64, max_cells: usize) -> Result<AreaReport> {
    s.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Area tolerance must be finite and positive",
    )?;
    check(
        (1..=100_000).contains(&max_cells),
        "Area work budget must be 1..100000 cell enclosures",
    )?;
    let mut heap = BinaryHeap::new();
    let mut cells = 0;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_u[u] < s.knots_u[u + 1] && s.knots_v[v] < s.knots_v[v + 1] {
                if cells == max_cells {
                    return Err(resource(
                        "Initial area knot rectangles exceed the work budget",
                    ));
                }
                heap.push(cell(
                    s,
                    [u, v],
                    [
                        [s.knots_u[u], s.knots_u[u + 1]],
                        [s.knots_v[v], s.knots_v[v + 1]],
                    ],
                    cells,
                )?);
                cells += 1;
            }
        }
    }
    let mut total = sum(&heap)?;
    loop {
        let r = report(total, cells, tolerance, StopReason::WorkLimit)?;
        if r.within_tolerance || cells + 2 > max_cells {
            total = sum(&heap)?;
            let recomputed = report(total, cells, tolerance, StopReason::WorkLimit)?;
            if recomputed.within_tolerance || cells + 2 > max_cells {
                return Ok(recomputed);
            }
        }
        let parent = heap.pop().unwrap();
        let mut best: Option<(Cell, Cell, Interval)> = None;
        // Try both axes, charging every constructed cell to the global budget.
        // Choose the lower uncertainty rather than splitting a constant axis.
        for axis in 0..2 {
            let [a, b] = parent.domain[axis];
            let mid = a + (b - a) / 2.;
            if !(a < mid && mid < b) || cells + 2 > max_cells {
                continue;
            }
            let mut left = parent.domain;
            let mut right = left;
            left[axis][1] = mid;
            right[axis][0] = mid;
            let left = cell(s, parent.span, left, cells)?;
            let right = cell(s, parent.span, right, cells + 1)?;
            cells += 2;
            let combined = left.area.add(right.area)?;
            if best
                .as_ref()
                .is_none_or(|(_, _, old)| combined.hi - combined.lo < old.hi - old.lo)
            {
                best = Some((left, right, combined));
            }
        }
        let Some((left, right, _)) = best else {
            heap.push(parent);
            return report(sum(&heap)?, cells, tolerance, StopReason::PrecisionLimit);
        };
        total = Interval::new(
            (total.lo - parent.area.lo).next_down(),
            (total.hi - parent.area.hi).next_up(),
        )?
        .add(left.area)?
        .add(right.area)?;
        total.lo = total.lo.max(0.);
        heap.push(left);
        heap.push(right);
    }
}
