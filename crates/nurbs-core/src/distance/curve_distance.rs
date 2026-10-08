//! Global distance bounds for positive-weight NURBS curves. Each nonempty knot
//! span is enclosed by interval de Boor evaluation; no display mesh is used.
use crate::distance::subdivision::{HeapQueue, Queue, Search, Split};
use crate::distance_bounds::{Interval, box_distance};
use crate::{Result, check, curve::Curve, resource};
#[cfg(feature = "codec")]
mod serialization;

/// Homogeneous de Boor, with outward rounding of every arithmetic operation.
/// A span parameter stays inside all interpolation knot ranges, so alpha ∈ [0,1].
pub(crate) fn restricted_controls(
    curve: &Curve,
    span: usize,
    t: Interval,
) -> Result<Vec<Vec<Interval>>> {
    let degree = curve.degree;
    let dimension = curve.control_points[0].len();
    let mut d = Vec::with_capacity(degree + 1);
    let mut min_weight = f64::INFINITY;
    let mut max_weight: f64 = 0.;
    let origin = &curve.control_points[span - degree];
    let scale = (span - degree..=span)
        .map(|i| curve.weights[i])
        .fold(0., f64::max);
    for i in span - degree..=span {
        let ratio = curve.weights[i] / scale;
        let w = if curve.weights[i] == scale {
            Interval::point(1.)
        } else {
            Interval::new(ratio.next_down().max(0.), ratio.next_up())?
        };
        min_weight = min_weight.min(w.lo);
        max_weight = max_weight.max(w.hi);
        let mut p = curve.control_points[i]
            .iter()
            .enumerate()
            .map(|(axis, &x)| {
                Interval::point(x)
                    .sub(Interval::point(origin[axis]))?
                    .mul(w)
            })
            .collect::<Result<Vec<_>>>()?;
        p.push(w);
        d.push(p);
    }
    // Exact restricted Bernstein controls are blossom values at
    // lo^(degree-k), hi^k. Keep the construction interval-valued and then use
    // the positive rational convex hull, avoiding interval-parameter dependency.
    let count = if t.lo == t.hi { 1 } else { degree + 1 };
    let mut controls = Vec::with_capacity(count);
    for k in 0..count {
        let mut q = d.clone();
        for r in 1..=degree {
            let parameter = if r - 1 < degree - k { t.lo } else { t.hi };
            for j in (r..=degree).rev() {
                let i = span - degree + j;
                let denominator = Interval::point(curve.knots[i + degree - r + 1])
                    .sub(Interval::point(curve.knots[i]))?;
                let alpha = Interval::point(parameter)
                    .sub(Interval::point(curve.knots[i]))?
                    .div(denominator)?
                    .intersect(0., 1.)?;
                let beta = Interval::point(1.).sub(alpha)?.intersect(0., 1.)?;
                for axis in 0..=dimension {
                    let a = q[j - 1][axis];
                    let b = q[j][axis];
                    q[j][axis] = a
                        .mul(beta)?
                        .add(b.mul(alpha)?)?
                        .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
                }
                q[j][dimension] = q[j][dimension].intersect(min_weight, max_weight)?;
            }
        }
        controls.push(q[degree].clone());
    }
    Ok(controls)
}

pub(crate) fn enclosure(curve: &Curve, span: usize, t: Interval) -> Result<Vec<Interval>> {
    let dimension = curve.control_points[0].len();
    let origin = &curve.control_points[span - curve.degree];
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; dimension];
    for control in restricted_controls(curve, span, t)? {
        for axis in 0..dimension {
            let lo = (span - curve.degree..=span)
                .map(|i| curve.control_points[i][axis])
                .fold(f64::INFINITY, f64::min);
            let hi = (span - curve.degree..=span)
                .map(|i| curve.control_points[i][axis])
                .fold(f64::NEG_INFINITY, f64::max);
            let p = control[axis]
                .div(control[dimension])?
                .add(Interval::point(origin[axis]))?
                .intersect(lo, hi)?;
            bounds[axis][0] = bounds[axis][0].min(p.lo);
            bounds[axis][1] = bounds[axis][1].max(p.hi);
        }
    }
    bounds
        .into_iter()
        .map(|[lo, hi]| Interval::new(lo, hi))
        .collect()
}

