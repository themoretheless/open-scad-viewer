//! Certified periodized winding queries, including both UV lattice axes.
use crate::trim_domain::{Classification, ClassificationReason, TrimDomain, exact_loop_joins};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicWindingReason {
    Complete,
    ClosureUnproven,
    PeriodUnresolved,
    IndexRange,
    ImageBudget,
    WorkLimit,
    ClassificationUnresolved,
    WindingOverflow,
}
#[derive(Clone, Debug)]
pub struct ImageWinding {
    pub tile: [i32; 2],
    /// Outward enclosure of point + tile * exact authored chart period.
    pub query: [[f64; 2]; 2],
    pub classification: Classification,
}
#[derive(Clone, Debug)]
pub struct PeriodicWindingReport {
    /// Full integer sum only after every potentially contributing image.
    pub winding: Option<i32>,
    pub reason: PeriodicWindingReason,
    pub cells: usize,
    pub required_images: Option<usize>,
    pub images: Vec<ImageWinding>,
}
/// Sum winding(point + k*period) over the complete two-dimensional UV lattice.
/// Positive-weight control hulls bound all possibly nonzero terms. Original
/// and shifted query arithmetic is outward-rounded; no rounded query point is
/// substituted for the exact authored lattice point. Closed loops are exact,
/// tolerance_uv controls only the unresolved boundary band, never a snap.
/// At most 256 curves, 4096 spans, 1..4096 images and 1..100000 shared cells.
pub fn inspect_periodic_winding(
    loops: &[Vec<Curve>],
    chart: [[f64; 2]; 2],
    point: [f64; 2],
    tolerance_uv: f64,
    max_images: usize,
    max_cells: usize,
) -> Result<PeriodicWindingReport> {
    check(
        !loops.is_empty()
            && loops.len() <= 256
            && loops.iter().all(|l| !l.is_empty())
            && loops.iter().map(Vec::len).sum::<usize>() <= 256
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1])
            && point.iter().all(|x| x.is_finite())
            && tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && (1..=4096).contains(&max_images)
            && (1..=100000).contains(&max_cells),
        "Invalid periodic winding loops, chart, query or budgets",
    )?;
    let mut out = PeriodicWindingReport {
        winding: None,
        reason: PeriodicWindingReason::ClosureUnproven,
        cells: 0,
        required_images: None,
        images: Vec::new(),
    };
    for contour in loops {
        for curve in contour {
            curve.validate()?;
        }
        if exact_loop_joins(contour)? != Some(true) {
            return Ok(out);
        }
    }
    let mut periods = [I::point(0.); 2];
    let mut ranges = [[0i32; 2]; 2];
    for axis in 0..2 {
        periods[axis] = I::point(chart[axis][1]).sub(I::point(chart[axis][0]))?;
        if periods[axis].lo <= 0. {
            out.reason = PeriodicWindingReason::PeriodUnresolved;
            return Ok(out);
        }
        let lo = loops
            .iter()
            .flatten()
            .flat_map(|c| c.control_points.iter())
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min);
        let hi = loops
            .iter()
            .flatten()
            .flat_map(|c| c.control_points.iter())
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        let quotient = I::new(lo, hi)?
            .sub(I::point(point[axis]))?
            .div(periods[axis])?;
        let first = quotient.lo.floor() - 1.;
        let last = quotient.hi.ceil() + 1.;
        if first < i32::MIN as f64 || last > i32::MAX as f64 {
            out.reason = PeriodicWindingReason::IndexRange;
            return Ok(out);
        }
        ranges[axis] = [first as i32, last as i32];
    }
    let widths = ranges.map(|r| (i64::from(r[1]) - i64::from(r[0]) + 1) as u64);
    out.required_images = widths[0]
        .checked_mul(widths[1])
        .and_then(|n| usize::try_from(n).ok());
    if out.required_images.is_none_or(|n| n > max_images) {
        out.reason = PeriodicWindingReason::ImageBudget;
        return Ok(out);
    }
    let domain = TrimDomain::new(loops, tolerance_uv)?;
    let mut winding = 0i32;
    for u in ranges[0][0]..=ranges[0][1] {
        for v in ranges[1][0]..=ranges[1][1] {
            if out.cells == max_cells {
                out.reason = PeriodicWindingReason::WorkLimit;
                return Ok(out);
            }
            let tile = [u, v];
            let mut query = [[0.; 2]; 2];
            for axis in 0..2 {
                let interval = I::point(point[axis])
                    .add(I::point(f64::from(tile[axis])).mul(periods[axis])?)?;
                query[axis] = [interval.lo, interval.hi];
            }
            let classification = domain.classify(query, max_cells - out.cells)?;
            out.cells += classification.cells;
            let value = classification.winding;
            out.images.push(ImageWinding {
                tile,
                query,
                classification,
            });
            let Some(value) = value else {
                out.reason = if out.images.last().unwrap().classification.reason
                    == ClassificationReason::WorkLimit
                {
                    PeriodicWindingReason::WorkLimit
                } else {
                    PeriodicWindingReason::ClassificationUnresolved
                };
                return Ok(out);
            };
            let Some(sum) = winding.checked_add(value) else {
                out.reason = PeriodicWindingReason::WindingOverflow;
                return Ok(out);
            };
            winding = sum;
        }
    }
    out.winding = Some(winding);
    out.reason = PeriodicWindingReason::Complete;
    Ok(out)
}
