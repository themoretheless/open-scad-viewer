use super::*;

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