fn spans(curve: &Curve) -> Vec<usize> {
    (curve.degree..curve.control_points.len())
        .filter(|&i| curve.knots[i] < curve.knots[i + 1])
        .collect()
}
fn midpoint(a: f64, b: f64) -> f64 {
    a * 0.5 + b * 0.5
}
#[derive(Clone)]
struct Part {
    span: usize,
    domain: [f64; 2],
    bounds: Vec<Interval>,
}
impl Part {
    fn new(curve: &Curve, span: usize, domain: [f64; 2]) -> Result<Self> {
        Ok(Self {
            span,
            domain,
            bounds: enclosure(curve, span, Interval::new(domain[0], domain[1])?)?,
        })
    }
    fn size(&self) -> f64 {
        self.bounds.iter().map(|i| i.hi - i.lo).fold(0., f64::max)
    }
}
struct Cell {
    a: Part,
    b: Part,
}

#[derive(Clone, Debug)]
pub struct CurveDistance {
    pub distance_interval_mm: [f64; 2],
    pub parameters: [f64; 2],
    pub points: [Vec<f64>; 2],
    pub point_enclosures: [Vec<[f64; 2]>; 2],
    pub converged: bool,
    pub reason: crate::DistanceStopReason,
    pub cells: usize,
    pub max_cells: usize,
    pub tolerance_mm: f64,
}

struct Witness {
    parameters: [f64; 2],
    points: [Vec<f64>; 2],
    bounds: [Vec<Interval>; 2],
    upper: f64,
}
fn witness(curve: &Curve, spans: &[usize], mut t: f64) -> Result<(f64, Vec<f64>, Vec<Interval>)> {
    let domain = curve.domain();
    if curve.periodic && t == domain[1] {
        t = domain[0];
    }
    let span = *spans.iter().rev().find(|&&i| curve.knots[i] <= t).unwrap();
    Ok((
        t,
        curve.evaluate(t)?.point,
        enclosure(curve, span, Interval::point(t))?,
    ))
}
fn consider(
    a: &Curve,
    b: &Curve,
    sa: &[usize],
    sb: &[usize],
    u: f64,
    v: f64,
    best: &mut Option<Witness>,
) -> Result<()> {
    let (u, pa, ba) = witness(a, sa, u)?;
    let (v, pb, bb) = witness(b, sb, v)?;
    let upper = box_distance(&ba, &bb)?.1;
    if best.as_ref().is_none_or(|x| upper < x.upper) {
        *best = Some(Witness {
            parameters: [u, v],
            points: [pa, pb],
            bounds: [ba, bb],
            upper,
        });
    }
    Ok(())
}

