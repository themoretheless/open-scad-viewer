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
    slowdown_from_peak_bounds(
        [report.speed.bounds, report.acceleration.bounds, report.jerk.bounds],
        limits,
    )
}

fn slowdown_from_peak_bounds(
    bounds: [[f64; 2]; 3],
    limits: DirectionalMotionLimits,
) -> Result<[f64; 2]> {
    use num_rational::BigRational;

    let exact = |value| {
        BigRational::from_float(value)
            .ok_or_else(|| crate::numeric_err("Nonfinite motion bound"))
    };
    let constraints = bounds.into_iter().zip([limits.speed, limits.acceleration, limits.jerk])
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
    let duration_bounds = uniform_motion_duration_bounds(time_window, slowdown.bounds)?;
    Ok(MultiAxisMotionDurationReport { slowdown, duration_bounds })
}

fn uniform_motion_duration_bounds(time_window: [f64; 2], slowdown: [f64; 2]) -> Result<[f64; 2]> {
    let width = exact_motion_operand(time_window[1])? - exact_motion_operand(time_window[0])?;
    let duration_endpoint = |factor: f64, upper: bool| -> Result<f64> {
        if factor == f64::INFINITY {
            return Ok(f64::INFINITY);
        }
        round_motion_duration(&(&width * exact_motion_operand(factor)?), upper)
    };
    Ok([
        duration_endpoint(slowdown[0], false)?,
        duration_endpoint(slowdown[1], true)?,
    ])
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

/// Classify direction against the closed counterclockwise sector from
/// the first ray to the second. Supports both acute and reflex sectors.
/// Coincident rays are ambiguous and rejected. A potentially zero
/// orientation leaves its cell unresolved.
pub fn partition_planar_orientation_sector(
    direction: &Curve,
    interval: [f64; 2],
    rays: [[f64; 2]; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    use DirectionalMotionStatus::{Exceeds, Undetermined, Within};

    direction.validate()?;
    check(direction.control_points[0].len() == 2, "Orientation sectors require a planar direction curve")?;
    let (normals, reflex) = prepare_orientation_sector(rays)?;
    let exact_status = fixed_cardinal_rotation(direction).map(|axis| {
        let inside = normals.map(|normal| {
            // Cardinal axes select exactly one normal component.
            normal[0] * axis[0] + normal[1] * axis[1] >= 0.
        });
        let within = if reflex { inside[0] || inside[1] } else { inside[0] && inside[1] };
        if within { Within } else { Exceeds }
    });
    let normals = normals.map(|normal| normal.map(Interval::point));
    let (cells, work_cells) = partition_motion_cells(
        direction, interval, parameter_tolerance, max_cells,
        |jets, _| {
            if let Some(status) = exact_status {
                return Ok((status, false));
            }
            let status = classify_orientation_sector(&jets[0], &normals, reflex)?;
            Ok((status, status == Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, status)| {
        DirectionalSpeedLimitCell { parameter_interval, span, status }
    }).collect();
    Ok(DirectionalSpeedLimitPartition { cells, work_cells })
}

#[derive(Clone, Copy, Debug)]
pub struct RelativePlanarOrientationCell {
    pub parameter_interval: [f64; 2],
    /// The original spans of the direction and reference, respectively.
    pub spans: [usize; 2],
    pub status: DirectionalMotionStatus,
}

#[derive(Clone, Debug)]
pub struct RelativePlanarOrientationPartition {
    pub cells: Vec<RelativePlanarOrientationCell>,
    pub work_cells: usize,
}

/// Classify the direction in the rotating reference frame. Both curves
/// use the same parameter clock. The sector is measured counterclockwise
/// from its first ray to its second in the reference frame.
pub fn partition_relative_planar_orientation_sector(
    direction: &Curve,
    reference: &Curve,
    interval: [f64; 2],
    rays: [[f64; 2]; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<RelativePlanarOrientationPartition> {
    validate(parameter_tolerance, max_cells)?;
    let (_, initial) = synchronized_clearance_cells([direction, reference], interval, [0.; 2])?;
    check(direction.control_points[0].len() == 2, "Orientation sectors require planar direction curves")?;
    let (normals, reflex) = prepare_orientation_sector(rays)?;
    let normals = normals.map(|normal| normal.map(Interval::point));
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, reference_span], domain| {
            let axis = curve_jets::enclose(direction, span, domain)?;
            let frame = curve_jets::enclose(reference, reference_span, domain)?;
            let axis = &axis[0];
            let frame = &frame[0];
            // The dot and cross components represent the relative angle,
            // with a positive common scale when both directions are nonzero.
            let relative = [
                project_jet(axis, frame)?,
                frame[0].mul(axis[1])?.sub(frame[1].mul(axis[0])?)?,
            ];
            let status = classify_orientation_sector(&relative, &normals, reflex)?;
            Ok((status, status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(spans, parameter_interval, status)| {
        RelativePlanarOrientationCell { parameter_interval, spans, status }
    }).collect();
    Ok(RelativePlanarOrientationPartition { cells, work_cells })
}

#[derive(Clone, Copy, Debug)]
pub struct RelativePlanarAngularMotionCell {
    pub parameter_interval: [f64; 2],
    /// The original spans of the direction and reference, respectively.
    pub spans: [usize; 2],
    /// Classifications of absolute relative speed, acceleration and jerk.
    pub statuses: [DirectionalMotionStatus; 3],
}

#[derive(Clone, Debug)]
pub struct RelativePlanarAngularMotionPartition {
    pub cells: Vec<RelativePlanarAngularMotionCell>,
    pub work_cells: usize,
}

/// Classify angular derivative limits in a rotating planar reference
/// frame. Both curves use the same source parameter. Knot spans use
/// one-sided derivatives and possible zero directions remain unresolved.
pub fn partition_relative_planar_angular_motion_limits(
    direction: &Curve,
    reference: &Curve,
    interval: [f64; 2],
    limits: DirectionalMotionLimits,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<RelativePlanarAngularMotionPartition> {
    use DirectionalMotionStatus::Undetermined;

    validate(parameter_tolerance, max_cells)?;
    let limits = [limits.speed, limits.acceleration, limits.jerk];
    validate_motion_partition_limits(limits)?;
    let (_, initial) = synchronized_clearance_cells([direction, reference], interval, [0.; 2])?;
    check(direction.control_points[0].len() == 2, "Angular derivatives require planar direction curves")?;
    let fixed = [direction, reference].map(|curve| fixed_cardinal_rotation(curve).is_some());
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, reference_span], domain| {
            let width = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
            let angular = |curve, span, fixed| {
                if fixed {
                    return Ok(Some([Interval::point(0.); 3]));
                }
                let jets = curve_jets::enclose(curve, span, domain)?;
                planar_angular_jets(&jets, width, 3)
            };
            let (Some(moving), Some(frame)) = (
                angular(direction, span, fixed[0])?,
                angular(reference, reference_span, fixed[1])?,
            ) else {
                return Ok(([Undetermined; 3], true));
            };
            if fixed == [true; 2] {
                return Ok(([DirectionalMotionStatus::Within; 3], false));
            }
            let mut statuses = [Undetermined; 3];
            for i in 0..3 {
                statuses[i] = classify_absolute_motion(moving[i].sub(frame[i])?, limits[i]);
            }
            Ok((statuses, statuses.contains(&Undetermined)))
        },
    )?;
    let cells = cells.into_iter().map(|(spans, parameter_interval, statuses)| {
        RelativePlanarAngularMotionCell { parameter_interval, spans, statuses }
    }).collect();
    Ok(RelativePlanarAngularMotionPartition { cells, work_cells })
}

fn prepare_orientation_sector(rays: [[f64; 2]; 2]) -> Result<([[f64; 2]; 2], bool)> {
    let [start, end] = rays;
    check(rays.iter().all(|ray| ray.iter().all(|x| x.is_finite()) && *ray != [0.; 2]),
        "Sector rays must be finite and nonzero")?;
    let determinant = exact_motion_operand(start[0])? * exact_motion_operand(end[1])?
        - exact_motion_operand(start[1])? * exact_motion_operand(end[0])?;
    let zero = exact_motion_operand(0.)?;
    if determinant == zero {
        let alignment = exact_motion_operand(start[0])? * exact_motion_operand(end[0])?
            + exact_motion_operand(start[1])? * exact_motion_operand(end[1])?;
        check(alignment < zero, "Sector boundary rays must not coincide")?;
    }
    let reflex = determinant < zero;
    let normals = [[-start[1], start[0]], [end[1], -end[0]]];
    Ok((normals, reflex))
}

fn classify_orientation_sector(
    axis: &[Interval],
    normals: &[[Interval; 2]; 2],
    reflex: bool,
) -> Result<DirectionalMotionStatus> {
    use DirectionalMotionStatus::{Exceeds, Undetermined, Within};
    if axis.iter().all(|value| value.lo <= 0. && value.hi >= 0.) {
        return Ok(Undetermined);
    }
    let bounds = [project_jet(axis, &normals[0])?, project_jet(axis, &normals[1])?];
    let inside = bounds.map(|value| value.lo >= 0.);
    let outside = bounds.map(|value| value.hi < 0.);
    let (within, exceeds) = if reflex {
        (inside[0] || inside[1], outside[0] && outside[1])
    } else {
        (inside[0] && inside[1], outside[0] || outside[1])
    };
    let status = if within { Within } else if exceeds { Exceeds } else { Undetermined };
    Ok(status)
}

/// Classify absolute angular speed of a planar direction curve against an
/// inclusive limit in radians per unit of the curve parameter. Each knot
/// span is evaluated on its own side. The direction must stay nonzero to
/// certify a cell, and jumps between spans are outside this derivative test.
pub fn partition_planar_angular_speed_limit(
    direction: &Curve,
    interval: [f64; 2],
    limit: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    partition_planar_angular_limit(
        direction, interval, limit, parameter_tolerance, max_cells, 1,
    )
}

/// Classify absolute angular acceleration in radians per squared unit of
/// the curve parameter. Each span uses its one-sided derivatives, and a
/// cell containing a potentially zero direction is unresolved.
pub fn partition_planar_angular_acceleration_limit(
    direction: &Curve,
    interval: [f64; 2],
    limit: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    partition_planar_angular_limit(
        direction, interval, limit, parameter_tolerance, max_cells, 2,
    )
}

/// Classify absolute angular jerk in radians per cubed unit of the curve
/// parameter. Knot spans are checked independently and possible zero
/// directions leave their cells unresolved.
pub fn partition_planar_angular_jerk_limit(
    direction: &Curve,
    interval: [f64; 2],
    limit: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    partition_planar_angular_limit(
        direction, interval, limit, parameter_tolerance, max_cells, 3,
    )
}

fn partition_planar_angular_limit(
    direction: &Curve,
    interval: [f64; 2],
    limit: f64,
    parameter_tolerance: f64,
    max_cells: usize,
    order: usize,
) -> Result<DirectionalSpeedLimitPartition> {
    let (cells, work_cells) = partition_planar_angular_derivatives(
        direction, interval, [limit], [order], parameter_tolerance, max_cells,
    )?;
    let cells = cells.into_iter().map(|cell| DirectionalSpeedLimitCell {
        parameter_interval: cell.parameter_interval,
        span: cell.span,
        status: cell.statuses[0],
    }).collect();
    Ok(DirectionalSpeedLimitPartition { cells, work_cells })
}

/// Classify absolute angular speed, acceleration and jerk on one shared
/// subdivision, in units of the original parameter. Subdivision continues
/// while any limit is unresolved. One-sided derivatives apply at knots.
pub fn partition_planar_angular_motion_limits(
    direction: &Curve,
    interval: [f64; 2],
    limits: DirectionalMotionLimits,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalMotionLimitPartition> {
    let (cells, work_cells) = partition_planar_angular_derivatives(
        direction, interval, [limits.speed, limits.acceleration, limits.jerk],
        [1, 2, 3], parameter_tolerance, max_cells,
    )?;
    Ok(DirectionalMotionLimitPartition { cells, work_cells })
}

fn partition_planar_angular_derivatives<const N: usize>(
    direction: &Curve,
    interval: [f64; 2],
    limits: [f64; N],
    orders: [usize; N],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<(Vec<DirectionalMotionLimitCell<N>>, usize)> {
    use DirectionalMotionStatus::{Undetermined, Within};

    validate_motion_partition_limits(limits)?;
    direction.validate()?;
    check(direction.control_points[0].len() == 2, "Angular derivatives require a planar direction curve")?;
    let fixed = fixed_cardinal_rotation(direction).is_some();
    let order = orders.into_iter().max().unwrap_or(1);
    let (cells, work_cells) = partition_motion_cells(
        direction, interval, parameter_tolerance, max_cells,
        |jets, width| {
            if fixed {
                return Ok(([Within; N], false));
            }
            let Some(values) = planar_angular_jets(jets, width, order)? else {
                return Ok(([Undetermined; N], true));
            };
            let mut statuses = [Undetermined; N];
            for ((status, order), limit) in statuses.iter_mut().zip(orders).zip(limits) {
                let value = values[order - 1];
                *status = classify_absolute_motion(value, limit);
            }
            Ok((statuses, statuses.contains(&Undetermined)))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, statuses)| {
        DirectionalMotionLimitCell { parameter_interval, span, statuses }
    }).collect();
    Ok((cells, work_cells))
}

/// Enclose the minimum uniform parameter dilation satisfying all three
/// angular limits. A finite upper bound certifies feasibility, while an
/// infinite upper bound can indicate unresolved orientation or an
/// unattainable limit. Jumps between knot spans are outside this check.
pub fn planar_angular_motion_slowdown_bounds(
    direction: &Curve,
    interval: [f64; 2],
    limits: DirectionalMotionLimits,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<[f64; 2]> {
    validate_motion_partition_limits([limits.speed, limits.acceleration, limits.jerk])?;
    let (peaks, unresolved) = planar_angular_peak_bounds(
        direction, interval, parameter_tolerance, max_cells,
    )?;
    let mut bounds = slowdown_from_peak_bounds(peaks, limits)?;
    if unresolved {
        bounds[1] = f64::INFINITY;
    }
    Ok(bounds)
}

fn planar_angular_peak_bounds(
    direction: &Curve,
    interval: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<([[f64; 2]; 3], bool)> {
    direction.validate()?;
    check(direction.control_points[0].len() == 2, "Angular derivatives require a planar direction curve")?;
    let fixed = fixed_cardinal_rotation(direction).is_some();
    let (cells, _) = partition_motion_cells(
        direction, interval, parameter_tolerance, max_cells,
        |jets, width| {
            if fixed {
                return Ok((Some([Interval::point(0.); 3]), false));
            }
            Ok((planar_angular_jets(jets, width, 3)?, true))
        },
    )?;
    let mut peaks = [[0_f64; 2]; 3];
    let mut unresolved = false;
    for (_, _, values) in cells {
        if let Some(values) = values {
            for (peak, value) in peaks.iter_mut().zip(values) {
                peak[0] = peak[0].max(value.lo.max(-value.hi));
                peak[1] = peak[1].max(value.lo.abs().max(value.hi.abs()));
            }
        } else {
            unresolved = true;
        }
    }
    Ok((peaks, unresolved))
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarAngularMotionDeadlineReport {
    pub status: DirectionalMotionStatus,
    /// Speed, acceleration and jerk in that order. The no-speedup rule
    /// affects the overall status.
    pub limit_statuses: [DirectionalMotionStatus; 3],
}

/// Check an inclusive deadline for a uniform traversal with slowdown only.
/// Compare the angular derivative bounds against the proposed clock exactly.
pub fn check_planar_angular_motion_deadline(
    direction: &Curve,
    time_window: [f64; 2],
    limits: DirectionalMotionLimits,
    deadline: f64,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarAngularMotionDeadlineReport> {
    use DirectionalMotionStatus::{Exceeds, Undetermined, Within};

    check(deadline.is_finite() && deadline > 0., "Deadline must be positive and finite")?;
    let limits = [limits.speed, limits.acceleration, limits.jerk];
    validate_motion_partition_limits(limits)?;
    let (peaks, unresolved) = planar_angular_peak_bounds(
        direction, time_window, parameter_tolerance, max_cells,
    )?;
    let factor = motion_clock_factor(time_window, deadline)?;
    let mut limit_statuses = [Undetermined; 3];
    let mut power = factor.clone();
    for (index, ([lo, hi], limit)) in peaks.into_iter().zip(limits).enumerate() {
        let allowed = &power * exact_motion_operand(limit)?;
        limit_statuses[index] = if exact_motion_operand(lo)? > allowed {
            Exceeds
        } else if !unresolved && exact_motion_operand(hi)? <= allowed {
            Within
        } else {
            Undetermined
        };
        power *= &factor;
    }
    let status = if factor < exact_motion_operand(1.)? || limit_statuses.contains(&Exceeds) {
        Exceeds
    } else if limit_statuses.contains(&Undetermined) {
        Undetermined
    } else {
        Within
    };
    Ok(PlanarAngularMotionDeadlineReport { status, limit_statuses })
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarAngularMotionDurationReport {
    /// Minimum uniform dilation, constrained to at least one.
    pub slowdown_bounds: [f64; 2],
    /// Minimum duration in source time units. A finite upper bound is
    /// sufficient to satisfy the angular limits for a uniform traversal.
    pub duration_bounds: [f64; 2],
}

/// Enclose the uniform traversal duration under planar angular limits.
/// The input parameter must use elapsed time for physical time units.
pub fn planar_angular_motion_duration_bounds(
    direction: &Curve,
    time_window: [f64; 2],
    limits: DirectionalMotionLimits,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarAngularMotionDurationReport> {
    let slowdown_bounds = planar_angular_motion_slowdown_bounds(
        direction, time_window, limits, parameter_tolerance, max_cells,
    )?;
    let duration_bounds = uniform_motion_duration_bounds(time_window, slowdown_bounds)?;
    Ok(PlanarAngularMotionDurationReport { slowdown_bounds, duration_bounds })
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarAngularTravelReport {
    /// Signed integral of angular speed in radians over the span interiors.
    pub net_bounds: [f64; 2],
    /// Integral of absolute angular speed in radians.
    pub total_bounds: [f64; 2],
    pub work_cells: usize,
}

/// Bound signed and total angular travel without wrapping angles at a
/// branch cut. The integration covers each original knot span separately
/// and excludes orientation jumps. Every retained cell must certify a
/// nonzero direction.
pub fn measure_planar_angular_travel(
    direction: &Curve,
    interval: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarAngularTravelReport> {
    measure_planar_angular_travel_from(direction, interval, None, parameter_tolerance, max_cells)
}

fn measure_planar_angular_travel_from(
    direction: &Curve,
    interval: [f64; 2],
    origin: Option<[Interval; 2]>,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarAngularTravelReport> {
    direction.validate()?;
    check(direction.control_points[0].len() == 2, "Angular travel requires a planar direction curve")?;
    let fixed = origin.is_none() && fixed_cardinal_rotation(direction).is_some();
    let region = origin.filter(|bounds| bounds.iter().any(|bound| bound.lo < bound.hi));
    let center = origin.map(|bounds| bounds.map(|bound| {
        let middle = crate::distance_bounds::midpoint(bound.lo, bound.hi);
        Interval::point(middle.clamp(bound.lo, bound.hi))
    }));
    let (cells, work_cells) = partition_motion_cells(
        direction, interval, parameter_tolerance, max_cells,
        |jets, _| {
            if fixed {
                return Ok((Some(Interval::point(0.)), false));
            }
            // The jet derivative is already in the local clock on [0, 1].
            // Its range also encloses its integral over the unit interval.
            if let Some(region) = region {
                let separated = jets[0].iter().zip(region).any(|(coordinate, bound)| {
                    coordinate.hi < bound.lo || coordinate.lo > bound.hi
                });
                if !separated {
                    return Ok((None, true));
                }
            }
            let shifted;
            let jets = if let Some(origin) = center {
                let mut translated = jets.to_vec();
                for (coordinate, offset) in translated[0].iter_mut().zip(origin) {
                    *coordinate = coordinate.sub(offset)?;
                }
                shifted = translated;
                &shifted
            } else {
                jets
            };
            let local = planar_angular_jets(jets, Interval::point(1.), 1)?;
            Ok((local.map(|values| values[0]), true))
        },
    )?;
    if fixed {
        return Ok(PlanarAngularTravelReport {
            net_bounds: [0.; 2], total_bounds: [0.; 2], work_cells,
        });
    }
    accumulate_angular_travel(cells.into_iter().map(|(_, _, speed)| speed), work_cells)
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarOrientationWindingReport {
    /// Positive winding is counterclockwise. None means more than one
    /// integer is compatible with the current angular enclosure.
    pub winding: Option<i64>,
    pub travel: PlanarAngularTravelReport,
}

/// Certify winding over the complete domain of a clamped planar direction
/// curve. The endpoint rays must agree exactly and interior knots must
/// guarantee positional continuity. Direction magnitude may vary.
pub fn measure_planar_orientation_winding(
    direction: &Curve,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarOrientationWindingReport> {
    let domain = validate_winding_curve(direction)?;
    let last = direction.control_points.len();
    let first = &direction.control_points[0];
    let end = &direction.control_points[last - 1];
    let [ax, ay, bx, by] = [
        exact_motion_operand(first[0])?, exact_motion_operand(first[1])?,
        exact_motion_operand(end[0])?, exact_motion_operand(end[1])?,
    ];
    let zero = exact_motion_operand(0.)?;
    check(
        &ax * &by == &ay * &bx && ax * bx + ay * by > zero,
        "Orientation winding requires identical nonzero endpoint rays",
    )?;
    let travel = measure_planar_angular_travel(direction, domain, parameter_tolerance, max_cells)?;
    winding_from_angular_travel(travel)
}

/// Measure the signed winding of a closed planar contour around a point.
/// The curve must be clamped and continuous, with equal endpoints.
/// A point on or too close to the curve for separation to be certified
/// prevents a winding report.
pub fn measure_planar_curve_winding_about_point(
    curve: &Curve,
    point: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarOrientationWindingReport> {
    check(point.iter().all(|value| value.is_finite()), "Winding point must be finite")?;
    let domain = validate_winding_curve(curve)?;
    check(curve.control_points.first() == curve.control_points.last(), "Winding contour must be closed")?;
    let origin = Some(point.map(Interval::point));
    let travel = measure_planar_angular_travel_from(
        curve, domain, origin, parameter_tolerance, max_cells,
    )?;
    winding_from_angular_travel(travel)
}

/// Certify a common winding for all points in a closed axis-aligned box.
/// Bounds are given as lower and upper corners. Every curve cell must be
/// separated from the box. The travel bounds in the report are for the
/// box center, whose winding agrees with that of every point in the box.
pub fn measure_planar_curve_winding_over_box(
    curve: &Curve,
    bounds: [[f64; 2]; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarOrientationWindingReport> {
    let [[x_min, y_min], [x_max, y_max]] = bounds;
    let region = [Interval::new(x_min, x_max)?, Interval::new(y_min, y_max)?];
    let domain = validate_winding_curve(curve)?;
    check(curve.control_points.first() == curve.control_points.last(), "Winding contour must be closed")?;
    let travel = measure_planar_angular_travel_from(
        curve, domain, Some(region), parameter_tolerance, max_cells,
    )?;
    winding_from_angular_travel(travel)
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarCurveFillReport {
    /// Sum of the oriented winding numbers. Uncertain contours leave the
    /// sum and the fill decision unresolved.
    pub winding: Option<i32>,
    pub filled: Option<bool>,
    pub work_cells: usize,
}

/// Classify a query point against a set of closed planar NURBS contours.
/// Reserve the initial knot cells of every contour before sharing the
/// remaining refinement budget. All contours must be separated from the
/// query point for a result.
pub fn classify_planar_curve_fill(
    contours: &[&Curve],
    point: [f64; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarCurveFillReport> {
    check(point.iter().all(|value| value.is_finite()), "Winding point must be finite")?;
    classify_planar_curve_fill_for_region(
        contours, point.map(Interval::point), rule, parameter_tolerance, max_cells,
    )
}

/// Certify the same fill classification for every point in a closed
/// axis-aligned box given by its lower and upper corners. A contour
/// intersecting or touching the box prevents certification.
pub fn classify_planar_curve_fill_over_box(
    contours: &[&Curve],
    bounds: [[f64; 2]; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarCurveFillReport> {
    let [[x_min, y_min], [x_max, y_max]] = bounds;
    let region = [Interval::new(x_min, x_max)?, Interval::new(y_min, y_max)?];
    classify_planar_curve_fill_for_region(contours, region, rule, parameter_tolerance, max_cells)
}

fn classify_planar_curve_fill_for_region(
    contours: &[&Curve],
    region: [Interval; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarCurveFillReport> {
    validate(parameter_tolerance, max_cells)?;
    check(!contours.is_empty(), "Curve fill requires at least one contour")?;
    check(contours.len() <= max_cells, "Curve fill budget must cover every contour")?;
    let mut initial_work = 0;
    let mut span_counts = Vec::with_capacity(contours.len());
    for &curve in contours {
        let domain = validate_winding_curve(curve)?;
        check(curve.control_points.first() == curve.control_points.last(), "Winding contour must be closed")?;
        let spans = active_spans(curve, domain).count();
        if spans > max_cells - initial_work {
            return Err(resource("Curve fill budget must cover every original knot span"));
        }
        initial_work += spans;
        span_counts.push(spans);
    }

    let mut extra = max_cells - initial_work;
    let mut work_cells = 0;
    let mut winding = Some(0_i32);
    for (index, (&curve, spans)) in contours.iter().zip(span_counts).enumerate() {
        let share = extra / (contours.len() - index);
        let travel = measure_planar_angular_travel_from(
            curve, curve.domain(), Some(region), parameter_tolerance, spans + share,
        )?;
        work_cells += travel.work_cells;
        extra -= travel.work_cells.saturating_sub(spans);
        let report = winding_from_angular_travel(travel)?;
        winding = match (winding, report.winding) {
            (Some(sum), Some(turns)) => {
                let turns = i32::try_from(turns).map_err(|_| resource("Contour winding exceeds integer range"))?;
                Some(sum.checked_add(turns).ok_or_else(|| resource("Total winding exceeds integer range"))?)
            }
            _ => None,
        };
    }
    Ok(PlanarCurveFillReport {
        winding,
        filled: winding.map(|turns| crate::chord_winding::filled(turns, rule)),
        work_cells,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarCurveSignedDistanceReport {
    /// Negative inside the filled region, positive outside. The magnitude
    /// measures distance to the nearest supplied contour.
    pub signed_bounds: [f64; 2],
    pub fill: PlanarCurveFillReport,
    pub witness_contour: usize,
    pub witness_parameter: f64,
    pub distance_cells: usize,
    pub distance_stop_reason: StopReason,
}

/// Measure distance to the nearest closed NURBS contour and sign it using
/// the selected fill rule. Contours that cancel in the fill still count
/// toward distance. Each stage has its own explicit cell budget.
pub fn measure_planar_curve_fill_signed_distance(
    contours: &[&Curve],
    point: [f64; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_fill_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarCurveSignedDistanceReport> {
    validate(distance_tolerance, max_distance_cells)?;
    let fill = classify_planar_curve_fill(contours, point, rule, parameter_tolerance, max_fill_cells)?;
    measure_signed_contour_distance(contours, point, fill, distance_tolerance, max_distance_cells)
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarCurveBoxSignedDistanceReport {
    /// Bounds covering the signed nearest contour distance at every point
    /// in the query box.
    pub signed_bounds: [f64; 2],
    pub center: PlanarCurveSignedDistanceReport,
}

/// Bound the signed nearest contour distance throughout a separated
/// axis-aligned box. The fill is certified over the entire box. The
/// center distance is expanded by a bound on the box's radius.
pub fn measure_planar_curve_fill_signed_distance_over_box(
    contours: &[&Curve],
    bounds: [[f64; 2]; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_fill_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarCurveBoxSignedDistanceReport> {
    validate(distance_tolerance, max_distance_cells)?;
    let fill = classify_planar_curve_fill_over_box(contours, bounds, rule, parameter_tolerance, max_fill_cells)?;
    let region = [Interval::new(bounds[0][0], bounds[1][0])?, Interval::new(bounds[0][1], bounds[1][1])?];
    let point = region.map(|axis| {
        crate::distance_bounds::midpoint(axis.lo, axis.hi).clamp(axis.lo, axis.hi)
    });
    let center = measure_signed_contour_distance(contours, point, fill, distance_tolerance, max_distance_cells)?;
    let [lo, hi] = center.signed_bounds;
    let signed_bounds = if bounds[0] == bounds[1] {
        [lo, hi]
    } else {
        let (_, radius) = box_distance(&region, &point.map(Interval::point))?;
        let expanded = Interval::new(lo, hi)?.add(Interval::new(-radius, radius)?)?;
        match fill.filled {
            Some(true) => [expanded.lo, expanded.hi.min(0.)],
            Some(false) => [expanded.lo.max(0.), expanded.hi],
            None => [expanded.lo, expanded.hi],
        }
    };
    Ok(PlanarCurveBoxSignedDistanceReport { signed_bounds, center })
}

/// Bound signed distance to the actual boundary of a certified filled
/// region. Contours across which fill does not change are ignored.
/// The witness contour index addresses the original input. The fill
/// report's winding counts only contours active under the chosen rule.
pub fn measure_planar_contour_fill_boundary_signed_distance(
    contours: &[&Curve],
    point: [f64; 2],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_separation_cells: usize,
    max_orientation_cells: usize,
    max_fill_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarCurveSignedDistanceReport> {
    validate(distance_tolerance, max_distance_cells)?;
    check(point.iter().all(|value| value.is_finite()), "Distance point must be finite")?;
    let (_, transitions) = certify_contour_fill_transitions(
        contours, rule, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_separation_cells, max_orientation_cells,
    )?;
    let indices: Vec<_> = transitions.into_iter().enumerate()
        .filter_map(|(index, value)| (value != 0).then_some(index)).collect();
    let active: Vec<_> = indices.iter().map(|&index| contours[index]).collect();
    // Every retained contour toggles fill, even for a nonzero source rule.
    let mut report = measure_planar_curve_fill_signed_distance(
        &active, point, crate::chord_winding::FillRule::EvenOdd, parameter_tolerance,
        distance_tolerance, max_fill_cells, max_distance_cells,
    )?;
    report.witness_contour = indices[report.witness_contour];
    Ok(report)
}

fn measure_signed_contour_distance(
    contours: &[&Curve],
    point: [f64; 2],
    fill: PlanarCurveFillReport,
    distance_tolerance: f64,
    max_distance_cells: usize,
) -> Result<PlanarCurveSignedDistanceReport> {
    let point = point.map(Interval::point);
    let nearest = measure_contour_set_distance(contours, distance_tolerance, max_distance_cells, |cell| {
        let (lo, hi) = box_distance(cell, &point)?;
        Interval::new(lo, hi)
    })?;
    let [lower, upper] = nearest.bounds;
    let signed_bounds = match fill.filled {
        Some(true) => [-upper, -lower],
        Some(false) => [lower, upper],
        None => [-upper, upper],
    };
    Ok(PlanarCurveSignedDistanceReport {
        signed_bounds, fill, witness_contour: nearest.witness_contour,
        witness_parameter: nearest.witness_parameter,
        distance_cells: nearest.work_cells, distance_stop_reason: nearest.stop_reason,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarContourDistanceReport {
    pub bounds: [f64; 2],
    pub witness_contour: usize,
    pub witness_parameter: f64,
    pub work_cells: usize,
    pub stop_reason: StopReason,
}

/// Measure the minimum unsigned distance from a closed axis-aligned box
/// to a set of closed planar NURBS contours. A zero upper bound certifies
/// intersection with the box. The work budget is shared across contours.
pub fn measure_planar_contour_set_box_distance(
    contours: &[&Curve],
    bounds: [[f64; 2]; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarContourDistanceReport> {
    validate(tolerance, max_cells)?;
    let [minimum, maximum] = bounds;
    for (lo, hi) in minimum.into_iter().zip(maximum) {
        Interval::new(lo, hi)?;
    }
    validate_distance_contours(contours, max_cells)?;
    measure_contour_set_distance(contours, tolerance, max_cells, |cell| {
        solid_box_gap(cell, &minimum, &maximum, 0.)
    })
}

/// Bound the minimum unsigned distance between a closed disk and a set
/// of closed planar NURBS contours. A zero upper bound certifies that a
/// contour meets the disk, including its interior.
pub fn measure_planar_contour_set_disk_distance(
    contours: &[&Curve],
    center: [f64; 2],
    radius: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarContourDistanceReport> {
    validate(tolerance, max_cells)?;
    check(center.iter().all(|value| value.is_finite()), "Disk center must be finite")?;
    check(radius.is_finite() && radius >= 0., "Disk radius must be finite and nonnegative")?;
    validate_distance_contours(contours, max_cells)?;
    let center = center.map(Interval::point);
    let radius = Interval::point(radius);
    measure_contour_set_distance(contours, tolerance, max_cells, |cell| {
        let (lo, hi) = box_distance(cell, &center)?;
        let gap = Interval::new(lo, hi)?.sub(radius)?;
        Interval::new(gap.lo.max(0.), gap.hi.max(0.))
    })
}

/// Bound the minimum unsigned distance between a closed capsule and a
/// set of planar NURBS contours. A capsule consists of a segment and
/// its fixed-radius disk expansion. A zero upper bound certifies contact.
pub fn measure_planar_contour_set_capsule_distance(
    contours: &[&Curve],
    capsule: CapsuleObstacle<'_>,
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarContourDistanceReport> {
    validate(tolerance, max_cells)?;
    validate_distance_contours(contours, max_cells)?;
    let prepared = prepare_capsules(contours[0], &[capsule], 0.)?;
    measure_contour_set_distance(contours, tolerance, max_cells, |cell| {
        let gap = prepared[0].gap(cell)?;
        Interval::new(gap.lo.max(0.), gap.hi.max(0.))
    })
}

/// Measure the minimum unsigned distance between an oriented rectangle
/// and a set of closed planar contours. The axes must be orthogonal, but
/// do not need unit length. A zero upper bound certifies contact.
pub fn measure_planar_contour_set_oriented_box_distance(
    contours: &[&Curve],
    rectangle: OrientedBoxObstacle<'_>,
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarContourDistanceReport> {
    validate(tolerance, max_cells)?;
    validate_distance_contours(contours, max_cells)?;
    let prepared = prepare_oriented_boxes(contours[0], &[rectangle])?;
    measure_contour_set_distance(contours, tolerance, max_cells, |cell| {
        prepared[0].gap(cell, 0.)
    })
}

#[derive(Clone, Copy, Debug)]
pub struct ContourPairDistanceReport {
    pub bounds: [f64; 2],
    /// Contour index and source parameter in each input set.
    pub witness_contours: [usize; 2],
    pub witness_parameters: [f64; 2],
    pub work_cells: usize,
    pub converged: bool,
}

pub type PlanarContourPairDistanceReport = ContourPairDistanceReport;

/// Bound the minimum distance between two sets of closed planar NURBS
/// contours. Every original knot-span pair receives a cell before
/// refinement work is distributed across the contour pairs.
pub fn measure_planar_contour_sets_distance(
    first: &[&Curve],
    second: &[&Curve],
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarContourPairDistanceReport> {
    validate(tolerance, max_cells)?;
    validate_distance_contours(first, max_cells)?;
    validate_distance_contours(second, max_cells)?;
    measure_validated_contour_pairs(first, second, tolerance, max_cells, false)
}

/// Enclose the minimum 3D distance between two collections of clamped,
/// continuous closed spatial curves, with witnesses in source parameters.
pub fn measure_spatial_contour_sets_distance(
    first: &[&Curve], second: &[&Curve], tolerance: f64, max_cells: usize,
) -> Result<ContourPairDistanceReport> {
    validate(tolerance,max_cells)?;
    validate_spatial_distance_set(first, max_cells)?;
    validate_spatial_distance_set(second, max_cells)?;
    measure_validated_contour_pairs(first, second, tolerance, max_cells, false)
}

fn validate_spatial_distance_set(set: &[&Curve], max_cells: usize) -> Result<()> {
    check(!set.is_empty() && set.len() <= max_cells, "Expected bounded nonempty spatial contour set")?;
    for curve in set {
        validate_continuous_clamped_curve(curve, 3)?;
        check(curve.control_points.first() == curve.control_points.last(), "Spatial contour must be closed")?;
    }
    Ok(())
}

/// Bound clearance between distinct members of one collection. Witness
/// indices are ordered. Two coincident entries are distinct members and
/// therefore can attain zero clearance.
pub fn closest_spatial_contour_pair(
    contours: &[&Curve], tolerance: f64, max_cells: usize,
) -> Result<ContourPairDistanceReport> {
    validate(tolerance, max_cells)?;
    check(contours.len() >= 2, "Closest pair needs at least two contours")?;
    validate_spatial_distance_set(contours, max_cells)?;
    measure_validated_contour_pairs(contours, contours, tolerance, max_cells, true)
}

fn measure_validated_contour_pairs(
    first: &[&Curve], second: &[&Curve], tolerance: f64, max_cells: usize, distinct_members: bool,
) -> Result<ContourPairDistanceReport> {
    let mut report = PlanarContourPairDistanceReport {
        bounds: [f64::INFINITY; 2],
        witness_contours: [0; 2],
        witness_parameters: [0.; 2],
        work_cells: 0,
        converged: false,
    };
    visit_contour_pairs(first, second, tolerance, max_cells, distinct_members, |[i,j], distance| {
            report.work_cells += distance.cells;
            report.bounds[0] = report.bounds[0].min(distance.distance_interval_mm[0]);
            if distance.distance_interval_mm[1] < report.bounds[1] {
                report.bounds[1] = distance.distance_interval_mm[1];
                report.witness_contours = [i, j];
                report.witness_parameters = distance.parameters;
            }
            Ok(())
    })?;
    report.converged = (report.bounds[1] - report.bounds[0]).next_up() <= tolerance;
    Ok(report)
}

fn visit_contour_pairs(
    first: &[&Curve], second: &[&Curve], tolerance: f64, max_cells: usize, distinct_members: bool,
    mut visit: impl FnMut([usize;2], crate::curve_distance::CurveDistance) -> Result<()>,
) -> Result<()> {
    let counts = [first, second].map(|set| {
        set.iter().map(|curve| active_spans(curve, curve.domain()).count()).collect::<Vec<_>>()
    });
    let start = |i| if distinct_members { i + 1 } else { 0 };
    let mut extra = max_cells;
    let mut pairs = 0;
    for (i, &left) in counts[0].iter().enumerate() {
        for &right in &counts[1][start(i)..] {
            extra = extra.checked_sub(left * right)
                .ok_or_else(|| resource("Contour distance budget must cover every original span pair"))?;
            pairs += 1;
        }
    }
    for (i, a) in first.iter().enumerate() {
        for (j, b) in second.iter().enumerate().skip(start(i)) {
            let reserved = counts[0][i] * counts[1][j];
            let distance = crate::curve_distance::distance(a, b, tolerance, reserved + extra / pairs)?;
            pairs -= 1;
            extra -= distance.cells.saturating_sub(reserved);
            visit([i,j],distance)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearanceDecision { MeetsMinimum, BelowMinimum, Unresolved }

#[derive(Debug, Clone)]
pub struct ContourClearanceCheck {
    pub contours: [usize;2],
    pub bounds: [f64;2],
    pub witness_parameters: [f64;2],
    pub cells: usize,
    pub decision: ClearanceDecision,
}

// Positive-weight rational curves stay inside the box of their Cartesian
// poles. Compare squared box separation in exact binary64-input arithmetic
// so that an inclusive clearance can be certified even at equality.
fn spatial_control_box(curve: &Curve) -> [[f64;2];3] {
    std::array::from_fn(|axis| curve.control_points.iter().fold(
        [f64::INFINITY, f64::NEG_INFINITY],
        |[lo,hi], pole| [lo.min(pole[axis]), hi.max(pole[axis])],
    ))
}

fn box_proves_clearance(a: &[[f64;2];3], b: &[[f64;2];3], minimum: f64) -> bool {
    use num_rational::BigRational;
    // Validation of the curves and threshold guarantees finite inputs.
    let exact = |x| BigRational::from_float(x).expect("validated finite coordinate");
    let mut squared = exact(0.);
    for (left,right) in a.iter().zip(b) {
        let gap = if left[1] < right[0] {exact(right[0])-exact(left[1])}
            else if right[1] < left[0] {exact(left[0])-exact(right[1])}
            else {continue};
        squared += &gap * &gap;
    }
    let threshold = exact(minimum);
    squared >= &threshold * &threshold
}

// Compare a known pair of on-curve points enclosed by outward coordinate
// intervals. Use the farthest opposite faces on each axis for an upper proof.
fn witnesses_below_clearance(a: &[[f64;2]], b: &[[f64;2]], minimum: f64) -> bool {
    use num_rational::BigRational;
    use num_traits::Signed;
    let exact = |x| BigRational::from_float(x).expect("finite witness coordinate");
    let mut squared = exact(0.);
    for (left,right) in a.iter().zip(b) {
        let delta = (exact(left[0])-exact(right[1])).abs()
            .max((exact(left[1])-exact(right[0])).abs());
        squared += &delta * &delta;
    }
    let limit = exact(minimum);
    squared < &limit * &limit
}

/// Sufficient clearance certificate from exact control-pole projections on
/// a supplied separating normal. A false result only means the direction
/// did not prove the threshold. The normal need not have unit length.
pub fn prove_spatial_contour_clearance_along(
    first: &Curve, second: &Curve, normal: [f64; 3], minimum: f64,
) -> Result<bool> {
    use num_rational::BigRational as Q;
    validate_spatial_distance_set(&[first, second], 2)?;
    check(minimum.is_finite() && minimum >= 0., "Expected finite nonnegative clearance")?;
    check(normal.iter().all(|v| v.is_finite()) && normal.iter().any(|&v| v != 0.),
          "Expected finite nonzero separating normal")?;
    let exact = |x| Q::from_float(x).expect("validated finite input");
    Ok(projected_support_proves(first, second, normal.map(exact), minimum))
}

fn projected_support_proves(first: &Curve, second: &Curve, n: [num_rational::BigRational;3], minimum: f64) -> bool {
    pole_support_proves(first.control_points.iter(), second.control_points.iter(), n, minimum)
}

fn pole_support_proves<'a>(
    first: impl Iterator<Item=&'a Vec<f64>>, second: impl Iterator<Item=&'a Vec<f64>>,
    n: [num_rational::BigRational;3], minimum: f64,
) -> bool {
    use num_rational::BigRational as Q;
    use num_traits::Zero;
    if n.iter().all(Q::is_zero) { return false; }
    let exact = |x| Q::from_float(x).expect("validated finite input");
    let support = |poles: &mut dyn Iterator<Item=&Vec<f64>>| {
        let mut projections = poles.map(|p| {
            n.iter().zip(p).map(|(a,&b)| a * exact(b)).sum::<Q>()
        });
        let first = projections.next().expect("validated nonempty control net");
        projections.fold((first.clone(),first), |(lo,hi),x| (lo.min(x.clone()),hi.max(x)))
    };
    let (alo,ahi) = support(&mut first.into_iter());
    let (blo,bhi) = support(&mut second.into_iter());
    let gap = (blo-ahi).max(alo-bhi).max(Q::zero());
    let squared_norm = n.iter().map(|a| a*a).sum::<Q>();
    let threshold = exact(minimum);
    &gap * &gap >= &threshold * &threshold * squared_norm
}

// The point differences only propose normals. Even a rounded witness can
// suggest a valid direction: acceptance uses the entire exact support ranges.
fn automatic_support_proves(first: &Curve, second: &Curve, points: &[Vec<f64>;2], minimum: f64) -> bool {
    let direction = |a: &[f64], b: &[f64]| std::array::from_fn(|i|
        num_rational::BigRational::from_float(b[i]).unwrap()
        - num_rational::BigRational::from_float(a[i]).unwrap());
    let seam = direction(&first.control_points[0], &second.control_points[0]);
    if projected_support_proves(first,second,seam.clone(),minimum) { return true; }
    if points.iter().flatten().any(|v| !v.is_finite()) { return false; }
    let candidate = direction(&points[0], &points[1]);
    candidate != seam && projected_support_proves(first,second,candidate,minimum)
}

/// Signed separation of two constant-radius ball sweeps, including their end caps.
/// The witnesses use independent centerline parameters; this is a static routing check.
#[derive(Debug)]
pub struct TubePairClearance {
    pub signed_margin_mm: [f64; 2],
    pub centerline_distance: crate::curve_distance::CurveDistance,
}

pub fn tube_pair_clearance(
    first: (&Curve, f64), second: (&Curve, f64), tolerance: f64, max_cells: usize,
) -> Result<TubePairClearance> {
    for (curve, radius) in [first, second] {
        check(radius.is_finite() && radius >= 0., "Expected finite nonnegative tube radius")?;
        check(curve.control_points.first().is_some_and(|p| p.len() == 3), "Expected spatial tube centerline")?;
    }
    let centerline_distance = crate::curve_distance::distance(first.0, second.0, tolerance, max_cells)?;
    let signed_margin_mm = tube_margin(centerline_distance.distance_interval_mm, first.1, second.1)?;
    Ok(TubePairClearance { signed_margin_mm, centerline_distance })
}

fn tube_margin([lo, hi]: [f64; 2], a: f64, b: f64) -> Result<[f64; 2]> {
    let margin = Interval::new(lo, hi)?.sub(Interval::point(a))?.sub(Interval::point(b))?;
    Ok([margin.lo, margin.hi])
}

#[derive(Debug)]
pub struct TubeBundleClearance {
    pub signed_margin_mm: [f64; 2],
    /// Pair that furnished the global upper bound; other pairs may tie or overlap its interval.
    pub witness_pair: [usize; 2],
    pub witness_parameters: [f64; 2],
    pub work_cells: usize,
}

/// Find the least signed clearance among all distinct routed tubes under one cell budget.
pub fn tube_bundle_clearance(
    tubes: &[(&Curve, f64)], tolerance: f64, max_cells: usize,
) -> Result<TubeBundleClearance> {
    validate(tolerance, max_cells)?;
    check(tubes.len() >= 2, "Expected at least two tubes")?;
    if tubes.len().saturating_mul(tubes.len()-1)/2 > max_cells {
        return Err(resource("Tube budget must cover every pair"));
    }
    let mut curves = Vec::with_capacity(tubes.len());
    for &(curve, radius) in tubes {
        curve.validate()?;
        check(curve.control_points[0].len() == 3, "Expected spatial tube centerline")?;
        check(radius.is_finite() && radius >= 0., "Expected finite nonnegative tube radius")?;
        curves.push(curve);
    }
    let mut report = TubeBundleClearance {
        signed_margin_mm: [f64::INFINITY; 2], witness_pair: [0, 1],
        witness_parameters: [0.; 2], work_cells: 0,
    };
    visit_contour_pairs(&curves, &curves, tolerance, max_cells, true, |[i,j], distance| {
        let [lo, hi] = tube_margin(distance.distance_interval_mm, tubes[i].1, tubes[j].1)?;
        report.signed_margin_mm[0] = report.signed_margin_mm[0].min(lo);
        if hi < report.signed_margin_mm[1] {
            report.signed_margin_mm[1] = hi;
            report.witness_pair = [i, j];
            report.witness_parameters = distance.parameters;
        }
        report.work_cells += distance.cells;
        Ok(())
    })?;
    Ok(report)
}

/// Check clearance to the entire untrimmed tensor patch. The distance report
/// retains the curve and UV witness parameters. In ambiguous cases, use the
/// witness direction to certify the full control nets, or certify a strict
/// violation using the enclosed witness coordinates.
pub fn inspect_curve_surface_clearance(
    curve: &Curve, surface: &crate::surface::Surface, minimum: f64, tolerance: f64, max_cells: usize,
) -> Result<(ClearanceDecision, crate::curve_surface_distance::Report)> {
    check(minimum.is_finite() && minimum >= 0., "Expected finite nonnegative clearance")?;
    let report = crate::curve_surface_distance::distance(curve,surface,tolerance,max_cells)?;
    let decision = patch_clearance_decision(report.distance_interval, &report.points,
        [&report.point_enclosures[0], &report.point_enclosures[1]], minimum,
        curve.control_points.iter(), surface.control_points.iter().flatten());
    Ok((decision, report))
}

/// Check the inclusive minimum clearance across two full positive-weight tensor surfaces.
/// The returned UV pair locates the sampled upper bound on the original surfaces.
pub fn inspect_surface_surface_clearance(
    first: &crate::surface::Surface, second: &crate::surface::Surface,
    minimum: f64, tolerance: f64, max_cells: usize,
) -> Result<(ClearanceDecision, crate::surface_distance::SurfaceDistance)> {
    check(minimum.is_finite() && minimum >= 0., "Expected finite nonnegative clearance")?;
    let report = crate::surface_distance::distance(first, second, tolerance, max_cells)?;
    let decision = patch_clearance_decision(report.distance_interval_mm, &report.points,
        [&report.point_enclosures[0], &report.point_enclosures[1]], minimum,
        first.control_points.iter().flatten(), second.control_points.iter().flatten());
    Ok((decision, report))
}

/// Inspect clearance on the retained regions of two UV-trimmed surfaces.
/// A decision requires an admitted witness on each face, including for a zero threshold.
pub fn inspect_trimmed_surface_clearance(
    first: (&crate::surface::Surface, &crate::trim_domain::TrimDomain),
    second: (&crate::surface::Surface, &crate::trim_domain::TrimDomain),
    minimum: f64, tolerance: f64, max_cells: usize, max_domain_cells: usize,
) -> Result<(ClearanceDecision, crate::trimmed_surface_distance::TrimmedDistance)> {
    check(minimum.is_finite() && minimum >= 0., "Expected finite nonnegative clearance")?;
    let report = crate::trimmed_surface_distance::distance(
        first.0, first.1, second.0, second.1, tolerance, max_cells, max_domain_cells,
    )?;
    let decision = match (report.upper_bound_mm, &report.points, &report.point_enclosures) {
        (Some(upper), Some(points), Some(bounds)) => patch_clearance_decision(
            [report.lower_bound_mm, upper], points, [&bounds[0], &bounds[1]], minimum,
            first.0.control_points.iter().flatten(), second.0.control_points.iter().flatten(),
        ),
        _ => ClearanceDecision::Unresolved,
    };
    Ok((decision, report))
}

fn patch_clearance_decision<'a>(
    [lo, hi]: [f64; 2], points: &[impl AsRef<[f64]>; 2], bounds: [&[[f64; 2]]; 2], minimum: f64,
    first: impl Iterator<Item=&'a Vec<f64>>, second: impl Iterator<Item=&'a Vec<f64>>,
) -> ClearanceDecision {
    if lo >= minimum { return ClearanceDecision::MeetsMinimum; }
    if hi < minimum || witnesses_below_clearance(bounds[0], bounds[1], minimum) {
        return ClearanceDecision::BelowMinimum;
    }
    let exact = |x| num_rational::BigRational::from_float(x).expect("finite witness coordinate");
    let direction = std::array::from_fn(|i| exact(points[1].as_ref()[i]) - exact(points[0].as_ref()[i]));
    if pole_support_proves(first, second, direction, minimum) {
        ClearanceDecision::MeetsMinimum
    } else {
        ClearanceDecision::Unresolved
    }
}

/// Check every distinct pair, in index order, against an inclusive minimum.
/// At a threshold straddle, exact control-box and witness tests can still
/// certify the decision. A closed upper bound may equal a strictly violated threshold.
pub fn inspect_spatial_contour_clearances(
    contours: &[&Curve], minimum: f64, tolerance: f64, max_cells: usize,
) -> Result<Vec<ContourClearanceCheck>> {
    validate(tolerance,max_cells)?;
    check(minimum.is_finite() && minimum >= 0., "Expected finite nonnegative minimum clearance")?;
    check(contours.len() >= 2, "Clearance inspection requires at least two contours")?;
    validate_spatial_distance_set(contours,max_cells)?;
    let boxes: Vec<_> = contours.iter().map(|c| spatial_control_box(c)).collect();
    let mut checks = Vec::new();
    visit_contour_pairs(contours,contours,tolerance,max_cells,true,|indices,distance| {
        let [mut lo,mut hi] = distance.distance_interval_mm;
        let mut parameters = distance.parameters;
        let decision = if lo >= minimum {
            ClearanceDecision::MeetsMinimum
        } else if hi < minimum {
            ClearanceDecision::BelowMinimum
        } else if box_proves_clearance(&boxes[indices[0]], &boxes[indices[1]], minimum) {
            lo = minimum;
            ClearanceDecision::MeetsMinimum
        } else {
            let [a,b] = indices.map(|i| contours[i]);
            let witness = &distance.point_enclosures;
            // The first pole of a clamped curve is an exact endpoint. Its
            // enclosure need not inherit rounding from point evaluation.
            let endpoint = |c: &Curve| -> [[f64;2];3] {
                std::array::from_fn(|k| [c.control_points[0][k];2])
            };
            if witnesses_below_clearance(&witness[0], &witness[1], minimum) {
                hi = minimum;
                ClearanceDecision::BelowMinimum
            } else if witnesses_below_clearance(&endpoint(a), &endpoint(b), minimum) {
                parameters = [a.domain()[0], b.domain()[0]];
                hi = minimum;
                ClearanceDecision::BelowMinimum
            } else if automatic_support_proves(a,b,&distance.points,minimum) {
                lo = minimum;
                ClearanceDecision::MeetsMinimum
            } else {
                ClearanceDecision::Unresolved
            }
        };
        checks.push(ContourClearanceCheck { contours: indices, bounds:[lo,hi],
            witness_parameters:parameters,cells:distance.cells,decision });
        Ok(())
    })?;
    Ok(checks)
}

#[derive(Clone, Debug)]
pub struct PlanarContourPlacementReport {
    pub boundary_distance: PlanarContourPairDistanceReport,
    /// Classification for each tested contour. A missing entry means
    /// separation between the contour sets could not be proved.
    pub placements: Vec<Option<PlanarCurveFillReport>>,
    pub fill_work_cells: usize,
}

/// Classify entire tested contours against a filled planar contour set.
/// Positive separation makes the fill constant along each connected
/// tested contour, so its clamped endpoint determines the placement.
pub fn classify_planar_contour_placement(
    tested: &[&Curve],
    region: &[&Curve],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_fill_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarContourPlacementReport> {
    validate(parameter_tolerance, max_fill_cells)?;
    let boundary_distance = measure_planar_contour_sets_distance(
        tested, region, distance_tolerance, max_distance_cells,
    )?;
    let mut placements = vec![None; tested.len()];
    let mut fill_work_cells = 0;
    if boundary_distance.bounds[0] > 0. {
        for (index, (curve, placement)) in tested.iter().zip(&mut placements).enumerate() {
            let first = &curve.control_points[0];
            let budget = (max_fill_cells - fill_work_cells) / (tested.len() - index);
            if budget == 0 {
                return Err(resource("Placement budget must cover every contour query"));
            }
            let fill = classify_planar_curve_fill(
                region, [first[0], first[1]], rule, parameter_tolerance, budget,
            )?;
            fill_work_cells += fill.work_cells;
            *placement = Some(fill);
        }
    }
    Ok(PlanarContourPlacementReport { boundary_distance, placements, fill_work_cells })
}

#[derive(Clone, Copy, Debug)]
pub struct PlanarContourNestingPair {
    pub contours: [usize; 2],
    pub distance_bounds: [f64; 2],
    /// Winding of the second contour around the first, then of the first
    /// around the second. Missing values are unresolved.
    pub windings: [Option<i64>; 2],
}

#[derive(Clone, Debug)]
pub struct PlanarContourNestingReport {
    pub pairs: Vec<PlanarContourNestingPair>,
    pub distance_work_cells: usize,
    pub winding_work_cells: usize,
}

/// Report pairwise nesting of closed planar contours, retaining signed
/// winding even for self-intersecting inputs. Each pair is classified only
/// after proving separation. The two budgets cover all distance and
/// winding queries, respectively.
pub fn analyze_planar_contour_nesting(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_winding_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarContourNestingReport> {
    validate(parameter_tolerance, max_winding_cells)?;
    validate(distance_tolerance, max_distance_cells)?;
    validate_distance_contours(contours, max_distance_cells)?;
    let count = contours.len();
    let spans: Vec<_> = contours.iter().map(|curve| active_spans(curve, curve.domain()).count()).collect();
    let mut reserved_distance = 0;
    let mut earlier = 0_usize;
    for &current in &spans {
        let work = earlier.checked_mul(current)
            .filter(|&work| work <= max_distance_cells - reserved_distance)
            .ok_or_else(|| resource("Nesting distance budget must cover every span pair"))?;
        reserved_distance += work;
        earlier += current;
    }
    let reserved_winding = earlier.checked_mul(count - 1)
        .filter(|&work| work <= max_winding_cells)
        .ok_or_else(|| resource("Nesting winding budget must cover every contour query"))?;
    let mut distance_extra = max_distance_cells - reserved_distance;
    let mut winding_extra = max_winding_cells - reserved_winding;
    let mut remaining_pairs = count * (count - 1) / 2;
    let mut report = PlanarContourNestingReport {
        pairs: Vec::with_capacity(remaining_pairs), distance_work_cells: 0, winding_work_cells: 0,
    };
    for j in 1..count {
        for i in 0..j {
            let work = spans[i] * spans[j];
            let distance = crate::curve_distance::distance(
                contours[i], contours[j], distance_tolerance, work + distance_extra / remaining_pairs,
            )?;
            distance_extra -= distance.cells.saturating_sub(work);
            report.distance_work_cells += distance.cells;
            let mut windings = [None; 2];
            if distance.distance_interval_mm[0] > 0. {
                for (slot, (tested, reference)) in windings.iter_mut().zip([(i, j), (j, i)]) {
                    let point = &contours[tested].control_points[0];
                    let winding = measure_planar_curve_winding_about_point(
                        contours[reference], [point[0], point[1]], parameter_tolerance,
                        spans[reference] + winding_extra / (2 * remaining_pairs),
                    )?;
                    *slot = winding.winding;
                    winding_extra -= winding.travel.work_cells.saturating_sub(spans[reference]);
                    report.winding_work_cells += winding.travel.work_cells;
                }
            }
            report.pairs.push(PlanarContourNestingPair {
                contours: [i, j], distance_bounds: distance.distance_interval_mm, windings,
            });
            remaining_pairs -= 1;
        }
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct PlanarContourHierarchyReport {
    /// Immediate containing contour in input order. Outermost contours
    /// have no parent.
    pub parents: Vec<Option<usize>>,
    /// Number of containing contours. Odd depths bound holes under the
    /// even-odd fill rule.
    pub depths: Vec<usize>,
    pub nesting: PlanarContourNestingReport,
    pub simplicity_pairs: usize,
    pub simplicity_cells: usize,
}

/// Build a nesting forest for pairwise separated simple planar contours.
/// Exact span segmentation and the trim simplicity proof are required
/// before winding results can establish parent relationships.
pub fn certify_planar_contour_hierarchy(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
) -> Result<PlanarContourHierarchyReport> {
    validate(distance_tolerance, max_simplicity_work)?;
    validate_distance_contours(contours, max_simplicity_work)?;
    let budget = max_simplicity_work / contours.len();
    let mut simplicity_pairs = 0;
    let mut simplicity_cells = 0;
    for curve in contours {
        let spans = active_spans(curve, curve.domain()).count();
        check((2..=256).contains(&spans), "Hierarchy simplicity requires 2..256 exact span pieces")?;
        if spans * (spans - 1) / 2 > budget {
            return Err(resource("Hierarchy budget must cover every simplicity span pair"));
        }
        let pieces = crate::exact_curve_segments::inspect(curve)
            .ok_or_else(|| resource("Contour has no certified exact span representation"))?;
        let proof = crate::trim_simplicity::inspect(&pieces, distance_tolerance, budget, budget)?;
        check(proof.proven_simple, "Contour simplicity is unproven")?;
        simplicity_pairs += proof.pairs.len();
        simplicity_cells += proof.cells;
    }
    let nesting = analyze_planar_contour_nesting(
        contours, parameter_tolerance, distance_tolerance, max_winding_cells, max_distance_cells,
    )?;
    let mut containers = vec![Vec::new(); contours.len()];
    for pair in &nesting.pairs {
        let [a, b] = pair.contours;
        match pair.windings {
            [Some(0), Some(0)] => {}
            [Some(1 | -1), Some(0)] => containers[a].push(b),
            [Some(0), Some(1 | -1)] => containers[b].push(a),
            _ => return Err(resource("Hierarchy requires separated contours with certified nesting")),
        }
    }
    let depths: Vec<_> = containers.iter().map(Vec::len).collect();
    let parents = containers.into_iter().map(|items| {
        items.into_iter().max_by_key(|&index| depths[index])
    }).collect();
    Ok(PlanarContourHierarchyReport { parents, depths, nesting, simplicity_pairs, simplicity_cells })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanarContourOrientation {
    Clockwise,
    Counterclockwise,
}

#[derive(Clone, Debug)]
pub struct PlanarContourOrientationReport {
    pub hierarchy: PlanarContourHierarchyReport,
    pub orientations: Vec<PlanarContourOrientation>,
    /// Reverse these contours to orient even depths counterclockwise and
    /// odd depths clockwise, so nonzero fill reproduces even-odd fill.
    pub reverse: Vec<bool>,
    pub orientation_work_cells: usize,
}

/// Determine the current and desired orientation of every boundary in a
/// certified simple-contour hierarchy. The area integral retains the
/// original NURBS definition and must certify a nonzero sign.
pub fn certify_planar_contour_fill_orientations(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
) -> Result<PlanarContourOrientationReport> {
    let (report, _) = contour_fill_orientation_and_moments::<1>(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_orientation_cells,
    )?;
    Ok(report)
}

fn contour_fill_orientation_and_moments<const N: usize>(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
) -> Result<(PlanarContourOrientationReport, Vec<[Interval; N]>)> {
    validate(parameter_tolerance, max_orientation_cells)?;
    let hierarchy = certify_planar_contour_hierarchy(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells,
    )?;
    let mut orientations = Vec::with_capacity(contours.len());
    let mut signed_moments = Vec::with_capacity(contours.len());
    let mut orientation_work_cells = 0;
    for (index, curve) in contours.iter().enumerate() {
        let budget = (max_orientation_cells - orientation_work_cells) / (contours.len() - index);
        let origin = if N > 1 { &contours[0].control_points[0] }
            else { &curve.control_points[0] };
        let (cells, work) = partition_motion_cells(
            curve, curve.domain(), parameter_tolerance, budget,
            |jets, _| {
                // The local jet already includes the cell width.
                let x = jets[0][0].sub(Interval::point(origin[0]))?;
                let y = jets[0][1].sub(Interval::point(origin[1]))?;
                let [dx, dy] = [jets[1][0], jets[1][1]];
                let mut values = [Interval::point(0.); N];
                values[0] = x.mul(dy)?.sub(y.mul(dx)?)?;
                if N >= 3 {
                    let x_squared = x.mul(x)?;
                    let y_squared = y.mul(y)?;
                    values[1] = x_squared.mul(dy)?;
                    values[2] = Interval::point(-1.).mul(y_squared.mul(dx)?)?;
                    if N >= 6 {
                        values[3] = x_squared.mul(x)?.mul(dy)?;
                        values[4] = x_squared.mul(y)?.mul(dy)?;
                        values[5] = Interval::point(-1.).mul(y_squared.mul(y)?.mul(dx)?)?;
                    }
                }
                Ok((values, true))
            },
        )?;
        let mut integrals = [Interval::point(0.); N];
        for (_, _, cell) in cells {
            for (sum, value) in integrals.iter_mut().zip(cell) {
                *sum = sum.add(value)?;
            }
        }
        let doubled_area = integrals[0];
        let orientation = if doubled_area.lo > 0. {
            PlanarContourOrientation::Counterclockwise
        } else if doubled_area.hi < 0. {
            PlanarContourOrientation::Clockwise
        } else {
            return Err(resource("Contour orientation is unresolved under the area work budget"));
        };
        orientations.push(orientation);
        for (index, value) in integrals.iter_mut().enumerate() {
            let divisor = if index == 3 || index == 5 { 3. } else { 2. };
            *value = value.div(Interval::point(divisor))?;
        }
        signed_moments.push(integrals);
        orientation_work_cells += work;
    }
    let reverse = orientations.iter().zip(&hierarchy.depths).map(|(orientation, depth)| {
        let counterclockwise = *orientation == PlanarContourOrientation::Counterclockwise;
        counterclockwise != (depth % 2 == 0)
    }).collect();
    let report = PlanarContourOrientationReport { hierarchy, orientations, reverse, orientation_work_cells };
    Ok((report, signed_moments))
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillAreaReport {
    pub area_bounds: [f64; 2],
    /// Signed area of each original contour in input order.
    pub signed_contour_area_bounds: Vec<[f64; 2]>,
    pub orientation: PlanarContourOrientationReport,
}

/// Bound the area filled by a certified contour hierarchy using even-odd
/// parity. Odd depths subtract enclosed area and even depths add it,
/// regardless of the direction of the input contours.
pub fn measure_planar_contour_fill_area(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_area_cells: usize,
) -> Result<PlanarContourFillAreaReport> {
    let (orientation, areas) = contour_fill_orientation_and_moments::<1>(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_area_cells,
    )?;
    let mut total = Interval::point(0.);
    for ([area], &reverse) in areas.iter().zip(&orientation.reverse) {
        let contribution = if reverse { Interval::new(-area.hi, -area.lo)? } else { *area };
        total = total.add(contribution)?;
    }
    let signed_contour_area_bounds = areas.into_iter().map(|[area]| [area.lo, area.hi]).collect();
    Ok(PlanarContourFillAreaReport {
        area_bounds: [total.lo.max(0.), total.hi],
        signed_contour_area_bounds,
        orientation,
    })
}

#[derive(Clone, Debug)]
pub struct PlanarContourNonzeroFillAreaReport {
    pub area_bounds: [f64; 2],
    /// Winding just inside each contour, in input order.
    pub interior_windings: Vec<i32>,
    /// Each contour's unsigned area multiplier: add, subtract or skip.
    pub area_multipliers: Vec<i32>,
    pub orientation: PlanarContourOrientationReport,
}

/// Bound nonzero-fill area using the original directions of a certified
/// simple contour hierarchy. Only transitions between zero and nonzero
/// winding contribute to the area.
pub fn measure_planar_contour_nonzero_fill_area(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_area_cells: usize,
) -> Result<PlanarContourNonzeroFillAreaReport> {
    let (orientation, areas) = contour_fill_orientation_and_moments::<1>(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_area_cells,
    )?;
    let (interior_windings, area_multipliers) = nonzero_contour_transitions(&orientation);
    let mut total = Interval::point(0.);
    for (index, &multiplier) in area_multipliers.iter().enumerate() {
        if multiplier != 0 {
            let direction = contour_orientation_sign(orientation.orientations[index]);
            let factor = Interval::point(f64::from(multiplier * direction));
            total = total.add(areas[index][0].mul(factor)?)?;
        }
    }
    Ok(PlanarContourNonzeroFillAreaReport {
        area_bounds: [total.lo.max(0.), total.hi],
        interior_windings,
        area_multipliers,
        orientation,
    })
}

fn contour_orientation_sign(orientation: PlanarContourOrientation) -> i32 {
    match orientation {
        PlanarContourOrientation::Counterclockwise => 1,
        PlanarContourOrientation::Clockwise => -1,
    }
}

fn nonzero_contour_transitions(orientation: &PlanarContourOrientationReport) -> (Vec<i32>, Vec<i32>) {
    let count = orientation.orientations.len();
    let mut order: Vec<_> = (0..count).collect();
    order.sort_by_key(|&index| orientation.hierarchy.depths[index]);
    let mut windings = vec![0; count];
    let mut multipliers = vec![0; count];
    for index in order {
        let exterior = orientation.hierarchy.parents[index].map_or(0, |parent| windings[parent]);
        let interior = exterior + contour_orientation_sign(orientation.orientations[index]);
        windings[index] = interior;
        multipliers[index] = i32::from(interior != 0) - i32::from(exterior != 0);
    }
    (windings, multipliers)
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillPerimeterReport {
    pub perimeter_bounds: [f64; 2],
    /// Original indices of contours that separate filled and empty regions.
    pub boundary_contours: Vec<usize>,
    pub work_cells: usize,
    pub converged: bool,
}

/// Bound the perimeter of a simple contour hierarchy under the given
/// fill rule. Length work is shared only among actual fill boundaries.
pub fn measure_planar_contour_fill_perimeter(
    contours: &[&Curve],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    length_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
    max_length_cells: usize,
) -> Result<PlanarContourFillPerimeterReport> {
    validate(length_tolerance, max_length_cells)?;
    let (_, multipliers) = certify_contour_fill_transitions(
        contours, rule, parameter_tolerance, length_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_orientation_cells,
    )?;
    let boundary_contours: Vec<_> = multipliers.into_iter().enumerate()
        .filter_map(|(index, value)| (value != 0).then_some(index)).collect();
    let mut initial = 0;
    let mut span_counts = Vec::with_capacity(boundary_contours.len());
    for &index in &boundary_contours {
        let curve = contours[index];
        let spans = active_spans(curve, curve.domain()).count();
        if spans > max_length_cells - initial {
            return Err(resource("Perimeter budget must cover every boundary span"));
        }
        initial += spans;
        span_counts.push(spans);
    }
    let mut extra = max_length_cells - initial;
    let mut work_cells = 0;
    let mut total = Interval::point(0.);
    let count = boundary_contours.len();
    let per_contour_tolerance = (length_tolerance / 2. / count as f64).max(f64::from_bits(1));
    for (position, (&index, spans)) in boundary_contours.iter().zip(span_counts).enumerate() {
        let share = extra / (count - position);
        let report = length(contours[index], per_contour_tolerance, spans + share)?;
        extra -= report.cells.saturating_sub(spans);
        work_cells += report.cells;
        total = total.add(Interval::new(report.bounds[0], report.bounds[1])?)?;
    }
    Ok(PlanarContourFillPerimeterReport {
        perimeter_bounds: [total.lo.max(0.), total.hi],
        boundary_contours,
        work_cells,
        converged: total.width().next_up() <= length_tolerance,
    })
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillComponent {
    pub outer: usize,
    pub holes: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillTopology {
    pub components: Vec<PlanarContourFillComponent>,
    pub inactive_contours: Vec<usize>,
    pub euler_characteristic: isize,
    pub hierarchy: PlanarContourHierarchyReport,
}

/// Group the connected filled components and their holes from a
/// certified simple contour hierarchy. Indices refer to input contours.
pub fn analyze_planar_contour_fill_topology(
    contours: &[&Curve],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
) -> Result<PlanarContourFillTopology> {
    let (hierarchy, transitions) = certify_contour_fill_transitions(
        contours, rule, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_orientation_cells,
    )?;
    let mut order: Vec<_> = (0..contours.len()).collect();
    order.sort_by_key(|&index| hierarchy.depths[index]);
    let mut owners: Vec<Option<usize>> = vec![None; contours.len()];
    let mut components: Vec<PlanarContourFillComponent> = Vec::new();
    let mut inactive_contours = Vec::new();
    let mut euler_characteristic = 0;
    for index in order {
        let owner = hierarchy.parents[index].and_then(|parent| owners[parent]);
        match transitions[index] {
            1 => {
                owners[index] = Some(components.len());
                components.push(PlanarContourFillComponent { outer: index, holes: Vec::new() });
                euler_characteristic += 1;
            }
            -1 => {
                let owner = owner.ok_or_else(|| crate::numeric_err("Hole has no containing fill component"))?;
                components[owner].holes.push(index);
                euler_characteristic -= 1;
            }
            _ => {
                owners[index] = owner;
                inactive_contours.push(index);
            }
        }
    }
    Ok(PlanarContourFillTopology { components, inactive_contours, euler_characteristic, hierarchy })
}

fn certify_contour_fill_transitions(
    contours: &[&Curve],
    rule: crate::chord_winding::FillRule,
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
) -> Result<(PlanarContourHierarchyReport, Vec<i32>)> {
    use crate::chord_winding::FillRule;
    match rule {
        FillRule::EvenOdd => {
            let hierarchy = certify_planar_contour_hierarchy(
                contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
                max_winding_cells, max_distance_cells,
            )?;
            let transitions = hierarchy.depths.iter().map(|depth| if depth % 2 == 0 { 1 } else { -1 }).collect();
            Ok((hierarchy, transitions))
        }
        FillRule::NonZero => {
            let orientation = certify_planar_contour_fill_orientations(
                contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
                max_winding_cells, max_distance_cells, max_orientation_cells,
            )?;
            let (_, transitions) = nonzero_contour_transitions(&orientation);
            Ok((orientation.hierarchy, transitions))
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillCentroidReport {
    pub area_bounds: [f64; 2],
    /// Bounds for the X and Y coordinates in the source coordinate system.
    pub centroid_bounds: [[f64; 2]; 2],
    pub orientation: PlanarContourOrientationReport,
}

/// Enclose the centroid of an even-odd filled planar contour hierarchy.
/// Area and first moments use the same interval subdivision. A strictly
/// positive filled area is required before dividing by it.
pub fn measure_planar_contour_fill_centroid(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_moment_cells: usize,
) -> Result<PlanarContourFillCentroidReport> {
    let (orientation, moments) = contour_fill_orientation_and_moments::<3>(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_moment_cells,
    )?;
    let [area, x, y] = sum_oriented_contour_integrals(moments, &orientation.reverse)?;
    check(area.lo > 0., "Filled area must be certified positive for centroid measurement")?;
    let origin = &contours[0].control_points[0];
    let x = x.div(area)?.add(Interval::point(origin[0]))?;
    let y = y.div(area)?.add(Interval::point(origin[1]))?;
    Ok(PlanarContourFillCentroidReport {
        area_bounds: [area.lo, area.hi],
        centroid_bounds: [[x.lo, x.hi], [y.lo, y.hi]],
        orientation,
    })
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillCentralMomentsReport {
    pub area_bounds: [f64; 2],
    pub centroid_bounds: [[f64; 2]; 2],
    /// Integrals of (x-cx)^2, (x-cx)(y-cy) and (y-cy)^2, respectively.
    pub central_moment_bounds: [[f64; 2]; 3],
    pub orientation: PlanarContourOrientationReport,
}

/// Enclose the central second moments of an even-odd filled contour
/// hierarchy. Moments are for unit areal density and retain model units.
pub fn measure_planar_contour_fill_central_moments(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_moment_cells: usize,
) -> Result<PlanarContourFillCentralMomentsReport> {
    let (orientation, moments) = contour_fill_orientation_and_moments::<6>(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_moment_cells,
    )?;
    let [area, x, y, xx, xy, yy] = sum_oriented_contour_integrals(moments, &orientation.reverse)?;
    check(area.lo > 0., "Filled area must be certified positive for central moments")?;
    let local_x = x.div(area)?;
    let local_y = y.div(area)?;
    let xx = xx.sub(x.mul(local_x)?)?;
    let xy = xy.sub(x.mul(local_y)?)?;
    let yy = yy.sub(y.mul(local_y)?)?;
    let origin = &contours[0].control_points[0];
    let cx = local_x.add(Interval::point(origin[0]))?;
    let cy = local_y.add(Interval::point(origin[1]))?;
    Ok(PlanarContourFillCentralMomentsReport {
        area_bounds: [area.lo, area.hi],
        centroid_bounds: [[cx.lo, cx.hi], [cy.lo, cy.hi]],
        central_moment_bounds: [[xx.lo.max(0.), xx.hi], [xy.lo, xy.hi], [yy.lo.max(0.), yy.hi]],
        orientation,
    })
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillPrincipalMomentsReport {
    /// Ascending principal second moments about the centroid.
    pub principal_moment_bounds: [[f64; 2]; 2],
    /// Bounds for the difference of the larger and smaller moments.
    pub moment_gap_bounds: [f64; 2],
    pub central: PlanarContourFillCentralMomentsReport,
}

/// Enclose the two principal second moments of an even-odd filled region
/// and their separation. The interval calculation supports repeated
/// moments, where the principal axes are not unique.
pub fn measure_planar_contour_fill_principal_moments(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_moment_cells: usize,
) -> Result<PlanarContourFillPrincipalMomentsReport> {
    let central = measure_planar_contour_fill_central_moments(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_moment_cells,
    )?;
    let [xx, xy, yy] = central_moment_intervals(&central)?;
    let half = Interval::point(0.5);
    let mean = xx.mul(half)?.add(yy.mul(half)?)?;
    let difference = xx.mul(half)?.sub(yy.mul(half)?)?;
    let offset = norm(&[difference, xy])?;
    let small = mean.sub(offset)?;
    let large = mean.add(offset)?;
    let gap = offset.mul(Interval::point(2.))?;
    Ok(PlanarContourFillPrincipalMomentsReport {
        principal_moment_bounds: [[small.lo.max(0.), small.hi], [large.lo.max(0.), large.hi]],
        moment_gap_bounds: [gap.lo.max(0.), gap.hi],
        central,
    })
}

#[derive(Clone, Debug)]
pub struct PlanarContourFillPrincipalAxesReport {
    pub moments: PlanarContourFillPrincipalMomentsReport,
    /// Orthogonal projectors onto the principal axes, in ascending moment
    /// order. Each stores the XX, XY and YY entry bounds. None means that
    /// the moments could not be separated.
    pub projector_bounds: Option<[[[f64; 2]; 3]; 2]>,
}

/// Enclose the planar principal axes as sign-independent projectors.
/// Axes are only reported when a strictly positive spectral gap is proved.
pub fn measure_planar_contour_fill_principal_axes(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_moment_cells: usize,
) -> Result<PlanarContourFillPrincipalAxesReport> {
    let moments = measure_planar_contour_fill_principal_moments(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_moment_cells,
    )?;
    let [lower, upper] = moments.moment_gap_bounds;
    let projector_bounds = if lower > 0. {
        let [xx, xy, yy] = central_moment_intervals(&moments.central)?;
        let gap = Interval::new(lower, upper)?;
        let half = Interval::point(0.5);
        let difference = xx.mul(half)?.sub(yy.mul(half)?)?.div(gap)?;
        let off_diagonal = xy.div(gap)?.intersect(-0.5, 0.5)?;
        let major_x = half.add(difference)?.intersect(0., 1.)?;
        let major_y = half.sub(difference)?.intersect(0., 1.)?;
        let minor_xy = Interval::new(-off_diagonal.hi, -off_diagonal.lo)?;
        Some([[major_y, minor_xy, major_x], [major_x, off_diagonal, major_y]]
            .map(|axis| axis.map(|value| [value.lo, value.hi])))
    } else {
        None
    };
    Ok(PlanarContourFillPrincipalAxesReport { moments, projector_bounds })
}

fn central_moment_intervals(central: &PlanarContourFillCentralMomentsReport) -> Result<[Interval; 3]> {
    let [[a, b], [c, d], [e, f]] = central.central_moment_bounds;
    Ok([Interval::new(a, b)?, Interval::new(c, d)?, Interval::new(e, f)?])
}

fn sum_oriented_contour_integrals<const N: usize>(
    moments: Vec<[Interval; N]>,
    reversals: &[bool],
) -> Result<[Interval; N]> {
    let mut total = [Interval::point(0.); N];
    for (values, &reverse) in moments.into_iter().zip(reversals) {
        for (sum, value) in total.iter_mut().zip(values) {
            let oriented = if reverse { Interval::new(-value.hi, -value.lo)? } else { value };
            *sum = sum.add(oriented)?;
        }
    }
    Ok(total)
}

#[derive(Clone, Debug)]
pub struct OrientedPlanarContours {
    pub contours: Vec<Curve>,
    pub orientation: PlanarContourOrientationReport,
    pub reversal_work: u64,
}

/// Orient simple planar boundaries for nonzero fill, preserving their
/// exact geometry and original active parameter intervals. A required
/// reversal that cannot store all reflected knots exactly fails the edit.
pub fn orient_planar_contours_for_nonzero_fill(
    contours: &[&Curve],
    parameter_tolerance: f64,
    distance_tolerance: f64,
    max_simplicity_work: usize,
    max_winding_cells: usize,
    max_distance_cells: usize,
    max_orientation_cells: usize,
    max_reversal_work: u64,
) -> Result<OrientedPlanarContours> {
    check((1..=cad_predicates::MAX_WORK).contains(&max_reversal_work),
        "Contour reversal requires bounded positive exact work")?;
    let orientation = certify_planar_contour_fill_orientations(
        contours, parameter_tolerance, distance_tolerance, max_simplicity_work,
        max_winding_cells, max_distance_cells, max_orientation_cells,
    )?;
    let mut reversal_work = 0;
    let mut oriented = Vec::with_capacity(contours.len());
    for (&curve, &reverse) in contours.iter().zip(&orientation.reverse) {
        if !reverse {
            oriented.push(curve.clone());
            continue;
        }
        let remaining = max_reversal_work - reversal_work;
        if remaining == 0 {
            return Err(resource("Contour reversal work budget exhausted"));
        }
        let [start, end] = curve.domain();
        let report = crate::exact_geometry::exact_projective::affine_curve_clock(curve, [end, start], remaining)?;
        reversal_work += report.work_used;
        let reversed = report.geometry.ok_or_else(|| {
            resource("Contour reversal could not preserve the exact affine clock within the work budget")
        })?;
        oriented.push(reversed);
    }
    Ok(OrientedPlanarContours { contours: oriented, orientation, reversal_work })
}

fn validate_distance_contours(contours: &[&Curve], max_cells: usize) -> Result<()> {
    check(!contours.is_empty(), "Contour distance requires at least one contour")?;
    check(contours.len() <= max_cells, "Distance budget must cover every contour")?;
    for curve in contours {
        validate_winding_curve(curve)?;
        check(curve.control_points.first() == curve.control_points.last(), "Distance contour must be closed")?;
    }
    Ok(())
}

fn measure_contour_set_distance(
    contours: &[&Curve],
    tolerance: f64,
    max_cells: usize,
    gap: impl Fn(&[Interval]) -> Result<Interval>,
) -> Result<PlanarContourDistanceReport> {
    let initial = contours.iter().enumerate().flat_map(|(index, curve)| {
        active_spans(curve, curve.domain()).map(move |span| {
            ((index, span), [curve.knots[span], curve.knots[span + 1]])
        })
    });
    let measure = |(index, span): (usize, usize), domain: [f64; 2], witness: &mut ClearanceWitness| {
        let curve = contours[index];
        let cell = curve_jets::enclose(curve, span, domain)?;
        let lower = gap(&cell[0])?.lo;
        for (end, parameter) in [(false, domain[0]), (true, domain[1])] {
            let endpoint = curve_jets::endpoint(curve, span, domain, end)?;
            let upper = gap(&endpoint[0])?.hi;
            if upper < witness.0 {
                *witness = (upper, parameter, index);
            }
        }
        Ok(lower)
    };
    let (lower, witness, work_cells, stop_reason) = refine_minimum_clearance(
        initial, contours[0].domain()[0], tolerance, max_cells, measure,
    )?;
    Ok(PlanarContourDistanceReport {
        bounds: [lower, witness.0], witness_contour: witness.2,
        witness_parameter: witness.1, work_cells, stop_reason,
    })
}

fn validate_winding_curve(direction: &Curve) -> Result<[f64; 2]> {
    validate_continuous_clamped_curve(direction, 2)
}

fn validate_continuous_clamped_curve(direction: &Curve, dimension: usize) -> Result<[f64; 2]> {
    direction.validate()?;
    check(direction.control_points[0].len() == dimension, "Unexpected contour dimension")?;
    let domain = direction.domain();
    let degree = direction.degree;
    let last = direction.control_points.len();
    check(
        direction.knots[..=degree].iter().all(|&knot| knot == domain[0])
            && direction.knots[degree + 1] > domain[0]
            && direction.knots[last - 1] < domain[1]
            && direction.knots[last..].iter().all(|&knot| knot == domain[1]),
        "Orientation winding requires clamped endpoints",
    )?;
    let mut run = 0;
    let mut previous = domain[0];
    for &knot in &direction.knots[degree + 1..last] {
        run = if knot == previous { run + 1 } else { 1 };
        check(run <= degree, "Interior direction knots must guarantee continuity")?;
        previous = knot;
    }
    Ok(domain)
}

fn winding_from_angular_travel(travel: PlanarAngularTravelReport) -> Result<PlanarOrientationWindingReport> {
    let tau = std::f64::consts::TAU;
    let turns = Interval::new(travel.net_bounds[0], travel.net_bounds[1])?
        .div(Interval::new(tau.next_down(), tau.next_up())?)?;
    let first = turns.lo.ceil();
    let last = turns.hi.floor();
    check(first <= last, "Closed orientation winding enclosure contains no integer")?;
    let winding = (first == last && first.abs() < i64::MAX as f64).then_some(first as i64);
    Ok(PlanarOrientationWindingReport { winding, travel })
}

/// Bound relative angular travel for two directions on the same clock.
/// Both original knot grids participate in integration. Discontinuous
/// jumps are excluded, and a possible zero direction prevents a report.
pub fn measure_relative_planar_angular_travel(
    direction: &Curve,
    reference: &Curve,
    interval: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
) -> Result<PlanarAngularTravelReport> {
    validate(parameter_tolerance, max_cells)?;
    let curves = [direction, reference];
    let (_, initial) = synchronized_clearance_cells(curves, interval, [0.; 2])?;
    check(direction.control_points[0].len() == 2, "Angular travel requires planar direction curves")?;
    let fixed = curves.map(|curve| fixed_cardinal_rotation(curve).is_some());
    let same = std::ptr::eq(direction, reference);
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |spans, domain| {
            let local_speed = |index: usize| -> Result<Option<Interval>> {
                if fixed[index] {
                    return Ok(Some(Interval::point(0.)));
                }
                let jets = curve_jets::enclose(curves[index], spans[index], domain)?;
                Ok(planar_angular_jets(&jets, Interval::point(1.), 1)?.map(|jets| jets[0]))
            };
            let Some(moving) = local_speed(0)? else {
                return Ok((None, true));
            };
            if same || fixed == [true; 2] {
                return Ok((Some(Interval::point(0.)), false));
            }
            let Some(frame) = local_speed(1)? else {
                return Ok((None, true));
            };
            Ok((Some(moving.sub(frame)?), true))
        },
    )?;
    accumulate_angular_travel(cells.into_iter().map(|(_, _, speed)| speed), work_cells)
}

fn accumulate_angular_travel(
    speeds: impl IntoIterator<Item = Option<Interval>>,
    work_cells: usize,
) -> Result<PlanarAngularTravelReport> {
    let mut net = Interval::point(0.);
    let mut total = Interval::point(0.);
    for speed in speeds {
        let speed = speed.ok_or_else(|| resource("Angular travel requires a certified nonzero direction in every cell"))?;
        if speed.lo == 0. && speed.hi == 0. {
            continue;
        }
        net = net.add(speed)?;
        total = total.add(Interval::new(
            speed.lo.max(-speed.hi).max(0.),
            speed.lo.abs().max(speed.hi.abs()),
        )?)?;
    }
    Ok(PlanarAngularTravelReport {
        net_bounds: [net.lo, net.hi],
        total_bounds: [total.lo.max(0.), total.hi],
        work_cells,
    })
}

fn classify_absolute_motion(value: Interval, limit: f64) -> DirectionalMotionStatus {
    let upper = value.lo.abs().max(value.hi.abs());
    let lower = value.lo.max(-value.hi).max(0.);
    if upper <= limit {
        DirectionalMotionStatus::Within
    } else if lower > limit {
        DirectionalMotionStatus::Exceeds
    } else {
        DirectionalMotionStatus::Undetermined
    }
}

fn planar_angular_jets(
    jets: &[Vec<Interval>],
    width: Interval,
    order: usize,
) -> Result<Option<[Interval; 3]>> {
    let length = norm(&jets[0])?;
    let squared = length.mul(length)?;
    if squared.lo <= 0. || width.lo <= 0. {
        return Ok(None);
    }
    let cross = |a: usize, b: usize| {
        jets[a][0].mul(jets[b][1])?.sub(jets[a][1].mul(jets[b][0])?)
    };
    let twice = Interval::point(2.);
    let speed = cross(0, 1)?.div(squared)?;
    let mut values = [speed, Interval::point(0.), Interval::point(0.)];
    if order >= 2 {
        let squared_derivative = project_jet(&jets[0], &jets[1])?.mul(twice)?;
        let acceleration = cross(0, 2)?.sub(speed.mul(squared_derivative)?)?.div(squared)?;
        values[1] = acceleration;
        if order == 3 {
            let squared_second = project_jet(&jets[1], &jets[1])?
                .add(project_jet(&jets[0], &jets[2])?)?.mul(twice)?;
            // Differentiate |v|² times angular speed twice.
            values[2] = cross(1, 2)?.add(cross(0, 3)?)?
                .sub(acceleration.mul(squared_derivative)?.mul(twice)?)?
                .sub(speed.mul(squared_second)?)?.div(squared)?;
        }
    }
    // The jets use a cell-local clock.
    for (index, value) in values.iter_mut().take(order).enumerate() {
        for _ in 0..=index {
            *value = value.div(width)?;
        }
    }
    Ok(Some(values))
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
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    curve.validate()?;
    let offsets = prepare_workspace_hull(curve, vertex_offsets, planes.len())?;
    partition_curve_workspace_with_support(
        curve, interval, planes, parameter_tolerance, max_plane_cells,
        |normal| {
            let support = hull_plane_support(&offsets, normal)?;
            if radius == 0. {
                Ok(Some(support))
            } else {
                Ok(Some(support.add(norm(normal)?.mul(Interval::point(radius))?)?))
            }
        },
    )
}

fn prepare_workspace_hull(
    curve: &Curve,
    vertex_offsets: &[&[f64]],
    plane_count: usize,
) -> Result<Vec<Vec<Interval>>> {
    check(!vertex_offsets.is_empty(), "Tool hull requires at least one vertex")?;
    if vertex_offsets.len() > 1_000_000 / plane_count {
        return Err(resource("Tool hull support preprocessing exceeds one million vertex/plane pairs"));
    }
    let dimension = curve.control_points[0].len();
    let mut offsets = Vec::with_capacity(vertex_offsets.len());
    for &offset in vertex_offsets {
        check(
            offset.len() == dimension && offset.iter().all(|value| value.is_finite()),
            "Tool hull vertices must be finite vectors matching the curve dimension",
        )?;
        offsets.push(offset.iter().copied().map(Interval::point).collect::<Vec<_>>());
    }
    Ok(offsets)
}

fn hull_plane_support(offsets: &[Vec<Interval>], normal: &[Interval]) -> Result<Interval> {
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for offset in offsets {
        let projected = project_jet(offset, normal)?;
        lo = lo.max(projected.lo);
        hi = hi.max(projected.hi);
    }
    Interval::new(lo, hi)
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
            let status = workspace_residual_status(&residuals);
            Ok(((residuals, status), status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    let cells = cells.into_iter().map(|(span, parameter_interval, (plane_residuals, status))| {
        WorkspaceCell { parameter_interval, span, plane_residuals, status }
    }).collect();
    Ok(WorkspacePartition { cells, work_plane_cells: work_cells * planes.len() })
}

/// Check containment in closed half-spaces as the ball radius follows a
/// continuous piecewise-linear profile in the original curve parameter.
pub fn partition_profiled_radius_convex_workspace(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    partition_profiled_workspace_with_support(
        curve, radius_profile, planes, parameter_tolerance, max_plane_cells, |_| Ok(None),
    )
}

/// Check a translating vertex hull rounded by a piecewise-linear radius.
/// The hull's vertex offsets retain their orientation along the curve.
pub fn partition_profiled_rounded_hull_workspace(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    curve.validate()?;
    let offsets = prepare_workspace_hull(curve, vertex_offsets, planes.len())?;
    partition_profiled_workspace_with_support(
        curve, radius_profile, planes, parameter_tolerance, max_plane_cells,
        |normal| hull_plane_support(&offsets, normal).map(Some),
    )
}

fn partition_profiled_workspace_with_support(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    parameter_tolerance: f64,
    max_plane_cells: usize,
    support: impl Fn(&[Interval]) -> Result<Option<Interval>>,
) -> Result<WorkspacePartition> {
    validate(parameter_tolerance, max_plane_cells)?;
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    let max_cells = max_plane_cells / planes.len();
    let (initial, profiles) = prepare_radius_profile(curve, radius_profile, max_cells)?;
    let dimension = curve.control_points[0].len();
    let planes = prepare_workspace_planes(planes, dimension)?.into_iter().map(|(normal, offset)| {
        let length = norm(&normal)?;
        let hull = support(&normal)?;
        Ok((normal, offset, length, hull))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, profile], domain| {
            let radius = profiles[profile](domain)?;
            let jets = curve_jets::enclose(curve, span, domain)?;
            let plane_residuals = planes.iter().map(|(normal, offset, length, hull)| {
                let mut residual = project_jet(&jets[0], normal)?.sub(*offset)?.add(length.mul(radius)?)?;
                if let Some(hull) = hull {
                    residual = residual.add(*hull)?;
                }
                Ok([residual.lo, residual.hi])
            }).collect::<Result<Vec<_>>>()?;
            let status = workspace_residual_status(&plane_residuals);
            Ok((WorkspaceCell {
                parameter_interval: domain, span, plane_residuals, status,
            }, status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    Ok(WorkspacePartition {
        cells: cells.into_iter().map(|(_, _, cell)| cell).collect(),
        work_plane_cells: work_cells * planes.len(),
    })
}

/// Partition planar workspace containment for a translating and rotating
/// convex hull. The orientation curve gives the local positive X direction
/// at the same parameter as the position curve. Cells where that direction
/// may vanish remain undetermined.
pub fn partition_planar_rotating_hull_workspace(
    position: &Curve,
    direction: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    validate(parameter_tolerance, max_plane_cells)?;
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    let (_, initial) = synchronized_clearance_cells([position, direction], interval, [0.; 2])?;
    check(position.control_points[0].len() == 2, "Rotating hull workspace requires planar curves")?;
    let offsets = prepare_workspace_hull(position, vertex_offsets, planes.len())?;
    let planes = prepare_workspace_planes(planes, 2)?;
    let max_cells = (max_plane_cells / planes.len())
        .min(1_000_000 / offsets.len() / planes.len());
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, direction_span], domain| {
            let position_jets = curve_jets::enclose(position, span, domain)?;
            let direction_jets = curve_jets::enclose(direction, direction_span, domain)?;
            let (rotation, regular) = planar_rotation(&direction_jets[0])?;
            let plane_residuals = planes.iter().map(|(normal, offset)| {
                let support = rotating_hull_support(&offsets, normal, rotation)?;
                let residual = project_jet(&position_jets[0], normal)?.sub(*offset)?.add(support)?;
                Ok([residual.lo, residual.hi])
            }).collect::<Result<Vec<_>>>()?;
            let status = if regular {
                workspace_residual_status(&plane_residuals)
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok((WorkspaceCell {
                parameter_interval: domain, span, plane_residuals, status,
            }, status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    Ok(WorkspacePartition {
        cells: cells.into_iter().map(|(_, _, cell)| cell).collect(),
        work_plane_cells: work_cells * planes.len(),
    })
}

/// Classify workspace containment for a rotating planar convex hull
/// expanded by a disk with piecewise-linear radius. The radius profile
/// defines the checked parameter window.
pub fn partition_planar_rotating_profiled_hull_workspace(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspacePartition> {
    validate(parameter_tolerance, max_plane_cells)?;
    check(!planes.is_empty() && !vertex_offsets.is_empty(), "Workspace requires planes and hull vertices")?;
    let max_cells = (max_plane_cells / planes.len())
        .min(1_000_000 / vertex_offsets.len() / planes.len());
    let (initial, profiles) = prepare_planar_rotation_profile(position, direction, radius_profile, max_cells)?;
    let offsets = prepare_workspace_hull(position, vertex_offsets, planes.len())?;
    let planes = prepare_workspace_planes(planes, 2)?.into_iter().map(|(normal, offset)| {
        let length = norm(&normal)?;
        Ok((normal, offset, length))
    }).collect::<Result<Vec<_>>>()?;
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, direction_span, profile], domain| {
            let point = curve_jets::enclose(position, span, domain)?;
            let axis = curve_jets::enclose(direction, direction_span, domain)?;
            let (rotation, regular) = planar_rotation(&axis[0])?;
            let radius = profiles[profile](domain)?;
            let plane_residuals = planes.iter().map(|(normal, offset, length)| {
                let support = rotating_hull_support(&offsets, normal, rotation)?;
                let residual = project_jet(&point[0], normal)?.sub(*offset)?
                    .add(support)?.add(length.mul(radius)?)?;
                Ok([residual.lo, residual.hi])
            }).collect::<Result<Vec<_>>>()?;
            let status = if regular {
                workspace_residual_status(&plane_residuals)
            } else {
                DirectionalMotionStatus::Undetermined
            };
            Ok((WorkspaceCell {
                parameter_interval: domain, span, plane_residuals, status,
            }, status == DirectionalMotionStatus::Undetermined))
        },
    )?;
    Ok(WorkspacePartition {
        cells: cells.into_iter().map(|(_, _, cell)| cell).collect(),
        work_plane_cells: work_cells * planes.len(),
    })
}

fn prepare_planar_rotation_profile(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    max_cells: usize,
) -> Result<(Vec<([usize; 3], [f64; 2])>, Vec<impl Fn([f64; 2]) -> Result<Interval>>)> {
    let (_, profiles) = prepare_radius_profile(position, radius_profile, max_cells)?;
    let interval = [radius_profile[0].0, radius_profile[radius_profile.len() - 1].0];
    let (_, synchronized) = synchronized_clearance_cells([position, direction], interval, [0.; 2])?;
    check(position.control_points[0].len() == 2, "Rotating hull clearance requires planar curves")?;
    let mut initial = Vec::new();
    let mut profile = 0;
    for ([span, direction_span], [mut start, end]) in synchronized {
        while start < end {
            if initial.len() == max_cells {
                return Err(resource("Clearance budget must cover the combined motion and radius spans"));
            }
            let profile_end = radius_profile[profile + 1].0;
            let cell_end = end.min(profile_end);
            initial.push(([span, direction_span, profile], [start, cell_end]));
            start = cell_end;
            if start == profile_end {
                profile += 1;
            }
        }
    }
    Ok((initial, profiles))
}

fn planar_rotation(axis: &[Interval]) -> Result<([Interval; 2], bool)> {
    let length = norm(axis)?;
    if length.lo <= 0. {
        return Ok(([Interval::new(-1., 1.)?; 2], false));
    }
    let normalize = |component: Interval| {
        let bound = component.div(length)?;
        Interval::new(bound.lo.max(-1.), bound.hi.min(1.))
    };
    Ok(([normalize(axis[0])?, normalize(axis[1])?], true))
}

fn rotating_hull_support(
    offsets: &[Vec<Interval>],
    normal: &[Interval],
    rotation: [Interval; 2],
) -> Result<Interval> {
    hull_plane_support(offsets, &inverse_rotated_normal(normal, rotation)?)
}

fn inverse_rotated_normal(
    normal: &[Interval],
    [cosine, sine]: [Interval; 2],
) -> Result<[Interval; 2]> {
    Ok([
        normal[0].mul(cosine)?.add(normal[1].mul(sine)?)?,
        normal[1].mul(cosine)?.sub(normal[0].mul(sine)?)?,
    ])
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceMinimumClearance {
    /// Signed clearance to the most restrictive plane, in model units.
    /// Negative values indicate that the tool extends outside the workspace.
    pub gap_bounds: [f64; 2],
    pub witness_parameter: f64,
    pub witness_plane: usize,
    pub work_plane_cells: usize,
    pub stop_reason: StopReason,
}

/// Bound the minimum signed planar workspace clearance in model units.
/// An orientation that cannot be certified nonzero within the work budget
/// produces an error. Plane normals need not have unit length.
pub fn measure_minimum_planar_rotating_hull_workspace_clearance(
    position: &Curve,
    direction: &Curve,
    interval: [f64; 2],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspaceMinimumClearance> {
    measure_minimum_planar_rotating_profiled_hull_workspace_clearance(
        position, direction, &[(interval[0], 0.), (interval[1], 0.)],
        planes, vertex_offsets, tolerance, max_plane_cells,
    )
}

/// Bound the minimum signed workspace clearance for a rotating planar hull
/// rounded by a piecewise-linear radius. The profile defines the checked
/// window. The orientation must be certified nonzero throughout it.
pub fn measure_minimum_planar_rotating_profiled_hull_workspace_clearance(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspaceMinimumClearance> {
    validate(tolerance, max_plane_cells)?;
    check(!planes.is_empty() && !vertex_offsets.is_empty(), "Workspace requires planes and hull vertices")?;
    let max_cells = (max_plane_cells / planes.len())
        .min(1_000_000 / vertex_offsets.len() / planes.len());
    let (initial, profiles) = prepare_planar_rotation_profile(position, direction, radius_profile, max_cells)?;
    let offsets = prepare_workspace_hull(position, vertex_offsets, planes.len())?;
    let planes = prepare_workspace_planes(planes, 2)?.into_iter().map(|(normal, offset)| {
        let length = norm(&normal)?;
        Ok((normal, offset, length))
    }).collect::<Result<Vec<_>>>()?;

    let (regular_cells, orientation_work) = partition_enclosed_cells(
        initial, 0., max_cells,
        |[_, span, _], domain| {
            let jets = curve_jets::enclose(direction, span, domain)?;
            let regular = norm(&jets[0])?.lo > 0.;
            Ok((regular, !regular))
        },
    )?;
    check(regular_cells.iter().all(|(_, _, regular)| *regular),
        "Rotating hull clearance requires a certified nonzero orientation")?;
    let initial = regular_cells.into_iter().map(|(spans, domain, _)| (spans, domain));

    let measure = |[span, direction_span, profile]: [usize; 3], domain: [f64; 2], witness: &mut ClearanceWitness| {
        let position_cell = curve_jets::enclose(position, span, domain)?;
        let direction_cell = curve_jets::enclose(direction, direction_span, domain)?;
        let (rotation, _) = planar_rotation(&direction_cell[0])?;
        let radius = profiles[profile](domain)?;
        let mut lower = f64::INFINITY;
        for (normal, offset, length) in &planes {
            let support = rotating_hull_support(&offsets, normal, rotation)?;
            let gap = offset.sub(project_jet(&position_cell[0], normal)?)?.sub(support)?.div(*length)?.sub(radius)?;
            lower = lower.min(gap.lo);
        }
        for (end, parameter) in [(false, domain[0]), (true, domain[1])] {
            let point = curve_jets::endpoint(position, span, domain, end)?;
            let axis = curve_jets::endpoint(direction, direction_span, domain, end)?;
            let (rotation, _) = planar_rotation(&axis[0])?;
            let radius = profiles[profile]([parameter; 2])?;
            for (index, (normal, offset, length)) in planes.iter().enumerate() {
                let support = rotating_hull_support(&offsets, normal, rotation)?;
                let gap = offset.sub(project_jet(&point[0], normal)?)?.sub(support)?.div(*length)?.sub(radius)?;
                if gap.hi < witness.0 {
                    *witness = (gap.hi, parameter, index);
                }
            }
        }
        Ok(lower)
    };
    let (lower, witness, cells, stop_reason) = refine_minimum_clearance(
        initial, radius_profile[0].0, tolerance, max_cells - orientation_work, measure,
    )?;
    Ok(WorkspaceMinimumClearance {
        gap_bounds: [lower, witness.0],
        witness_parameter: witness.1,
        witness_plane: witness.2,
        work_plane_cells: (orientation_work + cells) * planes.len(),
        stop_reason,
    })
}

/// Refine the smallest signed plane clearance along a ball toolpath with a
/// piecewise-linear radius. Plane normals need not be normalized.
pub fn measure_minimum_profiled_radius_workspace_clearance(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspaceMinimumClearance> {
    measure_minimum_profiled_workspace_with_support(
        curve, radius_profile, planes, tolerance, max_plane_cells, |_| Ok(None),
    )
}

/// Refine the minimum workspace clearance for a translating convex vertex
/// hull rounded by a variable radius, in the original model units.
pub fn measure_minimum_profiled_rounded_hull_workspace_clearance(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    vertex_offsets: &[&[f64]],
    tolerance: f64,
    max_plane_cells: usize,
) -> Result<WorkspaceMinimumClearance> {
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    curve.validate()?;
    let offsets = prepare_workspace_hull(curve, vertex_offsets, planes.len())?;
    measure_minimum_profiled_workspace_with_support(
        curve, radius_profile, planes, tolerance, max_plane_cells,
        |normal| hull_plane_support(&offsets, normal).map(Some),
    )
}

fn measure_minimum_profiled_workspace_with_support(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    planes: &[WorkspaceHalfSpace<'_>],
    tolerance: f64,
    max_plane_cells: usize,
    support: impl Fn(&[Interval]) -> Result<Option<Interval>>,
) -> Result<WorkspaceMinimumClearance> {
    validate(tolerance, max_plane_cells)?;
    check(!planes.is_empty(), "Workspace requires at least one half-space")?;
    let max_cells = max_plane_cells / planes.len();
    let (initial, profiles) = prepare_radius_profile(curve, radius_profile, max_cells)?;
    let dimension = curve.control_points[0].len();
    let planes = prepare_workspace_planes(planes, dimension)?.into_iter().map(|(normal, offset)| {
        let length = norm(&normal)?;
        let hull = support(&normal)?;
        Ok((normal, offset, length, hull))
    }).collect::<Result<Vec<_>>>()?;
    let minimum = measure_minimum_radial_cells(
        curve, initial, radius_profile[0].0, tolerance, max_cells, planes.len(),
        |point, index, profile, domain| {
            let (normal, offset, length, hull) = &planes[index];
            let mut residual = offset.sub(project_jet(point, normal)?)?;
            if let Some(hull) = hull {
                residual = residual.sub(*hull)?;
            }
            residual.div(*length)?.sub(profiles[profile](domain)?)
        },
    )?;
    Ok(WorkspaceMinimumClearance {
        gap_bounds: minimum.gap_bounds,
        witness_parameter: minimum.witness_parameter,
        witness_plane: minimum.witness_obstacle,
        work_plane_cells: minimum.work_obstacle_cells,
        stop_reason: minimum.stop_reason,
    })
}

fn workspace_residual_status(residuals: &[[f64; 2]]) -> DirectionalMotionStatus {
    if residuals.iter().any(|[lo, _]| *lo > 0.) {
        DirectionalMotionStatus::Exceeds
    } else if residuals.iter().all(|[_, hi]| *hi <= 0.) {
        DirectionalMotionStatus::Within
    } else {
        DirectionalMotionStatus::Undetermined
    }
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

/// Partition clearance from closed convex obstacles for a rotating planar
/// vertex hull. A plane separating all tool vertices proves clearance, while
/// one tool vertex inside all planes proves contact. Other cases, including
/// possible zero orientation, remain undetermined.
pub fn partition_planar_rotating_hull_obstacle_clearance(
    position: &Curve,
    direction: &Curve,
    interval: [f64; 2],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_planar_rotating_rounded_hull_obstacle_clearance(
        position, direction, interval, obstacles, vertex_offsets, 0.,
        parameter_tolerance, max_vertex_plane_cells,
    )
}

/// Check a rotating planar convex hull expanded by a disk of constant
/// radius. A plane must separate the whole rounded tool to prove clearance.
/// A vertex of the unrounded hull inside the obstacle still proves contact.
/// Contacts that only involve the rounding can remain undetermined.
pub fn partition_planar_rotating_rounded_hull_obstacle_clearance(
    position: &Curve,
    direction: &Curve,
    interval: [f64; 2],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    radius: f64,
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_planar_rotating_profiled_hull_obstacle_clearance(
        position, direction, &[(interval[0], radius), (interval[1], radius)],
        obstacles, vertex_offsets, parameter_tolerance, max_vertex_plane_cells,
    )
}

/// Check a rotating planar hull with a piecewise-linear rounding radius.
/// Profile parameters define the checked window and must increase strictly.
/// A shared grid splits at both curves' knots and the radius breakpoints.
pub fn partition_planar_rotating_profiled_hull_obstacle_clearance(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 0, ChordWitness::None,
    )
}

/// Also seek a rounded contact witness by displacing a hull vertex inward
/// along one obstacle plane normal. The displaced vertex must satisfy every
/// half-space throughout the cell. The budget reserves two vertex/plane
/// passes per cell to account for checking these witnesses.
pub fn partition_planar_rotating_profiled_hull_obstacle_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 1, ChordWitness::None,
    )
}

/// Also seek rounded contact along the bisector of the two most violated
/// obstacle planes. Each candidate is verified against every half-space
/// across the cell. The budget reserves three vertex/plane passes per cell.
pub fn partition_planar_rotating_profiled_hull_obstacle_corner_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 2, ChordWitness::None,
    )
}

/// Also seek an asymmetric rounded contact by weighting inward normals
/// by the relative plane violations. Each displacement must stay within
/// the rounding disk and satisfy every half-space across the cell. The
/// work budget reserves four vertex/plane passes per cell.
pub fn partition_planar_rotating_profiled_hull_obstacle_asymmetric_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 3, ChordWitness::None,
    )
}

/// Also seek a rounded contact direction by solving the two most violated
/// plane constraints together. All obstacle half-spaces are verified over
/// the cell. The work budget reserves five vertex/plane passes per cell.
pub fn partition_planar_rotating_profiled_hull_obstacle_skew_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 4, ChordWitness::None,
    )
}

/// Test convex-hull chord midpoints as additional contact witnesses, using
/// the same rounding and whole-cell half-space checks as vertex witnesses.
/// Every unordered vertex pair contributes one midpoint to the work budget.
pub fn partition_planar_rotating_profiled_hull_obstacle_chord_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 4, ChordWitness::Midpoint,
    )
}

/// Clip each hull chord against conservative obstacle plane bounds to find
/// a fixed point that lies inside the obstacle throughout the cell.
/// This supplements rounded vertex checks with unrounded chord witnesses.
pub fn partition_planar_rotating_profiled_hull_obstacle_clipped_chord_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 4, ChordWitness::Clipped,
    )
}

/// Clip the conservative chord bounds using exact rational arithmetic.
/// A feasible chord parameter may be a singleton or need more than binary64
/// precision. Rounded vertex witnesses are also checked.
pub fn partition_planar_rotating_profiled_hull_obstacle_exact_chord_contact(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
) -> Result<ObstacleClearancePartition> {
    partition_rotating_profiled_obstacles(
        position, direction, radius_profile, obstacles, vertex_offsets,
        parameter_tolerance, max_vertex_plane_cells, 4, ChordWitness::Exact,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChordWitness {
    None,
    Midpoint,
    Clipped,
    Exact,
}

fn partition_rotating_profiled_obstacles(
    position: &Curve,
    direction: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[&[WorkspaceHalfSpace<'_>]],
    vertex_offsets: &[&[f64]],
    parameter_tolerance: f64,
    max_vertex_plane_cells: usize,
    witness_passes: usize,
    chords: ChordWitness,
) -> Result<ObstacleClearancePartition> {
    use DirectionalMotionStatus::{Exceeds, Undetermined, Within};

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
    let vertex_count = vertex_offsets.len();
    let chord_count = if chords != ChordWitness::None {
        vertex_count.checked_mul(vertex_count - 1)
            .ok_or_else(|| resource("Tool chord count exceeds the work budget"))? / 2
    } else {
        0
    };
    let chord_passes = match chords {
        ChordWitness::None => 0,
        ChordWitness::Midpoint => 1 + witness_passes,
        ChordWitness::Clipped | ChordWitness::Exact => 2,
    };
    let witness_work = chord_count.checked_mul(chord_passes)
        .and_then(|work| work.checked_add(vertex_count * (1 + witness_passes)))
        .ok_or_else(|| resource("Tool chord work exceeds the work budget"))?;
    if witness_work > max_vertex_plane_cells / plane_count {
        return Err(resource("Clearance budget must cover every witness and plane"));
    }
    let pairs = plane_count * witness_work;
    let max_cells = max_vertex_plane_cells / pairs;
    let (initial, profiles) = prepare_planar_rotation_profile(position, direction, radius_profile, max_cells)?;
    let mut vertices = prepare_workspace_hull(position, vertex_offsets, plane_count)?;
    if chords == ChordWitness::Midpoint {
        let half = Interval::point(0.5);
        vertices.reserve(chord_count);
        for end in 1..vertex_count {
            for start in 0..end {
                let a = &vertices[start];
                let b = &vertices[end];
                let midpoint = vec![
                    a[0].mul(half)?.add(b[0].mul(half)?)?,
                    a[1].mul(half)?.add(b[1].mul(half)?)?,
                ];
                vertices.push(midpoint);
            }
        }
    }
    let has_rounding = radius_profile.iter().any(|&(_, radius)| radius > 0.);
    let obstacles = obstacles.iter().map(|planes| {
        prepare_workspace_planes(planes, 2)?.into_iter().map(|(normal, offset)| {
            let length = if has_rounding { norm(&normal)? } else { Interval::point(0.) };
            Ok((normal, offset, length))
        }).collect::<Result<Vec<_>>>()
    }).collect::<Result<Vec<_>>>()?;

    let exact_rotation = if chords == ChordWitness::Exact {
        fixed_cardinal_rotation(direction)
    } else {
        None
    };
    let constant_position = [0, 1].map(|axis| {
        let value = position.control_points[0][axis];
        position.control_points.iter().all(|point| point[axis] == value).then_some(value)
    });

    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, direction_span, profile], domain| {
            let axis = curve_jets::enclose(direction, direction_span, domain)?;
            let (rotation, regular) = planar_rotation(&axis[0])?;
            let mut clearance = vec![Undetermined; obstacles.len()];
            if regular {
                let radius = profiles[profile](domain)?;
                let point = curve_jets::enclose(position, span, domain)?;
                let mut candidates = vec![true; vertices.len()];
                let mut witness_planes = vec![[(0, 0.); 2]; vertices.len()];
                for (obstacle, status) in obstacles.iter().zip(&mut clearance) {
                    candidates.fill(true);
                    witness_planes.fill([(0, 0.); 2]);
                    let seek_rounded = witness_passes > 0 && radius.lo > 0.;
                    let mut residuals = Vec::new();
                    for (plane, (normal, offset, length)) in obstacle.iter().enumerate() {
                        let margin = length.mul(radius)?.hi;
                        let inverse = inverse_rotated_normal(normal, rotation)?;
                        let center = project_jet(&point[0], normal)?.sub(*offset)?;
                        let mut separated = true;
                        for (vertex, offset) in vertices.iter().enumerate() {
                            let residual = center.add(project_jet(offset, &inverse)?)?;
                            separated &= residual.lo > margin;
                            candidates[vertex] &= residual.hi <= 0.;
                            if seek_rounded || matches!(chords, ChordWitness::Clipped | ChordWitness::Exact) {
                                residuals.push(residual);
                            }
                            if seek_rounded {
                                let violation = residual.hi / length.hi;
                                let [first, second] = &mut witness_planes[vertex];
                                if violation > first.1 {
                                    *second = *first;
                                    *first = (plane, violation);
                                } else if violation > second.1 {
                                    *second = (plane, violation);
                                }
                            }
                        }
                        if separated {
                            *status = Within;
                            break;
                        }
                    }
                    if *status == Within {
                        continue;
                    }
                    if candidates.contains(&true)
                        || (chords == ChordWitness::Clipped && clipped_chord_contacts(&residuals, vertex_count)?)
                        || (chords == ChordWitness::Exact && exact_chord_contacts(
                            &residuals, vertex_offsets, obstacle, exact_rotation, constant_position,
                        )?)
                        || (seek_rounded && rounded_vertex_contacts(
                        obstacle, &residuals, &witness_planes, radius.lo, witness_passes,
                    )?) {
                        *status = Exceeds;
                    }
                }
            }
            let refine = clearance.contains(&Undetermined);
            let status = if clearance.contains(&Exceeds) {
                Exceeds
            } else if refine {
                Undetermined
            } else {
                Within
            };
            Ok((ObstacleClearanceCell {
                parameter_interval: domain, span, obstacles: clearance, status,
            }, refine))
        },
    )?;
    Ok(ObstacleClearancePartition {
        cells: cells.into_iter().map(|(_, _, cell)| cell).collect(),
        work_plane_cells: work_cells * pairs,
    })
}

fn clipped_chord_contacts(residuals: &[Interval], vertex_count: usize) -> Result<bool> {
    for end in 1..vertex_count {
        'chords: for start in 0..end {
            let mut lower: f64 = 0.;
            let mut upper: f64 = 1.;
            for plane in residuals.chunks_exact(vertex_count) {
                let (a, b) = (plane[start].hi, plane[end].hi);
                if a > 0. && b > 0. {
                    continue 'chords;
                }
                if a > 0. || b > 0. {
                    let scale = a.abs().max(b.abs());
                    let left = a.abs() / scale;
                    let right = b.abs() / scale;
                    let crossing = left / (left + right);
                    if a > 0. {
                        lower = lower.max(crossing);
                    } else {
                        upper = upper.min(crossing);
                    }
                }
                if lower > upper {
                    continue 'chords;
                }
            }
            // Clipping only proposes a candidate. Verify the interpolation
            // with interval arithmetic before treating it as a witness.
            let t = Interval::point(crate::distance_bounds::midpoint(lower, upper));
            let complement = Interval::point(1.).sub(t)?;
            for plane in residuals.chunks_exact(vertex_count) {
                let bound = plane[start].mul(complement)?.add(plane[end].mul(t)?)?;
                if bound.hi > 0. {
                    continue 'chords;
                }
            }
            return Ok(true);
        }
    }
    Ok(false)
}

fn fixed_cardinal_rotation(direction: &Curve) -> Option<[f64; 2]> {
    for axis in [0, 1] {
        let sign = direction.control_points[0][axis].signum();
        if direction.control_points.iter().all(|point| {
            point[1 - axis] == 0. && point[axis] * sign > 0.
        }) {
            let mut rotation = [0.; 2];
            rotation[axis] = sign;
            return Some(rotation);
        }
    }
    None
}

fn exact_chord_contacts(
    residuals: &[Interval],
    vertices: &[&[f64]],
    planes: &[(Vec<Interval>, Interval, Interval)],
    rotation: Option<[f64; 2]>,
    position: [Option<f64>; 2],
) -> Result<bool> {
    let zero = exact_motion_operand(0.)?;
    let one = exact_motion_operand(1.)?;
    let vertex_count = vertices.len();
    let mut bounds = Vec::with_capacity(residuals.len());
    for (plane, (normal, offset, _)) in residuals.chunks_exact(vertex_count).zip(planes) {
        let [nx, ny] = [normal[0].lo, normal[1].lo];
        let exact = rotation.filter(|_| {
            (nx == 0. || position[0].is_some()) && (ny == 0. || position[1].is_some())
        });
        if let Some([cosine, sine]) = exact {
            // Positive weights preserve a constant coordinate throughout
            // the curve. A cardinal rotation only swaps or negates axes.
            let center = exact_motion_operand(nx)? * exact_motion_operand(position[0].unwrap_or(0.))?
                + exact_motion_operand(ny)? * exact_motion_operand(position[1].unwrap_or(0.))?
                - exact_motion_operand(offset.lo)?;
            let local_x = exact_motion_operand(nx * cosine + ny * sine)?;
            let local_y = exact_motion_operand(ny * cosine - nx * sine)?;
            for point in vertices {
                bounds.push(&center + &local_x * exact_motion_operand(point[0])?
                    + &local_y * exact_motion_operand(point[1])?);
            }
        } else {
            for residual in plane {
                bounds.push(exact_motion_operand(residual.hi)?);
            }
        }
    }
    for end in 1..vertex_count {
        'chords: for start in 0..end {
            let mut lower = zero.clone();
            let mut upper = one.clone();
            for plane in bounds.chunks_exact(vertex_count) {
                let (a, b) = (&plane[start], &plane[end]);
                if a > &zero && b > &zero {
                    continue 'chords;
                }
                if a > &zero || b > &zero {
                    let crossing = a / (a - b);
                    if a > &zero {
                        lower = lower.max(crossing);
                    } else {
                        upper = upper.min(crossing);
                    }
                    if lower > upper {
                        continue 'chords;
                    }
                }
            }
            // The exact affine interpolation of the endpoint upper bounds
            // bounds the same fixed tool-local chord point across the cell.
            return Ok(true);
        }
    }
    Ok(false)
}

fn rounded_vertex_contacts(
    planes: &[(Vec<Interval>, Interval, Interval)],
    residuals: &[Interval],
    witness_planes: &[[(usize, f64); 2]],
    radius: f64,
    witness_passes: usize,
) -> Result<bool> {
    let vertex_count = witness_planes.len();
    for (vertex, &[(first, first_violation), (second, second_violation)]) in witness_planes.iter().enumerate() {
        let inside_after_displacement = |axis: [Interval; 2], length_upper: f64| -> Result<bool> {
            if length_upper == 0. {
                return Ok(false);
            }
            // Dividing by the upper length keeps the displacement within
            // the lower rounding radius for the entire cell.
            let scale = Interval::point(-radius).div(Interval::point(length_upper))?;
            let displacement = [axis[0].mul(scale)?, axis[1].mul(scale)?];
            for (index, (normal, _, _)) in planes.iter().enumerate() {
                let shifted = residuals[index * vertex_count + vertex]
                    .add(project_jet(&displacement, normal)?)?;
                if shifted.hi > 0. {
                    return Ok(false);
                }
            }
            Ok(true)
        };
        let (normal, _, length) = &planes[first];
        if inside_after_displacement([normal[0], normal[1]], length.hi)? {
            return Ok(true);
        }
        if witness_passes >= 2 && second_violation > 0. {
            let (other, _, other_length) = &planes[second];
            let unit = [
                normal[0].div(Interval::point(length.hi))?,
                normal[1].div(Interval::point(length.hi))?,
            ];
            let other_unit = [
                other[0].div(Interval::point(other_length.hi))?,
                other[1].div(Interval::point(other_length.hi))?,
            ];
            let bisector = [unit[0].add(other_unit[0])?, unit[1].add(other_unit[1])?];
            let length = norm(&bisector)?;
            if length.lo > 0. && inside_after_displacement(bisector, length.hi)? {
                return Ok(true);
            }
            if witness_passes >= 3 && first_violation.is_finite() {
                let weight = Interval::point(second_violation / first_violation);
                let axis = [
                    unit[0].add(other_unit[0].mul(weight)?)?,
                    unit[1].add(other_unit[1].mul(weight)?)?,
                ];
                let length = norm(&axis)?;
                if length.lo > 0. && inside_after_displacement(axis, length.hi)? {
                    return Ok(true);
                }
            }
            if witness_passes >= 4 && first_violation.is_finite() {
                let determinant = unit[0].mul(other_unit[1])?.sub(unit[1].mul(other_unit[0])?)?;
                if determinant.lo > 0. || determinant.hi < 0. {
                    let sign = Interval::point(if determinant.lo > 0. { 1. } else { -1. });
                    let weight = Interval::point(second_violation / first_violation);
                    // Only the direction of the solution is needed. Cancel
                    // the determinant magnitude before normalizing it.
                    let axis = [
                        other_unit[1].sub(unit[1].mul(weight)?)?.mul(sign)?,
                        unit[0].mul(weight)?.sub(other_unit[0])?.mul(sign)?,
                    ];
                    let length = norm(&axis)?;
                    if length.lo > 0. && inside_after_displacement(axis, length.hi)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
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
        |point, index, _| {
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
    gap: impl Fn(&[Interval], usize, [f64; 2]) -> Result<Interval>,
) -> Result<SphericalMinimumClearance> {
    let initial = active_spans(curve, interval).map(|span| {
        ([span, 0], [curve.knots[span].max(interval[0]), curve.knots[span + 1].min(interval[1])])
    });
    measure_minimum_radial_cells(
        curve, initial, interval[0], tolerance, max_cells, obstacle_count,
        |point, index, _, domain| gap(point, index, domain),
    )
}

fn measure_minimum_radial_cells(
    curve: &Curve,
    initial: impl IntoIterator<Item = ([usize; 2], [f64; 2])>,
    start_parameter: f64,
    tolerance: f64,
    max_cells: usize,
    obstacle_count: usize,
    gap: impl Fn(&[Interval], usize, usize, [f64; 2]) -> Result<Interval>,
) -> Result<SphericalMinimumClearance> {
    let measure = |[span, profile]: [usize; 2], domain: [f64; 2], witness: &mut (f64, f64, usize)| -> Result<f64> {
        let jets = curve_jets::enclose(curve, span, domain)?;
        let start = curve_jets::endpoint(curve, span, domain, false)?;
        let end = curve_jets::endpoint(curve, span, domain, true)?;
        let mut lower = f64::INFINITY;
        for index in 0..obstacle_count {
            lower = lower.min(gap(&jets[0], index, profile, domain)?.lo);
            for (point, parameter) in [(&start[0], domain[0]), (&end[0], domain[1])] {
                let bound = gap(point, index, profile, [parameter; 2])?.hi;
                if bound < witness.0 {
                    *witness = (bound, parameter, index);
                }
            }
        }
        Ok(lower)
    };
    let (lower, witness, cells, stop_reason) =
        refine_minimum_clearance(initial, start_parameter, tolerance, max_cells, measure)?;
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
    let phases = prepare_schedule_with_holds(segments, start_times, radii)?;
    let phase_count = phases.len();
    if phase_count > max_cells {
        return Err(resource("Clearance budget must cover every scheduled phase"));
    }
    let mut cells = Vec::new();
    let mut remaining = max_cells;
    for (index, phase) in phases.iter().enumerate() {
        let report = partition_scheduled_phase(
            phase, progress_tolerance, remaining / (phase_count - index),
        )?;
        remaining -= report.work_cells;
        cells.extend(report.cells);
    }
    Ok(ScheduledClearancePartition { cells, work_cells: max_cells - remaining })
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduledMinimumClearance {
    pub gap_bounds: [f64; 2],
    pub witness_time_bounds: [f64; 2],
    pub work_cells: usize,
    pub stop_reason: StopReason,
}

/// Refine the minimum gap over the full schedule, holding each tool at the
/// corresponding endpoint outside its move. All phases share one work budget.
pub fn measure_scheduled_clearance_with_holds(
    segments: [TimedMotionSegment<'_>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<ScheduledMinimumClearance> {
    validate(tolerance, max_cells)?;
    let phases = prepare_schedule_with_holds(segments, start_times, radii)?;
    measure_scheduled_phase_minimum(&phases, tolerance, max_cells).map(|(report, _)| report)
}

/// A spherical tool with an independently scheduled curve traversal.
#[derive(Clone, Copy, Debug)]
pub struct ScheduledTool<'a> {
    pub segment: TimedMotionSegment<'a>,
    pub start_time: f64,
    pub radius: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduledToolsMinimumClearance {
    pub minimum: ScheduledMinimumClearance,
    /// Input indices of the pair supplying the upper-bound witness.
    pub witness_tools: [usize; 2],
}

/// Refine the smallest gap among all unordered tool pairs with one budget.
/// Each tool stays at its corresponding endpoint outside its traversal.
pub fn measure_scheduled_tools_clearance(
    tools: &[ScheduledTool<'_>],
    tolerance: f64,
    max_cells: usize,
) -> Result<ScheduledToolsMinimumClearance> {
    validate(tolerance, max_cells)?;
    let (phases, pairs) = prepare_scheduled_tool_pairs(tools, max_cells)?;
    let (minimum, phase) = measure_scheduled_phase_minimum(&phases, tolerance, max_cells)?;
    Ok(ScheduledToolsMinimumClearance { minimum, witness_tools: pairs[phase] })
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduledToolPairCell {
    pub tools: [usize; 2],
    pub clearance: ScheduledClearanceCell,
}

#[derive(Clone, Debug)]
pub struct ScheduledToolsClearancePartition {
    /// Cells grouped by pair and ordered in time within each pair.
    pub cells: Vec<ScheduledToolPairCell>,
    pub work_cells: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FirstScheduledToolContact {
    Clear,
    Possible { earliest_time: f64 },
    Confirmed {
        time_bounds: [f64; 2],
        /// Pair whose contact supplies the upper time bound.
        witness_tools: [usize; 2],
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LastScheduledToolContact {
    Clear,
    Possible { latest_time: f64 },
    Confirmed {
        time_bounds: [f64; 2],
        /// Pair whose contact supplies the lower time bound.
        witness_tools: [usize; 2],
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimultaneousContactWindows {
    /// Closed intervals in which the requested number of pairs is confirmed.
    pub confirmed: Vec<[f64; 2]>,
    /// Closed intervals covering every time the threshold could be met.
    pub possible: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeakScheduledContactWitness {
    pub time: f64,
    /// Distinct pairs whose simultaneous contact is confirmed at this time.
    pub tools: Vec<[usize; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeakScheduledContactReport {
    pub pair_count_bounds: [usize; 2],
    /// First time attaining the confirmed lower bound, if it is positive.
    pub witness: Option<PeakScheduledContactWitness>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledContactPairs {
    /// Sorted distinct pairs proved to be in contact.
    pub confirmed: Vec<[usize; 2]>,
    /// Sorted distinct pairs whose contact remains possible, including confirmed pairs.
    pub possible: Vec<[usize; 2]>,
}

impl ScheduledToolsClearancePartition {
    /// Query contact at a finite absolute time within the represented
    /// pair timelines. Shared endpoints are included in both adjacent cells.
    pub fn contact_pairs_at_time(&self, time: f64) -> Result<ScheduledContactPairs> {
        check(time.is_finite(), "Contact query time must be finite")?;
        let mut confirmed = Vec::new();
        let mut possible = Vec::new();
        for cell in &self.cells {
            let [[start_lo, start_hi], [end_lo, end_hi]] = cell.clearance.time_endpoint_bounds;
            if cell.clearance.status != DirectionalMotionStatus::Within
                && start_lo <= time && time <= end_hi
            {
                possible.push(cell.tools);
            }
            if cell.clearance.status == DirectionalMotionStatus::Exceeds
                && start_hi <= time && time <= end_lo
            {
                confirmed.push(cell.tools);
            }
        }
        for pairs in [&mut confirmed, &mut possible] {
            pairs.sort_unstable();
            pairs.dedup();
        }
        Ok(ScheduledContactPairs { confirmed, possible })
    }

    /// Bound the maximum number of distinct pairs in simultaneous contact
    /// across the represented timelines. Contacts include shared endpoints.
    pub fn peak_contact_pair_count_bounds(&self) -> [usize; 2] {
        self.contact_pair_intervals().map(|intervals| peak_pair_count(&intervals))
    }

    /// Identify a concrete time and the contacting pairs that establish
    /// the lower bound on peak contact multiplicity.
    pub fn peak_contact_report(&self) -> PeakScheduledContactReport {
        let [confirmed, possible] = self.contact_pair_intervals();
        let pair_count_bounds = [peak_pair_count(&confirmed), peak_pair_count(&possible)];
        let mut witness = None;
        if pair_count_bounds[0] > 0 {
            sweep_contact_pairs(&confirmed, |active, [start, _]| {
                if witness.is_none() && active.len() == pair_count_bounds[0] {
                    witness = Some(PeakScheduledContactWitness {
                        time: start,
                        tools: active.keys().copied().collect(),
                    });
                }
            });
        }
        PeakScheduledContactReport { pair_count_bounds, witness }
    }

    /// Bound elapsed time with at least the requested number of distinct
    /// pairs in contact. An isolated instant contributes zero duration.
    pub fn simultaneous_contact_duration_bounds(&self, minimum_pairs: usize) -> Result<[f64; 2]> {
        let windows = self.simultaneous_contact_windows(minimum_pairs)?;
        let duration = |intervals: Vec<[f64; 2]>, upper| {
            let intervals = intervals.into_iter().filter(|[a, b]| a < b).collect::<Vec<_>>();
            if intervals.iter().flatten().any(|time| !time.is_finite()) {
                return Ok(f64::INFINITY);
            }
            round_motion_duration(&time_interval_union_duration(intervals, |a, b| a + b)?, upper)
        };
        Ok([duration(windows.confirmed, false)?, duration(windows.possible, true)?])
    }

    /// Locate intervals where at least the requested number of distinct
    /// pairs are in contact. Instantaneous contacts appear as point intervals.
    pub fn simultaneous_contact_windows(
        &self,
        minimum_pairs: usize,
    ) -> Result<SimultaneousContactWindows> {
        check(minimum_pairs > 0, "Simultaneous contact requires a positive pair count")?;
        let [confirmed, possible] = self.contact_pair_intervals().map(|intervals| {
            let mut windows: Vec<[f64; 2]> = Vec::new();
            sweep_contact_pairs(&intervals, |active, [start, end]| {
                if active.len() >= minimum_pairs {
                    match windows.last_mut() {
                        Some([_, previous_end]) if start == *previous_end => *previous_end = end,
                        _ => windows.push([start, end]),
                    }
                }
            });
            windows
        });
        Ok(SimultaneousContactWindows { confirmed, possible })
    }

    fn contact_pair_intervals(&self) -> [Vec<ToolPairInterval>; 2] {
        let mut confirmed = Vec::new();
        let mut possible = Vec::new();
        for cell in &self.cells {
            let [[start_lo, start_hi], [end_lo, end_hi]] = cell.clearance.time_endpoint_bounds;
            if cell.clearance.status == DirectionalMotionStatus::Exceeds
                && start_hi.is_finite() && end_lo.is_finite() && start_hi <= end_lo
            {
                confirmed.push((cell.tools, [start_hi, end_lo]));
            }
            if cell.clearance.status != DirectionalMotionStatus::Within {
                possible.push((cell.tools, [start_lo, end_hi]));
            }
        }
        [confirmed, possible]
    }

    /// Bound the total duration of contact across the represented pair
    /// timelines. Simultaneous contacts contribute once. Confirmed cells
    /// contribute inward-rounded intervals to the lower bound; all nonclear
    /// cells contribute outward-rounded intervals to the upper bound.
    pub fn contact_duration_bounds(&self) -> Result<[f64; 2]> {
        self.contact_duration_bounds_with(|total, next| total + next)
    }

    /// Bound the longest continuous interval with contact on at least one
    /// pair, including transitions between different pairs.
    pub fn longest_contact_duration_bounds(&self) -> Result<[f64; 2]> {
        self.contact_duration_bounds_with(num_rational::BigRational::max)
    }

    fn contact_duration_bounds_with(
        &self,
        combine: impl Fn(num_rational::BigRational, num_rational::BigRational) -> num_rational::BigRational,
    ) -> Result<[f64; 2]> {
        let mut confirmed = Vec::new();
        let mut possible = Vec::new();
        let mut unbounded_upper = false;
        for cell in &self.cells {
            let [[start_lo, start_hi], [end_lo, end_hi]] = cell.clearance.time_endpoint_bounds;
            if cell.clearance.status == DirectionalMotionStatus::Exceeds
                && start_hi.is_finite() && end_lo.is_finite() && start_hi < end_lo
            {
                confirmed.push([start_hi, end_lo]);
            }
            if cell.clearance.status != DirectionalMotionStatus::Within {
                if start_lo.is_finite() && end_hi.is_finite() {
                    possible.push([start_lo, end_hi]);
                } else {
                    unbounded_upper = true;
                }
            }
        }
        let lower = round_motion_duration(&time_interval_union_duration(confirmed, &combine)?, false)?;
        let upper = if unbounded_upper {
            f64::INFINITY
        } else {
            round_motion_duration(&time_interval_union_duration(possible, &combine)?, true)?
        };
        Ok([lower, upper])
    }

    /// Bracket the last contact across the represented pair timelines.
    /// A possible later contact on any pair contributes to the upper bound.
    pub fn last_contact(&self) -> LastScheduledToolContact {
        let mut latest: Option<f64> = None;
        let mut witness: Option<(f64, [usize; 2])> = None;
        for cell in &self.cells {
            let [lo, hi] = cell.clearance.time_endpoint_bounds[1];
            if cell.clearance.status != DirectionalMotionStatus::Within {
                latest = Some(latest.map_or(hi, |previous| previous.max(hi)));
            }
            if cell.clearance.status == DirectionalMotionStatus::Exceeds
                && witness.is_none_or(|(time, tools)| lo > time || (lo == time && cell.tools < tools))
            {
                witness = Some((lo, cell.tools));
            }
        }
        match (latest, witness) {
            (Some(upper), Some((lower, witness_tools))) => LastScheduledToolContact::Confirmed {
                time_bounds: [lower, upper],
                witness_tools,
            },
            (Some(latest_time), None) => LastScheduledToolContact::Possible { latest_time },
            (None, _) => LastScheduledToolContact::Clear,
        }
    }

    /// Bracket the first contact across the represented pair timelines.
    /// A possible earlier contact on any pair contributes to the lower bound.
    pub fn first_contact(&self) -> FirstScheduledToolContact {
        let mut earliest: Option<f64> = None;
        let mut witness: Option<(f64, [usize; 2])> = None;
        for cell in &self.cells {
            let [lo, hi] = cell.clearance.time_endpoint_bounds[0];
            if cell.clearance.status != DirectionalMotionStatus::Within {
                earliest = Some(earliest.map_or(lo, |previous| previous.min(lo)));
            }
            if cell.clearance.status == DirectionalMotionStatus::Exceeds
                && witness.is_none_or(|(time, tools)| (hi, cell.tools) < (time, tools))
            {
                witness = Some((hi, cell.tools));
            }
        }
        match (earliest, witness) {
            (Some(lower), Some((upper, witness_tools))) => FirstScheduledToolContact::Confirmed {
                time_bounds: [lower, upper],
                witness_tools,
            },
            (Some(earliest_time), None) => FirstScheduledToolContact::Possible { earliest_time },
            (None, _) => FirstScheduledToolContact::Clear,
        }
    }
}

type ToolPairInterval = ([usize; 2], [f64; 2]);

fn peak_pair_count(intervals: &[ToolPairInterval]) -> usize {
    let mut peak = 0;
    sweep_contact_pairs(intervals, |active, _| peak = peak.max(active.len()));
    peak
}

fn sweep_contact_pairs(
    intervals: &[ToolPairInterval],
    mut visit: impl FnMut(&BTreeMap<[usize; 2], usize>, [f64; 2]),
) {
    let mut events = Vec::with_capacity(2 * intervals.len());
    for &(pair, [start, end]) in intervals {
        events.push((if start == 0. { 0. } else { start }, false, pair));
        events.push((if end == 0. { 0. } else { end }, true, pair));
    }
    // Starts precede ends at the same instant because the cells are closed.
    events.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut active = BTreeMap::<[usize; 2], usize>::new();
    let mut previous = events.first().map_or(0., |event| event.0);
    for (time, ending, pair) in events {
        visit(&active, [previous, time]);
        previous = time;
        if ending {
            if let std::collections::btree_map::Entry::Occupied(mut entry) = active.entry(pair) {
                *entry.get_mut() -= 1;
                if *entry.get() == 0 {
                    entry.remove();
                }
            }
        } else {
            *active.entry(pair).or_default() += 1;
        }
    }
}

fn time_interval_union_duration(
    mut intervals: Vec<[f64; 2]>,
    combine: impl Fn(num_rational::BigRational, num_rational::BigRational) -> num_rational::BigRational,
) -> Result<num_rational::BigRational> {
    intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut total = num_rational::BigRational::default();
    let mut current: Option<[f64; 2]> = None;
    for [start, end] in intervals {
        match current {
            Some([a, b]) if start <= b => current = Some([a, b.max(end)]),
            Some([a, b]) => {
                total = combine(total, exact_motion_operand(b)? - exact_motion_operand(a)?);
                current = Some([start, end]);
            }
            None => current = Some([start, end]),
        }
    }
    if let Some([a, b]) = current {
        total = combine(total, exact_motion_operand(b)? - exact_motion_operand(a)?);
    }
    Ok(total)
}

/// Classify clearance for every unordered tool pair with one budget.
/// A pair's timeline runs from its first start to its last finish.
/// Outside that window the pair retains its corresponding endpoint poses.
pub fn partition_scheduled_tools_clearance(
    tools: &[ScheduledTool<'_>],
    progress_tolerance: f64,
    max_cells: usize,
) -> Result<ScheduledToolsClearancePartition> {
    validate(progress_tolerance, max_cells)?;
    let (phases, pairs) = prepare_scheduled_tool_pairs(tools, max_cells)?;
    let mut remaining = max_cells;
    let mut cells = Vec::new();
    for (index, (phase, tools)) in phases.iter().zip(pairs).enumerate() {
        let report = partition_scheduled_phase(
            phase, progress_tolerance, remaining / (phases.len() - index),
        )?;
        remaining -= report.work_cells;
        cells.extend(report.cells.into_iter().map(|clearance| ScheduledToolPairCell {
            tools,
            clearance,
        }));
    }
    Ok(ScheduledToolsClearancePartition { cells, work_cells: max_cells - remaining })
}

fn prepare_scheduled_tool_pairs<'a>(
    tools: &[ScheduledTool<'a>],
    max_cells: usize,
) -> Result<(Vec<ScheduledOverlap<'a>>, Vec<[usize; 2]>)> {
    check(tools.len() >= 2, "Scheduled clearance requires at least two tools")?;
    if tools.len().saturating_mul(tools.len() - 1) / 2 > max_cells {
        return Err(resource("Clearance budget must cover every tool pair"));
    }
    let mut phases = Vec::new();
    let mut pairs = Vec::new();
    for (second, b) in tools.iter().enumerate() {
        for (first, a) in tools[..second].iter().enumerate() {
            let pair_phases = prepare_schedule_with_holds(
                [a.segment, b.segment], [a.start_time, b.start_time], [a.radius, b.radius],
            )?;
            if pair_phases.len() > max_cells - phases.len() {
                return Err(resource("Clearance budget must cover every tool pair phase"));
            }
            pairs.extend(pair_phases.iter().map(|_| [first, second]));
            phases.extend(pair_phases);
        }
    }
    Ok((phases, pairs))
}

fn measure_scheduled_phase_minimum(
    phases: &[ScheduledOverlap<'_>],
    tolerance: f64,
    max_cells: usize,
) -> Result<(ScheduledMinimumClearance, usize)> {
    let initial = (0..phases.len()).map(|index| (index, [0., 1.]));
    let measure = |index: usize, domain: [f64; 2], witness: &mut ClearanceWitness| {
        let phase = &phases[index];
        let lower = phase.gap(domain)?.lo;
        for progress in domain {
            let upper = phase.gap([progress; 2])?.hi;
            if upper < witness.0 {
                *witness = (upper, progress, index);
            }
        }
        Ok(lower)
    };
    let (lower, witness, work_cells, stop_reason) =
        refine_minimum_clearance(initial, 0., tolerance, max_cells, measure)?;
    let time = phases[witness.2].time(witness.1)?;
    Ok((ScheduledMinimumClearance {
        gap_bounds: [lower, witness.0],
        witness_time_bounds: [round_motion_duration(&time, false)?, round_motion_duration(&time, true)?],
        work_cells,
        stop_reason,
    }, witness.2))
}

fn prepare_schedule_with_holds<'a>(
    segments: [TimedMotionSegment<'a>; 2],
    start_times: [f64; 2],
    radii: [f64; 2],
) -> Result<Vec<ScheduledOverlap<'a>>> {
    let (starts, durations, radius) = validate_motion_schedule(segments, start_times, radii)?;
    let mut events = [starts[0].clone(), starts[1].clone(),
        &starts[0] + &durations[0], &starts[1] + &durations[1]].to_vec();
    events.sort();
    events.dedup();
    events.windows(2).map(|times| {
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
        Ok(ScheduledOverlap {
            segments,
            origin: origin.clone(),
            duration: end - origin,
            maps: [map(0)?, map(1)?],
            radius,
        })
    }).collect()
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
        |point, index, _| obstacles[index].gap(point),
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
        |point, index, _| {
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
        |point, index, _| obstacles[index].gap(point, tool_radius),
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
        |point, index, _| obstacles[index].gap(point, tool_radius),
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

/// Classify clearance against spherical obstacles with a tool radius that
/// varies linearly over the source parameter window. Radii are in model units.
pub fn partition_variable_radius_spherical_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[SphericalObstacle<'_>],
    tool_radii: [f64; 2],
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(tool_radii.iter().all(|r| r.is_finite() && *r >= 0.),
        "Tool radii must be finite and nonnegative")?;
    check(!obstacles.is_empty(), "Spherical clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every sphere"));
    }
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let obstacles = prepare_spherical_obstacles(curve, obstacles, 0.)?;
    partition_variable_radial_clearance(
        curve, interval, tool_radii, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| {
            let (center, radius) = &obstacles[index];
            let (lo, hi) = box_distance(point, center)?;
            Interval::new(lo, hi)?.sub(*radius)
        },
    )
}

/// Check a linearly changing tool radius against a mixed obstacle scene.
/// Every gap array follows the original obstacle order.
pub fn partition_variable_radius_toolpath_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radii: [f64; 2],
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(tool_radii.iter().all(|r| r.is_finite() && *r >= 0.),
        "Tool radii must be finite and nonnegative")?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if max_cells == 0 {
        return Err(resource("Clearance budget must cover every obstacle"));
    }
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, 0.)?;
    partition_variable_radial_clearance(
        curve, interval, tool_radii, parameter_tolerance, max_cells, obstacles.len(),
        |point, index| obstacles[index].gap(point, 0.),
    )
}

fn partition_variable_radial_clearance(
    curve: &Curve,
    interval: [f64; 2],
    tool_radii: [f64; 2],
    parameter_tolerance: f64,
    max_cells: usize,
    obstacle_count: usize,
    base_gap: impl Fn(&[Interval], usize) -> Result<Interval>,
) -> Result<SphericalClearancePartition> {
    let radius_range = linear_tool_radius(interval, tool_radii)?;
    let initial = active_spans(curve, interval).map(|span| ([span, 0], [
        curve.knots[span].max(interval[0]),
        curve.knots[span + 1].min(interval[1]),
    ]));
    partition_profile_clearance_cells(
        curve, initial, parameter_tolerance, max_cells, obstacle_count,
        |_, domain| radius_range(domain), base_gap,
    )
}

/// Check a continuous piecewise-linear radius profile against a mixed scene.
/// Each entry is `(source parameter, radius)`. The first and last entries
/// define the traversal window; parameters must be strictly increasing.
pub fn partition_profiled_radius_toolpath_clearance(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[ToolpathObstacle<'_>],
    parameter_tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalClearancePartition> {
    validate(parameter_tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    let (initial, profiles) = prepare_radius_profile(curve, radius_profile, max_cells)?;
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, 0.)?;
    partition_profile_clearance_cells(
        curve, initial, parameter_tolerance, max_cells, obstacles.len(),
        |index, domain| profiles[index](domain),
        |point, index| obstacles[index].gap(point, 0.),
    )
}

/// Refine the minimum signed gap in a mixed scene with a continuous
/// piecewise-linear tool radius. Radius breaks are sampled exactly.
pub fn measure_minimum_profiled_radius_toolpath_clearance(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    obstacles: &[ToolpathObstacle<'_>],
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    validate(tolerance, max_obstacle_cells)?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    let max_cells = max_obstacle_cells / obstacles.len();
    let (initial, profiles) = prepare_radius_profile(curve, radius_profile, max_cells)?;
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, 0.)?;
    measure_minimum_radial_cells(
        curve, initial, radius_profile[0].0, tolerance, max_cells, obstacles.len(),
        |point, index, profile, domain| {
            obstacles[index].gap(point, 0.)?.sub(profiles[profile](domain)?)
        },
    )
}

fn prepare_radius_profile(
    curve: &Curve,
    radius_profile: &[(f64, f64)],
    max_cells: usize,
) -> Result<(Vec<([usize; 2], [f64; 2])>, Vec<impl Fn([f64; 2]) -> Result<Interval>>)> {
    check(radius_profile.len() >= 2, "Radius profile requires at least two entries")?;
    if radius_profile.len() - 1 > max_cells {
        return Err(resource("Clearance budget must cover every radius profile span"));
    }
    curve.validate()?;
    check(radius_profile.iter().all(|(u, radius)| {
        u.is_finite() && radius.is_finite() && *radius >= 0.
    }), "Radius profile entries must have finite parameters and nonnegative finite radii")?;
    let mut initial = Vec::new();
    let mut profiles = Vec::with_capacity(radius_profile.len() - 1);
    for (index, pair) in radius_profile.windows(2).enumerate() {
        let [(a, r0), (b, r1)] = [pair[0], pair[1]];
        let interval = [a, b];
        validate_travel_interval(curve, interval)?;
        for span in active_spans(curve, interval) {
            if initial.len() == max_cells {
                return Err(resource("Clearance budget must cover the combined knot and radius spans"));
            }
            initial.push(([span, index], [a.max(curve.knots[span]), b.min(curve.knots[span + 1])]));
        }
        profiles.push(linear_tool_radius(interval, [r0, r1])?);
    }
    Ok((initial, profiles))
}

fn partition_profile_clearance_cells(
    curve: &Curve,
    initial: impl IntoIterator<Item = ([usize; 2], [f64; 2])>,
    parameter_tolerance: f64,
    max_cells: usize,
    obstacle_count: usize,
    radius_range: impl Fn(usize, [f64; 2]) -> Result<Interval>,
    base_gap: impl Fn(&[Interval], usize) -> Result<Interval>,
) -> Result<SphericalClearancePartition> {
    let (cells, work_cells) = partition_enclosed_cells(
        initial, parameter_tolerance, max_cells,
        |[span, profile], domain| {
            let tool = radius_range(profile, domain)?;
            let jets = curve_jets::enclose(curve, span, domain)?;
            let gaps = (0..obstacle_count).map(|index| {
                let gap = base_gap(&jets[0], index)?.sub(tool)?;
                Ok([gap.lo, gap.hi])
            }).collect::<Result<Vec<_>>>()?;
            let (status, refine) = classify_radial_gaps(&gaps);
            Ok((SphericalClearanceCell {
                parameter_interval: domain,
                span,
                gap_bounds: gaps,
                status,
            }, refine))
        },
    )?;
    Ok(SphericalClearancePartition {
        cells: cells.into_iter().map(|(_, _, cell)| cell).collect(),
        work_obstacle_cells: work_cells * obstacle_count,
    })
}

/// Refine the minimum signed gap while the tool radius varies linearly.
/// The witness evaluates position and radius at the same source parameter.
pub fn measure_minimum_variable_radius_spherical_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[SphericalObstacle<'_>],
    tool_radii: [f64; 2],
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    validate(tolerance, max_obstacle_cells)?;
    check(tool_radii.iter().all(|r| r.is_finite() && *r >= 0.),
        "Tool radii must be finite and nonnegative")?;
    check(!obstacles.is_empty(), "Spherical clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_spherical_obstacles(curve, obstacles, 0.)?;
    let radius_range = linear_tool_radius(interval, tool_radii)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index, domain| {
            let (center, radius) = &obstacles[index];
            let (lo, hi) = box_distance(point, center)?;
            Interval::new(lo, hi)?.sub(*radius)?.sub(radius_range(domain)?)
        },
    )
}

/// Refine the minimum signed gap against a mixed obstacle scene while the
/// spherical tool's radius varies linearly over the parameter window.
pub fn measure_minimum_variable_radius_toolpath_clearance(
    curve: &Curve,
    interval: [f64; 2],
    obstacles: &[ToolpathObstacle<'_>],
    tool_radii: [f64; 2],
    tolerance: f64,
    max_obstacle_cells: usize,
) -> Result<SphericalMinimumClearance> {
    validate(tolerance, max_obstacle_cells)?;
    check(tool_radii.iter().all(|r| r.is_finite() && *r >= 0.),
        "Tool radii must be finite and nonnegative")?;
    check(!obstacles.is_empty(), "Toolpath clearance requires at least one obstacle")?;
    curve.validate()?;
    validate_travel_interval(curve, interval)?;
    let max_cells = max_obstacle_cells / obstacles.len();
    if active_spans(curve, interval).count() > max_cells {
        return Err(resource("Clearance budget must cover every initial obstacle cell"));
    }
    let obstacles = prepare_toolpath_obstacles(curve, obstacles, 0.)?;
    let radius_range = linear_tool_radius(interval, tool_radii)?;
    measure_minimum_radial_clearance(
        curve, interval, tolerance, max_cells, obstacles.len(),
        |point, index, domain| obstacles[index].gap(point, 0.)?.sub(radius_range(domain)?),
    )
}

fn linear_tool_radius(
    interval: [f64; 2],
    tool_radii: [f64; 2],
) -> Result<impl Fn([f64; 2]) -> Result<Interval>> {
    let start = exact_motion_operand(interval[0])?;
    let width = exact_motion_operand(interval[1])? - &start;
    let radius_start = exact_motion_operand(tool_radii[0])?;
    let slope = (exact_motion_operand(tool_radii[1])? - &radius_start) / width;
    Ok(move |domain: [f64; 2]| {
            let radius_at = |parameter| -> Result<_> {
                Ok(&radius_start + &slope * (exact_motion_operand(parameter)? - &start))
            };
            let a = radius_at(domain[0])?;
            let b = radius_at(domain[1])?;
            Interval::new(
                round_motion_duration(std::cmp::min(&a, &b), false)?,
                round_motion_duration(std::cmp::max(&a, &b), true)?,
            )
    })
}

fn classify_radial_gaps(gaps: &[[f64; 2]]) -> (DirectionalMotionStatus, bool) {
    let status = if gaps.iter().any(|[_, hi]| *hi <= 0.) {
        DirectionalMotionStatus::Exceeds
    } else if gaps.iter().all(|[lo, _]| *lo > 0.) {
        DirectionalMotionStatus::Within
    } else {
        DirectionalMotionStatus::Undetermined
    };
    let refine = gaps.iter().any(|[lo, hi]| *lo <= 0. && *hi > 0.);
    (status, refine)
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
            let (status, refine) = classify_radial_gaps(&gaps);
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
