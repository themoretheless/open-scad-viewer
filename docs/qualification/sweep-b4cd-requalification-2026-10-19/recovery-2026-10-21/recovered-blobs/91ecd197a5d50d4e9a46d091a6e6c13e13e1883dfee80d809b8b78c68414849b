//! Continuous whole-surface phase bounds for a periodic seam edit.
use super::Verification;
use crate::{
    DistanceStopReason, Result, check,
    distance_bounds::{Interval as I, box_distance},
    resource,
    surface::{Axis, Surface},
    surface_distance::{enclosure, rectangle_bounds},
};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Debug)]
pub struct SurfaceReport {
    pub surface: Option<Surface>,
    pub verification: Verification,
}
struct Cell {
    domain: [[f64; 2]; 2],
    span: [usize; 2],
    upper: f64,
}
impl PartialEq for Cell {
    fn eq(&self, b: &Self) -> bool {
        self.upper == b.upper
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
        self.upper.total_cmp(&b.upper)
    }
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    crate::certificates::audit::surface_domains(s)
}
fn original_image(
    source: &Surface,
    candidate: &Surface,
    axis: usize,
    domain: [[f64; 2]; 2],
) -> Result<Vec<I>> {
    let [a, b] = domains(source)[axis];
    let [start, end] = domains(candidate)[axis];
    let period = I::point(b).sub(I::point(a))?;
    let t = I::new(domain[axis][0], domain[axis][1])?;
    let phase = t
        .sub(I::point(start))?
        .div(I::point(end).sub(I::point(start))?)?
        .intersect(0., 1.)?;
    let unwrapped = I::point(start).add(phase.mul(period)?)?;
    let mut intervals = Vec::new();
    if unwrapped.lo <= b {
        intervals.push(unwrapped.intersect(a, b)?);
    }
    if unwrapped.hi >= b {
        intervals.push(unwrapped.sub(period)?.intersect(a, b)?);
    }
    let mut images = Vec::new();
    for interval in intervals {
        let mut d = domain;
        d[axis] = [interval.lo, interval.hi];
        images.push(
            rectangle_bounds(source, d)?
                .into_iter()
                .map(|x| I::new(x[0], x[1]))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    let mut out = images.remove(0);
    for next in images {
        for (a, b) in out.iter_mut().zip(next) {
            a.lo = a.lo.min(b.lo);
            a.hi = a.hi.max(b.hi);
        }
    }
    Ok(out)
}
fn cell(
    source: &Surface,
    candidate: &Surface,
    axis: usize,
    domain: [[f64; 2]; 2],
    span: [usize; 2],
) -> Result<Cell> {
    let upper = box_distance(
        &original_image(source, candidate, axis, domain)?,
        &enclosure(candidate, span, domain)?,
    )?
    .1;
    Ok(Cell {
        domain,
        span,
        upper,
    })
}
/// Verify the entire tensor-product image, with normalized periodic phase on
/// the selected axis and identical original parameter on the other axis.
/// Refusal is inconclusive; acceptance continuously bounds all corresponding
/// positions and symmetric Hausdorff deviation of the whole untrimmed domain.
pub fn verify_surface(
    source: &Surface,
    candidate: &Surface,
    axis: Axis,
    tolerance: f64,
    max_cells: usize,
) -> Result<Verification> {
    source.validate()?;
    candidate.validate()?;
    let axis = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let periodic = |s: &Surface| {
        if axis == 0 {
            s.periodic_u
        } else {
            s.periodic_v
        }
    };
    check(
        periodic(source) && periodic(candidate),
        "Surface phase axis must be periodic",
    )?;
    let natural = domains(source);
    let target = domains(candidate);
    check(
        target[1 - axis] == natural[1 - axis],
        "Other-axis domains must agree for phase verification",
    )?;
    check(
        natural[axis][0] <= target[axis][0] && target[axis][0] < natural[axis][1],
        "Surface seam must lie inside the original half-open domain",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Surface phase tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Surface phase verification needs 1..100000 cells",
    )?;
    let us = crate::certificates::audit::nonempty_spans(
        &candidate.knots_u,
        candidate.degree_u,
        candidate.control_points.len(),
    );
    let vs = crate::certificates::audit::nonempty_spans(
        &candidate.knots_v,
        candidate.degree_v,
        candidate.control_points[0].len(),
    );
    if us.len() * vs.len() > max_cells {
        return Err(resource(
            "Initial surface phase rectangles exceed the cell budget",
        ));
    }
    let mut heap = BinaryHeap::new();
    for u in us {
        for &v in &vs {
            heap.push(cell(
                source,
                candidate,
                axis,
                [
                    [candidate.knots_u[u], candidate.knots_u[u + 1]],
                    [candidate.knots_v[v], candidate.knots_v[v + 1]],
                ],
                [u, v],
            )?);
        }
    }
    let mut work = heap.len();
    loop {
        let c = heap.pop().unwrap();
        let stop = if c.upper <= tolerance {
            Some(DistanceStopReason::Tolerance)
        } else if work + 2 > max_cells {
            Some(DistanceStopReason::WorkLimit)
        } else {
            None
        };
        if let Some(reason) = stop {
            return Ok(Verification {
                error_upper: c.upper,
                accepted: reason == DistanceStopReason::Tolerance,
                cells: work,
                reason,
            });
        }
        let width = c.domain.map(|d| d[1] - d[0]);
        let preferred = usize::from(
            width[1] / (target[1][1] - target[1][0]) > width[0] / (target[0][1] - target[0][0]),
        );
        let choice = [preferred, 1 - preferred].into_iter().find_map(|k| {
            let m = c.domain[k][0] * 0.5 + c.domain[k][1] * 0.5;
            (c.domain[k][0] < m && m < c.domain[k][1]).then_some((k, m))
        });
        let Some((k, m)) = choice else {
            return Ok(Verification {
                error_upper: c.upper,
                accepted: false,
                cells: work,
                reason: DistanceStopReason::PrecisionLimit,
            });
        };
        for half in 0..2 {
            let mut d = c.domain;
            d[k][1 - half] = m;
            heap.push(cell(source, candidate, axis, d, c.span)?);
        }
        work += 2;
    }
}
/// Return the seam-edited surface only if the full continuous bound fits.
pub fn surface_report(
    source: &Surface,
    axis: Axis,
    parameter: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<SurfaceReport> {
    let candidate = super::surface_candidate(source, axis, parameter)?;
    let verification = verify_surface(source, &candidate, axis, tolerance, max_cells)?;
    Ok(SurfaceReport {
        surface: verification.accepted.then_some(candidate),
        verification,
    })
}
