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

/// Certify peak axial speed over an original-parameter window. The tolerance
/// limits the width of the reported enclosure.
pub fn directional_peak_speed(
    curve: &Curve,
    interval: [f64; 2],
    direction: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<DirectionalPeakSpeedReport> {
    curve.validate()?;
    validate(tolerance, max_cells)?;
    validate_travel_interval(curve, interval)?;
    let unit = travel_direction(curve, direction)?;
    let mut pending = BTreeMap::new();
    let mut cells = 0;
    let mut lower = 0.0_f64;
    let measure = |span, domain: [f64; 2]| -> Result<(f64, f64)> {
        let width = Interval::point(domain[1]).sub(Interval::point(domain[0]))?;
        let jets = curve_jets::enclose(curve, span, domain)?;
        let velocity = project_jet(&jets[1], &unit)?.div(width)?;
        let upper = velocity.lo.abs().max(velocity.hi.abs());
        let sample_lower = |end| -> Result<f64> {
            let jets = curve_jets::endpoint(curve, span, domain, end)?;
            let velocity = project_jet(&jets[1], &unit)?.div(width)?;
            Ok(velocity.lo.max(-velocity.hi).max(0.))
        };
        Ok((sample_lower(false)?.max(sample_lower(true)?), upper))
    };
    for span in active_spans(curve, interval) {
        if cells == max_cells {
            return Err(resource("Initial speed spans exceed the work budget"));
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
            .ok_or_else(|| crate::numeric_err("Missing speed span coverage"))?;
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
    Ok(DirectionalPeakSpeedReport { bounds: [lower, upper], cells, stop_reason })
}
