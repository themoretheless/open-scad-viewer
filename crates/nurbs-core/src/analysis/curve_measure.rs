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
mod division_partition;

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
    mapped_cell(curve, span, domain, serial, Ok)
}

fn mapped_cell(
    curve: &Curve, span: usize, domain: [f64;2], serial: usize,
    map: impl Fn(Vec<Vec<Interval>>) -> Result<Vec<Vec<Interval>>>,
) -> Result<Cell> {
    let jets = map(curve_jets::enclose(curve, span, domain)?)?;
    let speed = norm(&jets[1])?;
    let start = map(curve_jets::endpoint(curve, span, domain, false)?)?;
    let end = map(curve_jets::endpoint(curve, span, domain, true)?)?;
    let chord = box_distance(&start[0], &end[0])?.0;
    let mut lo = speed.lo.max(chord);
    let mut hi = speed.hi;
    if speed.lo > 0. {
        // |(|C'|)''| <= |C'''| + |C''|^2 / min |C'|.
        // For the local [0,1] parameter, trapezoid error <= max |f''|/12.
        let acceleration = norm(&jets[2])?;
        let third = norm(&jets[3])?;
        let mut remainder = third
            .add(
                acceleration
                    .mul(acceleration)?
                    .div(Interval::point(speed.lo))?,
            )?
            .div(Interval::point(12.))?
            .hi;
        // Enclose the signed speed second derivative as well. The previous
        // triangle bound discards cancellation between v·j and |a|².
        // Both bounds concern the same original local jets; intersect their
        // valid remainder magnitudes, never sampled derivative estimates.
        let tight=(|| -> Result<Interval> {
            let dot=|a:&[Interval],b:&[Interval]| -> Result<Interval> {
                a.iter().zip(b).try_fold(Interval::point(0.),|sum,(a,b)|sum.add(a.mul(*b)?))
            };
            let va=dot(&jets[1],&jets[2])?;
            let speed3=speed.mul(speed)?.mul(speed)?;
            let second=acceleration.mul(acceleration)?.add(dot(&jets[1],&jets[3])?)?.div(speed)?
                .sub(va.mul(va)?.div(speed3)?)?;
            second.div(Interval::point(12.))
        })();
        if let Ok(tight)=tight {
            remainder=remainder.min(tight.lo.abs().max(tight.hi.abs()));
        }
        let trapezoid = norm(&start[1])?
            .add(norm(&end[1])?)?
            .mul(Interval::point(0.5))?;
        // Peano's trapezoid kernel is nonnegative and has integral1/12.
        // Hence T - integral(f) lies in the signed second-jet interval/12.
        // Keeping its sign further encloses cancellation without new samples.
        if let Ok(tight)=tight {
            if let Ok(signed)=trapezoid.sub(tight) {
                lo=lo.max(signed.lo.max(0.));hi=hi.min(signed.hi);
            }
        }
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
    measure_with_cells(curve, domain, tolerance, max_cells, cell)
}

fn measure_with_cells(
    curve: &Curve, domain: [f64;2], tolerance: f64, max_cells: usize,
    make_cell: impl Fn(&Curve,usize,[f64;2],usize) -> Result<Cell>,
) -> Result<(LengthReport,Vec<Cell>)> {
    if domain[0] == domain[1] {
        return Ok((report(Interval::point(0.), 0, tolerance, StopReason::Tolerance)?,Vec::new()));
    }
    let span_cap = curve.control_points.len().saturating_sub(curve.degree);
    let mut heap = BinaryHeap::with_capacity(span_cap.max(8));
    let mut cells = 0;
    for span in active_spans(curve, domain) {
        let a = curve.knots[span].max(domain[0]);
        let b = curve.knots[span + 1].min(domain[1]);
        if a < b {
            if cells == max_cells {
                return Err(resource("Initial length spans exceed the work budget"));
            }
            heap.push(make_cell(curve, span, [a, b], cells)?);
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
        let left = make_cell(curve, largest.span, [a, mid], cells)?;
        let right = make_cell(curve, largest.span, [mid, b], cells + 1)?;
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
pub(crate) fn length_on_interval(
    curve: &Curve,
    interval: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    let [start, end] = curve.domain();
    let [lo, hi] = interval;
    check(
        start <= lo && lo < hi && hi <= end,
        "Length interval must lie inside the curve domain",
    )?;
    measure(curve, interval, tolerance, max_cells)
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
    let shared_partition = division_partition::get(curve, tolerance / 8., max_cells)?;
    let (total,partition) = shared_partition.as_ref();
    let mut cells = total.cells;
    let mut points = Vec::with_capacity(segments + 1);
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
            invert_with_prefix_cache(curve,target,total,tolerance,max_cells-cells,&mut prefix_cache,partition)?
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
        total: total.clone(),
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

#[cfg(test)]
mod signed_speed_remainder_tests {
 use super::*;
 #[test]
 fn closed_spatial_original_length_meets_same_tolerance_without_extra_budget(){
  let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,1.],vec![-3.,3.,1.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],None).unwrap();
  let measured=length(&path,1e-6/8.,10000).unwrap();
  assert!(measured.within_tolerance,"{measured:?}");assert!(measured.cells<=10000);
  let velocity=path.control_points.windows(2).map(|p|std::array::from_fn::<_,3,_>(|k|7.*(p[1][k]-p[0][k]))).collect::<Vec<_>>();
  let speed=|u:f64|{let mut q=velocity.clone();for size in (1..q.len()).rev(){for i in 0..size{q[i]=std::array::from_fn(|k|(1.-u)*q[i][k]+u*q[i+1][k]);}}q[0].iter().map(|v|v*v).sum::<f64>().sqrt()};
  let n=8192;let mut independent=0.;for i in 0..n{let a=i as f64/n as f64;let b=(i+1) as f64/n as f64;independent+=(b-a)*(speed(a)+4.*speed((a+b)/2.)+speed(b))/6.;}
  assert!(measured.bounds[0]<=independent&&independent<=measured.bounds[1]);
  assert!(!length(&path,1e-6/8.,measured.cells-1).unwrap().within_tolerance);
  eprintln!("closed original length cells={} bounds={:?} independent={independent}",measured.cells,measured.bounds);
 }
 #[test]
 fn signed_remainder_retains_rational_line_and_conic_length_enclosures(){
  let line=Curve{degree:1,knots:vec![17.,17.,29.,29.],control_points:vec![vec![0.,0.],vec![3.,4.]],weights:vec![1.,2.],periodic:false};
  let saved=line.clone();let measured=length(&line,1e-7,10000).unwrap();
  assert!(measured.within_tolerance);assert!(measured.bounds[0]<=5.&&5.<=measured.bounds[1]);assert_eq!(line,saved);
  let circle=crate::primitives::circle([1.,2.,3.],[0.,0.,1.],2.).unwrap();
  let measured=length(&circle,1e-7,10000).unwrap();let analytic=4.*std::f64::consts::PI;
  assert!(measured.within_tolerance,"{measured:?}");assert!(measured.bounds[0]<=analytic&&analytic<=measured.bounds[1]);
 }

}


pub(super) fn length_under_plane_metric(
    curve: &Curve, slopes: [Interval; 2], tolerance: f64, max_cells: usize,
) -> Result<LengthReport> {
    validate(tolerance, max_cells)?;
    let mapped = |curve: &Curve, span, domain, serial| {
        mapped_cell(curve, span, domain, serial, |mut jets| {
            for vector in &mut jets {
                let height = slopes[0].mul(vector[0])?.add(slopes[1].mul(vector[1])?)?;
                vector.push(height);
            }
            Ok(jets)
        })
    };
    Ok(measure_with_cells(curve,curve.domain(),tolerance,max_cells,mapped)?.0)
}

/// Enclose total travel along a fixed direction, including every reversal.
/// The direction is normalized, so bounds and tolerance use model units.
pub fn directional_travel(
    curve: &Curve,
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    let unit = travel_direction(curve, direction)?;
    measure_directional_travel(curve, curve.domain(), &unit, tolerance, max_cells)
}

#[derive(Clone, Debug)]
pub struct DirectionalTravelReport {
    pub total: LengthReport,
    pub forward_bounds: [f64; 2],
    pub reverse_bounds: [f64; 2],
    /// Integral of signed axial velocity over the active spans.
    pub signed_travel_bounds: [f64; 2],
}

/// Separate positive and negative travel along the supplied direction.
/// Each knot span is measured on its own, so jumps do not add travel.
pub fn directional_travel_by_sign(
    curve: &Curve,
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalTravelReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    let unit = travel_direction(curve, direction)?;
    measure_signed_directional_travel(curve, curve.domain(), &unit, tolerance, max_cells)
}

/// Enclose the positive and negative axial travel on a finite, increasing
/// interval of the original knot domain.
pub fn directional_travel_on_interval(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalTravelReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    validate_travel_interval(curve, interval)?;
    let unit = travel_direction(curve, direction)?;
    measure_signed_directional_travel(curve, interval, &unit, tolerance, max_cells)
}

/// Measure an ordered batch under one cell limit. Overlapping windows are
/// measured independently. Each window is guaranteed initial span coverage
/// before remaining work is divided among the windows.
pub fn directional_travel_on_intervals(
    curve: &Curve,
    intervals: &[[f64; 2]],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<Vec<DirectionalTravelReport>> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    check(
        !intervals.is_empty() && intervals.len() <= max_cells,
        "Directional travel batch requires at least one cell per window",
    )?;
    let unit = travel_direction(curve, direction)?;
    let mut minima = Vec::with_capacity(intervals.len());
    let mut reserve = 0;
    for &interval in intervals {
        validate_travel_interval(curve, interval)?;
        let minimum = active_spans(curve, interval).count();
        reserve += minimum;
        if reserve > max_cells {
            return Err(resource("Initial directional travel spans exceed the shared budget"));
        }
        minima.push(minimum);
    }
    let mut reports = Vec::with_capacity(intervals.len());
    let mut remaining = max_cells;
    for (index, (&interval, minimum)) in intervals.iter().zip(minima).enumerate() {
        let budget = minimum + (remaining - reserve) / (intervals.len() - index);
        let report = measure_signed_directional_travel(curve, interval, &unit, tolerance, budget)?;
        reserve -= minimum;
        remaining -= report.total.cells;
        reports.push(report);
    }
    Ok(reports)
}

fn validate_travel_interval(curve: &Curve, interval: [f64; 2]) -> Result<()> {
    let [start, end] = curve.domain();
    let [lo, hi] = interval;
    check(
        start <= lo && lo < hi && hi <= end,
        "Directional travel interval must lie inside the curve domain",
    )
}

fn active_spans(curve: &Curve, [lo, hi]: [f64; 2]) -> impl Iterator<Item = usize> + '_ {
    let degree = curve.degree;
    let count = curve.control_points.len();
    let first = degree + curve.knots[degree + 1..=count].partition_point(|&knot| knot <= lo);
    let last = degree + curve.knots[degree..count].partition_point(|&knot| knot < hi);
    (first..last).filter(|&span| curve.knots[span] < curve.knots[span + 1])
}

fn measure_signed_directional_travel(
    curve: &Curve,
    interval: [f64; 2],
    unit: &[Interval],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalTravelReport> {
    let total = measure_directional_travel(curve, interval, unit, tolerance, max_cells)?;
    let mut signed = Interval::point(0.);
    for span in active_spans(curve, interval) {
        let domain = [
            curve.knots[span].max(interval[0]),
            curve.knots[span + 1].min(interval[1]),
        ];
        let left = curve_jets::endpoint(curve, span, domain, false)?;
        let right = curve_jets::endpoint(curve, span, domain, true)?;
        let delta = right[0]
            .iter()
            .zip(&left[0])
            .map(|(end, start)| end.sub(*start))
            .collect::<Result<Vec<_>>>()?;
        signed = signed.add(project_jet(&delta, unit)?)?;
    }
    let absolute = Interval::new(total.bounds[0], total.bounds[1])?;
    let split = |sum: Interval| -> Result<[f64; 2]> {
        let travel = sum.mul(Interval::point(0.5))?.intersect(0., absolute.hi)?;
        Ok([travel.lo, travel.hi])
    };
    Ok(DirectionalTravelReport {
        forward_bounds: split(absolute.add(signed)?)?,
        reverse_bounds: split(absolute.sub(signed)?)?,
        signed_travel_bounds: [signed.lo, signed.hi],
        total,
    })
}

fn travel_direction(curve: &Curve, direction: &[f64]) -> Result<Vec<Interval>> {
    check(
        direction.len() == curve.control_points[0].len()
            && direction.iter().all(|value| value.is_finite()),
        "Directional travel requires a finite vector matching the curve dimension",
    )?;
    let scale = direction.iter().map(|value| value.abs()).fold(0., f64::max);
    check(scale > 0., "Directional travel requires a nonzero vector")?;
    let coefficients = direction
        .iter()
        .map(|&value| {
            let ratio = value / scale;
            if value == 0. || value.abs() == scale {
                Ok(Interval::point(ratio))
            } else {
                Interval::new(ratio.next_down(), ratio.next_up())
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let length = norm(&coefficients)?;
    coefficients
        .into_iter()
        .map(|coefficient| coefficient.div(length))
        .collect()
}

fn project_jet(jet: &[Interval], unit: &[Interval]) -> Result<Interval> {
    jet.iter()
        .zip(unit)
        .filter(|(_, coefficient)| coefficient.lo != 0. || coefficient.hi != 0.)
        .try_fold(Interval::point(0.), |sum, (&value, &coefficient)| {
            sum.add(value.mul(coefficient)?)
        })
}

fn measure_directional_travel(
    curve: &Curve,
    interval: [f64; 2],
    unit: &[Interval],
    tolerance: f64,
    max_cells: usize,
) -> Result<LengthReport> {
    let project = |jets: Vec<Vec<Interval>>| -> Result<Vec<Vec<Interval>>> {
        jets.into_iter()
            .map(|jet| Ok(vec![project_jet(&jet, unit)?]))
            .collect()
    };
    let mapped = |curve: &Curve, span, domain, serial| {
        mapped_cell(curve, span, domain, serial, project)
    };
    Ok(measure_with_cells(curve, interval, tolerance, max_cells, mapped)?.0)
}

/// Classification of reverse travel against an inclusive upper limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReverseTravelStatus {
    Within,
    Exceeds,
    Undetermined,
}

impl DirectionalTravelReport {
    /// Compare the certified reverse travel enclosure against an inclusive limit.
    pub fn reverse_status(&self, limit: f64) -> Result<ReverseTravelStatus> {
        check(
            limit.is_finite() && limit >= 0.,
            "Reverse travel limit must be finite and nonnegative",
        )?;
        let [lo, hi] = self.reverse_bounds;
        Ok(if hi <= limit {
            ReverseTravelStatus::Within
        } else if lo > limit {
            ReverseTravelStatus::Exceeds
        } else {
            ReverseTravelStatus::Undetermined
        })
    }
}

/// Check reverse travel on an original-parameter interval. Classification
/// remains valid on budget exhaustion if the bounds separate from the limit.
pub fn check_directional_backtracking(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    reverse_limit: f64,
    measurement_tolerance: f64,
    max_cells: usize,
) -> Result<(DirectionalTravelReport, ReverseTravelStatus)> {
    check(
        reverse_limit.is_finite() && reverse_limit >= 0.,
        "Reverse travel limit must be finite and nonnegative",
    )?;
    let travel = directional_travel_on_interval(
        curve,
        interval,
        direction,
        measurement_tolerance,
        max_cells,
    )?;
    let status = travel.reverse_status(reverse_limit)?;
    Ok((travel, status))
}

/// Bounds on the supremum of absolute axial speed in model units per
/// original parameter unit, including one-sided speeds at knots.
#[derive(Clone, Debug)]
pub struct DirectionalPeakSpeedReport {
    pub bounds: [f64; 2],
    pub cells: usize,
    pub stop_reason: StopReason,
}

/// Bounds on the supremum of absolute axial acceleration in model units
/// per squared original parameter unit.
#[derive(Clone, Debug)]
pub struct DirectionalPeakAccelerationReport {
    pub bounds: [f64; 2],
    pub cells: usize,
    pub stop_reason: StopReason,
}

/// Certify the peak axial second derivative over an original-parameter window.
/// The result covers span derivatives, including their one-sided limits.
pub fn directional_peak_acceleration(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakAccelerationReport> {
    let report = measure_directional_derivative_peak(
        curve, interval, direction, tolerance, max_cells, None, 2,
    )?.peak;
    Ok(DirectionalPeakAccelerationReport {
        bounds: report.bounds,
        cells: report.cells,
        stop_reason: report.stop_reason,
    })
}

/// Bounds on the supremum of absolute axial jerk in model units
/// per cubed original parameter unit.
#[derive(Clone, Debug)]
pub struct DirectionalPeakJerkReport {
    pub bounds: [f64; 2],
    pub cells: usize,
    pub stop_reason: StopReason,
}

/// Certify the peak axial third derivative over an original-parameter window.
/// Only the span derivatives and their one-sided limits are included.
pub fn directional_peak_jerk(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakJerkReport> {
    let report = measure_directional_derivative_peak(
        curve, interval, direction, tolerance, max_cells, None, 3,
    )?
    .peak;
    Ok(DirectionalPeakJerkReport {
        bounds: report.bounds,
        cells: report.cells,
        stop_reason: report.stop_reason,
    })
}

/// Certify peak axial speed over an original-parameter window. The tolerance
/// limits the width of the reported enclosure.
pub fn directional_peak_speed(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakSpeedReport> {
    Ok(measure_directional_derivative_peak(curve, interval, direction, tolerance, max_cells, None, 1)?.peak)
}

#[derive(Clone, Debug)]
pub struct DirectionalPeakLocations<Peak = DirectionalPeakSpeedReport> {
    pub peak: Peak,
    /// Sorted, merged source intervals covering every possible maximizer.
    /// Width depends on peak separation, measurement tolerance and cell budget.
    pub parameter_intervals: Vec<[f64; 2]>,
}

/// Enclose all locations that can attain the peak absolute axial speed.
/// A knot is included when either one-sided derivative can attain the peak.
pub fn locate_directional_peak_speed(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakLocations> {
    measure_directional_derivative_peak(
        curve, interval, direction, tolerance, max_cells, Some(parameter_tolerance), 1,
    )
}

/// Enclose all locations that can attain the peak absolute axial acceleration.
/// A knot is included when either one-sided derivative can attain the peak.
pub fn locate_directional_peak_acceleration(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakLocations<DirectionalPeakAccelerationReport>> {
    let report = measure_directional_derivative_peak(
        curve, interval, direction, tolerance, max_cells, Some(parameter_tolerance), 2,
    )?;
    Ok(DirectionalPeakLocations {
        peak: DirectionalPeakAccelerationReport {
            bounds: report.peak.bounds,
            cells: report.peak.cells,
            stop_reason: report.peak.stop_reason,
        },
        parameter_intervals: report.parameter_intervals,
    })
}

/// Enclose all locations that can attain the peak absolute axial jerk.
/// A knot is included when either one-sided derivative can attain the peak.
pub fn locate_directional_peak_jerk(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakLocations<DirectionalPeakJerkReport>> {
    let report = measure_directional_derivative_peak(
        curve, interval, direction, tolerance, max_cells, Some(parameter_tolerance), 3,
    )?;
    Ok(DirectionalPeakLocations {
        peak: DirectionalPeakJerkReport {
            bounds: report.peak.bounds,
            cells: report.peak.cells,
            stop_reason: report.peak.stop_reason,
        },
        parameter_intervals: report.parameter_intervals,
    })
}

/// Inclusive bounds on absolute derivatives in the original parameter clock.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalMotionLimits {
    pub speed: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionalMotionStatus {
    Within,
    Exceeds,
    Undetermined,
}

#[derive(Clone, Debug)]
pub struct DirectionalMotionReport {
    pub speed: DirectionalPeakSpeedReport,
    pub acceleration: DirectionalPeakAccelerationReport,
    pub jerk: DirectionalPeakJerkReport,
    pub status: DirectionalMotionStatus,
}

/// Check all three inclusive limits, using a separate tolerance and cell
/// budget per derivative. Tolerances use the units of the corresponding limit.
pub fn check_directional_motion_limits(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
    tolerances: [f64; 3],
    max_cells_per_derivative: usize,
) -> Result<DirectionalMotionReport> {
    let limits = [limits.speed, limits.acceleration, limits.jerk];
    check(
        limits.iter().all(|limit| limit.is_finite() && *limit >= 0.),
        "Motion limits must be finite and nonnegative",
    )?;
    for tolerance in tolerances {
        validate(tolerance, max_cells_per_derivative)?;
    }
    let [speed_tol, acceleration_tol, jerk_tol] = tolerances;
    let speed =
        directional_peak_speed(curve, interval, direction, speed_tol, max_cells_per_derivative)?;
    let acceleration = directional_peak_acceleration(
        curve, interval, direction, acceleration_tol, max_cells_per_derivative,
    )?;
    let jerk =
        directional_peak_jerk(curve, interval, direction, jerk_tol, max_cells_per_derivative)?;
    let bounds = [speed.bounds, acceleration.bounds, jerk.bounds];
    let status = if bounds.iter().zip(limits).any(|([lo, _], limit)| *lo > limit) {
        DirectionalMotionStatus::Exceeds
    } else if bounds.iter().zip(limits).all(|([_, hi], limit)| *hi <= limit) {
        DirectionalMotionStatus::Within
    } else {
        DirectionalMotionStatus::Undetermined
    };
    Ok(DirectionalMotionReport { speed, acceleration, jerk, status })
}

/// Enclose the minimum uniform parameter dilation, constrained to at least 1,
/// that satisfies the inclusive axis limits. A factor of 2 halves speed,
/// quarters acceleration and divides jerk by 8. A finite upper endpoint is
/// a certified feasible factor. Infinity means no finite factor was certified.
/// The original parameter must represent elapsed time for physical units.
pub fn directional_motion_slowdown_bounds(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
    tolerances: [f64; 3],
    max_cells_per_derivative: usize,
) -> Result<[f64; 2]> {
    let report = check_directional_motion_limits(
        curve, interval, direction, limits, tolerances, max_cells_per_derivative,
    )?;
    slowdown_from_motion_report(&report, limits)
}

fn slowdown_from_motion_report(
    report: &DirectionalMotionReport,
    limits: DirectionalMotionLimits,
) -> Result<[f64; 2]> {
    use num_rational::BigRational;

    let exact = |value| {
        BigRational::from_float(value)
            .ok_or_else(|| crate::numeric_err("Nonfinite motion bound"))
    };
    let constraints = [
        (report.speed.bounds, limits.speed),
        (report.acceleration.bounds, limits.acceleration),
        (report.jerk.bounds, limits.jerk),
    ]
    .into_iter()
    .map(|([lo, hi], limit)| Ok(([exact(lo)?, exact(hi)?], exact(limit)?)))
    .collect::<Result<Vec<_>>>()?;

    let bracket = |endpoint| -> Result<[f64; 2]> {
        let feasible = |factor: f64| -> Result<bool> {
            let factor = exact(factor)?;
            let mut power = factor.clone();
            for (index, (peak, limit)) in constraints.iter().enumerate() {
                if &peak[endpoint] > &(&power * limit) {
                    return Ok(false);
                }
                if index < 2 {
                    power *= &factor;
                }
            }
            Ok(true)
        };
        if feasible(1.)? {
            return Ok([1.; 2]);
        }
        if !feasible(f64::MAX)? {
            return Ok([f64::MAX, f64::INFINITY]);
        }

        // Positive finite binary64 values have the same order as their bits.
        let mut lo = 1_f64.to_bits();
        let mut hi = f64::MAX.to_bits();
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if feasible(f64::from_bits(mid))? {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Ok([f64::from_bits(lo), f64::from_bits(hi)])
    };
    Ok([bracket(0)?[0], bracket(1)?[1]])
}

/// Limits for one direction, which is normalized before projecting the curve.
#[derive(Clone, Copy, Debug)]
pub struct MotionAxis<'a> {
    pub direction: &'a [f64],
    pub limits: DirectionalMotionLimits,
}

#[derive(Clone, Debug)]
pub struct MultiAxisSlowdownReport {
    /// Enclosure of the minimum common slowdown. A finite upper endpoint
    /// satisfies all axes simultaneously.
    pub bounds: [f64; 2],
    /// Individual slowdown enclosures in input order.
    pub axis_bounds: Vec<[f64; 2]>,
    pub cells: usize,
}

/// Compute one uniform parameter dilation under a shared cell budget.
/// Reserve span coverage for each axis and derivative before refinement.
pub fn multi_axis_motion_slowdown_bounds(
    curve: &Curve,
    interval: [f64; 2],
    axes: &[MotionAxis<'_>],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<MultiAxisSlowdownReport> {
    let mut axis_bounds = Vec::new();
    let mut bounds = [1_f64; 2];
    let cells = measure_motion_axes(curve, interval, axes, tolerances, max_cells, |axis, report| {
        let axis_bound = slowdown_from_motion_report(report, axis.limits)?;
        bounds[0] = bounds[0].max(axis_bound[0]);
        bounds[1] = bounds[1].max(axis_bound[1]);
        axis_bounds.push(axis_bound);
        Ok(())
    })?;
    Ok(MultiAxisSlowdownReport { bounds, axis_bounds, cells })
}

fn measure_motion_axes(
    curve: &Curve,
    interval: [f64; 2],
    axes: &[MotionAxis<'_>],
    tolerances: [f64; 3],
    max_cells: usize,
    mut visit: impl FnMut(&MotionAxis<'_>, &DirectionalMotionReport) -> Result<()>,
) -> Result<usize> {
    curve.validate()?;
    for tolerance in tolerances {
        validate(tolerance, max_cells)?;
    }
    validate_travel_interval(curve, interval)?;
    check(!axes.is_empty(), "Motion analysis requires at least one axis")?;
    let minimum = 3 * active_spans(curve, interval).count();
    if axes.len() > max_cells / minimum {
        return Err(resource("Initial motion spans exceed the shared budget"));
    }

    let mut remaining = max_cells;
    for (index, axis) in axes.iter().enumerate() {
        let budget = remaining / (axes.len() - index) / 3;
        let report = check_directional_motion_limits(
            curve, interval, axis.direction, axis.limits, tolerances, budget,
        )?;
        remaining -= report.speed.cells + report.acceleration.cells + report.jerk.cells;
        visit(axis, &report)?;
    }
    Ok(max_cells - remaining)
}

#[derive(Clone, Debug)]
pub struct MultiAxisMotionDurationReport {
    pub slowdown: MultiAxisSlowdownReport,
    /// Enclosure of the minimum duration. A finite upper endpoint is
    /// sufficient when traversing the window with a uniform clock.
    pub duration_bounds: [f64; 2],
}

/// Compute the feasible uniform traversal duration for a source time window.
/// Duration uses the same time units as the original curve parameter.
pub fn multi_axis_motion_duration_bounds(
    curve: &Curve,
    time_window: [f64; 2],
    axes: &[MotionAxis<'_>],
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<MultiAxisMotionDurationReport> {
    let slowdown =
        multi_axis_motion_slowdown_bounds(curve, time_window, axes, tolerances, max_cells)?;
    let width = exact_motion_operand(time_window[1])? - exact_motion_operand(time_window[0])?;
    let duration_endpoint = |factor: f64, upper: bool| -> Result<f64> {
        if factor == f64::INFINITY {
            return Ok(f64::INFINITY);
        }
        round_motion_duration(&(&width * exact_motion_operand(factor)?), upper)
    };
    let duration_bounds = [
        duration_endpoint(slowdown.bounds[0], false)?,
        duration_endpoint(slowdown.bounds[1], true)?,
    ];
    Ok(MultiAxisMotionDurationReport { slowdown, duration_bounds })
}

fn round_motion_duration(duration: &num_rational::BigRational, upper: bool) -> Result<f64> {
    use num_traits::ToPrimitive;

    let rounded = duration
        .to_f64()
        .ok_or_else(|| crate::numeric_err("Cannot represent duration endpoint"))?;
    if rounded == f64::INFINITY {
        return Ok(if upper { f64::INFINITY } else { f64::MAX });
    }
    if rounded == f64::NEG_INFINITY {
        return Ok(if upper { -f64::MAX } else { f64::NEG_INFINITY });
    }
    let represented = exact_motion_operand(rounded)?;
    Ok(if upper && &represented < duration {
        rounded.next_up()
    } else if !upper && &represented > duration {
        rounded.next_down()
    } else {
        rounded
    })
}

#[derive(Clone, Debug)]
pub struct MultiAxisMotionDeadlineReport {
    pub status: DirectionalMotionStatus,
    /// Results for the axis limits in input order. The original-duration floor
    /// applies to the overall status.
    pub axis_statuses: Vec<DirectionalMotionStatus>,
    pub cells: usize,
}

/// Check an inclusive deadline for uniform traversal with slowdown only.
/// Compare derivative bounds in the proposed clock using exact rational
/// arithmetic, even when the duration ratio is not representable as binary64.
pub fn check_multi_axis_motion_deadline(
    curve: &Curve,
    time_window: [f64; 2],
    axes: &[MotionAxis<'_>],
    deadline: f64,
    tolerances: [f64; 3],
    max_cells: usize,
) -> Result<MultiAxisMotionDeadlineReport> {
    use num_rational::BigRational;

    check(deadline.is_finite() && deadline > 0., "Deadline must be positive and finite")?;
    curve.validate()?;
    validate_travel_interval(curve, time_window)?;
    let factor = motion_clock_factor(time_window, deadline)?;
    let powers = [factor.clone(), &factor * &factor, factor.pow(3)];
    let mut axis_statuses = Vec::new();
    let cells = measure_motion_axes(curve, time_window, axes, tolerances, max_cells, |axis, report| {
        axis_statuses.push(motion_status_in_clock(report, axis.limits, &powers)?);
        Ok(())
    })?;
    let status = if factor < BigRational::from_integer(1.into())
        || axis_statuses.contains(&DirectionalMotionStatus::Exceeds)
    {
        DirectionalMotionStatus::Exceeds
    } else if axis_statuses.contains(&DirectionalMotionStatus::Undetermined) {
        DirectionalMotionStatus::Undetermined
    } else {
        DirectionalMotionStatus::Within
    };
    Ok(MultiAxisMotionDeadlineReport { status, axis_statuses, cells })
}

fn motion_clock_factor(
    interval: [f64; 2],
    duration: f64,
) -> Result<num_rational::BigRational> {
    Ok(exact_motion_operand(duration)?
        / (exact_motion_operand(interval[1])? - exact_motion_operand(interval[0])?))
}

fn exact_motion_operand(value: f64) -> Result<num_rational::BigRational> {
    num_rational::BigRational::from_float(value)
        .ok_or_else(|| crate::numeric_err("Nonfinite motion operand"))
}

fn motion_status_in_clock(
    report: &DirectionalMotionReport,
    limits: DirectionalMotionLimits,
    powers: &[num_rational::BigRational; 3],
) -> Result<DirectionalMotionStatus> {
    let mut status = DirectionalMotionStatus::Within;
    for (([lo, hi], limit), power) in [
        (report.speed.bounds, limits.speed),
        (report.acceleration.bounds, limits.acceleration),
        (report.jerk.bounds, limits.jerk),
    ]
    .into_iter()
    .zip(powers)
    {
        let allowed = exact_motion_operand(limit)? * power;
        if exact_motion_operand(lo)? > allowed {
            return Ok(DirectionalMotionStatus::Exceeds);
        }
        if exact_motion_operand(hi)? > allowed {
            status = DirectionalMotionStatus::Undetermined;
        }
    }
    Ok(status)
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalSpeedLimitCell {
    pub parameter_interval: [f64; 2],
    /// Original knot span. Values at either endpoint use this span's limit.
    pub span: usize,
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct DirectionalSpeedLimitPartition {
    /// Ordered closed cells covering the full requested window.
    pub cells: Vec<DirectionalSpeedLimitCell>,
    /// Number of derivative enclosures, including replaced cells.
    pub work_cells: usize,
}

/// Classify where the absolute axial speed is within or above an inclusive
/// limit. Unresolved cells are retained when the requested parameter width,
/// floating-point resolution or the shared work budget prevents subdivision.
pub fn partition_directional_speed_limit(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limit: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    let (cells, work_cells) = partition_axial_derivatives(
        curve, interval, direction, [limit], parameter_tolerance, max_cells,
    )?;
    let cells = cells
        .into_iter()
        .map(|cell| DirectionalSpeedLimitCell {
            parameter_interval: cell.parameter_interval,
            span: cell.span,
            status: cell.statuses[0],
        })
        .collect();
    Ok(DirectionalSpeedLimitPartition { cells, work_cells })
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalMotionLimitCell<const N: usize = 3> {
    pub parameter_interval: [f64; 2],
    /// Original knot span for one-sided evaluation.
    pub span: usize,
    /// Classifications in derivative order (speed, acceleration, jerk).
    pub statuses: [DirectionalMotionStatus; N],
}

#[derive(Clone, Debug)]
pub struct DirectionalMotionLimitPartition {
    pub cells: Vec<DirectionalMotionLimitCell>,
    /// Number of enclosed cells, including replaced cells.
    pub work_cells: usize,
}

/// Classify speed, acceleration and jerk using one shared subdivision.
/// Each classification pertains to the absolute derivative with respect
/// to the original parameter. Unresolved cells retain their other results.
pub fn partition_directional_motion_limits(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalMotionLimitPartition> {
    let (cells, work_cells) = partition_axial_derivatives(
        curve,
        interval,
        direction,
        [limits.speed, limits.acceleration, limits.jerk],
        parameter_tolerance,
        max_cells,
    )?;
    Ok(DirectionalMotionLimitPartition { cells, work_cells })
}

fn partition_axial_derivatives<const N: usize>(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: [f64; N],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<(Vec<DirectionalMotionLimitCell<N>>, usize)> {
    validate_motion_partition_limits(limits)?;
    curve.validate()?;
    let unit = travel_direction(curve, direction)?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, width| {
            let statuses = classify_motion_cell(jets, width, &unit, limits)?;
            Ok((statuses, statuses.contains(&DirectionalMotionStatus::Undetermined)))
        },
    )?;
    let cells = cells
        .into_iter()
        .map(|(span, parameter_interval, statuses)| DirectionalMotionLimitCell {
            span,
            parameter_interval,
            statuses,
        })
        .collect();
    Ok((cells, work_cells))
}

#[derive(Clone, Debug)]
pub struct MultiAxisMotionLimitCell {
    pub parameter_interval: [f64; 2],
    /// Original knot span for one-sided evaluation.
    pub span: usize,
    /// Speed, acceleration and jerk classifications for each axis in input order.
    pub axis_statuses: Vec<[DirectionalMotionStatus; 3]>,
}

#[derive(Clone, Debug)]
pub struct MultiAxisMotionLimitPartition {
    /// Ordered closed cells covering the full requested window.
    pub cells: Vec<MultiAxisMotionLimitCell>,
    /// Axis evaluations, including cells replaced during subdivision.
    pub work_axis_cells: usize,
}

/// Evaluate all axes on a shared subdivision. The budget counts each axis
/// checked in each cell. Each cell shares one enclosure of the curve derivatives.
pub fn partition_multi_axis_motion_limits(
    curve: &Curve,
    interval: [f64; 2],
    axes: &[MotionAxis<'_>],
    parameter_tolerance: f64,
    max_axis_cells: usize,
) -> Result<MultiAxisMotionLimitPartition> {
    validate(parameter_tolerance, max_axis_cells)?;
    check(!axes.is_empty(), "Motion partition requires at least one axis")?;
    let max_cells = max_axis_cells / axes.len();
    if max_cells == 0 {
        return Err(resource("Motion partition budget must cover every axis"));
    }
    curve.validate()?;
    let axes = axes.iter().map(|axis| {
        let limits = [axis.limits.speed, axis.limits.acceleration, axis.limits.jerk];
        validate_motion_partition_limits(limits)?;
        Ok((travel_direction(curve, axis.direction)?, limits))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, width| {
            let statuses = axes.iter()
                .map(|(unit, limits)| classify_motion_cell(jets, width, unit, *limits))
                .collect::<Result<Vec<_>>>()?;
            let refine = statuses.iter().flatten().any(|status| {
                *status == DirectionalMotionStatus::Undetermined
            });
            Ok((statuses, refine))
        },
    )?;
    let cells = cells.into_iter()
        .map(|(span, parameter_interval, axis_statuses)| MultiAxisMotionLimitCell {
            span,
            parameter_interval,
            axis_statuses,
        })
        .collect();
    Ok(MultiAxisMotionLimitPartition { cells, work_axis_cells: work_cells * axes.len() })
}

fn validate_motion_partition_limits<const N: usize>(limits: [f64; N]) -> Result<()> {
    check(
        limits.iter().all(|limit| limit.is_finite() && *limit >= 0.),
        "Motion limits must be finite and nonnegative",
    )
}

fn classify_motion_cell<const N: usize>(
    jets: &[Vec<Interval>],
    width: Interval,
    unit: &[Interval],
    limits: [f64; N],
) -> Result<[DirectionalMotionStatus; N]> {
    let mut statuses = [DirectionalMotionStatus::Undetermined; N];
    for (index, (status, limit)) in statuses.iter_mut().zip(limits).enumerate() {
        let mut derivative = project_jet(&jets[index + 1], unit)?;
        for _ in 0..=index {
            derivative = derivative.div(width)?;
        }
        *status = classify_absolute_limit(derivative, limit);
    }
    Ok(statuses)
}

fn classify_absolute_limit(value: Interval, limit: f64) -> DirectionalMotionStatus {
    if value.lo.abs().max(value.hi.abs()) <= limit {
        DirectionalMotionStatus::Within
    } else if value.lo.max(-value.hi) > limit {
        DirectionalMotionStatus::Exceeds
    } else {
        DirectionalMotionStatus::Undetermined
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalPositionCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    pub position_bounds: [f64; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct DirectionalPositionPartition {
    /// Ordered closed cells covering the requested parameter window.
    pub cells: Vec<DirectionalPositionCell>,
    pub work_cells: usize,
}

/// Classify the signed projection onto a normalized axis through the origin.
/// Cells outside the inclusive position range are marked Exceeds. Cells
/// overlapping a range boundary retain an unresolved status if refinement stops.
pub fn partition_directional_position_range(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    allowed: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPositionPartition> {
    validate_position_range(allowed)?;
    curve.validate()?;
    let unit = travel_direction(curve, direction)?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, _| {
            let (position, status) = classify_position(&jets[0], &unit, allowed)?;
            Ok(((position, status), status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (position, status))| {
        DirectionalPositionCell {
            span,
            parameter_interval,
            position_bounds: [position.lo, position.hi],
            status,
        }
    }).collect();
    Ok(DirectionalPositionPartition { cells, work_cells })
}

/// An inclusive range along a normalized direction through the origin.
#[derive(Clone, Copy, Debug)]
pub struct PositionAxis<'a> {
    pub direction: &'a [f64],
    pub allowed: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct AxisPositionBounds {
    pub bounds: [f64; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct MultiAxisPositionCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    /// Axis results in input order.
    pub axes: Vec<AxisPositionBounds>,
}

#[derive(Clone, Debug)]
pub struct MultiAxisPositionPartition {
    pub cells: Vec<MultiAxisPositionCell>,
    pub work_axis_cells: usize,
}

/// Check all signed position ranges on one shared subdivision. Each axis
/// evaluation contributes one unit to the work budget.
pub fn partition_multi_axis_position_ranges(
    curve: &Curve,
    interval: [f64; 2],
    axes: &[PositionAxis<'_>],
    parameter_tolerance: f64,
    max_axis_cells: usize,
) -> Result<MultiAxisPositionPartition> {
    validate(parameter_tolerance, max_axis_cells)?;
    check(!axes.is_empty(), "Position partition requires at least one axis")?;
    let max_cells = max_axis_cells / axes.len();
    if max_cells == 0 {
        return Err(resource("Position partition budget must cover every axis"));
    }
    curve.validate()?;
    let axes = axes.iter().map(|axis| {
        validate_position_range(axis.allowed)?;
        Ok((travel_direction(curve, axis.direction)?, axis.allowed))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, _| {
            let results = axes.iter().map(|(unit, allowed)| {
                let (position, status) = classify_position(&jets[0], unit, *allowed)?;
                Ok(AxisPositionBounds { bounds: [position.lo, position.hi], status })
            }).collect::<Result<Vec<_>>>()?;
            let refine = results.iter().any(|axis| axis.status == DirectionalMotionStatus::Undetermined);
            Ok((results, refine))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, axes)| {
        MultiAxisPositionCell { parameter_interval, span, axes }
    }).collect();
    Ok(MultiAxisPositionPartition { cells, work_axis_cells: work_cells * axes.len() })
}

/// Signed position range and absolute derivative limits for one axis.
#[derive(Clone, Copy, Debug)]
pub struct TrajectoryAxis<'a> {
    pub position: PositionAxis<'a>,
    pub motion: DirectionalMotionLimits,
}

#[derive(Clone, Copy, Debug)]
pub struct TrajectoryAxisResult {
    pub position: AxisPositionBounds,
    /// Speed, acceleration and jerk, measured in the original parameter clock.
    pub motion: [DirectionalMotionStatus; 3],
}

#[derive(Clone, Debug)]
pub struct TrajectoryLimitCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    pub axes: Vec<TrajectoryAxisResult>,
}

#[derive(Clone, Debug)]
pub struct TrajectoryLimitPartition {
    pub cells: Vec<TrajectoryLimitCell>,
    pub work_axis_cells: usize,
}

/// Classify position and derivative limits together. All axes share one
/// subdivision and one enclosure of the curve per cell.
pub fn partition_trajectory_limits(
    curve: &Curve,
    interval: [f64; 2],
    axes: &[TrajectoryAxis<'_>],
    parameter_tolerance: f64,
    max_axis_cells: usize,
) -> Result<TrajectoryLimitPartition> {
    validate(parameter_tolerance, max_axis_cells)?;
    check(!axes.is_empty(), "Trajectory partition requires at least one axis")?;
    let max_cells = max_axis_cells / axes.len();
    if max_cells == 0 {
        return Err(resource("Trajectory partition budget must cover every axis"));
    }
    curve.validate()?;
    let axes = axes.iter().map(|axis| {
        let allowed = axis.position.allowed;
        let limits = [axis.motion.speed, axis.motion.acceleration, axis.motion.jerk];
        validate_position_range(allowed)?;
        validate_motion_partition_limits(limits)?;
        Ok((travel_direction(curve, axis.position.direction)?, allowed, limits))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, width| {
            let results = axes.iter().map(|(unit, allowed, limits)| {
                let (position, status) = classify_position(&jets[0], unit, *allowed)?;
                let motion = classify_motion_cell(jets, width, unit, *limits)?;
                Ok(TrajectoryAxisResult {
                    position: AxisPositionBounds { bounds: [position.lo, position.hi], status },
                    motion,
                })
            }).collect::<Result<Vec<_>>>()?;
            let refine = results.iter().any(|axis| {
                axis.position.status == DirectionalMotionStatus::Undetermined
                    || axis.motion.contains(&DirectionalMotionStatus::Undetermined)
            });
            Ok((results, refine))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, axes)| {
        TrajectoryLimitCell { parameter_interval, span, axes }
    }).collect();
    Ok(TrajectoryLimitPartition { cells, work_axis_cells: work_cells * axes.len() })
}

fn validate_position_range(allowed: [f64; 2]) -> Result<()> {
    check(
        allowed.iter().all(|value| value.is_finite()) && allowed[0] <= allowed[1],
        "Position range must have finite ordered endpoints",
    )
}

fn classify_position(
    point: &[Interval],
    unit: &[Interval],
    allowed: [f64; 2],
) -> Result<(Interval, DirectionalMotionStatus)> {
    let position = project_jet(point, unit)?;
    let status = if position.lo >= allowed[0] && position.hi <= allowed[1] {
        DirectionalMotionStatus::Within
    } else if position.hi < allowed[0] || position.lo > allowed[1] {
        DirectionalMotionStatus::Exceeds
    } else {
        DirectionalMotionStatus::Undetermined
    };
    Ok((position, status))
}

/// A closed half-space defined by `normal · point <= offset`.
#[derive(Clone, Copy, Debug)]
pub struct WorkspaceHalfSpace<'a> {
    pub normal: &'a [f64],
    pub offset: f64,
}

#[derive(Clone, Debug)]
pub struct WorkspaceCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    /// Maximum plane residual over the tool at each center, in input order.
    /// A point tool has residual `normal · point - offset`.
    pub plane_residuals: Vec<[f64; 2]>,
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct WorkspacePartition {
    pub cells: Vec<WorkspaceCell>,
    pub work_plane_cells: usize,
}

/// Check a convex workspace defined by intersecting closed half-spaces.
/// A cell exceeds the workspace if a plane excludes the whole cell.
/// Each plane evaluation contributes one unit to the shared work budget.
pub fn partition_curve_convex_workspace(
    curve: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    partition_curve_ball_convex_workspace(
        curve, interval, planes, 0., parameter_tolerance, max_plane_cells,
    )
}

/// Test containment of a closed ball at every center along the curve.
/// The plane residual includes the support radius, `radius * |normal|`.
/// A positive residual throughout a cell proves the ball exceeds that plane.
pub fn partition_curve_ball_convex_workspace(
    curve: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    radius: f64,
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    check(radius.is_finite() && radius >= 0., "Tool radius must be finite and nonnegative")?;
    partition_curve_workspace_with_support(
        curve, interval, planes, parameter_tolerance, max_plane_cells,
        |normal| {
            if radius == 0. {
                Ok(None)
            } else {
                Ok(Some(norm(normal)?.mul(Interval::point(radius))?))
            }
        },
    )
}

/// A fixed-orientation capsule whose segment endpoints are offsets from
/// the curve point. A zero radius describes the segment itself.
#[derive(Clone, Copy, Debug)]
pub struct WorkspaceCapsule<'a> {
    pub endpoint_offsets: [&'a [f64]; 2],
    pub radius: f64,
}

/// Check a translating capsule against the workspace planes. At each curve
/// point the support is the greater endpoint projection plus the ball support.
pub fn partition_curve_capsule_convex_workspace(
    curve: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    capsule: WorkspaceCapsule<'_>,
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    partition_curve_rounded_hull_workspace(
        curve, interval, planes, &capsule.endpoint_offsets, capsule.radius,
        parameter_tolerance, max_plane_cells,
    )
}

/// Check a translating convex hull of vertex offsets, optionally rounded by
/// a ball of the specified radius. The profile keeps its orientation and
/// vertices may be unordered. At most one million vertex/plane pairs are
/// evaluated during support preprocessing.
pub fn partition_curve_rounded_hull_workspace(
    curve: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    radius: f64,
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    check(radius.is_finite() && radius >= 0., "Tool radius must be finite and nonnegative")?;
    check(!vertex_offsets.is_empty(), "Tool hull requires at least one vertex")?;
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    if vertex_offsets.len() > 1_000_000 / planes.len() {
        return Err(resource("Tool hull support preprocessing exceeds one million vertex/plane pairs"));
    }
    curve.validate()?;
    let dimension = curve.control_points[0].len();
    let mut offsets = Vec::with_capacity(vertex_offsets.len());
    for &offset in vertex_offsets {
        check(
            offset.len() == dimension && offset.iter().all(|value| value.is_finite()),
            "Tool hull vertices must be finite vectors matching the curve dimension",
        )?;
        offsets.push(offset.iter().copied().map(Interval::point).collect::<Vec<_>>());
    }
    partition_curve_workspace_with_support(
        curve, interval, planes, parameter_tolerance, max_plane_cells,
        |normal| {
            let mut lo = f64::NEG_INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for offset in &offsets {
                let projected = project_jet(offset, normal)?;
                lo = lo.max(projected.lo);
                hi = hi.max(projected.hi);
            }
            let support = Interval::new(lo, hi)?;
            if radius == 0. {
                Ok(Some(support))
            } else {
                Ok(Some(support.add(norm(normal)?.mul(Interval::point(radius))?)?))
            }
        },
    )
}

fn partition_curve_workspace_with_support(
    curve: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    parameter_tolerance: f64,
    max_plane_cells: usize,
    support: impl Fn(&[Interval]) -> Result<Option<Interval>>,
) -> Result<WorkspacePartition> {
    validate(parameter_tolerance, max_plane_cells)?;
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    let max_cells = max_plane_cells / planes.len();
    if max_cells == 0 {
        return Err(resource("Workspace budget must cover every plane"));
    }
    curve.validate()?;
    let dimension = curve.control_points[0].len();
    let planes = prepare_workspace_planes(planes, dimension)?.into_iter().map(|(normal, offset)| {
        let margin = support(&normal)?;
        Ok((normal, offset, margin))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, _| {
            let residuals = planes.iter().map(|(normal, offset, margin)| {
                let mut residual = project_jet(&jets[0], normal)?.sub(*offset)?;
                if let Some(margin) = margin {
                    residual = residual.add(*margin)?;
                }
                Ok([residual.lo, residual.hi])
            }).collect::<Result<Vec<_>>>()?;
            let status = if residuals.iter().any(|[lo, _]| *lo > 0.) {
                DirectionalMotionStatus::Exceeds
            } else if residuals.iter().all(|[_, hi]| *hi <= 0.) {
                DirectionalMotionStatus::Within
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok(((residuals, status), status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (plane_residuals, status))| {
        WorkspaceCell { parameter_interval, span, plane_residuals, status }
    }).collect();
    Ok(WorkspacePartition { cells, work_plane_cells: work_cells * planes.len() })
}

type PreparedWorkspacePlane = (Vec<Interval>, Interval);

fn prepare_workspace_planes(
    planes: &[WorkspaceHalfSpace<'_>],
    dimension: usize,
) -> Result<Vec<PreparedWorkspacePlane>> {
    planes.iter().map(|plane| {
        check(
            plane.normal.len() == dimension
                && plane.normal.iter().all(|x| x.is_finite())
                && plane.normal.iter().any(|x| *x != 0.)
                && plane.offset.is_finite(),
            "Workspace plane requires a finite nonzero normal and finite offset",
        )?;
        let normal = plane.normal.iter().copied().map(Interval::point).collect::<Vec<_>>();
        Ok((normal, Interval::point(plane.offset)))
    }).collect()
}

#[derive(Clone, Debug)]
pub struct ObstacleClearanceCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    /// Clearance classifications in obstacle order. Exceeds means that the
    /// whole cell is in the closed obstacle, including its boundary.
    pub obstacles: Vec<DirectionalMotionStatus>,
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct ObstacleClearancePartition {
    pub cells: Vec<ObstacleClearanceCell>,
    /// Reserved plane evaluations, including planes skipped after separation.
    /// For vertex hulls this counts vertex/plane pairs.
    pub work_plane_cells: usize,
}

/// Check a curve against the union of closed convex obstacles. A cell is
/// clear when at least one plane strictly separates it from each obstacle.
/// Cells whose clearance cannot be decided are retained in the partition.
pub fn partition_curve_obstacle_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_curve_ball_obstacle_clearance(
        curve, interval, obstacles, 0., parameter_tolerance, max_plane_cells,
    )
}

/// Certify clearance for a closed ball centered on the curve. A strictly
/// separating plane must leave room for the radius. A cell is classified
/// as colliding when every center in it lies in an obstacle. Other possible
/// contacts, including at edges and corners, may remain undetermined.
pub fn partition_curve_ball_obstacle_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    radius: f64,
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    check(radius.is_finite() && radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_plane_cells)?;
    check(!obstacles.is_empty(), "Obstacle clearance requires at least one obstacle")?;
    let mut plane_count = 0;
    for obstacle in obstacles {
        check(!obstacle.is_empty(), "Each obstacle requires at least one plane")?;
        if obstacle.len() > max_plane_cells - plane_count {
            return Err(resource("Clearance budget must cover every obstacle plane"));
        }
        plane_count += obstacle.len();
    }
    curve.validate()?;
    let dimension = curve.control_points[0].len();
    let obstacles = obstacles.iter().map(|planes| {
        prepare_workspace_planes(planes, dimension)?.into_iter().map(|(normal, offset)| {
            let margin = if radius == 0. {
                0.
            } else {
                norm(&normal)?.mul(Interval::point(radius))?.hi
            };
            Ok((normal, offset, margin))
        }).collect::<Result<Vec<_>>>()
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_plane_cells / plane_count,
        |jets, _| {
            let mut clearance = Vec::with_capacity(obstacles.len());
            for obstacle in &obstacles {
                let mut status = DirectionalMotionStatus::Exceeds;
                for (normal, offset, margin) in obstacle {
                    let residual = project_jet(&jets[0], normal)?.sub(*offset)?;
                    if residual.lo > *margin {
                        status = DirectionalMotionStatus::Within;
                        break;
                    }
                    if residual.hi > 0. {
                        status = DirectionalMotionStatus::Undetermined;
                    }
                }
                clearance.push(status);
            }
            let status = if clearance.contains(&DirectionalMotionStatus::Exceeds) {
                DirectionalMotionStatus::Exceeds
            } else if clearance.contains(&DirectionalMotionStatus::Undetermined) {
                DirectionalMotionStatus::Undetermined
            } else {
                DirectionalMotionStatus::Within
            };
            // Refine unresolved obstacles even when another already excludes the cell.
            let refine = clearance.contains(&DirectionalMotionStatus::Undetermined);
            Ok(((clearance, status), refine))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (obstacles, status))| {
        ObstacleClearanceCell { parameter_interval, span, obstacles, status }
    }).collect();
    Ok(ObstacleClearancePartition { cells, work_plane_cells: work_cells * plane_count })
}

/// Check clearance for a translating convex hull of offsets, rounded by a
/// ball of the specified radius. A separating plane certifies clearance.
/// A translated vertex inside all planes certifies collision. Other contact
/// configurations may remain undetermined. The work budget accounts for
/// every vertex/plane pair per enclosed cell.
pub fn partition_curve_rounded_hull_obstacle_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    radius: f64,
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    check(radius.is_finite() && radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_vertex_plane_cells)?;
    check(!obstacles.is_empty() && !vertex_offsets.is_empty(), "Clearance requires obstacles and hull vertices")?;
    let plane_budget = max_vertex_plane_cells / vertex_offsets.len();
    let mut plane_count = 0;
    for obstacle in obstacles {
        check(!obstacle.is_empty(), "Each obstacle requires at least one plane")?;
        if obstacle.len() > plane_budget - plane_count {
            return Err(resource("Clearance budget must cover every vertex and plane"));
        }
        plane_count += obstacle.len();
    }
    curve.validate()?;
    let dimension = curve.control_points[0].len();
    let vertices = vertex_offsets.iter().map(|vertex| {
        check(
            vertex.len() == dimension && vertex.iter().all(|value| value.is_finite()),
            "Tool hull vertices must be finite vectors matching the curve dimension",
        )?;
        Ok(vertex.iter().copied().map(Interval::point).collect::<Vec<_>>())
    }).collect::<Result<Vec<_>>>()?;
    let obstacles = obstacles.iter().map(|planes| {
        prepare_workspace_planes(planes, dimension)?.into_iter().map(|(normal, offset)| {
            let projected = vertices.iter().map(|vertex| project_jet(vertex, &normal))
                .collect::<Result<Vec<_>>>()?;
            let minimum = projected.iter().map(|value| value.lo).fold(f64::INFINITY, f64::min);
            let support = if radius == 0. {
                minimum
            } else {
                Interval::point(minimum).sub(norm(&normal)?.mul(Interval::point(radius))?)?.lo
            };
            Ok((normal, offset, projected, support))
        }).collect::<Result<Vec<_>>>()
    }).collect::<Result<Vec<_>>>()?;
    let pairs = plane_count * vertices.len();
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_vertex_plane_cells / pairs,
        |jets, _| {
            let mut clearance = Vec::with_capacity(obstacles.len());
            let mut candidates = vec![true; vertices.len()];
            for obstacle in &obstacles {
                candidates.fill(true);
                let mut separated = false;
                for (normal, offset, projected, minimum) in obstacle {
                    let residual = project_jet(&jets[0], normal)?.sub(*offset)?;
                    if residual.add(Interval::point(*minimum))?.lo > 0. {
                        separated = true;
                        break;
                    }
                    for (candidate, projection) in candidates.iter_mut().zip(projected) {
                        *candidate &= residual.add(*projection)?.hi <= 0.;
                    }
                }
                clearance.push(if separated {
                    DirectionalMotionStatus::Within
                } else if candidates.contains(&true) {
                    DirectionalMotionStatus::Exceeds
                } else {
                    DirectionalMotionStatus::Undetermined
                });
            }
            let refine = clearance.contains(&DirectionalMotionStatus::Undetermined);
            let status = if clearance.contains(&DirectionalMotionStatus::Exceeds) {
                DirectionalMotionStatus::Exceeds
            } else if refine {
                DirectionalMotionStatus::Undetermined
            } else {
                DirectionalMotionStatus::Within
            };
            Ok(((clearance, status), refine))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (obstacles, status))| {
        ObstacleClearanceCell { parameter_interval, span, obstacles, status }
    }).collect();
    Ok(ObstacleClearancePartition { cells, work_plane_cells: work_cells * pairs })
}

#[derive(Clone, Copy, Debug)]
pub struct SphericalObstacle<'a> {
    pub center: &'a [f64],
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub struct SphericalClearanceCell {
    pub parameter_interval: [f64; 2],
    pub span: usize,
    /// Signed surface gaps in obstacle order. A nonpositive gap is contact.
    pub gap_bounds: Vec<[f64; 2]>,
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct SphericalClearancePartition {
    pub cells: Vec<SphericalClearanceCell>,
    pub work_obstacle_cells: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FirstToolpathContact {
    Clear,
    Possible {
        earliest_parameter: f64,
    },
    Confirmed {
        parameter_bounds: [f64; 2],
        /// An obstacle proved to be in contact at the upper endpoint.
        obstacle_index: usize,
    },
}

#[derive(Clone, Debug)]
pub struct MinimumToolpathClearance {
    pub gap_bounds: [f64; 2],
    /// Ordered parameter intervals covering every possible minimizer.
    pub parameter_intervals: Vec<[f64; 2]>,
    /// A cell and obstacle whose gap upper bound establishes the report's upper bound.
    pub upper_witness: [usize; 2],
}

impl SphericalClearancePartition {
    /// Enclose the minimum signed gap over all cells and obstacles and
    /// localize its possible minimizers. Resolution follows the partition.
    pub fn minimum_clearance(&self) -> Option<MinimumToolpathClearance> {
        let mut lower = f64::INFINITY;
        let mut upper = f64::INFINITY;
        let mut witness = None;
        for (cell_index, cell) in self.cells.iter().enumerate() {
            for (obstacle_index, &[lo, hi]) in cell.gap_bounds.iter().enumerate() {
                lower = lower.min(lo);
                if hi < upper {
                    upper = hi;
                    witness = Some([cell_index, obstacle_index]);
                }
            }
        }
        let upper_witness = witness?;
        let mut parameter_intervals: Vec<[f64; 2]> = Vec::new();
        for cell in &self.cells {
            if cell.gap_bounds.iter().any(|[lo, _]| *lo <= upper) {
                let [start, end] = cell.parameter_interval;
                match parameter_intervals.last_mut() {
                    Some([_, previous_end]) if start <= *previous_end => {
                        *previous_end = previous_end.max(end);
                    }
                    _ => parameter_intervals.push([start, end]),
                }
            }
        }
        Some(MinimumToolpathClearance {
            gap_bounds: [lower, upper],
            parameter_intervals,
            upper_witness,
        })
    }

    /// Summarize the first contact in the original window. The lower bound
    /// comes from the first cell whose clearance was not proved. The upper
    /// bound is the start of the first cell proved in contact throughout.
    pub fn first_contact(&self) -> FirstToolpathContact {
        let mut earliest = None;
        for cell in &self.cells {
            let start = cell.parameter_interval[0];
            if cell.status != DirectionalMotionStatus::Within {
                let lower = *earliest.get_or_insert(start);
                if let Some(obstacle_index) = cell.gap_bounds.iter().position(|[_, hi]| *hi <= 0.) {
                    return FirstToolpathContact::Confirmed {
                        parameter_bounds: [lower, start],
                        obstacle_index,
                    };
                }
            }
        }
        match earliest {
            None => FirstToolpathContact::Clear,
            Some(earliest_parameter) => FirstToolpathContact::Possible { earliest_parameter },
        }
    }
}

/// Bound clearance for a ball swept along the curve against closed balls.
/// A cell is clear if all gap lower bounds are positive, and colliding if
/// one obstacle has a nonpositive gap upper bound throughout the cell.
pub fn partition_curve_spherical_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[SphericalObstacle<'_>],
    tool_radius: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Spherical clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every sphere"));
    }
    curve.validate()?;
    let obstacles = prepare_spherical_obstacles(curve, obstacles, tool_radius)?;
    partition_radial_clearance(
        curve, interval, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| {
            let (center, radius) = &obstacles[index];
            let (lo, hi) = box_distance(point, center)?;
            Interval::new(lo, hi)?.sub(*radius)
        },
    )
}

type PreparedSphere = (Vec<Interval>, Interval);

fn prepare_spherical_obstacles(
    curve: &Curve,
    obstacles: &[SphericalObstacle<'_>],
    tool_radius: f64,
) -> Result<Vec<PreparedSphere>> {
    let dimension = curve.control_points[0].len();
    obstacles.iter().map(|obstacle| {
        check(
            obstacle.center.len() == dimension
                && obstacle.center.iter().all(|value| value.is_finite())
                && obstacle.radius.is_finite()
                && obstacle.radius >= 0.,
            "Sphere requires a finite matching center and nonnegative finite radius",
        )?;
        let center = obstacle.center.iter().copied().map(Interval::point).collect::<Vec<_>>();
        let radius = Interval::point(obstacle.radius).add(Interval::point(tool_radius))?;
        Ok((center, radius))
    }).collect()
}

#[derive(Clone, Copy, Debug)]
pub struct SphericalMinimumClearance {
    pub gap_bounds: [f64; 2],
    /// Evaluated parameter and obstacle providing an upper bound.
    pub witness_parameter: f64,
    pub witness_obstacle: usize,
    pub work_obstacle_cells: usize,
    pub stop_reason: StopReason,
}

/// Refine the smallest gap bound first until the signed minimum is enclosed
/// to the requested tolerance or refinement reaches a work or precision limit.
pub fn measure_minimum_spherical_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[SphericalObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Spherical clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_spherical_obstacles(curve, obstacles, tool_radius)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index| {
            let (center, radius) = &obstacles[index];
            let (lo, hi) = box_distance(point, center)?;
            Interval::new(lo, hi)?.sub(*radius)
        },
    )
}

fn measure_minimum_radial_clearance(
    curve: &Curve,
    interval: [f64; 2],
    tolerance: f64,
    max_cells: usize,
    obstacle_count: usize,
    gap: impl Fn(&[Interval], usize) -> Result<Interval>,
) -> Result<SphericalMinimumClearance> {
    let measure = |span, domain: [f64; 2], witness: &mut (f64, f64, usize)| -> Result<f64> {
        let jets = curve_jets::enclose(curve, span, domain)?;
        let start = curve_jets::endpoint(curve, span, domain, false)?;
        let end = curve_jets::endpoint(curve, span, domain, true)?;
        let mut lower = f64::INFINITY;
        for index in 0..obstacle_count {
            lower = lower.min(gap(&jets[0], index)?.lo);
            for (point, parameter) in [(&start[0], domain[0]), (&end[0], domain[1])] {
                let bound = gap(point, index)?.hi;
                if bound < witness.0 {
                    *witness = (bound, parameter, index);
                }
            }
        }
        Ok(lower)
    };
    let initial = active_spans(curve, interval).map(|span| {
        (span, [curve.knots[span].max(interval[0]), curve.knots[span + 1].min(interval[1])])
    });
    let (lower, witness, cells, stop_reason) =
        refine_minimum_clearance(initial, interval[0], tolerance, max_cells, measure)?;
    Ok(SphericalMinimumClearance {
        gap_bounds: [lower, witness.0],
        witness_parameter: witness.1,
        witness_obstacle: witness.2,
        work_obstacle_cells: cells * obstacle_count,
        stop_reason,
    })
}

type ClearanceWitness = (f64, f64, usize);

fn refine_minimum_clearance<Span: Copy>(
    initial: impl IntoIterator<Item = (Span, [f64; 2])>,
    start: f64,
    tolerance: f64,
    max_cells: usize,
    measure: impl Fn(Span, [f64; 2], &mut ClearanceWitness) -> Result<f64>,
) -> Result<(f64, ClearanceWitness, usize, StopReason)> {
    let mut pending = BTreeMap::new();
    let mut cells = 0;
    let mut witness = (f64::INFINITY, start, 0);
    // Map finite signed floats to unsigned keys preserving ascending order.
    let order = |value: f64| {
        let bits = value.to_bits();
        if value.is_sign_negative() { !bits } else { bits ^ (1 << 63) }
    };
    for (span, domain) in initial {
        if cells == max_cells {
            return Err(resource("Clearance budget must cover every initial cell"));
        }
        let lower = measure(span, domain, &mut witness)?;
        pending.insert((order(lower), cells), (lower, span, domain));
        cells += 1;
    }
    let (lower, stop_reason) = loop {
        let (&key, &(lower, span, [a, b])) = pending.first_key_value()
            .ok_or_else(|| crate::numeric_err("Missing clearance span coverage"))?;
        if (witness.0 - lower).next_up() <= tolerance {
            break (lower, StopReason::Tolerance);
        }
        if cells + 2 > max_cells {
            break (lower, StopReason::WorkLimit);
        }
        let middle = crate::distance_bounds::midpoint(a, b);
        if !(a < middle && middle < b) {
            break (lower, StopReason::PrecisionLimit);
        }
        pending.remove(&key);
        for domain in [[a, middle], [middle, b]] {
            let child = measure(span, domain, &mut witness)?.max(lower);
            pending.insert((order(child), cells), (child, span, domain));
            cells += 1;
        }
    };
    Ok((lower, witness, cells, stop_reason))
}

#[derive(Clone, Copy, Debug)]
pub struct SynchronizedCurveClearance {
    pub gap_bounds: [f64; 2],
    pub witness_parameter: f64,
    pub work_cells: usize,
    pub stop_reason: StopReason,
}

/// Enclose the minimum signed gap between balls moving on two curves with
/// the same parameter clock. Both curves must contain the requested window.
pub fn measure_synchronized_curve_clearance(
    curves: [&Curve; 2],
    interval: [f64; 2],
    radii: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<SynchronizedCurveClearance> {
    validate(tolerance, max_cells)?;
    let [first, second] = curves;
    let (radius, initial) = synchronized_clearance_cells(curves, interval, radii)?;
    let measure = |[a, b]: [usize; 2], domain: [f64; 2], witness: &mut ClearanceWitness| {
        let left = curve_jets::enclose(first, a, domain)?;
        let right = curve_jets::enclose(second, b, domain)?;
        let (lo, hi) = box_distance(&left[0], &right[0])?;
        for (end, parameter) in [(false, domain[0]), (true, domain[1])] {
            let left = curve_jets::endpoint(first, a, domain, end)?;
            let right = curve_jets::endpoint(second, b, domain, end)?;
            let (lo, hi) = box_distance(&left[0], &right[0])?;
            let upper = Interval::new(lo, hi)?.sub(radius)?.hi;
            if upper < witness.0 {
                *witness = (upper, parameter, 0);
            }
        }
        Ok(Interval::new(lo, hi)?.sub(radius)?.lo)
    };
    let (lower, witness, work_cells, stop_reason) =
        refine_minimum_clearance(initial, interval[0], tolerance, max_cells, measure)?;
    Ok(SynchronizedCurveClearance {
        gap_bounds: [lower, witness.0],
        witness_parameter: witness.1,
        work_cells,
        stop_reason,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct SynchronizedClearanceCell {
    pub parameter_interval: [f64; 2],
    pub spans: [usize; 2],
    pub gap_bounds: [f64; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct SynchronizedClearancePartition {
    pub cells: Vec<SynchronizedClearanceCell>,
    pub work_cells: usize,
}

/// Classify simultaneous ball clearance over a shared parameter window.
/// Every cell is evaluated against both curves' original knot grids.
pub fn partition_synchronized_curve_clearance(
    curves: [&Curve; 2],
    interval: [f64; 2],
    radii: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<SynchronizedClearancePartition> {
    validate(parameter_tolerance, max_cells)?;
    let (radius, initial) = synchronized_clearance_cells(curves, interval, radii)?;
    let [first, second] = curves;
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[a, b], domain| {
            let left = curve_jets::enclose(first, a, domain)?;
            let right = curve_jets::enclose(second, b, domain)?;
            let (lo, hi) = box_distance(&left[0], &right[0])?;
            let gap = Interval::new(lo, hi)?.sub(radius)?;
            let status = if gap.lo > 0. {
                DirectionalMotionStatus::Within
            } else if gap.hi <= 0. {
                DirectionalMotionStatus::Exceeds
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok(((gap, status), status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(spans, parameter_interval, (gap, status))| {
        SynchronizedClearanceCell { parameter_interval, spans, gap_bounds: [gap.lo, gap.hi], status }
    }).collect();
    Ok(SynchronizedClearancePartition { cells, work_cells })
}

/// Compare two windows traversed at uniform parameter rates over the same
/// elapsed time. The witness parameter is normalized progress in [0, 1].
/// Each work cell may enclose multiple source spans across the two curves.
pub fn measure_affine_synchronized_curve_clearance(
    curves: [&Curve; 2],
    windows: [[f64; 2]; 2],
    radii: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<SynchronizedCurveClearance> {
    validate(tolerance, max_cells)?;
    let radius = validate_affine_clearance(curves, windows, radii)?;
    let gap = |progress| affine_clearance_gap(curves, windows, radius, progress);
    let measure = |(), domain: [f64; 2], witness: &mut ClearanceWitness| {
        let lower = gap(domain)?.lo;
        for parameter in domain {
            let upper = gap([parameter; 2])?.hi;
            if upper < witness.0 {
                *witness = (upper, parameter, 0);
            }
        }
        Ok(lower)
    };
    let (lower, witness, work_cells, stop_reason) =
        refine_minimum_clearance([((), [0., 1.])], 0., tolerance, max_cells, measure)?;
    Ok(SynchronizedCurveClearance {
        gap_bounds: [lower, witness.0],
        witness_parameter: witness.1,
        work_cells,
        stop_reason,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct AffineSynchronizedClearanceCell {
    /// Fraction of the shared traversal time, in [0, 1].
    pub progress_interval: [f64; 2],
    pub gap_bounds: [f64; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct AffineSynchronizedClearancePartition {
    pub cells: Vec<AffineSynchronizedClearanceCell>,
    pub work_cells: usize,
}

/// Classify contact between two balls traversing independent curve windows
/// over the same elapsed time. The tolerance bounds the normalized width
/// of refined unresolved cells, subject to the work and precision limits.
pub fn partition_affine_synchronized_curve_clearance(
    curves: [&Curve; 2],
    windows: [[f64; 2]; 2],
    radii: [f64; 2],
    progress_tolerance: f64,
    max_cells: usize,
) -> Result<AffineSynchronizedClearancePartition> {
    validate(progress_tolerance, max_cells)?;
    let radius = validate_affine_clearance(curves, windows, radii)?;
    let (cells, work_cells) = partition_enclosed_cells(
        [((), [0., 1.])], progress_tolerance, max_cells,
        |(), progress_interval| {
            let gap = affine_clearance_gap(curves, windows, radius, progress_interval)?;
            let status = if gap.lo > 0. {
                DirectionalMotionStatus::Within
            } else if gap.hi <= 0. {
                DirectionalMotionStatus::Exceeds
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok((AffineSynchronizedClearanceCell {
                progress_interval,
                gap_bounds: [gap.lo, gap.hi],
                status,
            }, status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(_, _, cell)| cell).collect();
    Ok(AffineSynchronizedClearancePartition { cells, work_cells })
}

fn validate_affine_clearance(
    curves: [&Curve; 2],
    windows: [[f64; 2]; 2],
    radii: [f64; 2],
) -> Result<Interval> {
    check(radii.iter().all(|r| r.is_finite() && *r >= 0.),
        "Tool radii must be finite and nonnegative")?;
    for (curve, window) in curves.into_iter().zip(windows) {
        curve.validate()?;
        validate_travel_interval(curve, window)?;
    }
    check(curves[0].control_points[0].len() == curves[1].control_points[0].len(),
        "Synchronized curves must have matching dimensions")?;
    Interval::point(radii[0]).add(Interval::point(radii[1]))
}

fn affine_clearance_gap(
    curves: [&Curve; 2],
    windows: [[f64; 2]; 2],
    radius: Interval,
    progress: [f64; 2],
) -> Result<Interval> {
    let left = affine_progress_position(curves[0], windows[0], progress)?;
    let right = affine_progress_position(curves[1], windows[1], progress)?;
    let (lo, hi) = box_distance(&left, &right)?;
    Interval::new(lo, hi)?.sub(radius)
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduledCurveClearance {
    pub minimum: SynchronizedCurveClearance,
    /// Scheduled time of the witness, rounded outwards.
    pub witness_time_bounds: [f64; 2],
}

/// Measure simultaneous clearance during the overlap of two scheduled
/// traversals. Each segment moves uniformly in its own parameter window.
/// Returns None when the closed traversal times do not overlap.
pub fn measure_scheduled_curve_clearance(
    segments: [TimedMotionSegment<'_>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<Option<ScheduledCurveClearance>> {
    validate(tolerance, max_cells)?;
    let Some(overlap) = prepare_scheduled_overlap(segments, start_times, radii)? else {
        return Ok(None);
    };
    let gap = |progress| overlap.gap(progress);
    let measure = |(), domain: [f64; 2], witness: &mut ClearanceWitness| {
        let lower = gap(domain)?.lo;
        for progress in domain {
            let upper = gap([progress; 2])?.hi;
            if upper < witness.0 {
                *witness = (upper, progress, 0);
            }
        }
        Ok(lower)
    };
    let (lower, witness, work_cells, stop_reason) =
        refine_minimum_clearance([((), [0., 1.])], 0., tolerance, max_cells, measure)?;
    let time = overlap.time(witness.1)?;
    Ok(Some(ScheduledCurveClearance {
        minimum: SynchronizedCurveClearance {
            gap_bounds: [lower, witness.0],
            witness_parameter: witness.1,
            work_cells,
            stop_reason,
        },
        witness_time_bounds: [round_motion_duration(&time, false)?, round_motion_duration(&time, true)?],
    }))
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduledClearanceCell {
    /// Time enclosures for the start and end. Status holds between the exact endpoints.
    pub time_endpoint_bounds: [[f64; 2]; 2],
    pub gap_bounds: [f64; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct ScheduledClearancePartition {
    pub cells: Vec<ScheduledClearanceCell>,
    pub work_cells: usize,
}

/// Classify clearance over the time when both tools are scheduled to move.
/// Progress tolerance is a fraction of the overlap duration.
pub fn partition_scheduled_curve_clearance(
    segments: [TimedMotionSegment<'_>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
    progress_tolerance: f64,
    max_cells: usize,
) -> Result<Option<ScheduledClearancePartition>> {
    validate(progress_tolerance, max_cells)?;
    let Some(overlap) = prepare_scheduled_overlap(segments, start_times, radii)? else {
        return Ok(None);
    };
    partition_scheduled_phase(&overlap, progress_tolerance, max_cells).map(Some)
}

/// Check the schedule from the first start to the last finish. Each tool
/// holds its start position before moving and its final position afterward.
/// Progress tolerance is relative to each phase between start or finish events.
pub fn partition_scheduled_clearance_with_holds(
    segments: [TimedMotionSegment<'_>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
    progress_tolerance: f64,
    max_cells: usize,
) -> Result<ScheduledClearancePartition> {
    validate(progress_tolerance, max_cells)?;
    let (starts, durations, radius) = validate_motion_schedule(segments, start_times, radii)?;
    let mut events = [starts[0].clone(), starts[1].clone(),
        &starts[0] + &durations[0], &starts[1] + &durations[1]].to_vec();
    events.sort();
    events.dedup();
    let phase_count = events.len() - 1;
    if phase_count > max_cells {
        return Err(resource("Clearance budget must cover every scheduled phase"));
    }
    let mut cells = Vec::new();
    let mut remaining = max_cells;
    for (index, times) in events.windows(2).enumerate() {
        let origin = &times[0];
        let end = &times[1];
        let map = |tool: usize| -> Result<_> {
            let [a, b] = segments[tool].interval;
            let a = exact_motion_operand(a)?;
            let width = exact_motion_operand(b)? - &a;
            let parameter = |time: &num_rational::BigRational| {
                let elapsed = (time - &starts[tool])
                    .max(num_rational::BigRational::default())
                    .min(durations[tool].clone());
                &a + &width * elapsed / &durations[tool]
            };
            let first = parameter(origin);
            Ok((first.clone(), parameter(end) - first))
        };
        let phase = ScheduledOverlap {
            segments,
            origin: origin.clone(),
            duration: end - origin,
            maps: [map(0)?, map(1)?],
            radius,
        };
        let report = partition_scheduled_phase(
            &phase, progress_tolerance, remaining / (phase_count - index),
        )?;
        remaining -= report.work_cells;
        cells.extend(report.cells);
    }
    Ok(ScheduledClearancePartition { cells, work_cells: max_cells - remaining })
}

fn partition_scheduled_phase(
    overlap: &ScheduledOverlap<'_>,
    progress_tolerance: f64,
    max_cells: usize,
) -> Result<ScheduledClearancePartition> {
    use num_traits::Zero;

    let progress = if overlap.duration.is_zero() { [0.; 2] } else { [0., 1.] };
    let (cells, work_cells) = partition_enclosed_cells(
        [((), progress)], progress_tolerance, max_cells,
        |(), domain| {
            let gap = overlap.gap(domain)?;
            let status = if gap.lo > 0. {
                DirectionalMotionStatus::Within
            } else if gap.hi <= 0. {
                DirectionalMotionStatus::Exceeds
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok(((gap, status), status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(_, [a, b], (gap, status))| {
        let endpoint = |progress| -> Result<[f64; 2]> {
            let time = overlap.time(progress)?;
            Ok([round_motion_duration(&time, false)?, round_motion_duration(&time, true)?])
        };
        Ok(ScheduledClearanceCell {
            time_endpoint_bounds: [endpoint(a)?, endpoint(b)?],
            gap_bounds: [gap.lo, gap.hi],
            status,
        })
    }).collect::<Result<Vec<_>>>()?;
    Ok(ScheduledClearancePartition { cells, work_cells })
}

struct ScheduledOverlap<'a> {
    segments: [TimedMotionSegment<'a>; 2],
    origin: num_rational::BigRational,
    duration: num_rational::BigRational,
    maps: [(num_rational::BigRational, num_rational::BigRational); 2],
    radius: Interval,
}

impl ScheduledOverlap<'_> {
    fn time(&self, progress: f64) -> Result<num_rational::BigRational> {
        Ok(&self.origin + &self.duration * exact_motion_operand(progress)?)
    }

    fn gap(&self, progress: [f64; 2]) -> Result<Interval> {
        let position = |index: usize| {
            let (start, width) = &self.maps[index];
            let lo = start + width * exact_motion_operand(progress[0])?;
            let hi = start + width * exact_motion_operand(progress[1])?;
            let segment = self.segments[index];
            rounded_window_position(segment.curve, segment.interval, [&lo, &hi])
        };
        let (lo, hi) = box_distance(&position(0)?, &position(1)?)?;
        Interval::new(lo, hi)?.sub(self.radius)
    }
}

fn prepare_scheduled_overlap<'a>(
    segments: [TimedMotionSegment<'a>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
) -> Result<Option<ScheduledOverlap<'a>>> {
    let (starts, durations, radius) = validate_motion_schedule(segments, start_times, radii)?;
    let windows = segments.map(|segment| segment.interval);
    let origin = starts[0].clone().max(starts[1].clone());
    let end = (&starts[0] + &durations[0]).min(&starts[1] + &durations[1]);
    if end < origin {
        return Ok(None);
    }
    let overlap = end - &origin;
    let map = |index: usize| -> Result<_> {
        let [a, b] = windows[index];
        let a = exact_motion_operand(a)?;
        let rate = (exact_motion_operand(b)? - &a) / &durations[index];
        Ok((&a + (&origin - &starts[index]) * &rate, &overlap * rate))
    };
    let maps = [map(0)?, map(1)?];
    Ok(Some(ScheduledOverlap { segments, origin, duration: overlap, maps, radius }))
}

fn validate_motion_schedule(
    segments: [TimedMotionSegment<'_>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
) -> Result<([num_rational::BigRational; 2], [num_rational::BigRational; 2], Interval)> {
    let curves = segments.map(|segment| segment.curve);
    let windows = segments.map(|segment| segment.interval);
    let radius = validate_affine_clearance(curves, windows, radii)?;
    check(start_times.iter().all(|time| time.is_finite()), "Start times must be finite")?;
    check(segments.iter().all(|segment| segment.duration.is_finite() && segment.duration > 0.),
        "Motion segment duration must be finite and positive")?;
    let starts = [exact_motion_operand(start_times[0])?, exact_motion_operand(start_times[1])?];
    let durations = [
        exact_motion_operand(segments[0].duration)?,
        exact_motion_operand(segments[1].duration)?,
    ];
    Ok((starts, durations, radius))
}

fn affine_progress_position(
    curve: &Curve,
    window: [f64; 2],
    progress: [f64; 2],
) -> Result<Vec<Interval>> {
    let start = exact_motion_operand(window[0])?;
    let width = exact_motion_operand(window[1])? - &start;
    let lo = &start + &width * exact_motion_operand(progress[0])?;
    let hi = &start + &width * exact_motion_operand(progress[1])?;
    rounded_window_position(curve, window, [&lo, &hi])
}

fn rounded_window_position(
    curve: &Curve,
    window: [f64; 2],
    parameters: [&num_rational::BigRational; 2],
) -> Result<Vec<Interval>> {
    let lo = round_motion_duration(parameters[0], false)?.max(window[0]);
    let hi = round_motion_duration(parameters[1], true)?.min(window[1]);
    if lo == hi {
        let end = lo > window[0];
        let adjacent = if end { [window[0], lo] } else { window };
        let (span, domain) = motion_window_endpoint(curve, adjacent, end)?;
        return Ok(curve_jets::endpoint(curve, span, domain, end)?.swap_remove(0));
    }
    let mut position: Option<Vec<Interval>> = None;
    for span in active_spans(curve, [lo, hi]) {
        let domain = [lo.max(curve.knots[span]), hi.min(curve.knots[span + 1])];
        let enclosed = curve_jets::enclose(curve, span, domain)?.swap_remove(0);
        if let Some(hull) = &mut position {
            for (axis, bound) in hull.iter_mut().zip(enclosed) {
                axis.lo = axis.lo.min(bound.lo);
                axis.hi = axis.hi.max(bound.hi);
            }
        } else {
            position = Some(enclosed);
        }
    }
    position.ok_or_else(|| crate::numeric_err("Missing affine progress span coverage"))
}

fn synchronized_clearance_cells(
    curves: [&Curve; 2],
    interval: [f64; 2],
    radii: [f64; 2],
) -> Result<(Interval, Vec<([usize; 2], [f64; 2])>)> {
    check(radii.iter().all(|radius| radius.is_finite() && *radius >= 0.),
        "Tool radii must be finite and nonnegative")?;
    let [first, second] = curves;
    for curve in curves {
        curve.validate()?;
        validate_travel_interval(curve, interval)?;
    }
    check(first.control_points[0].len() == second.control_points[0].len(),
        "Synchronized curves must have matching dimensions")?;
    let radius = Interval::point(radii[0]).add(Interval::point(radii[1]))?;
    let mut first_spans = active_spans(first, interval).peekable();
    let mut second_spans = active_spans(second, interval).peekable();
    let mut initial = Vec::new();
    let mut start = interval[0];
    while let (Some(&a), Some(&b)) = (first_spans.peek(), second_spans.peek()) {
        let first_end = first.knots[a + 1];
        let second_end = second.knots[b + 1];
        let end = first_end.min(second_end).min(interval[1]);
        initial.push(([a, b], [start, end]));
        start = end;
        if end == interval[1] {
            break;
        }
        if end == first_end {
            first_spans.next();
        }
        if end == second_end {
            second_spans.next();
        }
    }
    Ok((radius, initial))
}

#[derive(Clone, Copy, Debug)]
pub struct CapsuleObstacle<'a> {
    pub endpoints: [&'a [f64]; 2],
    pub radius: f64,
}

/// Check a swept ball against closed capsules using the distance to the
/// clamped projection onto each capsule axis. Coincident endpoints give a ball.
pub fn partition_curve_capsule_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[CapsuleObstacle<'_>],
    tool_radius: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Capsule clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every capsule"));
    }
    curve.validate()?;
    let obstacles = prepare_capsules(curve, obstacles, tool_radius)?;
    partition_radial_clearance(
        curve, interval, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point),
    )
}

/// Refine the minimum signed gap between a swept ball and fixed capsules.
pub fn measure_minimum_capsule_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[CapsuleObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Capsule clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_capsules(curve, obstacles, tool_radius)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point),
    )
}

struct PreparedCapsule {
    start: Vec<Interval>,
    axis: Option<(Vec<Interval>, Interval)>,
    radius: Interval,
}

impl PreparedCapsule {
    fn gap(&self, point: &[Interval]) -> Result<Interval> {
        let distance = if let Some((delta, length_squared)) = &self.axis {
            let relative = point.iter().zip(&self.start).map(|(&p, &a)| p.sub(a))
                .collect::<Result<Vec<_>>>()?;
            let projected = project_jet(&relative, delta)?.div(*length_squared)?;
            let t = Interval::new(projected.lo.clamp(0., 1.), projected.hi.clamp(0., 1.))?;
            let residual = relative.into_iter().zip(delta).map(|(p, &d)| p.sub(d.mul(t)?))
                .collect::<Result<Vec<_>>>()?;
            norm(&residual)?
        } else {
            let (lo, hi) = box_distance(point, &self.start)?;
            Interval::new(lo, hi)?
        };
        distance.sub(self.radius)
    }
}

fn prepare_capsules(
    curve: &Curve,
    obstacles: &[CapsuleObstacle<'_>],
    tool_radius: f64,
) -> Result<Vec<PreparedCapsule>> {
    let dimension = curve.control_points[0].len();
    obstacles.iter().map(|obstacle| {
        let [a, b] = obstacle.endpoints;
        check(
            [a, b].iter().all(|p| p.len() == dimension && p.iter().all(|x| x.is_finite()))
                && obstacle.radius.is_finite() && obstacle.radius >= 0.,
            "Capsule requires finite matching endpoints and nonnegative finite radius",
        )?;
        let start = a.iter().copied().map(Interval::point).collect::<Vec<_>>();
        let axis = if a == b {
            None
        } else {
            let delta = b.iter().zip(a).map(|(&b, &a)| {
                if a == b { Ok(Interval::point(0.)) }
                else { Interval::point(b).sub(Interval::point(a)) }
            }).collect::<Result<Vec<_>>>()?;
            let length = norm(&delta)?;
            Some((delta, length.mul(length)?))
        };
        let radius = Interval::point(obstacle.radius).add(Interval::point(tool_radius))?;
        Ok(PreparedCapsule { start, axis, radius })
    }).collect()
}

/// A closed axis-aligned box, including boxes of zero thickness.
#[derive(Clone, Copy, Debug)]
pub struct BoxObstacle<'a> {
    pub minimum: &'a [f64],
    pub maximum: &'a [f64],
}

/// Enclose distance from the swept ball's center to each solid box, minus
/// the tool radius. Euclidean distance accounts for rounded edge and corner
/// contacts when the radius is positive.
pub fn partition_curve_box_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[BoxObstacle<'_>],
    tool_radius: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Box clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every box"));
    }
    curve.validate()?;
    validate_box_obstacles(curve, obstacles)?;
    partition_radial_clearance(
        curve, interval, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| {
            let obstacle = obstacles[index];
            solid_box_gap(point, obstacle.minimum, obstacle.maximum, tool_radius)
        },
    )
}

/// Refine the minimum signed gap to solid axis-aligned boxes. The report
/// includes a parameter attaining the sampled upper bound within rounding.
pub fn measure_minimum_box_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[BoxObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Box clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    validate_box_obstacles(curve, obstacles)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index| {
            let obstacle = obstacles[index];
            solid_box_gap(point, obstacle.minimum, obstacle.maximum, tool_radius)
        },
    )
}

fn validate_box_obstacles(curve: &Curve, obstacles: &[BoxObstacle<'_>]) -> Result<()> {
    let dimension = curve.control_points[0].len();
    for obstacle in obstacles {
        check(
            obstacle.minimum.len() == dimension && obstacle.maximum.len() == dimension
                && obstacle.minimum.iter().zip(obstacle.maximum).all(|(&a, &b)| {
                    a.is_finite() && b.is_finite() && a <= b
                }),
            "Box requires finite ordered bounds matching the curve dimension",
        )?;
    }
    Ok(())
}

/// A rectangular box centered at a world point. Axes must be pairwise
/// orthogonal, but need not have unit length. Extents are physical lengths.
#[derive(Clone, Copy, Debug)]
pub struct OrientedBoxObstacle<'a> {
    pub center: &'a [f64],
    pub axes: &'a [&'a [f64]],
    pub half_extents: &'a [f64],
}

/// Check swept ball clearance in each obstacle's orthonormal frame.
/// Orthogonality is checked exactly for the supplied binary64 directions.
pub fn partition_curve_oriented_box_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[OrientedBoxObstacle<'_>],
    tool_radius: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Box clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every box"));
    }
    curve.validate()?;
    let obstacles = prepare_oriented_boxes(curve, obstacles)?;
    partition_radial_clearance(
        curve, interval, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point, tool_radius),
    )
}

/// Refine the minimum signed gap to oriented solid boxes with a certified
/// endpoint witness. Directions follow the same rules as the partition API.
pub fn measure_minimum_oriented_box_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[OrientedBoxObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Box clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_oriented_boxes(curve, obstacles)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point, tool_radius),
    )
}

struct PreparedOrientedBox<'a> {
    center: Vec<Interval>,
    units: Vec<Vec<Interval>>,
    minimum: Vec<f64>,
    maximum: &'a [f64],
}

impl PreparedOrientedBox<'_> {
    fn gap(&self, point: &[Interval], radius: f64) -> Result<Interval> {
        let relative = point.iter().zip(&self.center).map(|(&p, &c)| p.sub(c))
            .collect::<Result<Vec<_>>>()?;
        let local = self.units.iter().map(|unit| project_jet(&relative, unit))
            .collect::<Result<Vec<_>>>()?;
        solid_box_gap(&local, &self.minimum, self.maximum, radius)
    }
}

fn prepare_oriented_boxes<'a>(
    curve: &Curve,
    obstacles: &[OrientedBoxObstacle<'a>],
) -> Result<Vec<PreparedOrientedBox<'a>>> {
    use num_traits::Zero;

    let dimension = curve.control_points[0].len();
    obstacles.iter().map(|obstacle| {
        check(
            obstacle.center.len() == dimension
                && obstacle.center.iter().all(|value| value.is_finite())
                && obstacle.axes.len() == dimension
                && obstacle.half_extents.len() == dimension
                && obstacle.half_extents.iter().all(|value| value.is_finite() && *value >= 0.),
            "Oriented box requires a finite center, a full frame and nonnegative half-extents",
        )?;
        let units = obstacle.axes.iter().map(|axis| travel_direction(curve, axis))
            .collect::<Result<Vec<_>>>()?;
        let exact = obstacle.axes.iter().map(|axis| {
            axis.iter().copied().map(exact_motion_operand).collect::<Result<Vec<_>>>()
        }).collect::<Result<Vec<_>>>()?;
        for (i, a) in exact.iter().enumerate() {
            for b in &exact[..i] {
                let dot: num_rational::BigRational = a.iter().zip(b).map(|(a, b)| a * b).sum();
                check(dot.is_zero(), "Oriented box axes must be pairwise orthogonal")?;
            }
        }
        let center = obstacle.center.iter().copied().map(Interval::point).collect::<Vec<_>>();
        let minimum = obstacle.half_extents.iter().map(|value| -*value).collect::<Vec<_>>();
        Ok(PreparedOrientedBox { center, units, minimum, maximum: obstacle.half_extents })
    }).collect()
}

/// A fixed obstacle for a spherical tool moving along a NURBS curve.
#[derive(Clone, Copy, Debug)]
pub enum ToolpathObstacle<'a> {
    Sphere(SphericalObstacle<'a>),
    Capsule(CapsuleObstacle<'a>),
    Box(BoxObstacle<'a>),
    OrientedBox(OrientedBoxObstacle<'a>),
}

enum PreparedToolpathObstacle<'a> {
    Sphere(PreparedSphere),
    Capsule(PreparedCapsule),
    Box(BoxObstacle<'a>),
    OrientedBox(PreparedOrientedBox<'a>),
}

impl PreparedToolpathObstacle<'_> {
    fn gap(&self, point: &[Interval], tool_radius: f64) -> Result<Interval> {
        match self {
            Self::Sphere((center, radius)) => {
                let (lo, hi) = box_distance(point, center)?;
                Interval::new(lo, hi)?.sub(*radius)
            }
            Self::Capsule(capsule) => capsule.gap(point),
            Self::Box(bounds) => solid_box_gap(point, bounds.minimum, bounds.maximum, tool_radius),
            Self::OrientedBox(bounds) => bounds.gap(point, tool_radius),
        }
    }
}

/// Refine the minimum signed clearance against mixed obstacle shapes in one
/// search. The witness obstacle index refers to the original input order.
pub fn measure_minimum_toolpath_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, tool_radius)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point, tool_radius),
    )
}

/// Classify clearance for a spherical tool in a mixed obstacle scene.
/// The gap arrays preserve scene order, including on unresolved cells.
pub fn partition_toolpath_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    partition_toolpath_clearance_with_margin(
        curve, interval, obstacles, tool_radius, 0., parameter_tolerance, max_obstacle_cells,
    )
}

/// Classify a required clearance margin in the same units as the tool radius.
/// Gap bounds express surplus over the margin. A nonpositive surplus is a
/// violation, and `first_contact` reports the first possible such violation.
pub fn partition_toolpath_clearance_with_margin(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    check(margin.is_finite() && margin >= 0., "Clearance margin must be finite and nonnegative")?;
    check(tool_radius.is_finite() && tool_radius >= 0., "Tool radius must be finite and nonnegative")?;
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every obstacle"));
    }
    curve.validate()?;
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, tool_radius)?;
    partition_radial_clearance(
        curve, interval, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| {
            let gap = obstacles[index].gap(point, tool_radius)?;
            if margin == 0. { Ok(gap) } else { gap.sub(Interval::point(margin)) }
        },
    )
}

#[derive(Clone, Copy, Debug)]
pub struct ToolpathViolationLength {
    /// Arc length of the union of all clearance violations, in model units.
    pub bounds: [f64; 2],
    pub work_obstacle_cells: usize,
    pub work_length_cells: usize,
}

/// Bound arc length on the part of the curve with clearance at or below
/// the margin. Unresolved cells contribute only to the upper bound. The
/// length tolerance controls integration error separately from contact
/// localization, which uses the parameter tolerance.
pub fn measure_toolpath_clearance_violation_length(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    length_tolerance: f64,
    max_obstacle_cells: usize,
    max_length_cells: usize,
) -> Result<ToolpathViolationLength> {
    measure_toolpath_violation_lengths(
        curve, interval, obstacles, tool_radius, margin, parameter_tolerance,
        length_tolerance, max_obstacle_cells, max_length_cells, false,
    ).map(|report| report.union)
}

#[derive(Clone, Debug)]
pub struct ToolpathObstacleViolationLengths {
    pub union: ToolpathViolationLength,
    /// Arc length bounds in obstacle order. Overlaps contribute to each
    /// relevant obstacle, while the union counts the shared length once.
    pub by_obstacle: Vec<[f64; 2]>,
}

/// Integrate each candidate cell once and attribute its bounded length to
/// every obstacle whose clearance may violate the margin.
pub fn measure_toolpath_violation_lengths_by_obstacle(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    length_tolerance: f64,
    max_obstacle_cells: usize,
    max_length_cells: usize,
) -> Result<ToolpathObstacleViolationLengths> {
    measure_toolpath_violation_lengths(
        curve, interval, obstacles, tool_radius, margin, parameter_tolerance,
        length_tolerance, max_obstacle_cells, max_length_cells, true,
    )
}

fn measure_toolpath_violation_lengths(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    length_tolerance: f64,
    max_obstacle_cells: usize,
    max_length_cells: usize,
    by_obstacle: bool,
) -> Result<ToolpathObstacleViolationLengths> {
    validate(length_tolerance, max_length_cells)?;
    let partition = partition_toolpath_clearance_with_margin(
        curve, interval, obstacles, tool_radius, margin, parameter_tolerance, max_obstacle_cells,
    )?;
    let candidates = partition.cells.iter()
        .filter(|cell| cell.status != DirectionalMotionStatus::Within)
        .collect::<Vec<_>>();
    if candidates.len() > max_length_cells {
        return Err(resource("Violation length budget must cover every candidate cell"));
    }
    let tolerance = length_tolerance / candidates.len().max(1) as f64;
    let mut bounds = Interval::point(0.);
    let mut per_obstacle = if by_obstacle {
        vec![Interval::point(0.); obstacles.len()]
    } else {
        Vec::new()
    };
    let mut remaining = max_length_cells;
    for (index, candidate) in candidates.iter().enumerate() {
        let budget = remaining / (candidates.len() - index);
        let length = measure(curve, candidate.parameter_interval, tolerance, budget)?;
        remaining -= length.cells;
        let lower = if candidate.status == DirectionalMotionStatus::Exceeds {
            length.bounds[0]
        } else {
            0.
        };
        bounds = bounds.add(Interval::new(lower, length.bounds[1])?)?;
        for (total, &[lo, hi]) in per_obstacle.iter_mut().zip(&candidate.gap_bounds) {
            if lo <= 0. {
                let lower = if hi <= 0. { length.bounds[0] } else { 0. };
                *total = total.add(Interval::new(lower, length.bounds[1])?)?;
            }
        }
    }
    Ok(ToolpathObstacleViolationLengths {
        union: ToolpathViolationLength {
            bounds: [bounds.lo.max(0.), bounds.hi],
            work_obstacle_cells: partition.work_obstacle_cells,
            work_length_cells: max_length_cells - remaining,
        },
        by_obstacle: per_obstacle.into_iter().map(|value| [value.lo.max(0.), value.hi]).collect(),
    })
}

#[derive(Clone, Debug)]
pub struct ToolpathViolationDuration {
    /// Time spent at or below the required clearance from any obstacle.
    pub bounds: [f64; 2],
    /// Time bounds per obstacle, in input order.
    pub by_obstacle: Vec<[f64; 2]>,
    pub work_obstacle_cells: usize,
}

/// Bound the duration of clearance violations under a uniform parameter clock.
/// The union counts overlapping violations once and unresolved cells only
/// contribute to upper bounds.
pub fn measure_toolpath_clearance_violation_duration(
    segment: TimedMotionSegment<'_>,
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<ToolpathViolationDuration> {
    use num_rational::BigRational;

    check(segment.duration.is_finite() && segment.duration > 0., "Motion segment duration must be finite and positive")?;
    let partition = partition_toolpath_clearance_with_margin(
        segment.curve, segment.interval, obstacles, tool_radius, margin,
        parameter_tolerance, max_obstacle_cells,
    )?;
    let rate = motion_clock_factor(segment.interval, segment.duration)?;
    let zero = [BigRational::default(), BigRational::default()];
    let mut union = zero.clone();
    let mut per_obstacle = vec![zero; obstacles.len()];
    for cell in &partition.cells {
        if cell.status == DirectionalMotionStatus::Within {
            continue;
        }
        let [a, b] = cell.parameter_interval;
        let width = exact_motion_operand(b)? - exact_motion_operand(a)?;
        union[1] += &width;
        if cell.status == DirectionalMotionStatus::Exceeds {
            union[0] += &width;
        }
        for (total, &[lo, hi]) in per_obstacle.iter_mut().zip(&cell.gap_bounds) {
            if lo <= 0. {
                total[1] += &width;
            }
            if hi <= 0. {
                total[0] += &width;
            }
        }
    }
    let bounds = |[lo, hi]: [BigRational; 2]| -> Result<[f64; 2]> {
        Ok([
            round_motion_duration(&(lo * &rate), false)?,
            round_motion_duration(&(hi * &rate), true)?,
        ])
    };
    Ok(ToolpathViolationDuration {
        bounds: bounds(union)?,
        by_obstacle: per_obstacle.into_iter().map(bounds).collect::<Result<Vec<_>>>()?,
        work_obstacle_cells: partition.work_obstacle_cells,
    })
}

#[derive(Clone, Debug)]
pub struct ToolpathChainViolationDuration {
    pub total: ToolpathViolationDuration,
    /// Reports in traversal order, each using its own segment clock.
    pub segments: Vec<ToolpathViolationDuration>,
}

/// Bound violation time across a sequence of timed curve windows. The
/// obstacle-cell budget reserves coverage for every span of every segment.
/// Segment durations describe movement only.
pub fn measure_toolpath_chain_violation_duration(
    segments: &[TimedMotionSegment<'_>],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<ToolpathChainViolationDuration> {
    use num_rational::BigRational;

    validate(parameter_tolerance, max_obstacle_cells)?;
    let (minima, mut reserve) =
        reserve_toolpath_chain_cells(segments, obstacles.len(), max_obstacle_cells)?;
    let zero = [BigRational::default(), BigRational::default()];
    let mut total = zero.clone();
    let mut per_obstacle = vec![zero; obstacles.len()];
    let mut reports = Vec::with_capacity(segments.len());
    let mut remaining = max_obstacle_cells;
    for (index, (&segment, minimum)) in segments.iter().zip(minima).enumerate() {
        let budget = minimum + (remaining - reserve) / (segments.len() - index);
        let report = measure_toolpath_clearance_violation_duration(
            segment, obstacles, tool_radius, margin, parameter_tolerance, budget,
        )?;
        for (sum, bounds) in std::iter::once(&mut total).chain(&mut per_obstacle)
            .zip(std::iter::once(report.bounds).chain(report.by_obstacle.iter().copied()))
        {
            sum[0] += exact_motion_operand(bounds[0])?;
            sum[1] += exact_motion_operand(bounds[1])?;
        }
        reserve -= minimum;
        remaining -= report.work_obstacle_cells;
        reports.push(report);
    }
    let bounds = |[lo, hi]: [BigRational; 2]| -> Result<[f64; 2]> {
        Ok([round_motion_duration(&lo, false)?, round_motion_duration(&hi, true)?])
    };
    Ok(ToolpathChainViolationDuration {
        total: ToolpathViolationDuration {
            bounds: bounds(total)?,
            by_obstacle: per_obstacle.into_iter().map(bounds).collect::<Result<Vec<_>>>()?,
            work_obstacle_cells: max_obstacle_cells - remaining,
        },
        segments: reports,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct ToolpathChainMinimumClearance {
    pub gap_bounds: [f64; 2],
    pub within_tolerance: bool,
    pub witness_segment: usize,
    pub witness_obstacle: usize,
    pub witness_parameter: f64,
    /// Outward-rounded elapsed time of the witness.
    pub witness_time_bounds: [f64; 2],
    pub work_obstacle_cells: usize,
}

/// Measure the minimum signed gap along a timed sequence using one work
/// budget. Segment clocks affect witness time, while geometry determines gap.
pub fn measure_minimum_toolpath_chain_clearance(
    segments: &[TimedMotionSegment<'_>],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<ToolpathChainMinimumClearance> {
    validate(tolerance, max_obstacle_cells)?;
    let (minima, mut reserve) =
        reserve_toolpath_chain_cells(segments, obstacles.len(), max_obstacle_cells)?;
    let mut remaining = max_obstacle_cells;
    let mut elapsed = num_rational::BigRational::default();
    let mut lower = f64::INFINITY;
    let mut witness: Option<(usize, SphericalMinimumClearance, num_rational::BigRational)> = None;
    for (index, (&segment, minimum)) in segments.iter().zip(minima).enumerate() {
        let budget = minimum + (remaining - reserve) / (segments.len() - index);
        let report = measure_minimum_toolpath_clearance(
            segment.curve, segment.interval, obstacles, tool_radius, tolerance, budget,
        )?;
        lower = lower.min(report.gap_bounds[0]);
        if witness.as_ref().is_none_or(|(_, current, _)| report.gap_bounds[1] < current.gap_bounds[1]) {
            let rate = motion_clock_factor(segment.interval, segment.duration)?;
            let parameter = exact_motion_operand(report.witness_parameter)?;
            let start = exact_motion_operand(segment.interval[0])?;
            let time = &elapsed + (parameter - start) * rate;
            witness = Some((index, report, time));
        }
        elapsed += exact_motion_operand(segment.duration)?;
        reserve -= minimum;
        remaining -= report.work_obstacle_cells;
    }
    let (witness_segment, report, time) =
        witness.ok_or_else(|| crate::numeric_err("Missing chain clearance witness"))?;
    let upper = report.gap_bounds[1];
    Ok(ToolpathChainMinimumClearance {
        gap_bounds: [lower, upper],
        within_tolerance: (upper - lower).next_up() <= tolerance,
        witness_segment,
        witness_obstacle: report.witness_obstacle,
        witness_parameter: report.witness_parameter,
        witness_time_bounds: [round_motion_duration(&time, false)?, round_motion_duration(&time, true)?],
        work_obstacle_cells: max_obstacle_cells - remaining,
    })
}

fn reserve_toolpath_chain_cells(
    segments: &[TimedMotionSegment<'_>],
    obstacle_count: usize,
    max_obstacle_cells: usize,
) -> Result<(Vec<usize>, usize)> {
    check(!segments.is_empty() && obstacle_count != 0, "Chain clearance requires segments and obstacles")?;
    if segments.len() > max_obstacle_cells / obstacle_count {
        return Err(resource("Clearance budget must cover every segment and obstacle"));
    }
    let mut minima = Vec::with_capacity(segments.len());
    let mut reserve = 0;
    for segment in segments {
        segment.curve.validate()?;
        validate_travel_interval(segment.curve, segment.interval)?;
        check(segment.duration.is_finite() && segment.duration > 0., "Motion segment duration must be finite and positive")?;
        let spans = active_spans(segment.curve, segment.interval).count();
        if spans > (max_obstacle_cells - reserve) / obstacle_count {
            return Err(resource("Clearance budget must cover every segment span"));
        }
        let minimum = spans * obstacle_count;
        reserve += minimum;
        minima.push(minimum);
    }
    Ok((minima, reserve))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FirstToolpathChainViolation {
    Clear,
    Possible {
        earliest_time: f64,
        earliest_segment: usize,
    },
    Confirmed {
        time_bounds: [f64; 2],
        earliest_segment: usize,
        witness_segment: usize,
        obstacle_index: usize,
    },
}

/// Locate the first possible or confirmed clearance violation in elapsed
/// chain time. Earlier unresolved contacts still bound the lower endpoint
/// when a later segment provides the first confirmed witness.
pub fn first_toolpath_chain_violation(
    segments: &[TimedMotionSegment<'_>],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<FirstToolpathChainViolation> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    let (minima, mut reserve) =
        reserve_toolpath_chain_cells(segments, obstacles.len(), max_obstacle_cells)?;
    let dimension = segments[0].curve.control_points[0].len();
    check(
        segments.iter().all(|segment| segment.curve.control_points[0].len() == dimension),
        "Toolpath segments must match obstacle dimensions",
    )?;
    let mut elapsed = num_rational::BigRational::default();
    let mut earliest = None;
    let mut remaining = max_obstacle_cells;
    for (index, (&segment, minimum)) in segments.iter().zip(minima).enumerate() {
        let budget = minimum + (remaining - reserve) / (segments.len() - index);
        let partition = partition_toolpath_clearance_with_margin(
            segment.curve, segment.interval, obstacles, tool_radius, margin,
            parameter_tolerance, budget,
        )?;
        let rate = motion_clock_factor(segment.interval, segment.duration)?;
        let start = exact_motion_operand(segment.interval[0])?;
        let time = |parameter, upper| {
            let exact = &elapsed + (exact_motion_operand(parameter)? - &start) * &rate;
            round_motion_duration(&exact, upper)
        };
        match partition.first_contact() {
            FirstToolpathContact::Clear => {}
            FirstToolpathContact::Possible { earliest_parameter } => {
                if earliest.is_none() {
                    earliest = Some((time(earliest_parameter, false)?, index));
                }
            }
            FirstToolpathContact::Confirmed { parameter_bounds: [lo, hi], obstacle_index } => {
                let (lower, earliest_segment) = match earliest {
                    Some(bound) => bound,
                    None => (time(lo, false)?, index),
                };
                return Ok(FirstToolpathChainViolation::Confirmed {
                    time_bounds: [lower, time(hi, true)?],
                    earliest_segment,
                    witness_segment: index,
                    obstacle_index,
                });
            }
        }
        elapsed += exact_motion_operand(segment.duration)?;
        reserve -= minimum;
        remaining -= partition.work_obstacle_cells;
    }
    Ok(match earliest {
        None => FirstToolpathChainViolation::Clear,
        Some((earliest_time, earliest_segment)) => FirstToolpathChainViolation::Possible {
            earliest_time,
            earliest_segment,
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LastToolpathChainViolation {
    Clear,
    Possible {
        latest_time: f64,
        latest_segment: usize,
    },
    Confirmed {
        time_bounds: [f64; 2],
        latest_segment: usize,
        witness_segment: usize,
        obstacle_index: usize,
    },
}

/// Bracket the last clearance violation in elapsed chain time. An unresolved
/// later cell sets the upper endpoint even if the witness is earlier.
pub fn last_toolpath_chain_violation(
    segments: &[TimedMotionSegment<'_>],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<LastToolpathChainViolation> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    let (minima, mut reserve) =
        reserve_toolpath_chain_cells(segments, obstacles.len(), max_obstacle_cells)?;
    let dimension = segments[0].curve.control_points[0].len();
    check(
        segments.iter().all(|segment| segment.curve.control_points[0].len() == dimension),
        "Toolpath segments must match obstacle dimensions",
    )?;
    let durations = segments.iter().map(|segment| exact_motion_operand(segment.duration))
        .collect::<Result<Vec<_>>>()?;
    let mut elapsed: num_rational::BigRational = durations.iter().cloned().sum();
    let mut latest = None;
    let mut remaining = max_obstacle_cells;
    for (index, (&segment, minimum)) in segments.iter().zip(minima).enumerate().rev() {
        let budget = minimum + (remaining - reserve) / (index + 1);
        let partition = partition_toolpath_clearance_with_margin(
            segment.curve, segment.interval, obstacles, tool_radius, margin,
            parameter_tolerance, budget,
        )?;
        elapsed -= &durations[index];
        let rate = motion_clock_factor(segment.interval, segment.duration)?;
        let start = exact_motion_operand(segment.interval[0])?;
        let time = |parameter, upper| {
            let exact = &elapsed + (exact_motion_operand(parameter)? - &start) * &rate;
            round_motion_duration(&exact, upper)
        };
        for cell in partition.cells.iter().rev() {
            if cell.status == DirectionalMotionStatus::Within {
                continue;
            }
            let end = cell.parameter_interval[1];
            let (upper, latest_segment) = match latest {
                Some(bound) => bound,
                None => {
                    let bound = (time(end, true)?, index);
                    latest = Some(bound);
                    bound
                }
            };
            if let Some(obstacle_index) = cell.gap_bounds.iter().position(|[_, hi]| *hi <= 0.) {
                return Ok(LastToolpathChainViolation::Confirmed {
                    time_bounds: [time(end, false)?, upper],
                    latest_segment,
                    witness_segment: index,
                    obstacle_index,
                });
            }
        }
        reserve -= minimum;
        remaining -= partition.work_obstacle_cells;
    }
    Ok(match latest {
        None => LastToolpathChainViolation::Clear,
        Some((latest_time, latest_segment)) => LastToolpathChainViolation::Possible {
            latest_time,
            latest_segment,
        },
    })
}

#[derive(Clone, Copy, Debug)]
pub struct TimedToolpathClearanceInterval {
    pub segment_index: usize,
    pub parameter_interval: [f64; 2],
    /// Elapsed-time enclosures for the start and end, respectively. The
    /// classification applies between the exact endpoints.
    pub time_endpoint_bounds: [[f64; 2]; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct ToolpathChainClearanceTimeline {
    pub intervals: Vec<TimedToolpathClearanceInterval>,
    pub work_obstacle_cells: usize,
}

impl ToolpathChainClearanceTimeline {
    /// Bound the longest continuous clearance violation. Confirmed intervals
    /// establish the lower bound, and runs without a clear interval establish
    /// the upper bound. Both runs may extend across segment boundaries.
    pub fn longest_violation_duration(&self) -> [f64; 2] {
        let mut confirmed_start = None;
        let mut possible_start = None;
        let mut longest = [0_f64; 2];
        for interval in &self.intervals {
            let [start, end] = interval.time_endpoint_bounds;
            if interval.status == DirectionalMotionStatus::Within {
                confirmed_start = None;
                possible_start = None;
                continue;
            }
            let earliest = *possible_start.get_or_insert(start[0]);
            let upper = if end[1].is_infinite() {
                f64::INFINITY
            } else {
                (end[1] - earliest).next_up()
            };
            longest[1] = longest[1].max(upper);
            if interval.status == DirectionalMotionStatus::Exceeds {
                let latest = *confirmed_start.get_or_insert(start[1]);
                let lower = if latest.is_infinite() {
                    0.
                } else {
                    (end[0] - latest).next_down()
                };
                longest[0] = longest[0].max(lower);
            } else {
                confirmed_start = None;
            }
        }
        longest
    }
}

/// Map clearance cells to elapsed chain time and merge adjacent cells with
/// equal status within each segment. Each segment uses its own uniform clock.
pub fn toolpath_chain_clearance_timeline(
    segments: &[TimedMotionSegment<'_>],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radius: f64,
    margin: f64,
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<ToolpathChainClearanceTimeline> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    let (minima, mut reserve) =
        reserve_toolpath_chain_cells(segments, obstacles.len(), max_obstacle_cells)?;
    let mut elapsed = num_rational::BigRational::default();
    let mut remaining = max_obstacle_cells;
    let mut intervals: Vec<TimedToolpathClearanceInterval> = Vec::new();
    for (segment_index, (&segment, minimum)) in segments.iter().zip(minima).enumerate() {
        let budget = minimum + (remaining - reserve) / (segments.len() - segment_index);
        let partition = partition_toolpath_clearance_with_margin(
            segment.curve, segment.interval, obstacles, tool_radius, margin,
            parameter_tolerance, budget,
        )?;
        let rate = motion_clock_factor(segment.interval, segment.duration)?;
        let start = exact_motion_operand(segment.interval[0])?;
        let time = |parameter| -> Result<[f64; 2]> {
            let exact = &elapsed + (exact_motion_operand(parameter)? - &start) * &rate;
            Ok([round_motion_duration(&exact, false)?, round_motion_duration(&exact, true)?])
        };
        for cell in &partition.cells {
            let [a, b] = cell.parameter_interval;
            if let Some(last) = intervals.last_mut()
                && last.segment_index == segment_index
                && last.status == cell.status
                && last.parameter_interval[1] == a
            {
                last.parameter_interval[1] = b;
                last.time_endpoint_bounds[1] = time(b)?;
                continue;
            }
            intervals.push(TimedToolpathClearanceInterval {
                segment_index,
                parameter_interval: [a, b],
                time_endpoint_bounds: [time(a)?, time(b)?],
                status: cell.status,
            });
        }
        elapsed += exact_motion_operand(segment.duration)?;
        reserve -= minimum;
        remaining -= partition.work_obstacle_cells;
    }
    Ok(ToolpathChainClearanceTimeline {
        intervals,
        work_obstacle_cells: max_obstacle_cells - remaining,
    })
}

fn prepare_toolpath_obstacles<'a>(
    curve: &Curve,
    obstacles: &[ToolpathObstacle<'a>],
    tool_radius: f64,
) -> Result<Vec<PreparedToolpathObstacle<'a>>> {
    obstacles.iter().map(|obstacle| {
        Ok(match obstacle {
            ToolpathObstacle::Sphere(sphere) => PreparedToolpathObstacle::Sphere(
                prepare_spherical_obstacles(curve, std::slice::from_ref(sphere), tool_radius)?.remove(0),
            ),
            ToolpathObstacle::Capsule(capsule) => PreparedToolpathObstacle::Capsule(
                prepare_capsules(curve, std::slice::from_ref(capsule), tool_radius)?.remove(0),
            ),
            ToolpathObstacle::Box(bounds) => {
                validate_box_obstacles(curve, std::slice::from_ref(bounds))?;
                PreparedToolpathObstacle::Box(*bounds)
            }
            ToolpathObstacle::OrientedBox(bounds) => PreparedToolpathObstacle::OrientedBox(
                prepare_oriented_boxes(curve, std::slice::from_ref(bounds))?.remove(0),
            ),
        })
    }).collect()
}

fn solid_box_gap(
    point: &[Interval],
    minimum: &[f64],
    maximum: &[f64],
    tool_radius: f64,
) -> Result<Interval> {
    let mut residuals = Vec::with_capacity(point.len());
    for ((coord, &a), &b) in point.iter().zip(minimum).zip(maximum) {
        if a <= coord.lo && coord.hi <= b {
            residuals.push(Interval::point(0.));
            continue;
        }
        let below = Interval::point(a).sub(*coord)?;
        let above = coord.sub(Interval::point(b))?;
        residuals.push(Interval::new(
            below.lo.max(above.lo).max(0.),
            below.hi.max(above.hi).max(0.),
        )?);
    }
    if residuals.iter().all(|axis| axis.hi == 0.) {
        return Ok(Interval::point(-tool_radius));
    }
    let distance = norm(&residuals)?;
    if tool_radius == 0. { Ok(distance) } else { distance.sub(Interval::point(tool_radius)) }
}

fn partition_radial_clearance(
    curve: &Curve,
    interval: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
    obstacle_count: usize,
    gap: impl Fn(&[Interval], usize) -> Result<Interval>,
) -> Result<SphericalClearancePartition> {
    let (cells, work_cells) = partition_motion_cells(
        curve, interval, parameter_tolerance, max_cells,
        |jets, _| {
            let gaps = (0..obstacle_count).map(|index| {
                let gap = gap(&jets[0], index)?;
                Ok([gap.lo, gap.hi])
            }).collect::<Result<Vec<_>>>()?;
            let status = if gaps.iter().any(|[_, hi]| *hi <= 0.) {
                DirectionalMotionStatus::Exceeds
            } else if gaps.iter().all(|[lo, _]| *lo > 0.) {
                DirectionalMotionStatus::Within
            } else {
                DirectionalMotionStatus::Undetermined
            };
            let refine = gaps.iter().any(|[lo, hi]| *lo <= 0. && *hi > 0.);
            Ok(((gaps, status), refine))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (gap_bounds, status))| {
        SphericalClearanceCell { parameter_interval, span, gap_bounds, status }
    }).collect();
    Ok(SphericalClearancePartition { cells, work_obstacle_cells: work_cells * obstacle_count })
}

fn partition_motion_cells<T>(
    curve: &Curve,
    interval: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
    mut classify: impl FnMut(&[Vec<Interval>], Interval) -> Result<(T, bool)>,
) -> Result<(Vec<(usize, [f64; 2], T)>, usize)> {
    validate(parameter_tolerance, max_cells)?;
    validate_travel_interval(curve, interval)?;
    let initial = active_spans(curve, interval).map(|span| (span, [
        curve.knots[span].max(interval[0]),
        curve.knots[span + 1].min(interval[1]),
    ]));
    partition_enclosed_cells(initial, parameter_tolerance, max_cells, |span, domain| {
        let width = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
        let jets = curve_jets::enclose(curve, span, domain)?;
        classify(&jets, width)
    })
}

fn partition_enclosed_cells<Span: Copy, T>(
    initial: impl IntoIterator<Item = (Span, [f64; 2])>,
    parameter_tolerance: f64,
    max_cells: usize,
    mut classify: impl FnMut(Span, [f64; 2]) -> Result<(T, bool)>,
) -> Result<(Vec<(Span, [f64; 2], T)>, usize)> {
    use std::collections::VecDeque;

    let mut pending: VecDeque<_> = initial.into_iter().collect();
    if pending.len() > max_cells {
        return Err(resource("Initial motion spans exceed the work budget"));
    }

    let mut cells = Vec::new();
    let mut work_cells = 0;
    while let Some((span, domain @ [a, b])) = pending.pop_front() {
        work_cells += 1;
        let (statuses, refine) = classify(span, domain)?;
        let middle = crate::distance_bounds::midpoint(a, b);
        if refine
            && (b - a).next_up() > parameter_tolerance
            && a < middle
            && middle < b
            && work_cells + pending.len() + 2 <= max_cells
        {
            pending.push_back((span, [a, middle]));
            pending.push_back((span, [middle, b]));
        } else {
            cells.push((span, domain, statuses));
        }
    }
    cells.sort_by(|a, b| a.1[0].total_cmp(&b.1[0]));
    Ok((cells, work_cells))
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalMotionKnotJump {
    pub parameter: f64,
    /// Signed right-minus-left jumps in speed, acceleration and jerk.
    pub jump_bounds: [[f64; 2]; 3],
    /// Classifications against the inclusive absolute jump limits.
    pub statuses: [DirectionalMotionStatus; 3],
}

/// Classify jumps of the signed axial derivatives at every interior knot
/// of the requested window. Derivatives guaranteed continuous by the knot
/// multiplicity have exact zero jump. Other jumps use one-sided enclosures.
pub fn classify_directional_motion_knot_jumps(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
    max_knots: usize,
) -> Result<Vec<DirectionalMotionKnotJump>> {
    classify_motion_knot_jumps(curve, interval, direction, limits, max_knots, None)
}

/// Classify interior-knot jumps in the uniform clock of a timed segment.
/// Reported knot coordinates remain in the original parameter domain.
pub fn classify_timed_motion_knot_jumps(
    segment: TimedMotionSegment<'_>,
    direction: &[f64],
    limits: DirectionalMotionLimits,
    max_knots: usize,
) -> Result<Vec<DirectionalMotionKnotJump>> {
    check(
        segment.duration.is_finite() && segment.duration > 0.,
        "Motion segment duration must be finite and positive",
    )?;
    classify_motion_knot_jumps(
        segment.curve, segment.interval, direction, limits, max_knots, Some(segment.duration),
    )
}

fn classify_motion_knot_jumps(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
    max_knots: usize,
    duration: Option<f64>,
) -> Result<Vec<DirectionalMotionKnotJump>> {
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    check(max_knots <= 100_000, "Motion jump budget must be at most 100000 knots")?;
    let limits = [limits.speed, limits.acceleration, limits.jerk];
    validate_motion_partition_limits(limits)?;
    let unit = travel_direction(curve, direction)?;
    let source_rate = duration.map(|duration| {
        Interval::point(interval[1])
            .sub(Interval::point(interval[0]))?
            .div(Interval::point(duration))
    }).transpose()?;
    let mut jumps = Vec::new();
    let mut spans = active_spans(curve, interval);
    let mut left = spans.next().ok_or_else(|| crate::numeric_err("Missing motion span coverage"))?;
    for right in spans {
        if jumps.len() == max_knots {
            return Err(resource("Interior motion knots exceed the work budget"));
        }
        let continuity = curve.degree.saturating_sub(right - left);
        let parameter = curve.knots[right];
        let mut jump_bounds = [[0.; 2]; 3];
        let mut statuses = [DirectionalMotionStatus::Within; 3];
        if continuity < 3 {
            let left_domain = [curve.knots[left], parameter];
            let right_domain = [parameter, curve.knots[right + 1]];
            let left_jets = curve_jets::endpoint(curve, left, left_domain, true)?;
            let right_jets = curve_jets::endpoint(curve, right, right_domain, false)?;
            let left_width = Interval::point(parameter).sub(Interval::point(left_domain[0]))?;
            let right_width = Interval::point(right_domain[1]).sub(Interval::point(parameter))?;
            for (index, limit) in limits.into_iter().enumerate().skip(continuity) {
                let order = index + 1;
                let mut before = project_jet(&left_jets[order], &unit)?;
                let mut after = project_jet(&right_jets[order], &unit)?;
                for _ in 0..order {
                    before = before.div(left_width)?;
                    after = after.div(right_width)?;
                }
                let mut jump = after.sub(before)?;
                if let Some(rate) = source_rate {
                    for _ in 0..order {
                        jump = jump.mul(rate)?;
                    }
                }
                jump_bounds[index] = [jump.lo, jump.hi];
                statuses[index] = classify_absolute_limit(jump, limit);
            }
        }
        jumps.push(DirectionalMotionKnotJump { parameter, jump_bounds, statuses });
        left = right;
    }
    Ok(jumps)
}

#[derive(Clone, Copy, Debug)]
pub struct MotionKnotTurn {
    pub parameter: f64,
    /// Cosine between incoming and outgoing tangents, absent when either
    /// tangent cannot be proved nonzero.
    pub cosine_bounds: Option<[f64; 2]>,
    pub status: DirectionalMotionStatus,
}

/// Classify interior turns against a minimum tangent cosine in [-1, 1].
/// A larger cosine means a smaller turn. Each tangent uses its own span.
pub fn classify_motion_knot_turns(
    curve: &Curve,
    interval: [f64; 2],
    minimum_cosine: f64,
    max_knots: usize,
) -> Result<Vec<MotionKnotTurn>> {
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    check((-1.0..=1.0).contains(&minimum_cosine), "Minimum turn cosine must be in [-1, 1]")?;
    check(max_knots <= 100_000, "Motion turn budget must be at most 100000 knots")?;
    let mut turns = Vec::new();
    let mut spans = active_spans(curve, interval);
    let mut left = spans.next().ok_or_else(|| crate::numeric_err("Missing motion span coverage"))?;
    for right in spans {
        if turns.len() == max_knots {
            return Err(resource("Interior motion knots exceed the work budget"));
        }
        let parameter = curve.knots[right];
        let before = curve_jets::endpoint(curve, left, [curve.knots[left], parameter], true)?;
        let after = curve_jets::endpoint(curve, right, [parameter, curve.knots[right + 1]], false)?;
        let (cosine_bounds, status) =
            classify_tangent_turn(&before[1], &after[1], right - left < curve.degree, minimum_cosine)?;
        turns.push(MotionKnotTurn { parameter, cosine_bounds, status });
        left = right;
    }
    Ok(turns)
}

fn classify_tangent_turn(
    before: &[Interval],
    after: &[Interval],
    same_source_derivative: bool,
    minimum_cosine: f64,
) -> Result<(Option<[f64; 2]>, DirectionalMotionStatus)> {
    let left_norm = norm(before)?;
    let right_norm = norm(after)?;
    let cosine_bounds = if left_norm.lo <= 0. || right_norm.lo <= 0. {
        None
    } else if same_source_derivative {
        Some([1.; 2])
    } else {
        let cosine = before.iter().zip(after).try_fold(
            Interval::point(0.),
            |sum, (&a, &b)| sum.add(a.div(left_norm)?.mul(b.div(right_norm)?)?),
        )?;
        Some([cosine.lo.max(-1.), cosine.hi.min(1.)])
    };
    let status = match cosine_bounds {
        Some([lo, _]) if lo >= minimum_cosine => DirectionalMotionStatus::Within,
        Some([_, hi]) if hi < minimum_cosine => DirectionalMotionStatus::Exceeds,
        _ => DirectionalMotionStatus::Undetermined,
    };
    Ok((cosine_bounds, status))
}

#[derive(Clone, Copy, Debug)]
pub struct MotionJoinTurn {
    pub position_gap_bounds: [f64; 2],
    /// Cosine between the tangents in traversal order.
    pub cosine_bounds: Option<[f64; 2]>,
    pub turn_status: DirectionalMotionStatus,
}

/// Compare the last tangent of the first window with the first tangent of the
/// second, using increasing source parameters. The turn and gap are independent.
pub fn classify_motion_join_turn(
    first: (&Curve, [f64; 2]),
    second: (&Curve, [f64; 2]),
    minimum_cosine: f64,
) -> Result<MotionJoinTurn> {
    check((-1.0..=1.0).contains(&minimum_cosine), "Minimum turn cosine must be in [-1, 1]")?;
    let endpoint = |(curve, interval): (&Curve, [f64; 2]), end| -> Result<_> {
        curve.validate()?;
        validate_travel_interval(curve, interval)?;
        let (span, domain) = motion_window_endpoint(curve, interval, end)?;
        curve_jets::endpoint(curve, span, domain, end)
    };
    let before = endpoint(first, true)?;
    let after = endpoint(second, false)?;
    check(before[0].len() == after[0].len(), "Motion join requires matching curve dimensions")?;
    let (lo, hi) = box_distance(&before[0], &after[0])?;
    let (cosine_bounds, turn_status) =
        classify_tangent_turn(&before[1], &after[1], false, minimum_cosine)?;
    Ok(MotionJoinTurn { position_gap_bounds: [lo, hi], cosine_bounds, turn_status })
}

#[derive(Clone, Copy, Debug)]
pub struct MotionChainTurn {
    pub segment_indices: [usize; 2],
    pub join: MotionJoinTurn,
    pub gap_status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct MotionChainTurnReport {
    pub joins: Vec<MotionChainTurn>,
    pub status: DirectionalMotionStatus,
}

/// Check tangent turns and spatial gaps in traversal order, including the
/// last-to-first join for a closed chain. Both thresholds are inclusive.
pub fn classify_motion_chain_turns(
    windows: &[(&Curve, [f64; 2])],
    closed: bool,
    minimum_cosine: f64,
    maximum_gap: f64,
    max_joins: usize,
) -> Result<MotionChainTurnReport> {
    check(maximum_gap.is_finite() && maximum_gap >= 0., "Gap limit must be finite and nonnegative")?;
    let count = motion_chain_join_count(windows.len(), closed, max_joins)?;
    let mut joins = Vec::with_capacity(count);
    let mut status = DirectionalMotionStatus::Within;
    for first in 0..count {
        let second = (first + 1) % windows.len();
        let join = classify_motion_join_turn(windows[first], windows[second], minimum_cosine)?;
        let [lo, hi] = join.position_gap_bounds;
        let gap_status = classify_absolute_limit(Interval::new(lo, hi)?, maximum_gap);
        for result in [gap_status, join.turn_status] {
            if result == DirectionalMotionStatus::Exceeds || status == DirectionalMotionStatus::Within {
                status = result;
            }
        }
        joins.push(MotionChainTurn { segment_indices: [first, second], join, gap_status });
    }
    Ok(MotionChainTurnReport { joins, status })
}

fn motion_chain_join_count(segment_count: usize, closed: bool, max_joins: usize) -> Result<usize> {
    check(
        segment_count >= 2 || (closed && segment_count == 1),
        "An open motion chain needs two segments, and a closed chain needs one",
    )?;
    let count = if closed { segment_count } else { segment_count - 1 };
    if count > max_joins {
        return Err(resource("Motion chain joins exceed the work budget"));
    }
    Ok(count)
}

fn motion_window_endpoint(
    curve: &Curve,
    interval: [f64; 2],
    end: bool,
) -> Result<(usize, [f64; 2])> {
    let mut spans = active_spans(curve, interval);
    let span = if end { spans.last() } else { spans.next() }
        .ok_or_else(|| crate::numeric_err("Missing motion endpoint span"))?;
    Ok((span, [
        interval[0].max(curve.knots[span]),
        interval[1].min(curve.knots[span + 1]),
    ]))
}

/// A curve window traversed in increasing parameter order with a uniform clock.
#[derive(Clone, Copy, Debug)]
pub struct TimedMotionSegment<'a> {
    pub curve: &'a Curve,
    pub interval: [f64; 2],
    pub duration: f64,
}

#[derive(Clone, Debug)]
pub struct DirectionalMotionJoinReport {
    /// Euclidean gap between the segment endpoints.
    pub position_gap_bounds: [f64; 2],
    /// Signed jumps in axial speed, acceleration and jerk (second minus first).
    pub jump_bounds: [[f64; 2]; 3],
    pub statuses: [DirectionalMotionStatus; 3],
}

/// Compare the final state of the first segment with the initial state of the
/// second. Derivatives use each segment's supplied duration in common time units.
/// The position gap is reported independently of the derivative limits.
pub fn classify_directional_motion_join(
    segments: [TimedMotionSegment<'_>; 2],
    direction: &[f64],
    limits: DirectionalMotionLimits,
) -> Result<DirectionalMotionJoinReport> {
    let limits = [limits.speed, limits.acceleration, limits.jerk];
    validate_motion_partition_limits(limits)?;
    for segment in segments {
        segment.curve.validate()?;
        validate_travel_interval(segment.curve, segment.interval)?;
        check(
            segment.duration.is_finite() && segment.duration > 0.,
            "Motion segment duration must be finite and positive",
        )?;
        check(
            segment.curve.control_points[0].len() == direction.len(),
            "Motion join requires matching curve and axis dimensions",
        )?;
    }
    let [first, second] = segments;
    let unit = travel_direction(first.curve, direction)?;
    let endpoint = |segment: TimedMotionSegment<'_>, end| -> Result<_> {
        let (span, domain) = motion_window_endpoint(segment.curve, segment.interval, end)?;
        let [a, b] = segment.interval;
        let source_width = Interval::point(b).sub(Interval::point(a))?;
        let cell_width = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
        let scale = source_width.div(cell_width)?.div(Interval::point(segment.duration))?;
        let jets = curve_jets::endpoint(segment.curve, span, domain, end)?;
        Ok((jets, scale))
    };
    let (left, left_scale) = endpoint(first, true)?;
    let (right, right_scale) = endpoint(second, false)?;
    let (gap_lo, gap_hi) = box_distance(&left[0], &right[0])?;
    let mut jump_bounds = [[0.; 2]; 3];
    let mut statuses = [DirectionalMotionStatus::Within; 3];
    for (index, limit) in limits.into_iter().enumerate() {
        let order = index + 1;
        let mut before = project_jet(&left[order], &unit)?;
        let mut after = project_jet(&right[order], &unit)?;
        for _ in 0..order {
            before = before.mul(left_scale)?;
            after = after.mul(right_scale)?;
        }
        let jump = after.sub(before)?;
        jump_bounds[index] = [jump.lo, jump.hi];
        statuses[index] = classify_absolute_limit(jump, limit);
    }
    Ok(DirectionalMotionJoinReport {
        position_gap_bounds: [gap_lo, gap_hi],
        jump_bounds,
        statuses,
    })
}

#[derive(Clone, Debug)]
pub struct DirectionalMotionChainJoin {
    pub segment_indices: [usize; 2],
    pub join: DirectionalMotionJoinReport,
    pub position_status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct DirectionalMotionChainReport {
    pub joins: Vec<DirectionalMotionChainJoin>,
    /// Exceeds if any join violates a limit, Within if all limits are proved,
    /// or Undetermined if some limits remain unresolved.
    pub status: DirectionalMotionStatus,
}

/// Check adjacent joins in traversal order, optionally including the closing
/// join. Each join is evaluated in the independent clocks of its two segments.
pub fn classify_directional_motion_chain(
    segments: &[TimedMotionSegment<'_>],
    closed: bool,
    direction: &[f64],
    position_limit: f64,
    derivative_limits: DirectionalMotionLimits,
    max_joins: usize,
) -> Result<DirectionalMotionChainReport> {
    let join_count = motion_chain_join_count(segments.len(), closed, max_joins)?;
    check(
        position_limit.is_finite() && position_limit >= 0.,
        "Position gap limit must be finite and nonnegative",
    )?;
    let mut joins = Vec::with_capacity(join_count);
    let mut status = DirectionalMotionStatus::Within;
    for first in 0..join_count {
        let second = (first + 1) % segments.len();
        let join =
            classify_directional_motion_join([segments[first], segments[second]], direction, derivative_limits)?;
        let [lo, hi] = join.position_gap_bounds;
        let position_status = classify_absolute_limit(Interval::new(lo, hi)?, position_limit);
        for next in join.statuses.into_iter().chain([position_status]) {
            if next == DirectionalMotionStatus::Exceeds || status == DirectionalMotionStatus::Within {
                status = next;
            }
        }
        joins.push(DirectionalMotionChainJoin {
            segment_indices: [first, second],
            join,
            position_status,
        });
    }
    Ok(DirectionalMotionChainReport { joins, status })
}

#[derive(Clone, Copy, Debug)]
pub struct TimedMotionChainLimits {
    pub segment: DirectionalMotionLimits,
    pub join: DirectionalMotionLimits,
    pub position_gap: f64,
}

#[derive(Clone, Debug)]
pub struct TimedMotionChainReport {
    pub segment_statuses: Vec<DirectionalMotionStatus>,
    pub joins: DirectionalMotionChainReport,
    pub status: DirectionalMotionStatus,
    /// Enclosure cells for segment derivatives. Joins have a separate budget.
    pub cells: usize,
}

/// Certify axial limits inside each segment in its supplied physical clock,
/// and check spatial gaps and derivative jumps at the joins.
/// Tolerances control peak refinement in each original parameter clock.
pub fn check_timed_motion_chain(
    segments: &[TimedMotionSegment<'_>],
    closed: bool,
    direction: &[f64],
    limits: TimedMotionChainLimits,
    tolerances: [f64; 3],
    max_cells: usize,
    max_joins: usize,
) -> Result<TimedMotionChainReport> {
    for tolerance in tolerances {
        validate(tolerance, max_cells)?;
    }
    if segments.len() > max_cells / 3 {
        return Err(resource("Motion chain segments exceed the cell budget"));
    }
    let joins = classify_directional_motion_chain(
        segments, closed, direction, limits.position_gap, limits.join, max_joins,
    )?;
    let mut minima = Vec::with_capacity(segments.len());
    let mut reserve = 0;
    for segment in segments {
        let minimum = 3 * active_spans(segment.curve, segment.interval).count();
        reserve += minimum;
        if reserve > max_cells {
            return Err(resource("Initial motion chain spans exceed the cell budget"));
        }
        minima.push(minimum);
    }
    let mut remaining = max_cells;
    let mut status = joins.status;
    let mut segment_statuses = Vec::with_capacity(segments.len());
    for (index, (segment, minimum)) in segments.iter().zip(minima).enumerate() {
        let budget = (minimum + (remaining - reserve) / (segments.len() - index)) / 3;
        let report = check_directional_motion_limits(
            segment.curve, segment.interval, direction, limits.segment, tolerances, budget,
        )?;
        let factor = motion_clock_factor(segment.interval, segment.duration)?;
        let powers = [factor.clone(), &factor * &factor, factor.pow(3)];
        let next = motion_status_in_clock(&report, limits.segment, &powers)?;
        if next == DirectionalMotionStatus::Exceeds || status == DirectionalMotionStatus::Within {
            status = next;
        }
        segment_statuses.push(next);
        remaining -= report.speed.cells + report.acceleration.cells + report.jerk.cells;
        reserve -= minimum;
    }
    Ok(TimedMotionChainReport { segment_statuses, joins, status, cells: max_cells - remaining })
}

fn measure_directional_derivative_peak(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
    parameter_tolerance: Option<f64>,
    derivative_order: usize,
) -> Result<DirectionalPeakLocations> {
    if let Some(parameter_tolerance) = parameter_tolerance {
        check(
            parameter_tolerance.is_finite() && parameter_tolerance > 0.,
            "Peak parameter tolerance must be positive and finite",
        )?;
    }
    curve.validate()?;
    validate(tolerance, max_cells)?;
    validate_travel_interval(curve, interval)?;
    let unit = travel_direction(curve, direction)?;
    let mut pending = BTreeMap::new();
    let mut cells = 0;
    let mut lower = 0.0_f64;
    let measure = |span, domain: [f64; 2]| -> Result<(f64, f64)> {
        let width = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
        let derivative = |jets: Vec<Vec<Interval>>| -> Result<Interval> {
            let mut value = project_jet(&jets[derivative_order], &unit)?;
            for _ in 0..derivative_order {
                value = value.div(width)?;
            }
            Ok(value)
        };
        let bound = derivative(curve_jets::enclose(curve, span, domain)?)?;
        let upper = bound.lo.abs().max(bound.hi.abs());
        let sample_lower = |end| -> Result<f64> {
            let sample = derivative(curve_jets::endpoint(curve, span, domain, end)?)?;
            Ok(sample.lo.max(-sample.hi).max(0.))
        };
        Ok((sample_lower(false)?.max(sample_lower(true)?), upper))
    };
    for span in active_spans(curve, interval) {
        if cells == max_cells {
            return Err(resource("Initial derivative spans exceed the work budget"));
        }
        let domain = [
            curve.knots[span].max(interval[0]),
            curve.knots[span + 1].min(interval[1]),
        ];
        let (witness, upper) = measure(span, domain)?;
        lower = lower.max(witness);
        // Nonnegative finite binary64 values have the same order as their bits.
        pending.insert((upper.to_bits(), cells), (span, domain));
        cells += 1;
    }

    let (upper, stop_reason) = loop {
        let (&key, &(span, [a, b])) = pending
            .last_key_value()
            .ok_or_else(|| crate::numeric_err("Missing derivative span coverage"))?;
        let upper = f64::from_bits(key.0).max(lower);
        if (upper - lower).next_up() <= tolerance {
            break (upper, StopReason::Tolerance);
        }
        if cells + 2 > max_cells {
            break (upper, StopReason::WorkLimit);
        }
        let middle = crate::distance_bounds::midpoint(a, b);
        if !(a < middle && middle < b) {
            break (upper, StopReason::PrecisionLimit);
        }
        pending.remove(&key);
        for domain in [[a, middle], [middle, b]] {
            let (witness, child_upper) = measure(span, domain)?;
            lower = lower.max(witness);
            pending.insert((child_upper.min(upper).to_bits(), cells), (span, domain));
            cells += 1;
        }
    };
    let mut parameter_intervals: Vec<[f64; 2]> = Vec::new();
    if let Some(parameter_tolerance) = parameter_tolerance {
        let mut search = pending
            .into_iter()
            .map(|((upper_bits, _), (span, domain))| (f64::from_bits(upper_bits), span, domain))
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        while let Some((bound, span, domain @ [a, b])) = search.pop() {
            if bound < lower {
                continue;
            }
            let middle = crate::distance_bounds::midpoint(a, b);
            if (b - a).next_up() <= parameter_tolerance
                || cells + 2 > max_cells
                || !(a < middle && middle < b)
            {
                candidates.push((bound, domain));
                continue;
            }
            for part in [[a, middle], [middle, b]] {
                let (witness, child_upper) = measure(span, part)?;
                lower = lower.max(witness);
                search.push((child_upper.min(bound), span, part));
                cells += 1;
            }
        }
        let mut candidates = candidates
            .into_iter()
            .filter_map(|(bound, domain)| (bound >= lower).then_some(domain))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| a[0].total_cmp(&b[0]));
        for [start, end] in candidates {
            match parameter_intervals.last_mut() {
                Some([_, last_end]) if start <= *last_end => *last_end = last_end.max(end),
                _ => parameter_intervals.push([start, end]),
            }
        }
    }
    Ok(DirectionalPeakLocations {
        peak: DirectionalPeakSpeedReport {
            bounds: [lower, upper.max(lower)],
            cells,
            stop_reason,
        },
        parameter_intervals,
    })
}
