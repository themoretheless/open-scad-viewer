use super::*;

/// One path segment: straight line or cubic Bézier. Endpoints are absolute mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    Line {
        to: [f64; 2],
    },
    Cubic {
        c1: [f64; 2],
        c2: [f64; 2],
        to: [f64; 2],
    },
}

impl PathSegment {
    pub const fn end(&self) -> [f64; 2] {
        match *self {
            PathSegment::Line { to } | PathSegment::Cubic { to, .. } => to,
        }
    }
}

/// Editable cubic Bézier path. Anchor 0 is `start`; segment `i` runs from
/// anchor `i` to anchor `i+1`. Closed paths store an explicit closing segment.
#[derive(Debug, Clone, PartialEq)]
pub struct BezierPath {
    pub start: [f64; 2],
    pub segments: Vec<PathSegment>,
    pub closed: bool,
}

/// Default flatten tolerance in mm (chord deviation).
pub const FLATTEN_TOLERANCE: f64 = 0.25;
pub(super) const ELLIPSE_KAPPA: f64 = 0.552_285;

pub(super) use crate::{dist, lerp};

#[inline]
pub(super) fn perp_line_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let ab = sub2(b, a);
    let ap = sub2(p, a);
    let len = norm2(ab);
    if len < 1e-15 {
        return norm2(ap);
    }
    (cross2(ab, ap) / len).abs()
}