struct CurveSearch<'a> {
    a: &'a Curve,
    b: &'a Curve,
    sa: &'a [usize],
    sb: &'a [usize],
    best: Option<Witness>,
}
impl Search for CurveSearch<'_> {
    type Cell = Cell;
    fn upper(&self) -> Option<f64> {
        self.best.as_ref().map(|w| w.upper)
    }
    fn split(&mut self, cell: Cell) -> Result<Split<Cell>> {
        let ma = midpoint(cell.a.domain[0], cell.a.domain[1]);
        let mb = midpoint(cell.b.domain[0], cell.b.domain[1]);
        let can_a = cell.a.domain[0] < ma && ma < cell.a.domain[1];
        let can_b = cell.b.domain[0] < mb && mb < cell.b.domain[1];
        if !can_a && !can_b {
            return Ok(Split::Precision(cell));
        }
        let split_a = can_a && (!can_b || cell.a.size() >= cell.b.size());
        let selected = if split_a { &cell.a } else { &cell.b };
        let middle = if split_a { ma } else { mb };
        let mut children = [None, None];
        for (i, domain) in [
            [selected.domain[0], middle],
            [middle, selected.domain[1]],
        ]
        .into_iter()
        .enumerate()
        {
            let (pa, pb) = if split_a {
                (Part::new(self.a, cell.a.span, domain)?, cell.b.clone())
            } else {
                (cell.a.clone(), Part::new(self.b, cell.b.span, domain)?)
            };
            consider(
                self.a,
                self.b,
                self.sa,
                self.sb,
                midpoint(pa.domain[0], pa.domain[1]),
                midpoint(pb.domain[0], pb.domain[1]),
                &mut self.best,
            )?;
            let lower = box_distance(&pa.bounds, &pb.bounds)?.0;
            if self.best.as_ref().is_none_or(|w| lower <= w.upper) {
                children[i] = Some((Cell { a: pa, b: pb }, lower));
            }
        }
        Ok(Split::Children(children))
    }
}

