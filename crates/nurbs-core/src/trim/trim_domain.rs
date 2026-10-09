//! Nonzero winding of general rational UV loops, with conservative rectangle
//! classification. A curve piece may be replaced by its endpoint chord only
//! when an outward convex enclosure excludes the query rectangle. This is a
//! homotopy proof of winding, not a tessellation tolerance test.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance::enclosure,
    distance_bounds::{Interval, box_distance},
    resource,
};
#[cfg(feature = "codec")]
mod serialization;

/// Exact endpoint closure of an authored UV loop. This does not prove that
/// the loop is simple, oriented correctly, or disjoint from other loops.
/// Clamped endpoints equal their authored controls even with rational weights.
/// None means an unclamped/periodic endpoint has no exact proof here.
pub fn exact_loop_joins(curves: &[Curve]) -> Result<Option<bool>> {
    check((1..=256).contains(&curves.len()), "Exact UV closure needs 1..256 curves")?;
    for c in curves {
        c.validate()?;
        check(
            c.control_points[0].len() == 2,
            "Exact UV closure needs 2D curves",
        )?;
    }
    let endpoint = |c: &Curve, end: bool| -> Option<[f64; 2]> {
        if c.periodic {
            return None;
        }
        let n = c.control_points.len();
        if end {
            c.knots[n..]
                .iter()
                .all(|k| *k == c.knots[n])
                .then(|| [c.control_points[n - 1][0], c.control_points[n - 1][1]])
        } else {
            c.knots[..=c.degree]
                .iter()
                .all(|k| *k == c.knots[c.degree])
                .then(|| [c.control_points[0][0], c.control_points[0][1]])
        }
    };
    let mut complete = true;
    for i in 0..curves.len() {
        match (
            endpoint(&curves[i], true),
            endpoint(&curves[(i + 1) % curves.len()], false),
        ) {
            (Some(a), Some(b)) if a != b => return Ok(Some(false)),
            (Some(_), Some(_)) => {}
            _ => complete = false,
        }
    }
    Ok(complete.then_some(true))
}

type Point = [f64; 2];
type Rectangle = [[f64; 2]; 2];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Inside,
    Outside,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationReason {
    Separated,
    WorkLimit,
    JoinBand,
    BoundaryBand,
    PrecisionLimit,
}
#[derive(Clone, Debug)]
pub struct Classification {
    pub location: Location,
    pub winding: Option<i32>,
    pub reason: ClassificationReason,
    pub cells: usize,
    pub max_cells: usize,
    pub uncertain: Option<(usize, usize, [f64; 2])>,
    pub tolerance_uv: f64,
}

