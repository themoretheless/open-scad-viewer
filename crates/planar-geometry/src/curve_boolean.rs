//! Boolean arrangements retaining exact source line/quadratic/cubic spans.
//!
//! Adaptive chords locate candidate intersections; intersections are refined on
//! the original curves, which are split with de Casteljau. Chords are never used
//! as replacement output geometry. The result has filled material on its left.
use crate::curve::{BezPath, CurveSegment, PathEl, Point, Rect, Shape};
pub use crate::tessellation::FillRule;
use crate::{Result, check};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Union,
    Intersection,
    Difference,
    Xor,
}
#[derive(Clone, Debug)]
pub struct Contour {
    pub path: Vec<PathEl>,
}
#[derive(Clone, Debug, Default)]
pub struct Contours {
    contours: Vec<Contour>,
}
impl Contours {
    pub fn contours(&self) -> std::slice::Iter<'_, Contour> {
        self.contours.iter()
    }
    pub fn to_path(&self) -> BezPath {
        let mut path = BezPath::new();
        for contour in &self.contours {
            path.extend(contour.path.iter().copied());
        }
        path
    }
}

/// Apply one winding rule to both inputs (the Curvex migration entry point).
pub fn binary_op_with_eps(
    a: &BezPath,
    b: &BezPath,
    rule: FillRule,
    op: BinaryOp,
    tolerance: f64,
) -> Result<Contours> {
    binary_op(a, b, rule, rule, op, tolerance)
}

