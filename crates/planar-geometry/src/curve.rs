//! Compound Bézier paths and analytic curve queries.
//!
//! The familiar `Point`, `PathEl`, `BezPath` and `Shape` names form a narrow
//! migration boundary for vector-editor clients. This module is implemented
//! independently; it does not depend on or re-export another geometry engine.
//! Geometry stays as original polynomial curves until a consumer explicitly
//! requests flattening, stroking or tessellation.

use crate::path::{BezierPath, PathSegment};
use crate::{Result, check};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
    pub fn distance_squared(self, other: Self) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}
impl From<[f64; 2]> for Point {
    fn from(p: [f64; 2]) -> Self {
        Self::new(p[0], p[1])
    }
}
impl From<(f64, f64)> for Point {
    fn from(p: (f64, f64)) -> Self {
        Self::new(p.0, p.1)
    }
}
impl From<Point> for [f64; 2] {
    fn from(p: Point) -> Self {
        [p.x, p.y]
    }
}
impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl std::ops::Mul<f64> for Point {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}
impl std::ops::Div<f64> for Point {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
impl Rect {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self { x0, y0, x1, y1 }
    }
    pub fn min_x(self) -> f64 {
        self.x0
    }
    pub fn min_y(self) -> f64 {
        self.y0
    }
    pub fn max_x(self) -> f64 {
        self.x1
    }
    pub fn max_y(self) -> f64 {
        self.y1
    }
    pub fn width(self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(self) -> f64 {
        self.y1 - self.y0
    }
    pub fn area(self) -> f64 {
        self.width() * self.height()
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x0 && p.x <= self.x1 && p.y >= self.y0 && p.y <= self.y1
    }
    pub fn union(self, other: Self) -> Self {
        Self::new(
            self.x0.min(other.x0),
            self.y0.min(other.y0),
            self.x1.max(other.x1),
            self.y1.max(other.y1),
        )
    }
    fn include(&mut self, p: Point) {
        self.x0 = self.x0.min(p.x);
        self.y0 = self.y0.min(p.y);
        self.x1 = self.x1.max(p.x);
        self.y1 = self.y1.max(p.y);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathEl {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CurveTo(Point, Point, Point),
    ClosePath,
}

/// A polynomial segment with explicit start and end, suitable for preserving
/// source provenance through boolean splitting. Subdivision is de Casteljau.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveSegment {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

impl CurveSegment {
    pub fn start(self) -> Point {
        match self {
            Self::Line(a, _) | Self::Quad(a, _, _) | Self::Cubic(a, _, _, _) => a,
        }
    }
    pub fn end(self) -> Point {
        match self {
            Self::Line(_, b) | Self::Quad(_, _, b) | Self::Cubic(_, _, _, b) => b,
        }
    }
    pub fn eval(self, t: f64) -> Point {
        match self {
            Self::Line(a, b) => a.lerp(b, t),
            Self::Quad(a, b, c) => a.lerp(b, t).lerp(b.lerp(c, t), t),
            Self::Cubic(a, b, c, d) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let cd = c.lerp(d, t);
                ab.lerp(bc, t).lerp(bc.lerp(cd, t), t)
            }
        }
    }
    pub fn evaluate(self, t: f64) -> Point {
        self.eval(t)
    }
    pub fn derivative(self, t: f64) -> Point {
        match self {
            Self::Line(a, b) => b - a,
            Self::Quad(a, b, c) => (b - a).lerp(c - b, t) * 2.0,
            Self::Cubic(a, b, c, d) => {
                Self::Quad((b - a) * 3.0, (c - b) * 3.0, (d - c) * 3.0).eval(t)
            }
        }
    }
    pub fn tangent(self, t: f64) -> Point {
        self.derivative(t)
    }
    pub fn split(self, t: f64) -> (Self, Self) {
        match self {
            Self::Line(a, b) => {
                let m = a.lerp(b, t);
                (Self::Line(a, m), Self::Line(m, b))
            }
            Self::Quad(a, b, c) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let m = ab.lerp(bc, t);
                (Self::Quad(a, ab, m), Self::Quad(m, bc, c))
            }
            Self::Cubic(a, b, c, d) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let cd = c.lerp(d, t);
                let abc = ab.lerp(bc, t);
                let bcd = bc.lerp(cd, t);
                let m = abc.lerp(bcd, t);
                (Self::Cubic(a, ab, abc, m), Self::Cubic(m, bcd, cd, d))
            }
        }
    }
    /// Restrict to a parameter interval; a decreasing interval reverses it.
    /// Parameters are expected to be finite and in 0..=1.
    pub fn subsegment(self, t0: f64, t1: f64) -> Self {
        if t0 == 0.0 && t1 == 1.0 {
            return self;
        }
        if t1 < t0 {
            return self.subsegment(t1, t0).reverse();
        }
        if t0 == 0.0 {
            return self.split(t1).0;
        }
        if t1 == 1.0 {
            return self.split(t0).1;
        }
        if t0 == t1 {
            let p = self.eval(t0);
            return match self {
                Self::Line(..) => Self::Line(p, p),
                Self::Quad(..) => Self::Quad(p, p, p),
                Self::Cubic(..) => Self::Cubic(p, p, p, p),
            };
        }
        self.split(t1).0.split(t0 / t1).1
    }
    pub fn reverse(self) -> Self {
        match self {
            Self::Line(a, b) => Self::Line(b, a),
            Self::Quad(a, b, c) => Self::Quad(c, b, a),
            Self::Cubic(a, b, c, d) => Self::Cubic(d, c, b, a),
        }
    }
    pub fn to_path_el(self) -> PathEl {
        match self {
            Self::Line(_, b) => PathEl::LineTo(b),
            Self::Quad(_, b, c) => PathEl::QuadTo(b, c),
            Self::Cubic(_, b, c, d) => PathEl::CurveTo(b, c, d),
        }
    }
    /// Polynomial coefficients in increasing power order.
    fn coefficients(self) -> ([Point; 4], usize) {
        match self {
            Self::Line(a, b) => ([a, b - a, Point::ZERO, Point::ZERO], 1),
            Self::Quad(a, b, c) => ([a, (b - a) * 2.0, a - b * 2.0 + c, Point::ZERO], 2),
            Self::Cubic(a, b, c, d) => (
                [
                    a,
                    (b - a) * 3.0,
                    (a - b * 2.0 + c) * 3.0,
                    d - a + (b - c) * 3.0,
                ],
                3,
            ),
        }
    }
    /// Analytic 1/2 integral of x dy - y dx. Summing a closed contour gives
    /// signed area; no flatten tolerance is involved.
    pub fn signed_area(self) -> f64 {
        let (c, n) = self.coefficients();
        let mut integral = 0.0;
        for i in 0..=n {
            for j in 1..=n {
                integral += (c[i].x * c[j].y - c[i].y * c[j].x) * (j as f64) / ((i + j) as f64);
            }
        }
        integral * 0.5
    }
    fn extrema(self, axis: usize) -> ([f64; 2], usize) {
        let (c, n) = self.coefficients();
        let v = |p: Point| if axis == 0 { p.x } else { p.y };
        if n == 1 {
            return ([0.0; 2], 0);
        }
        quadratic_roots(3.0 * v(c[3]), 2.0 * v(c[2]), v(c[1]))
    }
    pub fn bounding_box(self) -> Rect {
        let p = self.start();
        let mut bounds = Rect::new(p.x, p.y, p.x, p.y);
        bounds.include(self.end());
        for axis in 0..2 {
            let (roots, n) = self.extrema(axis);
            for &t in &roots[..n] {
                if t > 0.0 && t < 1.0 {
                    bounds.include(self.eval(t));
                }
            }
        }
        bounds
    }
    /// Signed crossings of a horizontal ray. Y-extrema divide the curve into
    /// monotone intervals; half-open intervals prevent double counting shared
    /// vertices and tangent extrema. Root refinement allocates no polygons.
    pub fn winding(self, point: Point) -> i32 {
        if !point.is_finite() {
            return 0;
        }
        // Polygon inputs dominate interactive boolean and hit-test workloads.
        // Their ray intersections are exact linear solves, not root searches.
        if let Self::Line(a, b) = self {
            let direction = if a.y <= point.y && point.y < b.y {
                1
            } else if b.y <= point.y && point.y < a.y {
                -1
            } else {
                return 0;
            };
            let x = a.x + (b.x - a.x) * ((point.y - a.y) / (b.y - a.y));
            return if x > point.x { direction } else { 0 };
        }
        let (roots, n) = self.extrema(1);
        let mut cuts = [0.0, 1.0, 1.0, 1.0];
        let mut count = 1;
        for &t in &roots[..n] {
            if t > 0.0 && t < 1.0 {
                cuts[count] = t;
                count += 1;
            }
        }
        cuts[count] = 1.0;
        count += 1;
        cuts[..count].sort_by(f64::total_cmp);
        let mut winding = 0;
        for pair in cuts[..count].windows(2) {
            let y0 = self.eval(pair[0]).y;
            let y1 = self.eval(pair[1]).y;
            let direction = if y0 <= point.y && point.y < y1 {
                1
            } else if y1 <= point.y && point.y < y0 {
                -1
            } else {
                continue;
            };
            let mut lo = pair[0];
            let mut hi = pair[1];
            for _ in 0..56 {
                let mid = (lo + hi) * 0.5;
                if (self.eval(mid).y < point.y) == (direction > 0) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            if self.eval((lo + hi) * 0.5).x > point.x {
                winding += direction;
            }
        }
        winding
    }
}

/// Stable real roots after coefficient scaling, including linear degeneration.
fn quadratic_roots(a: f64, b: f64, c: f64) -> ([f64; 2], usize) {
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 || !scale.is_finite() {
        return ([0.0; 2], 0);
    }
    let (a, b, c) = (a / scale, b / scale, c / scale);
    if a.abs() <= f64::EPSILON * b.abs() {
        return if b == 0.0 {
            ([0.0; 2], 0)
        } else {
            ([-c / b, 0.0], 1)
        };
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return ([0.0; 2], 0);
    }
    if discriminant == 0.0 {
        return ([-b / (2.0 * a), 0.0], 1);
    }
    let q = -0.5 * (b + discriminant.sqrt().copysign(b));
    ([q / a, c / q], 2)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BezPath {
    elements: Vec<PathEl>,
}
impl BezPath {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_vec(elements: Vec<PathEl>) -> Self {
        Self { elements }
    }
    pub fn elements(&self) -> &[PathEl] {
        &self.elements
    }
    pub fn elements_mut(&mut self) -> &mut [PathEl] {
        &mut self.elements
    }
    pub fn iter(&self) -> std::iter::Copied<std::slice::Iter<'_, PathEl>> {
        self.elements.iter().copied()
    }
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
    pub fn len(&self) -> usize {
        self.elements.len()
    }
    pub fn push(&mut self, el: PathEl) {
        self.elements.push(el);
    }
    pub fn move_to(&mut self, p: impl Into<Point>) {
        self.push(PathEl::MoveTo(p.into()));
    }
    pub fn line_to(&mut self, p: impl Into<Point>) {
        self.push(PathEl::LineTo(p.into()));
    }
    pub fn quad_to(&mut self, p: impl Into<Point>, to: impl Into<Point>) {
        self.push(PathEl::QuadTo(p.into(), to.into()));
    }
    pub fn curve_to(&mut self, a: impl Into<Point>, b: impl Into<Point>, to: impl Into<Point>) {
        self.push(PathEl::CurveTo(a.into(), b.into(), to.into()));
    }
    pub fn close_path(&mut self) {
        self.push(PathEl::ClosePath);
    }
    /// Reverse each subpath independently, preserving its curve degree,
    /// closure state and the order of the subpaths in the compound.
    pub fn reverse_subpaths(&self) -> Self {
        let mut out = Self::new();
        let mut subpath = Self::new();
        let append = |out: &mut Self, path: &Self| {
            let segments: Vec<_> = path.segments().collect();
            if let Some(last) = segments.last() {
                out.move_to(last.end());
                for segment in segments.into_iter().rev() {
                    out.push(segment.reverse().to_path_el());
                }
                if path
                    .elements
                    .iter()
                    .any(|el| matches!(el, PathEl::ClosePath))
                {
                    out.close_path();
                }
            } else {
                out.extend(path.iter());
            }
        };
        for el in self.iter() {
            if matches!(el, PathEl::MoveTo(_)) && !subpath.is_empty() {
                append(&mut out, &subpath);
                subpath = Self::new();
            }
            subpath.push(el);
        }
        append(&mut out, &subpath);
        out
    }
    pub fn segments(&self) -> impl Iterator<Item = CurveSegment> + '_ {
        self.indexed_segments().map(|(_, s)| s)
    }
    /// IDs are positions in `elements()`, so provenance survives iterator filters.
    pub fn indexed_segments(&self) -> PathSegments<'_> {
        PathSegments {
            elements: self.elements.iter().enumerate(),
            current: None,
            start: None,
        }
    }
    pub fn to_bezier_paths(&self) -> Result<Vec<BezierPath>> {
        let mut out = Vec::new();
        let mut start = None;
        let mut current = Point::ZERO;
        let mut segments = Vec::new();
        let mut closed = false;
        let flush = |out: &mut Vec<BezierPath>,
                     start: &mut Option<Point>,
                     segments: &mut Vec<PathSegment>,
                     closed: bool|
         -> Result<()> {
            if let Some(p) = start.take() {
                check(p.is_finite(), "Non-finite curve start")?;
                if !segments.is_empty() {
                    out.push(if closed {
                        BezierPath::closed(p.into(), std::mem::take(segments))?
                    } else {
                        BezierPath::open(p.into(), std::mem::take(segments))?
                    });
                }
            }
            Ok(())
        };
        for el in self.iter() {
            match el {
                PathEl::MoveTo(p) => {
                    flush(&mut out, &mut start, &mut segments, closed)?;
                    start = Some(p);
                    current = p;
                    closed = false;
                }
                PathEl::LineTo(p) => {
                    check(start.is_some(), "Path segment precedes MoveTo")?;
                    segments.push(PathSegment::Line { to: p.into() });
                    current = p;
                }
                PathEl::QuadTo(p, to) => {
                    check(start.is_some(), "Path segment precedes MoveTo")?;
                    segments.push(PathSegment::Cubic {
                        c1: current.lerp(p, 2.0 / 3.0).into(),
                        c2: to.lerp(p, 2.0 / 3.0).into(),
                        to: to.into(),
                    });
                    current = to;
                }
                PathEl::CurveTo(c1, c2, to) => {
                    check(start.is_some(), "Path segment precedes MoveTo")?;
                    segments.push(PathSegment::Cubic {
                        c1: c1.into(),
                        c2: c2.into(),
                        to: to.into(),
                    });
                    current = to;
                }
                PathEl::ClosePath => {
                    if let Some(p) = start {
                        if current != p {
                            segments.push(PathSegment::Line { to: p.into() });
                        }
                        current = p;
                        closed = true;
                    }
                }
            }
        }
        flush(&mut out, &mut start, &mut segments, closed)?;
        Ok(out)
    }
    pub fn from_bezier_paths(paths: &[BezierPath]) -> Self {
        let mut out = Self::new();
        for path in paths {
            out.move_to(path.start);
            for segment in &path.segments {
                match *segment {
                    PathSegment::Line { to } => out.line_to(to),
                    PathSegment::Cubic { c1, c2, to } => out.curve_to(c1, c2, to),
                }
            }
            if path.closed {
                out.close_path();
            }
        }
        out
    }
}
impl Extend<PathEl> for BezPath {
    fn extend<T: IntoIterator<Item = PathEl>>(&mut self, iter: T) {
        self.elements.extend(iter);
    }
}
impl<'a> Extend<&'a PathEl> for BezPath {
    fn extend<T: IntoIterator<Item = &'a PathEl>>(&mut self, iter: T) {
        self.elements.extend(iter.into_iter().copied());
    }
}
impl FromIterator<PathEl> for BezPath {
    fn from_iter<T: IntoIterator<Item = PathEl>>(iter: T) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

pub struct PathSegments<'a> {
    elements: std::iter::Enumerate<std::slice::Iter<'a, PathEl>>,
    current: Option<Point>,
    start: Option<Point>,
}
impl Iterator for PathSegments<'_> {
    type Item = (usize, CurveSegment);
    fn next(&mut self) -> Option<Self::Item> {
        for (index, &el) in self.elements.by_ref() {
            let segment = match el {
                PathEl::MoveTo(p) => {
                    self.current = Some(p);
                    self.start = Some(p);
                    continue;
                }
                PathEl::LineTo(to) => {
                    let from = self.current?;
                    self.current = Some(to);
                    CurveSegment::Line(from, to)
                }
                PathEl::QuadTo(control, to) => {
                    let from = self.current?;
                    self.current = Some(to);
                    CurveSegment::Quad(from, control, to)
                }
                PathEl::CurveTo(c1, c2, to) => {
                    let from = self.current?;
                    self.current = Some(to);
                    CurveSegment::Cubic(from, c1, c2, to)
                }
                PathEl::ClosePath => {
                    let from = self.current?;
                    let to = self.start?;
                    self.current = Some(to);
                    if from == to {
                        continue;
                    }
                    CurveSegment::Line(from, to)
                }
            };
            return Some((index, segment));
        }
        None
    }
}

