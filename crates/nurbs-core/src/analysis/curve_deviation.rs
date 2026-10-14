//! Continuous corresponding-parameter deviation with outward interval bounds.
use crate::{DistanceStopReason, Result, check, curve::Curve, resource};
use crate::{
    curve_distance::enclosure,
    distance_bounds::{Interval, box_distance},
};
#[derive(Clone, Debug)]
pub struct Report {
    pub bounds: [f64; 2],
    pub converged: bool,
    pub reason: DistanceStopReason,
    pub cells: usize,
}
struct Cell {
    domain: [f64; 2],
    spans: [usize; 2],
    upper: f64,
    lower: f64,
}
fn midpoint(d: [f64; 2]) -> f64 {
    d[0] * 0.5 + d[1] * 0.5
}
fn make(a: &Curve, b: &Curve, domain: [f64; 2], spans: [usize; 2]) -> Result<Cell> {
    let interval = Interval::new(domain[0], domain[1])?;
    let upper = box_distance(
        &enclosure(a, spans[0], interval)?,
        &enclosure(b, spans[1], interval)?,
    )?
    .1;
    let mut lower = 0_f64;
    for t in [domain[0], midpoint(domain), domain[1]] {
        let i = Interval::point(t);
        lower =
            lower.max(box_distance(&enclosure(a, spans[0], i)?, &enclosure(b, spans[1], i)?)?.0);
    }
    Ok(Cell {
        domain,
        spans,
        upper,
        lower,
    })
}
/// Bound the supremum distance between corresponding points on equal domains.
/// This is not a minimum distance or a best reparameterization. `accuracy`
/// controls the width of the reported maximum-deviation interval.
pub fn inspect(a: &Curve, b: &Curve, accuracy: f64, max_cells: usize) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        a.domain() == b.domain(),
        "Curve deviation requires identical active domains",
    )?;
    check(
        a.control_points[0].len() == b.control_points[0].len(),
        "Curve deviation dimensions must match",
    )?;
    check(
        accuracy.is_finite() && accuracy > 0.,
        "Deviation accuracy must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Deviation needs 1..100000 cells",
    )?;
    let [lo, hi] = a.domain();
    for c in [a, b] {
        for &t in &c.knots {
            if lo < t && t < hi {
                check(
                    c.knots.iter().filter(|&&k| k == t).count() <= c.degree,
                    "Curve deviation requires continuous interior knots",
                )?;
            }
        }
    }
    if a == b {
        return Ok(Report {
            bounds: [0., 0.],
            converged: true,
            reason: DistanceStopReason::Tolerance,
            cells: 0,
        });
    }
    let mut breaks: Vec<_> = a
        .knots
        .iter()
        .chain(&b.knots)
        .copied()
        .filter(|&t| lo <= t && t <= hi)
        .collect();
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if breaks.len() - 1 > max_cells {
        return Err(resource("Initial deviation grid exceeds the cell budget"));
    }
    let span = |c: &Curve, t: f64| {
        (c.degree..c.control_points.len())
            .find(|&i| c.knots[i] <= t && t < c.knots[i + 1])
            .unwrap()
    };
    let mut cells = Vec::new();
    for d in breaks.windows(2) {
        cells.push(make(a, b, [d[0], d[1]], [span(a, d[0]), span(b, d[0])])?);
    }
    let mut work = cells.len();
    let mut lower = cells.iter().map(|x| x.lower).fold(0., f64::max);
    let reason;
    loop {
        cells.retain(|x| x.upper >= lower);
        let upper = cells.iter().map(|x| x.upper).fold(lower, f64::max);
        if upper - lower <= accuracy {
            reason = DistanceStopReason::Tolerance;
            break;
        }
        if work + 2 > max_cells {
            reason = DistanceStopReason::WorkLimit;
            break;
        }
        let index = cells
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.upper.total_cmp(&b.1.upper))
            .unwrap()
            .0;
        let d = cells[index].domain;
        let m = midpoint(d);
        if !(d[0] < m && m < d[1]) {
            reason = DistanceStopReason::PrecisionLimit;
            break;
        }
        let old = cells.swap_remove(index);
        for sub in [[d[0], m], [m, d[1]]] {
            let x = make(a, b, sub, old.spans)?;
            lower = lower.max(x.lower);
            cells.push(x);
            work += 1;
        }
    }
    let upper = cells.iter().map(|x| x.upper).fold(lower, f64::max);
    Ok(Report {
        bounds: [lower, upper],
        converged: reason == DistanceStopReason::Tolerance,
        reason,
        cells: work,
    })
}
