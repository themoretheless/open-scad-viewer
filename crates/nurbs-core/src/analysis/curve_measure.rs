//! Arc-length enclosures of the original positive-weight rational definition.
//! Every active knot span is treated separately. Interval local jets and a
//! trapezoid remainder enclose rounding and integration error; no curve fitting.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets,
    distance_bounds::{Interval, box_distance},
    numeric, resource,
};
use std::{cmp::Ordering, collections::{BinaryHeap,BTreeMap}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Tolerance,
    WorkLimit,
    PrecisionLimit,
    IterationLimit,
    TargetUncertainty,
}
#[derive(Clone, Debug)]
pub struct LengthReport {
    pub value: f64,
    pub bounds: [f64; 2],
    pub error_upper: f64,
    pub within_tolerance: bool,
    /// Number of whole-cell enclosures constructed (includes replaced cells).
    pub cells: usize,
    pub stop_reason: StopReason,
}
#[derive(Clone, Debug)]
pub struct LengthPoint {
    pub parameter: f64,
    /// Source-parameter enclosure for the exact length inverse. Bisection only
    /// discards a side after disjoint prefix/target interval proof. A bracket
    /// does not assert existence or uniqueness for arbitrary target distances.
    /// Normalized division endpoints select the original domain endpoints.
    pub inverse_parameter_bounds: [f64; 2],
    pub point: Vec<f64>,
    /// Original rational point coordinate enclosures at the returned parameter.
    pub point_bounds: Vec<[f64; 2]>,
    pub prefix_bounds: [f64; 2],
    pub target_bounds: [f64; 2],
    pub residual_upper: f64,
    pub within_tolerance: bool,
    pub cells: usize,
    pub stop_reason: StopReason,
}
#[derive(Clone, Debug)]
pub struct DivisionReport {
    /// May contain a partial sequence when the global budget is exhausted.
    pub points: Vec<LengthPoint>,
    pub total: LengthReport,
    pub cells: usize,
    pub within_tolerance: bool,
    pub stop_reason: StopReason,
}
struct Cell {
    span: usize,
    domain: [f64; 2],
    length: Interval,
    serial: usize,
}
impl Cell {
    fn gap(&self) -> f64 {
        self.length.hi - self.length.lo
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
fn validate(tolerance: f64, max_cells: usize) -> Result<()> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Length tolerance must be positive and finite",
    )?;
    check(
        (1..=100_000).contains(&max_cells),
        "Length work budget must be 1..100000 cell enclosures",
    )
}
fn norm(values: &[Interval]) -> Result<Interval> {
    let (lo, hi) = box_distance(values, &vec![Interval::point(0.); values.len()])?;
    Interval::new(lo, hi)
}
fn cell(curve: &Curve, span: usize, domain: [f64; 2], serial: usize) -> Result<Cell> {
    let jets = curve_jets::enclose(curve, span, domain)?;
    let speed = norm(&jets[1])?;
    let start = curve_jets::endpoint(curve, span, domain, false)?;
    let end = curve_jets::endpoint(curve, span, domain, true)?;
    let chord = box_distance(&start[0], &end[0])?.0;
    let mut lo = speed.lo.max(chord);
    let mut hi = speed.hi;
    if speed.lo > 0. {
        // |(|C'|)''| <= |C'''| + |C''|^2 / min |C'|.
        // For the local [0,1] parameter, trapezoid error <= max |f''|/12.
        let acceleration = norm(&jets[2])?;
        let third = norm(&jets[3])?;
        let remainder = third
            .add(
                acceleration
                    .mul(acceleration)?
                    .div(Interval::point(speed.lo))?,
            )?
            .div(Interval::point(12.))?
            .hi;
        let trapezoid = norm(&start[1])?
            .add(norm(&end[1])?)?
            .mul(Interval::point(0.5))?;
        lo = lo.max((trapezoid.lo - remainder).next_down().max(0.));
        hi = hi.min((trapezoid.hi + remainder).next_up());
    }
    numeric(lo <= hi, "Arc-length enclosures are inconsistent")?;
    Ok(Cell {
        span,
        domain,
        length: Interval::new(lo, hi)?,
        serial,
    })
}
fn report(
    bounds: Interval,
    cells: usize,
    tolerance: f64,
    reason: StopReason,
) -> Result<LengthReport> {
    let value = bounds.lo + (bounds.hi - bounds.lo) / 2.;
    let error_upper = (value - bounds.lo).max(bounds.hi - value).next_up();
    numeric(
        value.is_finite() && error_upper.is_finite(),
        "Arc length exceeded numeric range",
    )?;
    let within_tolerance = error_upper <= tolerance;
    Ok(LengthReport {
        value,
        bounds: [bounds.lo, bounds.hi],
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
fn sum(heap: &BinaryHeap<Cell>) -> Result<Interval> {
    let mut bounds = Interval::point(0.);
    for c in heap {
        bounds = bounds.add(c.length)?;
    }
    Interval::new(bounds.lo.max(0.), bounds.hi)
}
fn measure(
    curve: &Curve,
    domain: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthReport> {
    Ok(measure_partition(curve,domain,tolerance,max_cells)?.0)
}
fn measure_partition(curve:&Curve,domain:[f64;2],tolerance:f64,max_cells:usize)->Result<(LengthReport,Vec<Cell>)> {
    if domain[0] == domain[1] {
        return Ok((report(Interval::point(0.), 0, tolerance, StopReason::Tolerance)?,Vec::new()));
    }
    let mut heap = BinaryHeap::new();
    let mut cells = 0;
    for span in curve.degree..curve.control_points.len() {
        let a = curve.knots[span].max(domain[0]);
        let b = curve.knots[span + 1].min(domain[1]);
        if a < b {
            if cells == max_cells {
                return Err(resource("Initial length spans exceed the work budget"));
            }
            heap.push(cell(curve, span, [a, b], cells)?);
            cells += 1;
        }
    }
    let mut bounds = sum(&heap)?;
    loop {
        let result = report(bounds, cells, tolerance, StopReason::WorkLimit)?;
        if result.within_tolerance || cells + 2 > max_cells {
            bounds = sum(&heap)?;
            let recomputed = report(bounds, cells, tolerance, StopReason::WorkLimit)?;
            if recomputed.within_tolerance || cells + 2 > max_cells {
                return Ok((recomputed,heap.into_vec()));
            }
        }
        let largest = heap.pop().unwrap();
        let [a, b] = largest.domain;
        let mid = a + (b - a) / 2.;
        if !(a < mid && mid < b) {
            heap.push(largest);
            return Ok((report(sum(&heap)?, cells, tolerance, StopReason::PrecisionLimit)?,heap.into_vec()));
        }
        let left = cell(curve, largest.span, [a, mid], cells)?;
        let right = cell(curve, largest.span, [mid, b], cells + 1)?;
        cells += 2;
        // Remove the exact stored bound endpoints, then add child enclosures.
        // Outward subtraction preserves accumulated lower/upper-sum guarantees.
        bounds = Interval::new(
            (bounds.lo - largest.length.lo).next_down(),
            (bounds.hi - largest.length.hi).next_up(),
        )?
        .add(left.length)?
        .add(right.length)?;
        bounds.lo = bounds.lo.max(0.);
        heap.push(left);
        heap.push(right);
    }
}
/// Enclose the total geometric length in model units. Work exhaustion retains
/// valid bounds; `within_tolerance` must be inspected before using the estimate.
pub fn length(curve: &Curve, tolerance: f64, max_cells: usize) -> Result<LengthReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    measure(curve, curve.domain(), tolerance, max_cells)
}
fn point_report(
    curve: &Curve,
    parameter: f64,
    prefix: Interval,
    target: Interval,
    cells: usize,
    tolerance: f64,
    reason: StopReason,
) -> Result<LengthPoint> {
    let residual = prefix.sub(target)?;
    let residual_upper = residual.lo.abs().max(residual.hi.abs());
    let within_tolerance = residual_upper <= tolerance;
    let span = (curve.degree..curve.control_points.len())
        .find(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && parameter >= curve.knots[i]
                && parameter <= curve.knots[i + 1]
        })
        .ok_or_else(|| crate::numeric_err("Length point has no owning span"))?;
    let (domain, end) = if parameter == curve.knots[span] {
        ([parameter, curve.knots[span + 1]], false)
    } else {
        ([curve.knots[span], parameter], true)
    };
    let bounds = curve_jets::endpoint(curve, span, domain, end)?;
    Ok(LengthPoint {
        parameter,
        inverse_parameter_bounds: curve.domain(),
        point: curve.evaluate(parameter)?.point,
        point_bounds: bounds[0].iter().map(|i| [i.lo, i.hi]).collect(),
        prefix_bounds: [prefix.lo, prefix.hi],
        target_bounds: [target.lo, target.hi],
        residual_upper,
        within_tolerance,
        cells,
        stop_reason: if within_tolerance {
            StopReason::Tolerance
        } else {
            reason
        },
    })
}
fn invert(
    curve: &Curve,
    target: Interval,
    total: &LengthReport,
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthPoint> {
    let mut cache=BTreeMap::new();
    invert_with_prefix_cache(curve,target,total,tolerance,max_cells,&mut cache,&[])
}
fn invert_with_prefix_cache(
    curve:&Curve,target:Interval,total:&LengthReport,tolerance:f64,max_cells:usize,
    cache:&mut BTreeMap<u64,LengthReport>,
    partition:&[Cell],
)->Result<LengthPoint>{
    let [a, b] = curve.domain();
    let mut best = point_report(
        curve,
        a,
        Interval::point(0.),
        target,
        0,
        tolerance,
        StopReason::WorkLimit,
    )?;
    let end = point_report(
        curve,
        b,
        Interval::new(total.bounds[0], total.bounds[1])?,
        target,
        0,
        tolerance,
        StopReason::WorkLimit,
    )?;
    if end.residual_upper < best.residual_upper {
        best = end;
    }
    if best.within_tolerance {
        return Ok(best);
    }
    let mut lo = a;
    let mut hi = b;
    let mut cells = 0;
    for _ in 0..128 {
        let mid = lo + (hi - lo) / 2.;
        if !(lo < mid && mid < hi) {
            best.stop_reason = StopReason::PrecisionLimit;
            best.cells = cells;
            best.inverse_parameter_bounds = [lo, hi];
            return Ok(best);
        }
        let needed = (curve.degree..curve.control_points.len())
            .filter(|&i| curve.knots[i] < curve.knots[i + 1] && curve.knots[i] < mid)
            .count();
        // Cache belongs to one division of this exact source, tolerance and
        // total measurement. Completed prefix enclosures remain valid for all
        // station targets; charge their cell construction only once.
        let prefix=if let Some(prefix)=cache.get(&mid.to_bits()) {
            prefix.clone()
        } else {
            if max_cells - cells < needed {
                best.cells = cells;
                best.inverse_parameter_bounds = [lo, hi];
                return Ok(best);
            }
            // Completed cells belong to this exact source and total division.
            // Sum their original interval enclosures; only the cut cell needs
            // a new enclosure. Never subtract independent total/prefix boxes.
            let mut reused=None;
            if !partition.is_empty() {
                let mut bounds=Interval::point(0.);
                let mut cut_work=0;
                for part in partition {
                    if part.domain[1]<=mid {bounds=bounds.add(part.length)?;}
                    else if part.domain[0]<mid {
                        if cells<max_cells {
                            bounds=bounds.add(cell(curve,part.span,[part.domain[0],mid],0)?.length)?;
                            cut_work+=1;
                        }else {cut_work=usize::MAX;break;}
                    }
                }
                if cut_work!=usize::MAX {
                    let candidate=report(bounds,cut_work,tolerance/8.,StopReason::WorkLimit)?;
                    cells+=cut_work;
                    if candidate.within_tolerance {
                        reused=Some(LengthReport {cells:0,..candidate});
                    }
                }
            }
            let prefix=if let Some(reused)=reused {reused} else {
                if max_cells-cells<needed {
                    best.cells=cells;best.inverse_parameter_bounds=[lo,hi];return Ok(best);
                }
                measure(curve,[a,mid],tolerance/8.,max_cells-cells)?
            };
            cells+=prefix.cells;
            if prefix.within_tolerance {cache.insert(mid.to_bits(),prefix.clone());}
            prefix
        };
        let candidate = point_report(
            curve,
            mid,
            Interval::new(prefix.bounds[0], prefix.bounds[1])?,
            target,
            cells,
            tolerance,
            StopReason::WorkLimit,
        )?;
        if candidate.residual_upper < best.residual_upper {
            best = candidate;
        }
        if best.within_tolerance {
            best.cells = cells;
            best.inverse_parameter_bounds = [lo, hi];
            return Ok(best);
        }
        if prefix.bounds[1] < target.lo {
            lo = mid;
        } else if prefix.bounds[0] > target.hi {
            hi = mid;
        } else {
            // Overlapping uncertain intervals cannot choose a side safely.
            best.stop_reason = if prefix.stop_reason == StopReason::Tolerance {
                StopReason::TargetUncertainty
            } else {
                prefix.stop_reason
            };
            best.cells = cells;
            best.inverse_parameter_bounds = [lo, hi];
            return Ok(best);
        }
    }
    best.stop_reason = StopReason::IterationLimit;
    best.cells = cells;
    best.inverse_parameter_bounds = [lo, hi];
    Ok(best)
}
/// Point whose original prefix length differs from `distance` by at most the
/// returned residual bound. Distances above the total upper bound are rejected;
/// near the uncertain endpoint, only the residual guarantee is asserted.
pub fn point_at_length(
    curve: &Curve,
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthPoint> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    check(
        tolerance / 8. > 0.,
        "Inverse length tolerance is too small to subdivide",
    )?;
    check(
        distance.is_finite() && distance >= 0.,
        "Arc distance must be finite and nonnegative",
    )?;
    let total = measure(curve, curve.domain(), tolerance / 8., max_cells)?;
    check(
        distance <= total.bounds[1],
        "Arc distance exceeds the total length upper bound",
    )?;
    let mut point = invert(
        curve,
        Interval::point(distance),
        &total,
        tolerance,
        max_cells - total.cells,
    )?;
    point.cells += total.cells;
    Ok(point)
}
/// Parameters at fractions i/segments of the original true total length.
/// All prefix/target intervals include rounding. The work budget is global
/// across the total and all inverse queries; partial results retain diagnostics.
pub fn divide_by_length(
    curve: &Curve,
    segments: usize,
    tolerance: f64,
    max_cells: usize,
) -> Result<DivisionReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    check(
        tolerance / 8. > 0.,
        "Inverse length tolerance is too small to subdivide",
    )?;
    check(
        (1..=1024).contains(&segments),
        "Arc division needs 1..1024 segments",
    )?;
    let (total,partition) = measure_partition(curve, curve.domain(), tolerance / 8., max_cells)?;
    let mut cells = total.cells;
    let mut points = Vec::new();
    let mut prefix_cache=BTreeMap::new();
    let mut reason = StopReason::Tolerance;
    for i in 0..=segments {
        // Integer division is rounded; enclose the exact rational fraction.
        let ratio = if i == 0 {
            Interval::point(0.)
        } else if i == segments {
            Interval::point(1.)
        } else {
            Interval::new(
                (i as f64 / segments as f64).next_down(),
                (i as f64 / segments as f64).next_up(),
            )?
        };
        let target = Interval::new(total.bounds[0], total.bounds[1])?.mul(ratio)?;
        let point = if i == 0 || i == segments {
            let (parameter, prefix, target) = if i == 0 {
                (curve.domain()[0], Interval::point(0.), Interval::point(0.))
            } else {
                (
                    curve.domain()[1],
                    Interval::new(total.bounds[0], total.bounds[1])?,
                    Interval::new(total.bounds[0], total.bounds[1])?,
                )
            };
            let mut point = point_report(
                curve,
                parameter,
                prefix,
                target,
                0,
                tolerance,
                StopReason::Tolerance,
            )?;
            // Prefix and target are the same exact quantity at fraction 0/1.
            // Preserve this correlation rather than subtract independent boxes.
            point.residual_upper = 0.;
            point.inverse_parameter_bounds = [parameter, parameter];
            point.within_tolerance = true;
            point.stop_reason = StopReason::Tolerance;
            point
        } else {
            invert_with_prefix_cache(curve,target,&total,tolerance,max_cells-cells,&mut prefix_cache,&partition)?
        };
        cells += point.cells;
        let accepted = point.within_tolerance;
        reason = point.stop_reason;
        points.push(point);
        if !accepted {
            break;
        }
    }
    let within_tolerance =
        points.len() == segments + 1 && points.iter().all(|p| p.within_tolerance);
    Ok(DivisionReport {
        points,
        total,
        cells,
        within_tolerance,
        stop_reason: if within_tolerance {
            StopReason::Tolerance
        } else {
            reason
        },
    })
}

#[cfg(test)]
mod shared_partition_regression {
    use super::*;
    #[test]
    fn circular_division_reuses_original_total_cells_with_a_shared_budget() {
        let curve=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
        let division=divide_by_length(&curve,32,0.001,10000).unwrap();
        assert!(division.within_tolerance,"{division:?}");
        assert!(division.cells<=10000);
        assert_eq!(division.points.len(),33);
        for (i,point) in division.points.iter().enumerate() {
            assert!(point.residual_upper<=0.001);
            // Independent circle length reference supplements interval proof.
            let angle=if i==32 {2.*std::f64::consts::PI} else {
                point.point[1].atan2(point.point[0]).rem_euclid(2.*std::f64::consts::PI)
            };
            let expected=8.*std::f64::consts::PI*i as f64/32.;
            assert!((4.*angle-expected).abs()<=point.residual_upper+1e-12);
        }
        assert!(division.total.bounds[0]<=8.*std::f64::consts::PI
            && 8.*std::f64::consts::PI<=division.total.bounds[1]);
        let limited=divide_by_length(&curve,32,0.001,division.total.cells).unwrap();
        assert!(!limited.within_tolerance);
        assert!(limited.cells<=division.total.cells);
    }
}