/// Distance between the curve images, within tolerance when converged. A work or
/// precision stop retains global bounds and a valid pair of original parameters.
/// This is a curve primitive; it does not claim the distance between B-rep faces.
pub fn distance(
    a: &Curve,
    b: &Curve,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<CurveDistance> {
    a.validate()?;
    b.validate()?;
    check(
        a.control_points[0].len() == b.control_points[0].len(),
        "Curve distance dimensions must match",
    )?;
    check(
        tolerance_mm.is_finite() && tolerance_mm > 0.,
        "Curve distance tolerance must be positive and finite",
    )?;
    check(
        (1..=100_000).contains(&max_cells),
        "Curve distance needs 1..100000 cells",
    )?;
    let sa = spans(a);
    let sb = spans(b);
    if sa.len().saturating_mul(sb.len()) > max_cells {
        return Err(resource(
            "Curve distance initial span pairs exceed the cell budget",
        ));
    }
    let aa = sa
        .iter()
        .map(|&i| Part::new(a, i, [a.knots[i], a.knots[i + 1]]))
        .collect::<Result<Vec<_>>>()?;
    let bb = sb
        .iter()
        .map(|&i| Part::new(b, i, [b.knots[i], b.knots[i + 1]]))
        .collect::<Result<Vec<_>>>()?;
    let mut best = None;
    let mut queue = HeapQueue::default();
    let mut cells = 0;
    for pa in &aa {
        for pb in &bb {
            for u in [
                pa.domain[0],
                midpoint(pa.domain[0], pa.domain[1]),
                pa.domain[1],
            ] {
                for v in [
                    pb.domain[0],
                    midpoint(pb.domain[0], pb.domain[1]),
                    pb.domain[1],
                ] {
                    consider(a, b, &sa, &sb, u, v, &mut best)?;
                }
            }
            queue.push(
                Cell {
                    a: pa.clone(),
                    b: pb.clone(),
                },
                box_distance(&pa.bounds, &pb.bounds)?.0,
            );
            cells += 1;
        }
    }
    let mut search = CurveSearch {
        a,
        b,
        sa: &sa,
        sb: &sb,
        best,
    };
    let outcome =
        crate::distance::subdivision::run(&mut search, &mut queue, cells, max_cells, tolerance_mm)?;
    let best = search.best.unwrap();
    // Result control points must be finite before leaving the module (1093).
    crate::foundation::guards::require_finite_vec(&best.points[0], "curve distance point a")?;
    crate::foundation::guards::require_finite_vec(&best.points[1], "curve distance point b")?;
    Ok(CurveDistance {
        distance_interval_mm: [outcome.lower, best.upper],
        parameters: best.parameters,
        points: best.points,
        point_enclosures: best
            .bounds
            .map(|bounds| bounds.into_iter().map(|i| [i.lo, i.hi]).collect()),
        converged: outcome.reason == crate::DistanceStopReason::Tolerance,
        reason: outcome.reason,
        cells: outcome.cells,
        max_cells,
        tolerance_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a, b],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    fn verify(a: &Curve, b: &Curve, expected: f64, tolerance: f64, budget: usize) -> CurveDistance {
        let result = distance(a, b, tolerance, budget).unwrap();
        assert!(
            result.distance_interval_mm[0] <= expected + 1e-12,
            "{result:?}"
        );
        assert!(
            result.distance_interval_mm[1] >= expected - 1e-12,
            "{result:?}"
        );
        for (i, c) in [a, b].iter().enumerate() {
            assert_eq!(
                c.evaluate(result.parameters[i]).unwrap().point,
                result.points[i]
            );
        }
        if result.converged {
            assert!(result.distance_interval_mm[1] - result.distance_interval_mm[0] <= tolerance);
        }
        result
    }
    #[test]
    fn parallel_lines_and_skew_interior_minimum() {
        let a = line(vec![0., 0., 0.], vec![10., 0., 0.]);
        let b = line(vec![0., 3., 0.], vec![10., 3., 0.]);
        assert!(verify(&a, &b, 3., 1e-8, 1000).converged);
        let a = line(vec![-3.7, 0., 0.], vec![6.3, 0., 0.]);
        let b = line(vec![0., -6.2, 2.], vec![0., 3.8, 2.]);
        let result = verify(&a, &b, 2., 1e-6, 4096);
        assert!(result.converged, "{result:?}");
    }
    #[test]
    fn rational_circle_distance_has_original_curve_witnesses() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![10., 0., 0.], vec![10., 10., 0.], vec![0., 10., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let b = line(vec![15., 15., 0.], vec![15., 15., 0.]);
        let result = verify(&a, &b, 450_f64.sqrt() - 10., 1e-3, 10_000);
        assert!(result.converged, "{result:?}");
        assert!((result.points[0][0].hypot(result.points[0][1]) - 10.).abs() < 1e-12);
    }
    #[test]
    fn work_limit_is_explicit_and_retains_global_bounds() {
        let a = line(vec![0., 0.], vec![10., 10.]);
        let d = std::f64::consts::FRAC_1_SQRT_2;
        let b = line(vec![-d, d], vec![10. - d, 10. + d]);
        let result = verify(&a, &b, 1., 1e-10, 1);
        assert!(!result.converged);
        assert_eq!(result.reason, crate::DistanceStopReason::WorkLimit);
        assert_eq!(result.cells, 1);
    }
    #[test]
    fn covers_multiple_spans_weights_and_periodic_seam() {
        let a = Curve {
            degree: 1,
            knots: vec![0., 0., 0.4, 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]],
            weights: vec![1., 3., 2.],
            periodic: false,
        };
        let b = line(vec![1., 2.], vec![1., 2.]);
        assert!(verify(&a, &b, 1., 1e-8, 1000).converged);
        let square = Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 0.],
                vec![1., 1.],
                vec![0., 1.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        };
        let point = line(vec![-1., 0.], vec![-1., 0.]);
        assert!(verify(&square, &point, 1., 1e-8, 1000).converged);
    }
    #[test]
    fn rejects_invalid_contract_and_initial_budget() {
        let a = line(vec![0., 0.], vec![1., 0.]);
        let b = line(vec![0., 0., 0.], vec![1., 0., 0.]);
        assert!(distance(&a, &b, 1e-3, 100).is_err());
        assert!(distance(&a, &a, 0., 100).is_err());
        assert!(distance(&a, &a, 1e-3, 0).is_err());
        let multi = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(distance(&multi, &multi, 1e-3, 1).is_err());
    }

    #[test]
    fn nearest_point_rejects_non_finite_query() {
        let a = line(vec![0., 0.], vec![1., 0.]);
        for p in [[f64::NAN, 0.], [0., f64::INFINITY]] {
            let err = nearest_point(&a, &p, 1e-6, 100).unwrap_err();
            assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
            assert!(err.contains("point"), "{err}");
        }
        // Finite query resolves and returns finite result points.
        let r = nearest_point(&a, &[0.5, 3.], 1e-8, 1000).unwrap();
        assert!(r.converged);
        assert!(r.point.iter().all(|v| v.is_finite()));
    }
}

