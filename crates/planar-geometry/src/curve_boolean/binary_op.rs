use super::*;

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
    pub(super) contours: Vec<Contour>,
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
pub(super) struct Chord {
    pub(super) source: usize,
    pub(super) t0: f64,
    pub(super) t1: f64,
    pub(super) a: Point,
    pub(super) b: Point,
}
#[derive(Clone, Copy)]
pub(super) struct Boundary {
    pub(super) curve: CurveSegment,
    pub(super) from: usize,
    pub(super) to: usize,
}
/// Repeated boundary-side queries share analytic bounds. A horizontal ray can
/// skip curves outside its y range or entirely to its left. For a curve wholly
/// to the right, its signed crossing count is determined just by its endpoints:
/// internal up/down crossings cancel in pairs, including tangent extrema.
pub(super) struct PreparedWinding {
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
    pub(super) fn new(curve: CurveSegment, bounds: Rect) -> Self {
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
    pub(super) fn winding(&self, point: Point) -> i32 {
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
    pub(super) fn new(curves: &[CurveSegment], bounds: &[Rect]) -> Self {
        Self {
            segments: curves
                .iter()
                .zip(bounds)
                .map(|(&c, &b)| RaySegment::new(c, b))
                .collect(),
        }
    }
    pub(super) fn winding(&self, point: Point) -> i32 {
        self.segments
            .iter()
            .map(|segment| segment.winding(point))
            .sum()
    }
}
pub(super) fn sub(a: Point, b: Point) -> [f64; 2] {
    [a.x - b.x, a.y - b.y]
}
pub(super) use math_core::{cross2 as cross, norm2 as len};
pub(super) fn at(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
pub(super) fn controls(c: CurveSegment) -> ([Point; 4], usize) {
    match c {
        CurveSegment::Line(a, b) => ([a, b, Point::ZERO, Point::ZERO], 2),
        CurveSegment::Quad(a, b, c) => ([a, b, c, Point::ZERO], 3),
        CurveSegment::Cubic(a, b, c, d) => ([a, b, c, d], 4),
    }
}
pub(super) fn flat(c: CurveSegment, tolerance: f64) -> bool {
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
pub(super) fn has_monotone_axis(curve: CurveSegment) -> bool {
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
pub(super) fn no_interior_intersection(a: CurveSegment, b: CurveSegment, aa: Rect, bb: Rect) -> bool {
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
pub(super) fn flatten(
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