#[derive(Clone, Copy)]
struct Chord {
    source: usize,
    t0: f64,
    t1: f64,
    a: Point,
    b: Point,
}
#[derive(Clone, Copy)]
struct Boundary {
    curve: CurveSegment,
    from: usize,
    to: usize,
}
/// Repeated boundary-side queries share analytic bounds. A horizontal ray can
/// skip curves outside its y range or entirely to its left. For a curve wholly
/// to the right, its signed crossing count is determined just by its endpoints:
/// internal up/down crossings cancel in pairs, including tangent extrema.
struct PreparedWinding {
    segments: Vec<RaySegment>,
}
struct RaySegment {
    curve: CurveSegment,
    bounds: Rect,
    coefficients: [Point; 4],
    cuts: [f64; 4],
    count: usize,
}
impl RaySegment {
    fn new(curve: CurveSegment, bounds: Rect) -> Self {
        let a = curve.start();
        let c = match curve {
            CurveSegment::Line(_, b) => [a, b - a, Point::ZERO, Point::ZERO],
            CurveSegment::Quad(_, b, c) => [a, (b - a) * 2., (c - a) - (b - a) * 2., Point::ZERO],
            CurveSegment::Cubic(_, b, c, d) => [
                a,
                (b - a) * 3.,
                ((c - a) - (b - a) * 2.) * 3.,
                (d - a) - (c - a) * 3. + (b - a) * 3.,
            ],
        };
        let mut cuts = [0., 1., 1., 1.];
        let mut count = 1;
        let mut root = |t: f64| {
            if t > 0. && t < 1. && count < 3 {
                cuts[count] = t;
                count += 1;
            }
        };
        let (aa, bb, cc) = (3. * c[3].y, 2. * c[2].y, c[1].y);
        let scale = aa.abs().max(bb.abs()).max(cc.abs());
        if scale > 0. && scale.is_finite() {
            let (aa, bb, cc) = (aa / scale, bb / scale, cc / scale);
            if aa.abs() <= f64::EPSILON * bb.abs() {
                if bb != 0. {
                    root(-cc / bb);
                }
            } else {
                let d = bb * bb - 4. * aa * cc;
                if d == 0. {
                    root(-bb / (2. * aa));
                } else if d > 0. {
                    let q = -0.5 * (bb + d.sqrt().copysign(bb));
                    root(q / aa);
                    root(cc / q);
                }
            }
        }
        cuts[count] = 1.;
        count += 1;
        cuts[..count].sort_by(f64::total_cmp);
        Self {
            curve,
            bounds,
            coefficients: c,
            cuts,
            count,
        }
    }
    fn winding(&self, point: Point) -> i32 {
        let bounds = self.bounds;
        let curve = self.curve;
        if point.y < bounds.y0 || point.y >= bounds.y1 || point.x >= bounds.x1 {
            return 0;
        }
        if point.x < bounds.x0 {
            let a = curve.start().y;
            let b = curve.end().y;
            return if a <= point.y && point.y < b {
                1
            } else if b <= point.y && point.y < a {
                -1
            } else {
                0
            };
        }
        if matches!(curve, CurveSegment::Line(..)) {
            return curve.winding(point);
        }
        let c = &self.coefficients;
        let residual = |t: f64| ((c[3].y * t + c[2].y) * t + c[1].y) * t + (c[0].y - point.y);
        let derivative = |t: f64| (3. * c[3].y * t + 2. * c[2].y) * t + c[1].y;
        let mut winding = 0;
        for pair in self.cuts[..self.count].windows(2) {
            let mut lo = pair[0];
            let mut hi = pair[1];
            // De Casteljau endpoint values agree with the source's half-open
            // interval convention at extrema and shared path vertices.
            let y0 = curve.eval(lo).y;
            let y1 = curve.eval(hi).y;
            let direction = if y0 <= point.y && point.y < y1 {
                1
            } else if y1 <= point.y && point.y < y0 {
                -1
            } else {
                continue;
            };
            let mut t = lo + (hi - lo) * (point.y - y0) / (y1 - y0);
            for iteration in 0..64 {
                let f = residual(t);
                if f == 0. || hi - lo <= f64::EPSILON * 4. {
                    break;
                }
                if (f < 0.) == (direction > 0) {
                    lo = t;
                } else {
                    hi = t;
                }
                let next = t - f / derivative(t);
                // Newton is quadratically convergent for ordinary crossings.
                // A guaranteed bracketed fallback handles stationary roots.
                t = if iteration < 12 && next > lo && next < hi {
                    next
                } else {
                    (lo + hi) * 0.5
                };
            }
            let x = ((c[3].x * t + c[2].x) * t + c[1].x) * t + c[0].x;
            if x > point.x {
                winding += direction;
            }
        }
        winding
    }
}
impl PreparedWinding {
    fn new(curves: &[CurveSegment], bounds: &[Rect]) -> Self {
        Self {
            segments: curves
                .iter()
                .zip(bounds)
                .map(|(&c, &b)| RaySegment::new(c, b))
                .collect(),
        }
    }
    fn winding(&self, point: Point) -> i32 {
        self.segments
            .iter()
            .map(|segment| segment.winding(point))
            .sum()
    }
}
fn sub(a: Point, b: Point) -> [f64; 2] {
    [a.x - b.x, a.y - b.y]
}
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn len(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn at(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
fn controls(c: CurveSegment) -> ([Point; 4], usize) {
    match c {
        CurveSegment::Line(a, b) => ([a, b, Point::ZERO, Point::ZERO], 2),
        CurveSegment::Quad(a, b, c) => ([a, b, c, Point::ZERO], 3),
        CurveSegment::Cubic(a, b, c, d) => ([a, b, c, d], 4),
    }
}
fn flat(c: CurveSegment, tolerance: f64) -> bool {
    let (points, count) = controls(c);
    let p = &points[..count];
    let a = p[0];
    let b = *p.last().unwrap();
    let d = sub(b, a);
    let length = len(d);

    p.iter().all(|&q| {
        if length == 0. {
            q.distance(a) <= tolerance
        } else {
            let v = sub(q, a);
            let projection = (v[0] * d[0] + v[1] * d[1]) / (length * length);
            q.distance(at(a, b, projection.clamp(0., 1.))) <= tolerance
        }
    })
}
fn has_monotone_axis(curve: CurveSegment) -> bool {
    (0..2).any(|axis| has_monotone_coordinate(curve, axis))
}
fn has_monotone_coordinate(curve: CurveSegment, axis: usize) -> bool {
    let (points, n) = controls(curve);
    let coordinate = |p: Point| if axis == 0 { p.x } else { p.y };
    let mut positive = false;
    let mut negative = false;
    for pair in points[..n].windows(2) {
        let d = coordinate(pair[1]) - coordinate(pair[0]);
        positive |= d > 0.;
        negative |= d < 0.;
    }
    positive != negative
}
fn no_interior_intersection(a: CurveSegment, b: CurveSegment, aa: Rect, bb: Rect) -> bool {
    let overlap_x = aa.x1.min(bb.x1) - aa.x0.max(bb.x0);
    let overlap_y = aa.y1.min(bb.y1) - aa.y0.max(bb.y0);
    // A nonconstant monotone polynomial reaches either end of its coordinate
    // range only at a path endpoint. Thus curves whose ranges just meet on
    // that axis can share only already-known endpoints, or be fully disjoint.
    // Boxes can meet at a point belonging to neither curve: opposite ellipse
    // quadrants' boxes, for example, touch at the ellipse center.
    // Do not expand this condition by the numeric welding tolerance. A nearly
    // axis-aligned curve can move less than that tolerance along this axis;
    // even a tiny positive range overlap can then contain a real crossing far
    // from either curve's endpoint on the other axis.
    (overlap_x <= 0. && has_monotone_coordinate(a, 0) && has_monotone_coordinate(b, 0))
        || (overlap_y <= 0. && has_monotone_coordinate(a, 1) && has_monotone_coordinate(b, 1))
}
fn flatten(
    c: CurveSegment,
    source: usize,
    t0: f64,
    t1: f64,
    tolerance: f64,
    depth: usize,
    out: &mut Vec<Chord>,
) -> Result<()> {
    check(out.len() < 32768, "Curve Boolean chord budget exceeded")?;
    if flat(c, tolerance) {
        out.push(Chord {
            source,
            t0,
            t1,
            a: c.start(),
            b: c.end(),
        });
        return Ok(());
    }
    check(
        depth < 30,
        "Curve Boolean flattening failed to meet tolerance",
    )?;
    let (a, b) = c.split(0.5);
    let mid = (t0 + t1) * 0.5;
    flatten(a, source, t0, mid, tolerance, depth + 1, out)?;
    flatten(b, source, mid, t1, tolerance, depth + 1, out)
}

/// Boolean operation with a separate source winding rule for each operand.
pub fn binary_op(
    a: &BezPath,
    b: &BezPath,
    rule_a: FillRule,
    rule_b: FillRule,
    op: BinaryOp,
    tolerance: f64,
) -> Result<Contours> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Invalid curve Boolean tolerance",
    )?;
    // Parsing validates command order and finite coordinates; open contours are
    // rejected instead of silently closing editable open paths.
    for p in a
        .to_bezier_paths()?
        .iter()
        .chain(b.to_bezier_paths()?.iter())
    {
        check(p.closed, "Curve Boolean requires closed contours")?;
    }
    let a_count = a.segments().count();
    let curves: Vec<_> = a.segments().chain(b.segments()).collect();
    check(
        curves.len() <= 65_536,
        "Curve Boolean segment budget exceeded",
    )?;
    if curves.is_empty() {
        return Ok(Contours::default());
    }
    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    let mut magnitude = 0_f64;
    for &c in &curves {
        let (points, count) = controls(c);
        for p in &points[..count] {
            check(
                p.x.is_finite() && p.y.is_finite(),
                "Invalid curve Boolean coordinate",
            )?;
            minimum[0] = minimum[0].min(p.x);
            minimum[1] = minimum[1].min(p.y);
            maximum[0] = maximum[0].max(p.x);
            maximum[1] = maximum[1].max(p.y);
            magnitude = magnitude.max(p.x.abs()).max(p.y.abs());
        }
    }
    let extent = (maximum[0] - minimum[0]).max(maximum[1] - minimum[1]);
    check(
        extent.is_finite(),
        "Curve Boolean coordinate span overflows",
    )?;
    let numeric = (extent * 1e-10)
        .max(magnitude * f64::EPSILON * 64.)
        .max(1e-13);
    // Broad-phase exact curve bounds before constructing approximation chords.
    // Most existing compound holes are disjoint, so re-flattening every one on
    // each subtraction fold is avoidable. Monotone curves cannot self-cross;
    // shared endpoint-only contacts already have both required parameters.
    let bounds: Vec<_> = curves.iter().map(|&c| c.bounding_box()).collect();
    let mut active: Vec<_> = curves.iter().map(|&c| !has_monotone_axis(c)).collect();
    let mut possible_pairs = BTreeSet::new();
    let mut curve_order: Vec<_> = (0..curves.len()).collect();
    curve_order.sort_by(|&a, &b| bounds[a].x0.total_cmp(&bounds[b].x0));
    let mut broad_comparisons = 0usize;
    for (position, &i) in curve_order.iter().enumerate() {
        for &j in &curve_order[position + 1..] {
            broad_comparisons += 1;
            check(
                broad_comparisons <= 8_000_000,
                "Curve Boolean broad-phase work budget exceeded",
            )?;
            let aa = bounds[i];
            let bb = bounds[j];
            if bb.x0 > aa.x1 + numeric * 4. {
                break;
            }
            if bb.y0 > aa.y1 + numeric * 4. || aa.y0 > bb.y1 + numeric * 4. {
                continue;
            }
            if no_interior_intersection(curves[i], curves[j], aa, bb) {
                continue;
            }
            active[i] = true;
            active[j] = true;
            possible_pairs.insert((i.min(j), i.max(j)));
        }
    }
    let chord_tol = (tolerance * 0.125).min(extent * 0.001).max(numeric * 4.);
    let mut chords = Vec::new();
    let mut chord_ranges = Vec::new();
    for (i, &c) in curves.iter().enumerate() {
        let begin = chords.len();
        if active[i] {
            flatten(c, i, 0., 1., chord_tol, 0, &mut chords)?;
        }
        chord_ranges.push(begin..chords.len());
    }
    let mut parameters = vec![vec![0., 1.]; curves.len()];
    let mut checked_pairs = BTreeSet::new();
    let mut coincident_pairs = HashMap::new();
    let mut order: Vec<_> = (0..chords.len()).collect();
    order.sort_by(|&i, &j| {
        chords[i]
            .a
            .x
            .min(chords[i].b.x)
            .total_cmp(&chords[j].a.x.min(chords[j].b.x))
    });
    let mut comparisons = 0usize;
    for (position, &i) in order.iter().enumerate() {
        let x = chords[i];
        for &j in &order[position + 1..] {
            let y = chords[j];
            if y.a.x.min(y.b.x) > x.a.x.max(x.b.x) + 2. * chord_tol {
                break;
            }
            if y.a.y.min(y.b.y) > x.a.y.max(x.b.y) + 2. * chord_tol
                || x.a.y.min(x.b.y) > y.a.y.max(y.b.y) + 2. * chord_tol
            {
                continue;
            }
            if x.source == y.source && ((x.t1 - y.t0).abs() < 1e-12 || (y.t1 - x.t0).abs() < 1e-12)
            {
                continue;
            }
            comparisons += 1;
            check(
                comparisons <= 8_000_000,
                "Curve Boolean intersection budget exceeded",
            )?;
            let pair = (x.source.min(y.source), x.source.max(y.source));
            if x.source != y.source && !possible_pairs.contains(&pair) {
                continue;
            }
            if x.source != y.source
                && checked_pairs.insert(pair)
                && let Some(((t0, u0), (t1, u1))) = coincident_interval(
                    curves[pair.0],
                    curves[pair.1],
                    &chords[chord_ranges[pair.0].clone()],
                    &chords[chord_ranges[pair.1].clone()],
                    numeric,
                    chord_tol,
                )
            {
                parameters[pair.0].extend([t0, t1]);
                parameters[pair.1].extend([u0, u1]);
                coincident_pairs.insert(pair, ((t0, u0), (t1, u1)));
            }
            // Coincident arcs have infinitely many intersections. Splitting at
            // every approximate chord crossing creates mismatched micro-spans.
            // Their exact overlap endpoints partition both original curves.
            if let Some(&((t0, u0), (t1, u1))) = coincident_pairs.get(&pair) {
                let (first, second) = if x.source == pair.0 { (x, y) } else { (y, x) };
                let map = |u: f64| t0 + (u - u0) * (t1 - t0) / (u1 - u0);
                let a = map(second.t0);
                let b = map(second.t1);
                // Skip only matching parameter spans. A looping cubic may
                // cross a distinct part of itself beyond its shared span.
                if first.t0 <= a.max(b) + 1e-9 && first.t1 >= a.min(b) - 1e-9 {
                    continue;
                }
            }
            let d = sub(x.b, x.a);
            let e = sub(y.b, y.a);
            let delta = sub(y.a, x.a);
            let det = cross(d, e);
            if det.abs() > numeric * len(d).max(len(e)) {
                let t = cross(delta, e) / det;
                let u = cross(delta, d) / det;
                let margin_x = 2. * chord_tol / len(d).max(numeric);
                let margin_y = 2. * chord_tol / len(e).max(numeric);
                if t >= -margin_x
                    && t <= 1. + margin_x
                    && u >= -margin_y
                    && u <= 1. + margin_y
                    && let Some((t, u)) = refine(
                        curves[x.source],
                        curves[y.source],
                        x.t0 + (x.t1 - x.t0) * t.clamp(0., 1.),
                        y.t0 + (y.t1 - y.t0) * u.clamp(0., 1.),
                        numeric,
                    )
                    && t >= x.t0 - 1e-6
                    && t <= x.t1 + 1e-6
                    && u >= y.t0 - 1e-6
                    && u <= y.t1 + 1e-6
                {
                    parameters[x.source].push(t);
                    parameters[y.source].push(u);
                }
            } else if len(d) > numeric
                && len(e) > numeric
                && cross(delta, d).abs() <= 2. * chord_tol * len(d)
            {
                // Coincident lines/curves and tangencies: project endpoint
                // candidates onto the actual other curve, never onto output chords.
                for (s, t, other, lo, hi) in [
                    (x.source, x.t0, y.source, y.t0, y.t1),
                    (x.source, x.t1, y.source, y.t0, y.t1),
                    (y.source, y.t0, x.source, x.t0, x.t1),
                    (y.source, y.t1, x.source, x.t0, x.t1),
                ] {
                    let point = curves[s].eval(t);
                    let q = nearest_parameter(curves[other], point, lo, hi);
                    if curves[other].eval(q).distance(point) <= numeric * 4. {
                        parameters[s].push(t);
                        parameters[other].push(q);
                    }
                }
            }
        }
    }
    let prepared_a = PreparedWinding::new(&curves[..a_count], &bounds[..a_count]);
    let prepared_b = PreparedWinding::new(&curves[a_count..], &bounds[a_count..]);
    let inside = |p: Point| {
        let classify = |w: i32, rule| match rule {
            FillRule::NonZero => w != 0,
            FillRule::EvenOdd => w % 2 != 0,
        };
        let x = classify(prepared_a.winding(p), rule_a);
        let y = classify(prepared_b.winding(p), rule_b);
        match op {
            BinaryOp::Union => x || y,
            BinaryOp::Intersection => x && y,
            BinaryOp::Difference => x && !y,
            BinaryOp::Xor => x != y,
        }
    };
    let mut vertices = Vec::<Point>::new();
    let mut grid = HashMap::<(i64, i64), Vec<usize>>::new();
    let mut boundary = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, mut ts) in parameters.into_iter().enumerate() {
        for t in &mut ts {
            if *t < 1e-9 {
                *t = 0.;
            } else if *t > 1. - 1e-9 {
                *t = 1.;
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);
        for pair in ts.windows(2) {
            if pair[1] - pair[0] <= 1e-10 {
                continue;
            }
            let mut c = curves[i].subsegment(pair[0], pair[1]);
            // A cubic may have a stationary point at its exact midpoint.
            // The whole span remains a valid boundary; select another interior
            // point instead of mistaking the stationary tangent for zero area.
            let Some((middle, direction, length)) = [0.5, 0.3819660112501051, 0.6180339887498949]
                .into_iter()
                .find_map(|t| {
                    let tangent = c.derivative(t);
                    let direction = [tangent.x, tangent.y];
                    let length = len(direction);
                    (length > numeric).then(|| (c.eval(t), direction, length))
                })
            else {
                continue;
            };
            // Classification has a tighter precision budget than vertex welds:
            // a weld-sized probe can jump across a thin crescent between nearly
            // coincident curves and remove a real boundary.
            let probe = (magnitude * f64::EPSILON * 8.)
                .max(extent * 1e-14)
                .max(1e-15);
            let distance = (probe * 16.).min(length * 1e-3);
            let normal = [
                -direction[1] / length * distance,
                direction[0] / length * distance,
            ];
            let left = inside(Point::new(middle.x + normal[0], middle.y + normal[1]));
            let right = inside(Point::new(middle.x - normal[0], middle.y - normal[1]));
            if left == right {
                continue;
            }
            if !left {
                c = c.reverse();
            }
            let from = vertex_id(c.start(), numeric * 8., &mut vertices, &mut grid);
            let to = vertex_id(c.end(), numeric * 8., &mut vertices, &mut grid);
            if from == to && c.eval(0.5).distance(c.start()) < numeric {
                continue;
            }
            // Distinct arcs can share endpoints; include their midpoint in the key.
            // A straight cubic may traverse its line with nonuniform speed.
            // Its parametric midpoint is then different from the line's even
            // though their entire geometry is coincident. Canonicalize only
            // the key, retaining the actual original curve for output.
            let mid = if flat(c, numeric * 4.) {
                at(c.start(), c.end(), 0.5)
            } else {
                c.eval(0.5)
            };
            let key = (
                from,
                to,
                (mid.x / (numeric * 16.)).round() as i64,
                (mid.y / (numeric * 16.)).round() as i64,
            );
            if seen.insert(key) {
                boundary.push(Boundary { curve: c, from, to });
            }
        }
    }
    let mut outgoing = vec![Vec::new(); vertices.len()];
    for (i, edge) in boundary.iter().enumerate() {
        outgoing[edge.from].push(i);
    }
    let mut used = vec![false; boundary.len()];
    let mut result = Vec::new();
    for first in 0..boundary.len() {
        if used[first] {
            continue;
        }
        let start = boundary[first].from;
        let mut current = first;
        let mut elements = vec![PathEl::MoveTo(vertices[start])];
        let mut count = 0;
        loop {
            check(!used[current], "Curve Boolean reused boundary")?;
            used[current] = true;
            let edge = boundary[current];
            let c = snap_endpoints(edge.curve, vertices[edge.from], vertices[edge.to]);
            elements.push(c.to_path_el());
            count += 1;
            check(
                count <= boundary.len(),
                "Curve Boolean boundary budget exceeded",
            )?;
            if edge.to == start {
                break;
            }
            let t = c.derivative(1.);
            let incoming = [t.x, t.y];
            current=outgoing[edge.to].iter().filter(|&&i|!used[i]).copied().max_by(|&i,&j| {
                let turn=|k:usize|{let t=boundary[k].curve.derivative(0.);cross(incoming,[t.x,t.y]).atan2(incoming[0]*t.x+incoming[1]*t.y)};
                turn(i).total_cmp(&turn(j))
            }).ok_or_else(||crate::error(format!("Open curve Boolean arrangement boundary at {:?}, incoming {:?}, {} total edges, outgoing {:?}",vertices[edge.to],edge.curve,boundary.len(),outgoing[edge.to].iter().map(|&i|(used[i],boundary[i].curve)).collect::<Vec<_>>())))?;
        }
        elements.push(PathEl::ClosePath);
        let path = BezPath::from_vec(elements.clone());
        if path.area().abs() > numeric * numeric {
            result.push(Contour { path: elements });
        }
    }
    result.sort_by(|a, b| {
        BezPath::from_vec(b.path.clone())
            .area()
            .abs()
            .total_cmp(&BezPath::from_vec(a.path.clone()).area().abs())
    });
    Ok(Contours { contours: result })
}