#[cfg(test)]
mod conditioning_tests {
    use super::*;
    #[test]
    fn distance_survives_common_weight_scaling_and_translation() {
        for scale in [1e-11, 1., 1e11] {
            let a = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![1_000_010., 1_000_000.],
                    vec![1_000_010., 1_000_010.],
                    vec![1_000_000., 1_000_010.],
                ],
                weights: vec![scale, scale * std::f64::consts::FRAC_1_SQRT_2, scale],
                periodic: false,
            };
            let b = Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![vec![1_000_015., 1_000_015.]; 2],
                weights: vec![scale; 2],
                periodic: false,
            };
            let result = distance(&a, &b, 1e-3, 10_000).unwrap();
            let exact = 450_f64.sqrt() - 10.;
            assert!(result.converged, "{result:?}");
            assert!(result.distance_interval_mm[0] <= exact);
            assert!(result.distance_interval_mm[1] >= exact);
        }
    }
}

#[cfg(test)]
mod two_curve_tests {
    use super::*;
    #[test]
    fn separates_two_rational_arcs_at_a_non_sampled_minimum() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![10., 0., 0.], vec![10., 10., 0.], vec![0., 10., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let mut b = a.clone();
        for p in &mut b.control_points {
            p[0] += 30.;
        }
        let result = distance(&a, &b, 1e-3, 20_000).unwrap();
        let exact = 1000_f64.sqrt() - 10.;
        assert!(result.converged, "{result:?}");
        assert!(result.distance_interval_mm[0] <= exact);
        assert!(result.distance_interval_mm[1] >= exact);
        for (i, curve) in [&a, &b].iter().enumerate() {
            assert_eq!(
                curve.evaluate(result.parameters[i]).unwrap().point,
                result.points[i]
            );
        }
        assert!((result.points[0][0].hypot(result.points[0][1]) - 10.).abs() < 1e-10);
        assert!(((result.points[1][0] - 30.).hypot(result.points[1][1]) - 10.).abs() < 1e-10);
    }
}

/// A witness for the globally nearest point on a curve image.
/// The distance bounds remain valid when the search does not converge; the
/// parameter is a witness, not a certified unique minimizer.
#[derive(Clone, Debug)]
pub struct NearestPoint {
    pub parameter: f64,
    pub point: Vec<f64>,
    pub point_enclosure: Vec<[f64; 2]>,
    pub distance_interval: [f64; 2],
    pub converged: bool,
    pub reason: crate::DistanceStopReason,
    pub cells: usize,
}

/// Search the complete active curve domain, including its endpoints.
/// Dimensions and numeric limits follow `Curve::validate`; tolerance must be
/// positive and finite and the cell budget must be in 1..=100000.
pub fn nearest_point(
    curve: &Curve,
    point: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<NearestPoint> {
    curve.validate()?;
    check(
        point.len() == curve.control_points[0].len(),
        "Point dimensions must match the curve",
    )?;
    crate::foundation::guards::require_finite_vec(point, "point")?;
    let constant = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![point.to_vec(), point.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    let report = distance(curve, &constant, tolerance, max_cells)?;
    Ok(NearestPoint {
        parameter: report.parameters[0],
        point: report.points[0].clone(),
        point_enclosure: report.point_enclosures[0].clone(),
        distance_interval: report.distance_interval_mm,
        converged: report.converged,
        reason: report.reason,
        cells: report.cells,
    })
}