pub trait Shape {
    /// Signed Green area of the path's explicit segments and closures.
    fn area(&self) -> f64;
    fn bounding_box(&self) -> Rect;
    /// Signed winding; callers representing a region must close their contours.
    fn winding(&self, point: Point) -> i32;
}
impl Shape for BezPath {
    fn area(&self) -> f64 {
        // Translate to an existing point to avoid catastrophic cancellation in
        // the common case of a small closed figure at a large document offset.
        let origin = self
            .elements
            .iter()
            .find_map(|el| match *el {
                PathEl::MoveTo(p) => Some(p),
                _ => None,
            })
            .unwrap_or(Point::ZERO);
        let mut area = 0.0;
        let mut boundary = Point::ZERO;
        for s in self.segments() {
            let translated = match s {
                CurveSegment::Line(a, b) => CurveSegment::Line(a - origin, b - origin),
                CurveSegment::Quad(a, b, c) => {
                    CurveSegment::Quad(a - origin, b - origin, c - origin)
                }
                CurveSegment::Cubic(a, b, c, d) => {
                    CurveSegment::Cubic(a - origin, b - origin, c - origin, d - origin)
                }
            };
            area += translated.signed_area();
            boundary = boundary + (s.end() - s.start());
        }
        // Apply the open-curve translation term once. On closed paths it is
        // zero; adding huge cancelling terms per segment would lose precision.
        area + 0.5 * (origin.x * boundary.y - origin.y * boundary.x)
    }
    fn bounding_box(&self) -> Rect {
        let mut bounds: Option<Rect> = None;
        for segment in self.segments() {
            bounds =
                Some(bounds.map_or(segment.bounding_box(), |b| b.union(segment.bounding_box())));
        }
        if let Some(bounds) = bounds {
            return bounds;
        }
        self.elements
            .iter()
            .find_map(|el| match *el {
                PathEl::MoveTo(p) => Some(Rect::new(p.x, p.y, p.x, p.y)),
                _ => None,
            })
            .unwrap_or(Rect::ZERO)
    }
    fn winding(&self, point: Point) -> i32 {
        self.segments().map(|s| s.winding(point)).sum()
    }
}