fn refine(
    a: CurveSegment,
    b: CurveSegment,
    mut t: f64,
    mut u: f64,
    eps: f64,
) -> Option<(f64, f64)> {
    for _ in 0..32 {
        let p = a.eval(t);
        let q = b.eval(u);
        let f = sub(p, q);
        if len(f) <= eps {
            return Some((t, u));
        }
        let da = a.derivative(t);
        let db = b.derivative(u);
        let da = [da.x, da.y];
        let db = [db.x, db.y];
        let den = cross(da, db);
        if den.abs() <= 1e-18 * len(da) * len(db) {
            return None;
        }
        let dt = -cross(f, db) / den;
        let du = -cross(f, da) / den;
        let next_t = (t + dt).clamp(0., 1.);
        let next_u = (u + du).clamp(0., 1.);
        if (next_t - t).abs() + (next_u - u).abs() < 1e-15 {
            break;
        }
        t = next_t;
        u = next_u;
    }
    (a.eval(t).distance(b.eval(u)) <= eps * 4.).then_some((t, u))
}
fn coincident_interval(
    a: CurveSegment,
    b: CurveSegment,
    a_chords: &[Chord],
    b_chords: &[Chord],
    eps: f64,
    chord_tolerance: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let mut matches = Vec::new();
    for t in [0., 1.] {
        if let Some(u) = parameter_at_point(b, a.eval(t), b_chords, eps, chord_tolerance) {
            matches.push((t, u));
        }
    }
    for u in [0., 1.] {
        if let Some(t) = parameter_at_point(a, b.eval(u), a_chords, eps, chord_tolerance) {
            matches.push((t, u));
        }
    }
    matches.sort_by(|a, b| a.0.total_cmp(&b.0));
    let &(t0, u0) = matches.first()?;
    let &(t1, u1) = matches.last()?;
    if t1 - t0 < 1e-9 || (u1 - u0).abs() < 1e-9 {
        return None;
    }
    let a_span = a.subsegment(t0, t1);
    let b_span = b.subsegment(u0, u1);
    // Four samples characterize a cubic polynomial under an affine parameter
    // restriction. Additional samples improve roundoff discrimination.
    for t in [0., 0.2, 0.4, 0.6, 0.8, 1.] {
        if a_span.eval(t).distance(b_span.eval(t)) > eps * 4. {
            return None;
        }
    }
    Some(((t0, u0), (t1, u1)))
}
fn parameter_at_point(
    curve: CurveSegment,
    p: Point,
    chords: &[Chord],
    eps: f64,
    chord_tolerance: f64,
) -> Option<f64> {
    for t in [0., 1.] {
        if curve.eval(t).distance(p) <= eps {
            return Some(t);
        }
    }
    for chord in chords {
        let pad = chord_tolerance + eps;
        if p.x < chord.a.x.min(chord.b.x) - pad
            || p.x > chord.a.x.max(chord.b.x) + pad
            || p.y < chord.a.y.min(chord.b.y) - pad
            || p.y > chord.a.y.max(chord.b.y) + pad
        {
            continue;
        }
        let t = nearest_parameter(curve, p, chord.t0, chord.t1);
        if curve.eval(t).distance(p) <= eps {
            return Some(t);
        }
    }
    None
}
fn nearest_parameter(curve: CurveSegment, p: Point, mut lo: f64, mut hi: f64) -> f64 {
    // Convex local chord intervals; golden-section minimization remains stable
    // when a Newton intersection Jacobian is singular at a tangency.
    for _ in 0..50 {
        let t = lo + (hi - lo) / 3.;
        let u = hi - (hi - lo) / 3.;
        if curve.eval(t).distance(p) < curve.eval(u).distance(p) {
            hi = u;
        } else {
            lo = t;
        }
    }
    (lo + hi) * 0.5
}
fn vertex_id(
    p: Point,
    eps: f64,
    points: &mut Vec<Point>,
    grid: &mut HashMap<(i64, i64), Vec<usize>>,
) -> usize {
    let key = ((p.x / eps).round() as i64, (p.y / eps).round() as i64);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(ids) = grid.get(&(key.0 + dx, key.1 + dy)) {
                for &i in ids {
                    if points[i].distance(p) <= eps {
                        return i;
                    }
                }
            }
        }
    }
    let id = points.len();
    points.push(p);
    grid.entry(key).or_default().push(id);
    id
}
fn snap_endpoints(c: CurveSegment, start: Point, end: Point) -> CurveSegment {
    match c {
        CurveSegment::Line(..) => CurveSegment::Line(start, end),
        CurveSegment::Quad(_, p, _) => CurveSegment::Quad(start, p, end),
        CurveSegment::Cubic(_, p, q, _) => CurveSegment::Cubic(start, p, q, end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::BezierPath;

    fn rectangle(x: f64, y: f64, w: f64, h: f64) -> BezPath {
        BezPath::from_bezier_paths(&[BezierPath::from_rect([x, y], [x + w, y + h]).unwrap()])
    }
    fn ellipse(x: f64, y: f64, rx: f64, ry: f64) -> BezPath {
        BezPath::from_bezier_paths(&[BezierPath::from_ellipse([x, y], rx, ry).unwrap()])
    }
    fn reverse(path: &BezPath) -> BezPath {
        BezPath::from_bezier_paths(
            &path
                .to_bezier_paths()
                .unwrap()
                .iter()
                .map(BezierPath::reverse)
                .collect::<Vec<_>>(),
        )
    }
    fn combine(a: &BezPath, b: &BezPath, op: BinaryOp) -> BezPath {
        binary_op_with_eps(a, b, FillRule::NonZero, op, 0.05)
            .unwrap_or_else(|e| panic!("{op:?} {:?} with {:?}: {e:?}", a.elements(), b.elements()))
            .to_path()
    }
    fn expected(a: bool, b: bool, op: BinaryOp) -> bool {
        match op {
            BinaryOp::Union => a || b,
            BinaryOp::Intersection => a && b,
            BinaryOp::Difference => a && !b,
            BinaryOp::Xor => a != b,
        }
    }
    fn covered(path: &BezPath, point: Point, rule: FillRule) -> bool {
        let winding = path.winding(point);
        match rule {
            FillRule::NonZero => winding != 0,
            FillRule::EvenOdd => winding % 2 != 0,
        }
    }
    fn assert_grid(
        a: &BezPath,
        b: &BezPath,
        out: &BezPath,
        rule_a: FillRule,
        rule_b: FillRule,
        op: BinaryOp,
    ) {
        let bb = a.bounding_box().union(b.bounding_box());
        for iy in 0..33 {
            for ix in 0..33 {
                // Irrational-looking offsets avoid sampling exactly on source edges.
                let p = Point::new(
                    bb.x0 + (bb.width() + 0.7) * (ix as f64 + 0.371) / 33. - 0.35,
                    bb.y0 + (bb.height() + 0.7) * (iy as f64 + 0.619) / 33. - 0.35,
                );
                assert_eq!(
                    covered(out, p, FillRule::NonZero),
                    expected(covered(a, p, rule_a), covered(b, p, rule_b), op),
                    "{op:?} region mismatch at {p:?}"
                );
            }
        }
        for contour in out.to_bezier_paths().unwrap() {
            assert!(contour.closed);
        }
    }
    fn all_ops_grid(a: &BezPath, b: &BezPath) {
        for op in [
            BinaryOp::Union,
            BinaryOp::Intersection,
            BinaryOp::Difference,
            BinaryOp::Xor,
        ] {
            let out = combine(a, b, op);
            assert_grid(a, b, &out, FillRule::NonZero, FillRule::NonZero, op);
        }
    }
    #[test]
    fn four_rectangle_arrangement_is_a_complete_disjoint_partition() {
        let mut inputs = [
            rectangle(0., 0., 10., 10.),
            rectangle(4., 0., 10., 10.),
            rectangle(0., 4., 10., 10.),
            rectangle(4., 4., 10., 10.),
        ];
        for _ in 0..4 {
            let fold = |indices: Vec<usize>, op| {
                let mut iter = indices.into_iter();
                let mut result = inputs[iter.next().unwrap()].clone();
                for index in iter {
                    result = combine(&result, &inputs[index], op);
                }
                result
            };
            let mut cells = Vec::new();
            // This is Curvex Divide's complete subset arrangement: intersect each
            // subset, then subtract the union of every input outside that subset.
            for mask in 1..16 {
                let included = (0..4).filter(|i| mask & (1 << i) != 0).collect();
                let excluded: Vec<_> = (0..4).filter(|i| mask & (1 << i) == 0).collect();
                let intersection = fold(included, BinaryOp::Intersection);
                let cell = if excluded.is_empty() {
                    intersection
                } else {
                    combine(
                        &intersection,
                        &fold(excluded, BinaryOp::Union),
                        BinaryOp::Difference,
                    )
                };
                cells.push(cell);
            }
            assert!((cells.iter().map(Shape::area).sum::<f64>() - 196.).abs() < 1e-8);
            assert!((cells[14].area() - 36.).abs() < 1e-8);
            for yi in -1..15 {
                for xi in -1..15 {
                    let point = Point::new(xi as f64 + 0.5, yi as f64 + 0.5);
                    let actual = cells.iter().filter(|c| c.winding(point) != 0).count();
                    let expected = usize::from((0..14).contains(&xi) && (0..14).contains(&yi));
                    assert_eq!(actual, expected, "partition coverage at {point:?}");
                }
            }
            inputs.rotate_left(1);
        }
    }

    #[test]
    fn almost_axis_aligned_edges_keep_crossings_on_both_axes() {
        for transpose in [false, true] {
            let polygon = |vertices: &[(f64, f64)]| {
                let mut path = BezPath::new();
                for (index, &(x, y)) in vertices.iter().enumerate() {
                    let point = if transpose {
                        Point::new(y, x)
                    } else {
                        Point::new(x, y)
                    };
                    if index == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
                path.close_path();
                path
            };
            // The tiny positive x overlap of the first shape's left edge and
            // the cutter's bottom edge still contains an interior crossing.
            let source = polygon(&[(4., 0.), (10., 0.), (10., 10.), (4. + 1e-9, 10.)]);
            let cutter = polygon(&[(0., 4.), (14., 4.), (14., 14.), (0., 14.)]);
            let result = combine(&source, &cutter, BinaryOp::Difference);
            assert!((result.area() - 24.).abs() < 1e-7);
            all_ops_grid(&source, &cutter);
        }
    }

    #[test]
    fn overlapping_ellipses_preserve_cubic_boundaries_and_region() {
        let a = ellipse(0., 0., 10., 7.);
        let b = ellipse(8., 2., 9., 6.);
        all_ops_grid(&a, &b);
        let union = combine(&a, &b, BinaryOp::Union);
        assert!(
            union
                .segments()
                .all(|s| matches!(s, CurveSegment::Cubic(..))),
            "only original cubic spans belong on the ellipse boundary"
        );
        assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
    }
    #[test]
    fn rounded_protrusion_preserves_original_controls() {
        let a = rectangle(0., 0., 10., 10.);
        let b = ellipse(10., 5., 4., 3.);
        let union = combine(&a, &b, BinaryOp::Union);
        all_ops_grid(&a, &b);
        let retained: Vec<_> = b
            .segments()
            .filter(|s| s.start().x >= 10. && s.end().x >= 10.)
            .collect();
        assert!(!retained.is_empty());
        for source in retained {
            assert!(
                union
                    .segments()
                    .any(|out| out == source || out == source.reverse()),
                "unaffected rounded arcs must retain their actual controls"
            );
        }
        assert!((union.bounding_box().max_x() - 14.).abs() < 1e-8);
    }
    #[test]
    fn contained_disjoint_equal_and_reversed_curves() {
        let outer = ellipse(0., 0., 10., 10.);
        for other in [
            ellipse(0., 0., 3., 2.),
            ellipse(30., 0., 3., 2.),
            outer.clone(),
            reverse(&outer),
        ] {
            all_ops_grid(&outer, &other);
        }
        let union = combine(&outer, &reverse(&outer), BinaryOp::Union);
        assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
        assert!((union.area().abs() - outer.area().abs()).abs() < 1e-7);
    }
    #[test]
    fn evenodd_compound_keeps_holes_and_nested_islands() {
        let mut a = ellipse(0., 0., 10., 10.);
        a.extend(ellipse(0., 0., 7., 7.).iter());
        a.extend(ellipse(0., 0., 2., 2.).iter());
        let b = rectangle(-11., -1., 22., 2.);
        for op in [
            BinaryOp::Union,
            BinaryOp::Intersection,
            BinaryOp::Difference,
            BinaryOp::Xor,
        ] {
            let out = binary_op(&a, &b, FillRule::EvenOdd, FillRule::NonZero, op, 0.05)
                .unwrap()
                .to_path();
            assert_grid(&a, &b, &out, FillRule::EvenOdd, FillRule::NonZero, op);
        }
    }
    #[test]
    fn self_intersecting_cubic_normalizes_each_fill_rule() {
        let mut a = BezPath::new();
        a.move_to((0., 0.));
        a.curve_to((6., 10.), (-6., 10.), (3., 0.));
        a.close_path();
        let empty = BezPath::new();
        for rule in [FillRule::EvenOdd, FillRule::NonZero] {
            let out = binary_op_with_eps(&a, &empty, rule, BinaryOp::Union, 0.05)
                .unwrap()
                .to_path();
            assert_grid(&a, &empty, &out, rule, rule, BinaryOp::Union);
            assert!(out.segments().any(|s| matches!(s, CurveSegment::Cubic(..))));
        }
    }
    #[test]
    fn tangent_and_nearly_coincident_inputs_remain_closed() {
        let a = ellipse(0., 0., 10., 10.);
        for other in [
            ellipse(20., 0., 10., 10.),
            ellipse(19.999, 0., 10., 10.),
            ellipse(20.001, 0., 10., 10.),
            ellipse(0.001, 0., 10., 10.),
        ] {
            all_ops_grid(&a, &other);
        }
        all_ops_grid(&rectangle(0., 0., 10., 10.), &rectangle(10., 0., 10., 10.));
    }
    #[test]
    fn subdivided_coincident_cubic_spans_do_not_double_the_boundary() {
        let a = ellipse(0., 0., 10., 10.);
        let mut b = BezPath::new();
        b.move_to(a.segments().next().unwrap().start());
        for source in a.segments() {
            let (left, right) = source.split(0.317);
            b.push(left.to_path_el());
            b.push(right.to_path_el());
        }
        b.close_path();
        all_ops_grid(&a, &b);
        let union = combine(&a, &b, BinaryOp::Union);
        assert_eq!(union.to_bezier_paths().unwrap().len(), 1);
        assert!((union.area() - a.area().abs()).abs() < 1e-6);
    }
    #[test]
    fn partial_coincident_arc_preserves_the_remaining_regions() {
        let source = CurveSegment::Cubic(
            Point::new(0., 0.),
            Point::new(0., 10.),
            Point::new(10., 10.),
            Point::new(10., 0.),
        );
        let mut a = BezPath::new();
        a.move_to(source.start());
        a.push(source.to_path_el());
        a.close_path();
        let cut = source.subsegment(0.23, 0.81);
        let mut b = BezPath::new();
        b.move_to(cut.start());
        b.push(cut.to_path_el());
        b.close_path();
        all_ops_grid(&a, &b);
        all_ops_grid(&reverse(&a), &b);
    }
    #[test]
    fn stationary_cubic_midpoint_does_not_drop_a_valid_region() {
        // x=(t-.5)^2, y=(t-.5)^3 has a cusp where its derivative vanishes.
        let mut cusp = BezPath::new();
        cusp.move_to((0.25, -0.125));
        cusp.curve_to((-1. / 12., 0.125), (-1. / 12., -0.125), (0.25, 0.125));
        cusp.close_path();
        let empty = BezPath::new();
        let out = combine(&cusp, &empty, BinaryOp::Union);
        assert_grid(
            &cusp,
            &empty,
            &out,
            FillRule::NonZero,
            FillRule::NonZero,
            BinaryOp::Union,
        );
        assert!((out.area() - cusp.area().abs()).abs() < 1e-10);
        assert!(out.segments().any(|s| matches!(s, CurveSegment::Cubic(..))));
    }
    #[test]
    fn varied_curved_arrangements_match_independent_input_winding() {
        for i in 0..20 {
            let t = i as f64;
            let a = ellipse(
                t.sin() * 4.,
                t.cos() * 2.,
                2. + (t * 0.7).sin().abs() * 8.,
                1. + (t * 0.9).cos().abs() * 4.,
            );
            let b = ellipse(
                (t * 0.6).cos() * 7.,
                (t * 0.3).sin() * 3.,
                1. + (t * 0.13).cos().abs() * 5.,
                1. + (t * 0.41).sin().abs() * 6.,
            );
            all_ops_grid(&a, &b);
        }
    }
    #[test]
    fn cubic_equivalent_of_a_line_can_share_polygon_edges() {
        let a = rectangle(0., 0., 10., 10.);
        let mut b = BezPath::new();
        b.move_to((0., 0.));
        b.curve_to((0., 0.), (0., 0.), (10., 0.));
        b.line_to((10., 10.));
        b.line_to((0., 10.));
        b.close_path();
        all_ops_grid(&a, &b);
        assert_eq!(
            combine(&a, &b, BinaryOp::Union)
                .to_bezier_paths()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn shared_span_still_intersects_a_different_part_of_a_loop() {
        let curve = CurveSegment::Cubic(
            Point::new(0., 0.),
            Point::new(6., 10.),
            Point::new(-6., 10.),
            Point::new(3., 0.),
        );
        let mut a = BezPath::new();
        a.move_to(curve.start());
        a.push(curve.to_path_el());
        a.close_path();
        let partial = curve.subsegment(0., 0.2);
        let mut b = BezPath::new();
        b.move_to(partial.start());
        b.push(partial.to_path_el());
        b.close_path();
        all_ops_grid(&a, &b);
    }
    #[test]
    fn translated_geometry_beyond_former_coordinate_limit_is_preserved() {
        let a = ellipse(0., 0., 10., 7.);
        let b = ellipse(8., 2., 9., 6.);
        let shift = Point::new(1e7, -1e7);
        let translate = |path: &BezPath| {
            BezPath::from_vec(
                path.iter()
                    .map(|el| match el {
                        PathEl::MoveTo(p) => PathEl::MoveTo(p + shift),
                        PathEl::LineTo(p) => PathEl::LineTo(p + shift),
                        PathEl::QuadTo(a, b) => PathEl::QuadTo(a + shift, b + shift),
                        PathEl::CurveTo(a, b, c) => {
                            PathEl::CurveTo(a + shift, b + shift, c + shift)
                        }
                        PathEl::ClosePath => PathEl::ClosePath,
                    })
                    .collect(),
            )
        };
        let ta = translate(&a);
        let tb = translate(&b);
        for op in [
            BinaryOp::Union,
            BinaryOp::Intersection,
            BinaryOp::Difference,
            BinaryOp::Xor,
        ] {
            let original = combine(&a, &b, op);
            let translated = combine(&ta, &tb, op);
            assert!(
                (translated.area() - original.area()).abs() < 1e-4,
                "translated {op:?} changed area"
            );
            assert_grid(
                &ta,
                &tb,
                &translated,
                FillRule::NonZero,
                FillRule::NonZero,
                op,
            );
        }
    }
    #[test]
    fn five_thousand_simple_vertices_are_not_rejected_by_old_segment_cap() {
        let count = 5000;
        let mut path = BezPath::new();
        for i in 0..count {
            let angle = std::f64::consts::TAU * i as f64 / count as f64;
            let p = (100. * angle.cos(), 100. * angle.sin());
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close_path();
        let output = combine(&path, &BezPath::new(), BinaryOp::Union);
        assert_eq!(output.segments().count(), count);
        let expected = count as f64 * 0.5 * 10000. * (std::f64::consts::TAU / count as f64).sin();
        assert!((output.area() - expected).abs() < 1e-5);
        assert_eq!(output.winding(Point::ZERO), 1);
    }
}