#[derive(Clone)]
struct Endpoint {
    point: Point,
    bounds: [Interval; 2],
}
#[derive(Clone)]
struct Piece {
    span: usize,
    domain: [f64; 2],
    bounds: [Interval; 2],
    ends: [Endpoint; 2],
}
fn mid(a: f64, b: f64) -> f64 {
    a * 0.5 + b * 0.5
}
fn hull(a: [Interval; 2], b: [Interval; 2]) -> [Interval; 2] {
    std::array::from_fn(|k| Interval {
        lo: a[k].lo.min(b[k].lo),
        hi: a[k].hi.max(b[k].hi),
    })
}
fn endpoint(curve: &Curve, span: usize, t: f64) -> Result<Endpoint> {
    let domain = curve.domain();
    // A clamped endpoint is exactly its authored Cartesian control, regardless
    // of weight. This avoids unnecessary interval width at large UV offsets.
    let exact = if t == domain[0] && curve.knots[..=curve.degree].iter().all(|&k| k == t) {
        Some(&curve.control_points[0])
    } else if t == domain[1]
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|&k| k == t)
    {
        curve.control_points.last()
    } else {
        None
    };
    let point = if let Some(p) = exact {
        [p[0], p[1]]
    } else {
        let p = curve.evaluate(t)?.point;
        [p[0], p[1]]
    };
    let mut bounds = if exact.is_some() {
        point.map(Interval::point)
    } else {
        let p = enclosure(curve, span, Interval::point(t))?;
        [p[0], p[1]]
    };
    // The homotopy uses these reproducible binary64 endpoints. Include their
    // evaluator rounding in addition to the enclosure of the original curve.
    for k in 0..2 {
        bounds[k].lo = bounds[k].lo.min(point[k]);
        bounds[k].hi = bounds[k].hi.max(point[k]);
    }
    Ok(Endpoint { point, bounds })
}
impl Piece {
    fn new(
        curve: &Curve,
        span: usize,
        domain: [f64; 2],
        ends: Option<[Endpoint; 2]>,
    ) -> Result<Self> {
        let ends = match ends {
            Some(v) => v,
            None => [
                endpoint(curve, span, domain[0])?,
                endpoint(curve, span, domain[1])?,
            ],
        };
        let b = enclosure(curve, span, Interval::new(domain[0], domain[1])?)?;
        let bounds = hull([b[0], b[1]], hull(ends[0].bounds, ends[1].bounds));
        Ok(Self {
            span,
            domain,
            bounds,
            ends,
        })
    }
}
struct PreparedCurve {
    curve: Curve,
    pieces: Vec<Piece>,
    loop_index: usize,
    curve_index: usize,
}
struct Join {
    ends: [Endpoint; 2],
    parameter: f64,
    loop_index: usize,
    curve_index: usize,
}
/// Loops may have either orientation. Holes use winding opposite to the outer
/// boundary. This classifies a supplied path; it does not certify a simple or
/// manifold B-rep face. Small endpoint gaps have explicit bounded join regions.
pub struct TrimDomain {
    curves: Vec<PreparedCurve>,
    joins: Vec<Join>,
    tolerance_uv: f64,
}
impl TrimDomain {
    pub fn new(loops: &[Vec<Curve>], tolerance_uv: f64) -> Result<Self> {
        check(
            tolerance_uv.is_finite() && tolerance_uv > 0.,
            "Trim-domain UV tolerance must be positive and finite",
        )?;
        check(
            !loops.is_empty() && loops.iter().all(|l| !l.is_empty()),
            "Trim-domain loops must be nonempty",
        )?;
        if loops.iter().map(Vec::len).sum::<usize>() > 256 {
            return Err(resource("Trim domain exceeds 256 curves"));
        }
        let mut curves = Vec::new();
        let mut joins = Vec::new();
        let mut count = 0;
        for (li, contour) in loops.iter().enumerate() {
            let start = curves.len();
            for (ci, curve) in contour.iter().enumerate() {
                curve.validate()?;
                check(
                    curve.control_points[0].len() == 2,
                    "Trim-domain curves must have two coordinates",
                )?;
                let mut pieces = Vec::new();
                for span in curve.degree..curve.control_points.len() {
                    if curve.knots[span] < curve.knots[span + 1] {
                        count += 1;
                        if count > 4096 {
                            return Err(resource("Trim domain exceeds 4096 knot spans"));
                        }
                        pieces.push(Piece::new(
                            curve,
                            span,
                            [curve.knots[span], curve.knots[span + 1]],
                            None,
                        )?);
                    }
                }
                curves.push(PreparedCurve {
                    curve: curve.clone(),
                    pieces,
                    loop_index: li,
                    curve_index: ci,
                });
            }
            let end = curves.len();
            for i in start..end {
                let next = if i + 1 == end { start } else { i + 1 };
                let a = curves[i].pieces.last().unwrap().ends[1].clone();
                let b = curves[next].pieces[0].ends[0].clone();
                let upper = box_distance(&a.bounds, &b.bounds)?.1;
                check(
                    upper <= tolerance_uv,
                    &format!(
                        "Trim-domain loop {} curve {} endpoint gap upper bound {upper} exceeds UV tolerance {tolerance_uv}",
                        li + 1,
                        i - start + 1
                    ),
                )?;
                joins.push(Join {
                    ends: [a, b],
                    parameter: curves[i].curve.domain()[1],
                    loop_index: li,
                    curve_index: i - start,
                });
            }
        }
        Ok(Self {
            curves,
            joins,
            tolerance_uv,
        })
    }
    /// Axis-aligned authored trim spans suggest cuts on both sides of their
    /// tolerance band. These are search hints only; classification still proves
    /// admission or exclusion. General rational spans use normal subdivision.
    pub(crate) fn split_hint(&self, rectangle: Rectangle, axis: usize) -> Option<f64> {
        let [lo, hi] = rectangle[axis];
        let middle = mid(lo, hi);
        let mut best: Option<f64> = None;
        for source in &self.curves {
            for piece in &source.pieces {
                let curve = &source.curve;
                let first = piece.span - curve.degree;
                let coordinate = curve.control_points[first][axis];
                if !(first..=piece.span).all(|i| curve.control_points[i][axis] == coordinate) {
                    continue;
                }
                for t in [
                    (coordinate - 2. * self.tolerance_uv).next_down(),
                    (coordinate + 2. * self.tolerance_uv).next_up(),
                ] {
                    if t.is_finite()
                        && t > lo
                        && t < hi
                        && best.is_none_or(|b| (t - middle).abs() < (b - middle).abs())
                    {
                        best = Some(t);
                    }
                }
            }
        }
        best
    }
    /// A determinate result applies to every point of the rectangle. A band,
    /// precision or work stop never admits the rectangle as inside or outside.
    /// Classify a UV point using the same conservative tolerance band and
    /// winding proof as rectangle classification. Boundary bands are unresolved.
    pub fn classify_point(&self, point: [f64; 2], max_cells: usize) -> Result<Classification> {
        self.classify(point.map(|x| [x, x]), max_cells)
    }