pub use crate::stroke::{LineCap as Cap, LineJoin as Join};

/// The subset of editable-stroke style requested by Curvex.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    options: crate::stroke::StrokeOptions,
}
impl Stroke {
    pub fn new(width: f64) -> Self {
        Self {
            options: crate::stroke::StrokeOptions {
                width,
                ..Default::default()
            },
        }
    }
    pub fn with_caps(mut self, cap: Cap) -> Self {
        self.options.cap = cap;
        self
    }
    pub fn with_join(mut self, join: Join) -> Self {
        self.options.join = join;
        self
    }
    pub fn with_miter_limit(mut self, limit: f64) -> Self {
        self.options.miter_limit = limit;
        self
    }
    pub fn with_dashes(mut self, offset: f64, dashes: impl IntoIterator<Item = f64>) -> Self {
        self.options.dash = Some(dashes.into_iter().collect());
        self.options.dash_offset = offset;
        self
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct StrokeOpts {}

/// Compatibility entry point. Invalid geometry yields an empty path, matching
/// consumers that treat an empty stroke as a skipped operation. New code can
/// call `stroke_checked` to retain the kernel's explicit error information.
pub fn stroke(
    path: impl IntoIterator<Item = PathEl>,
    style: &Stroke,
    _opts: &StrokeOpts,
    tolerance: f64,
) -> BezPath {
    stroke_checked(path, style, tolerance).unwrap_or_default()
}
pub fn stroke_checked(
    path: impl IntoIterator<Item = PathEl>,
    style: &Stroke,
    tolerance: f64,
) -> Result<BezPath> {
    let input: BezPath = path.into_iter().collect();
    let mut out = Vec::new();
    for path in input.to_bezier_paths()? {
        out.extend(crate::stroke::outline_stroke_tol(
            &path,
            &style.options,
            tolerance,
        )?);
    }
    Ok(BezPath::from_bezier_paths(&out))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-10, "{a} != {b}");
    }
    fn square(x: f64, y: f64, side: f64) -> BezPath {
        let mut path = BezPath::new();
        path.move_to((x, y));
        path.line_to((x + side, y));
        path.line_to((x + side, y + side));
        path.line_to((x, y + side));
        path.close_path();
        path
    }
    #[test]
    fn polynomial_area_and_bounds_are_analytic() {
        let mut path = BezPath::new();
        path.move_to((0., 0.));
        path.curve_to((0., 2.), (2., 2.), (2., 0.));
        path.close_path();
        near(path.area(), -2.4);
        near(path.bounding_box().max_y(), 1.5);
        assert_eq!(path.winding(p(1., 1.)), -1);
        assert_eq!(path.winding(p(1., 1.6)), 0);
    }
    #[test]
    fn quadratic_promotion_preserves_the_curve() {
        let mut path = BezPath::new();
        path.move_to((0., 0.));
        path.quad_to((3., 6.), (9., 0.));
        let native = path.to_bezier_paths().unwrap();
        let rebuilt = BezPath::from_bezier_paths(&native);
        let a = path.segments().next().unwrap();
        let b = rebuilt.segments().next().unwrap();
        for t in [0., 0.13, 0.5, 0.88, 1.] {
            near(a.eval(t).distance(b.eval(t)), 0.);
        }
        near(path.bounding_box().max_y(), 3.);
    }
    #[test]
    fn exact_subdivision_and_reversal() {
        let curve = CurveSegment::Cubic(p(-2., 1.), p(4., 8.), p(-3., -6.), p(5., 2.));
        let cut = curve.subsegment(0.2, 0.73);
        let reverse = curve.subsegment(0.73, 0.2);
        for t in [0., 0.07, 0.25, 0.5, 0.9, 1.] {
            near(cut.eval(t).distance(curve.eval(0.2 + 0.53 * t)), 0.);
            near(reverse.eval(t).distance(cut.eval(1. - t)), 0.);
        }
        let (a, b) = curve.split(0.37);
        near(a.signed_area() + b.signed_area(), curve.signed_area());
    }
    #[test]
    fn signed_winding_distinguishes_holes_and_double_winding() {
        let outer = square(0., 0., 10.);
        let inner = square(2., 2., 6.);
        let mut same = outer.clone();
        same.extend(inner.iter());
        assert_eq!(same.winding(p(5., 5.)), 2);
        near(same.area(), 136.);
        let native = inner.to_bezier_paths().unwrap();
        let reversed: Vec<_> = native.iter().map(BezierPath::reverse).collect();
        let mut hole = outer;
        hole.extend(BezPath::from_bezier_paths(&reversed).iter());
        assert_eq!(hole.winding(p(5., 5.)), 0);
        assert_eq!(hole.winding(p(1., 5.)), 1);
        near(hole.area(), 64.);
    }
    #[test]
    fn closed_area_remains_accurate_far_from_origin() {
        near(square(1e12, -1e12, 2.).area(), 4.);
    }
    #[test]
    fn tangencies_and_shared_vertices_do_not_create_crossings() {
        let mut path = BezPath::new();
        path.move_to((0., 0.));
        path.curve_to((0., 2.), (2., 2.), (2., 0.));
        path.close_path();
        assert_eq!(path.winding(p(-1., 1.5)), 0);
        assert_eq!(path.winding(p(1., 1.5)), 0);
        assert_eq!(square(0., 0., 2.).winding(p(-1., 0.)), 0);
        assert_eq!(square(0., 0., 2.).winding(p(1., 0.)), 1);
        assert_eq!(square(0., 0., 2.).winding(p(1., 2.)), 0);
    }
    #[test]
    fn cubic_with_two_y_extrema_uses_every_monotone_piece() {
        let curve = CurveSegment::Cubic(p(0., 0.), p(1., 3.), p(2., -3.), p(3., 0.));
        assert_eq!(curve.winding(p(-1., 0.)), 0);
        assert_eq!(curve.winding(p(1., 0.)), -1);
        assert_eq!(curve.winding(p(2., 0.)), 0);
        let bb = curve.bounding_box();
        near(bb.max_y(), 3.0_f64.sqrt() / 2.0);
        near(bb.min_y(), -3.0_f64.sqrt() / 2.0);
    }
    #[test]
    fn compounds_convert_without_losing_closures_or_curves() {
        let mut path = square(0., 0., 2.);
        path.move_to((4., 0.));
        path.curve_to((5., 2.), (6., 2.), (7., 0.));
        let contours = path.to_bezier_paths().unwrap();
        assert_eq!(contours.len(), 2);
        assert!(contours[0].closed);
        assert!(!contours[1].closed);
        let rebuilt = BezPath::from_bezier_paths(&contours);
        near(path.area(), rebuilt.area());
        assert!(matches!(
            rebuilt.segments().last(),
            Some(CurveSegment::Cubic(..))
        ));
        assert_eq!(
            path.indexed_segments().map(|(i, _)| i).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 6]
        );
    }
    #[test]
    fn invalid_geometry_is_an_explicit_conversion_error() {
        let mut path = BezPath::new();
        path.move_to((f64::NAN, 0.));
        path.line_to((1., 1.));
        assert!(path.to_bezier_paths().is_err());
        let mut path = BezPath::new();
        path.move_to((0., 0.));
        path.curve_to((1., 1.), (2., f64::INFINITY), (3., 0.));
        assert!(path.to_bezier_paths().is_err());
    }
    #[test]
    fn stroke_uses_native_width_caps_and_contour_holes() {
        let path = square(0., 0., 10.);
        let out = stroke_checked(path.iter(), &Stroke::new(2.), 0.05).unwrap();
        assert_eq!(out.to_bezier_paths().unwrap().len(), 2);
        assert_eq!(out.winding(p(5., 5.)), 0);
        let mut line = BezPath::new();
        line.move_to((0., 0.));
        line.line_to((10., 0.));
        let cap =
            stroke_checked(line.iter(), &Stroke::new(2.).with_caps(Cap::Square), 0.05).unwrap();
        near(cap.bounding_box().min_x(), -1.);
        near(cap.bounding_box().max_x(), 11.);
    }
}
