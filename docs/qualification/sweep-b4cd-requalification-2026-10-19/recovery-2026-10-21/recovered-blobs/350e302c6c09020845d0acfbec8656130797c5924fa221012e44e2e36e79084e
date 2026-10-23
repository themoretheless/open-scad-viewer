//! Continuous phase-correspondence bounds, including binary64 seam refinement.
use crate::{
    DistanceStopReason, Result, check,
    curve::Curve,
    curve_distance::enclosure,
    distance_bounds::{Interval as I, box_distance},
    resource,
};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Debug)]
pub struct Verification {
    pub error_upper: f64,
    pub accepted: bool,
    pub cells: usize,
    pub reason: DistanceStopReason,
}
#[derive(Clone, Debug)]
pub struct CurveReport {
    /// Only present after the full continuous bound fits the requested tolerance.
    pub curve: Option<Curve>,
    pub verification: Verification,
}
struct Cell {
    domain: [f64; 2],
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
fn image(c: &Curve, t: I) -> Result<Vec<I>> {
    let mut out = vec![I::new(-f64::MAX, f64::MAX)?; c.control_points[0].len()];
    let mut first = true;
    for span in c.degree..c.control_points.len() {
        let lo = t.lo.max(c.knots[span]);
        let hi = t.hi.min(c.knots[span + 1]);
        if c.knots[span] >= c.knots[span + 1] || lo > hi {
            continue;
        }
        let b = enclosure(c, span, I::new(lo, hi)?)?;
        if first {
            out = b;
            first = false;
        } else {
            for (a, b) in out.iter_mut().zip(b) {
                a.lo = a.lo.min(b.lo);
                a.hi = a.hi.max(b.hi);
            }
        }
    }
    check(!first, "Phase image has no owning source span")?;
    Ok(out)
}
fn original_image(source: &Curve, candidate: &Curve, t: I) -> Result<Vec<I>> {
    let [a, b] = source.domain();
    let [start, end] = candidate.domain();
    let period = I::point(b).sub(I::point(a))?;
    let phase = t
        .sub(I::point(start))?
        .div(I::point(end).sub(I::point(start))?)?
        .intersect(0., 1.)?;
    let unwrapped = I::point(start).add(phase.mul(period)?)?;
    let mut images = Vec::new();
    if unwrapped.lo <= b {
        images.push(image(source, unwrapped.intersect(a, b)?)?);
    }
    if unwrapped.hi >= b {
        images.push(image(source, unwrapped.sub(period)?.intersect(a, b)?)?);
    }
    let mut out = images.remove(0);
    for next in images {
        for (x, y) in out.iter_mut().zip(next) {
            x.lo = x.lo.min(y.lo);
            x.hi = x.hi.max(y.hi);
        }
    }
    Ok(out)
}
fn cell(source: &Curve, candidate: &Curve, domain: [f64; 2]) -> Result<Cell> {
    let t = I::new(domain[0], domain[1])?;
    let upper = box_distance(
        &original_image(source, candidate, t)?,
        &image(candidate, t)?,
    )?
    .1;
    Ok(Cell { domain, upper })
}
/// Bound the entire phase correspondence C_new(t) versus C_old(wrap(start +
/// ((t-start)/(end-start))*old_period)). Normalized phase avoids silently
/// assuming that a rounded new endpoint preserves the exact original period.
/// Acceptance is sufficient; refusal does not prove tolerance was exceeded.
pub fn verify_curve(
    source: &Curve,
    candidate: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<Verification> {
    source.validate()?;
    candidate.validate()?;
    check(
        crate::section_phase::closed(source) && crate::section_phase::closed(candidate),
        "Phase verification requires periodic or exactly closed clamped curves",
    )?;
    check(
        source.control_points[0].len() == candidate.control_points[0].len(),
        "Phase curves must have matching dimension",
    )?;
    let [a, b] = source.domain();
    let start = candidate.domain()[0];
    check(
        a <= start && start < b,
        "Candidate seam must lie inside the original half-open domain",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Phase tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Phase verification needs 1..100000 cells",
    )?;
    let domains: Vec<_> = crate::certificates::audit::curve_span_domains(candidate)
        .into_iter()
        .map(|(_, d)| d)
        .collect();
    if domains.len() > max_cells {
        return Err(resource("Initial phase spans exceed the cell budget"));
    }
    let mut heap = BinaryHeap::new();
    for d in domains {
        heap.push(cell(source, candidate, d)?);
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
        let midpoint = c.domain[0] * 0.5 + c.domain[1] * 0.5;
        let stop = stop.or_else(|| {
            (!(c.domain[0] < midpoint && midpoint < c.domain[1]))
                .then_some(DistanceStopReason::PrecisionLimit)
        });
        if let Some(reason) = stop {
            return Ok(Verification {
                error_upper: c.upper,
                accepted: reason == DistanceStopReason::Tolerance,
                cells: work,
                reason,
            });
        }
        for d in [[c.domain[0], midpoint], [midpoint, c.domain[1]]] {
            heap.push(cell(source, candidate, d)?);
        }
        work += 2;
    }
}
/// Refine a seam and return the new curve only after continuous verification.
pub fn curve_report(
    source: &Curve,
    parameter: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CurveReport> {
    let candidate = super::curve_candidate(source, parameter)?;
    let verification = verify_curve(source, &candidate, tolerance, max_cells)?;
    Ok(CurveReport {
        curve: verification.accepted.then_some(candidate),
        verification,
    })
}