    pub fn classify(&self, rectangle: Rectangle, max_cells: usize) -> Result<Classification> {
        check(
            rectangle
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
            "Trim-domain rectangle must be finite and ordered",
        )?;
        check(
            (1..=100_000).contains(&max_cells),
            "Trim-domain classification needs 1..100000 cells",
        )?;
        let point = rectangle.map(|[lo, hi]| mid(lo, hi));
        let mut query = [Interval::point(0.); 2];
        for k in 0..2 {
            query[k] = Interval::new(
                (rectangle[k][0] - self.tolerance_uv).next_down(),
                (rectangle[k][1] + self.tolerance_uv).next_up(),
            )?;
        }
        let mut cells = 0;
        let mut winding = 0;
        let unresolved = |reason, cells, where_at| Classification {
            location: Location::Unresolved,
            winding: None,
            reason,
            cells,
            max_cells,
            uncertain: Some(where_at),
            tolerance_uv: self.tolerance_uv,
        };
        for join in &self.joins {
            let where_at = (join.loop_index, join.curve_index, [join.parameter; 2]);
            if cells == max_cells {
                return Ok(unresolved(ClassificationReason::WorkLimit, cells, where_at));
            }
            cells += 1;
            let bounds = hull(join.ends[0].bounds, join.ends[1].bounds);
            if !separated(bounds, query) {
                return Ok(unresolved(ClassificationReason::JoinBand, cells, where_at));
            }
            winding += chord_winding(join.ends[0].point, join.ends[1].point, bounds, point);
        }
        for source in &self.curves {
            let mut stack: Vec<_> = source.pieces.iter().rev().cloned().collect();
            while let Some(piece) = stack.pop() {
                let where_at = (source.loop_index, source.curve_index, piece.domain);
                if cells == max_cells {
                    return Ok(unresolved(ClassificationReason::WorkLimit, cells, where_at));
                }
                cells += 1;
                if separated(piece.bounds, query) {
                    winding += chord_winding(
                        piece.ends[0].point,
                        piece.ends[1].point,
                        piece.bounds,
                        point,
                    );
                    continue;
                }
                // If a known endpoint enclosure lies entirely in the query band,
                // subdivision cannot establish separation for this rectangle.
                let in_band = piece.ends.iter().any(|end| {
                    (0..2)
                        .all(|k| end.bounds[k].lo >= query[k].lo && end.bounds[k].hi <= query[k].hi)
                });
                if in_band
                    || piece
                        .bounds
                        .iter()
                        .all(|i| i.hi - i.lo <= self.tolerance_uv)
                {
                    return Ok(unresolved(
                        ClassificationReason::BoundaryBand,
                        cells,
                        where_at,
                    ));
                }
                let middle = mid(piece.domain[0], piece.domain[1]);
                if middle <= piece.domain[0] || middle >= piece.domain[1] {
                    return Ok(unresolved(
                        ClassificationReason::PrecisionLimit,
                        cells,
                        where_at,
                    ));
                }
                let center = endpoint(&source.curve, piece.span, middle)?;
                let left = Piece::new(
                    &source.curve,
                    piece.span,
                    [piece.domain[0], middle],
                    Some([piece.ends[0].clone(), center.clone()]),
                )?;
                let right = Piece::new(
                    &source.curve,
                    piece.span,
                    [middle, piece.domain[1]],
                    Some([center, piece.ends[1].clone()]),
                )?;
                stack.push(right);
                stack.push(left);
            }
        }
        Ok(Classification {
            location: if winding == 0 {
                Location::Outside
            } else {
                Location::Inside
            },
            winding: Some(winding),
            reason: ClassificationReason::Separated,
            cells,
            max_cells,
            uncertain: None,
            tolerance_uv: self.tolerance_uv,
        })
    }
}
fn separated(a: [Interval; 2], b: [Interval; 2]) -> bool {
    (0..2).any(|k| a[k].hi < b[k].lo || a[k].lo > b[k].hi)
}
/// A separated convex box that straddles the horizontal ray must lie wholly
/// left or right of it. No approximate root/intersection calculation is needed.
fn chord_winding(a: Point, b: Point, bounds: [Interval; 2], q: Point) -> i32 {
    if bounds[0].lo <= q[0] {
        return 0;
    }
    if a[1] <= q[1] && b[1] > q[1] {
        1
    } else if b[1] <= q[1] && a[1] > q[1] {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polygon(points: &[[f64; 2]]) -> Curve {
        Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap()
    }
    fn square(lo: f64, hi: f64) -> Curve {
        polygon(&[[lo, lo], [hi, lo], [hi, hi], [lo, hi], [lo, lo]])
    }
    fn at(domain: &TrimDomain, p: Point) -> Classification {
        domain.classify([[p[0]; 2], [p[1]; 2]], 10000).unwrap()
    }
    fn circle(r: f64) -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: vec![
                [r, 0.],
                [r, r],
                [0., r],
                [-r, r],
                [-r, 0.],
                [-r, -r],
                [0., -r],
                [r, -r],
                [r, 0.],
            ]
            .into_iter()
            .map(|p| p.to_vec())
            .collect(),
            weights: vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            periodic: false,
        }
    }
    #[test]
    fn exact_joins_distinguish_tiny_gaps_and_unclamped_endpoints() {
        let mut curves = vec![
            Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
            Curve::from_polyline(vec![vec![1., 0.], vec![0., 1.]]).unwrap(),
            Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
        ];
        curves[0].weights = vec![2., 3.];
        assert_eq!(exact_loop_joins(&curves).unwrap(), Some(true));
        curves[1].control_points[0][1] = 1e-12;
        assert!(TrimDomain::new(&[curves.clone()], 1e-6).is_ok());
        assert_eq!(exact_loop_joins(&curves).unwrap(), Some(false));
        curves[1].control_points[0][1] = 0.;
        curves[0].knots = vec![-1., 0., 1., 2.];
        assert_eq!(exact_loop_joins(&curves).unwrap(), None);
        assert!(exact_loop_joins(&[]).is_err());
    }
    #[test]
    fn square_regions_vertices_and_full_rectangles() {
        let d = TrimDomain::new(&[vec![square(0., 10.)]], 1e-7).unwrap();
        assert_eq!(at(&d, [5., 5.]).location, Location::Inside);
        assert_eq!(at(&d, [5., 5.]).winding, Some(1));
        assert_eq!(at(&d, [15., 5.]).location, Location::Outside);
        assert_eq!(at(&d, [-1., 0.]).location, Location::Outside);
        for p in [[0., 0.], [5., 0.], [10., 5.], [0., 5.]] {
            let r = at(&d, p);
            assert_eq!(r.location, Location::Unresolved, "{p:?}: {r:?}");
            assert!(r.winding.is_none());
            assert!(r.uncertain.is_some());
        }
        assert_eq!(
            d.classify([[1., 9.], [1., 9.]], 1000).unwrap().location,
            Location::Inside
        );
        assert_eq!(
            d.classify([[11., 12.], [1., 9.]], 1000).unwrap().location,
            Location::Outside
        );
        assert_eq!(
            d.classify([[9., 11.], [1., 9.]], 1000).unwrap().location,
            Location::Unresolved
        );
        assert_eq!(
            d.classify([[-1., 11.], [-1., 11.]], 1000).unwrap().location,
            Location::Unresolved
        );
    }
    #[test]
    fn rational_circle_hole_and_orientation() {
        let outer = circle(10.);
        let hole = circle(3.).reverse().unwrap();
        let d = TrimDomain::new(&[vec![outer.clone()], vec![hole]], 1e-6).unwrap();
        assert_eq!(at(&d, [0., 0.]).location, Location::Outside);
        assert_eq!(at(&d, [5., 0.]).location, Location::Inside);
        assert_eq!(at(&d, [11., 0.]).location, Location::Outside);
        assert_eq!(at(&d, [3., 0.]).location, Location::Unresolved);
        assert_eq!(
            d.classify([[2., 4.], [-0.1, 0.1]], 10000).unwrap().location,
            Location::Unresolved
        );
        let reversed = TrimDomain::new(&[vec![outer.reverse().unwrap()]], 1e-6).unwrap();
        assert_eq!(at(&reversed, [0., 0.]).winding, Some(-1));
    }
    #[test]
    fn general_weighted_cubic_loops_and_non_knot_boundary_point() {
        let lower = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 5., 5., 5., 5.],
            control_points: vec![vec![-2., 0.], vec![-1., -3.], vec![1., -1.], vec![2., 0.]],
            weights: vec![1., 0.6, 1.4, 1.],
            periodic: false,
        };
        let upper = Curve {
            control_points: vec![vec![2., 0.], vec![1., 3.], vec![-1., 2.], vec![-2., 0.]],
            weights: vec![1., 1.3, 0.7, 1.],
            ..lower.clone()
        };
        let d = TrimDomain::new(&[vec![lower.clone(), upper]], 1e-5).unwrap();
        assert_eq!(at(&d, [0., 0.]).location, Location::Inside);
        assert_eq!(at(&d, [0., 4.]).location, Location::Outside);
        assert_eq!(
            d.classify([[-0.1, 0.1], [-0.1, 0.1]], 10000)
                .unwrap()
                .location,
            Location::Inside
        );
        let p = lower.evaluate(3.137).unwrap().point;
        assert_eq!(at(&d, [p[0], p[1]]).location, Location::Unresolved);
    }
    #[test]
    fn periodic_curve_and_crossing_path_keep_winding_semantics() {
        let c = Curve {
            degree: 2,
            knots: vec![-2., -1., 0., 1., 2., 3., 4., 5., 6.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 0.],
                vec![1., 1.],
                vec![0., 1.],
                vec![0., 0.],
                vec![1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let d = TrimDomain::new(&[vec![c]], 1e-6).unwrap();
        assert_eq!(at(&d, [0.5, 0.5]).location, Location::Inside);
        assert_eq!(at(&d, [2., 2.]).location, Location::Outside);
        let d = TrimDomain::new(
            &[vec![polygon(&[
                [0., 0.],
                [2., 2.],
                [0., 2.],
                [2., 0.],
                [0., 0.],
            ])]],
            1e-6,
        )
        .unwrap();
        assert_eq!(at(&d, [1., 1.]).location, Location::Unresolved);
        assert_eq!(at(&d, [1., 0.2]).location, Location::Inside);
        assert_eq!(at(&d, [1., 1.8]).location, Location::Inside);
    }
    #[test]
    fn bounded_joins_are_explicit_and_large_gaps_are_rejected() {
        let mut c = square(0., 10.);
        c.control_points.last_mut().unwrap()[0] = 5e-7;
        let d = TrimDomain::new(&[vec![c.clone()]], 1e-6).unwrap();
        assert_eq!(at(&d, [5., 5.]).location, Location::Inside);
        let near = at(&d, [2e-7, 0.]);
        assert_eq!(near.location, Location::Unresolved);
        assert_eq!(near.reason, ClassificationReason::JoinBand);
        assert_eq!(near.uncertain.unwrap().2, [4., 4.]);
        c.control_points.last_mut().unwrap()[0] = 1e-3;
        assert!(TrimDomain::new(&[vec![c]], 1e-6).is_err());
    }
    #[test]
    fn translation_scaling_and_resource_stops() {
        for scale in [1e-6, 1., 1e6] {
            let mut c = square(0., 10.);
            for p in &mut c.control_points {
                for x in p {
                    *x = *x * scale + 1e6;
                }
            }
            let d = TrimDomain::new(&[vec![c]], scale * 1e-4).unwrap();
            assert_eq!(at(&d, [1e6 + 5. * scale; 2]).location, Location::Inside);
            let r = d.classify([[1e6 + 5. * scale; 2]; 2], 1).unwrap();
            assert_eq!(r.location, Location::Unresolved);
            assert_eq!(r.reason, ClassificationReason::WorkLimit);
            assert_eq!(r.cells, 1);
            assert_eq!(r.winding, None);
        }
        assert!(TrimDomain::new(&[], 1e-6).is_err());
        assert!(TrimDomain::new(&[vec![square(0., 1.)]], 0.).is_err());
        let d = TrimDomain::new(&[vec![square(0., 1.)]], 1e-6).unwrap();
        assert!(d.classify([[1., 0.], [0., 1.]], 100).is_err());
        assert!(d.classify([[0., 1.]; 2], 0).is_err());
    }
}
