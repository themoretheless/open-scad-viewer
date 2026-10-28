//! Native feature targets shared by geometric kernels; no host references.
use crate::Point;
pub use math_core::{cross, sub};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Vertex,
    Midpoint,
    Center,
    BoundsCenter,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Midpoint => "midpoint",
            Self::Center => "center",
            Self::BoundsCenter => "bounds-center",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Target {
    pub point: Point,
    pub kind: Kind,
}
#[derive(Debug, Clone)]
pub struct Interval {
    pub curve: usize,
    pub start: f64,
    pub end: f64,
}
#[derive(Debug, Clone)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
    pub interval: Option<Interval>,
}
#[derive(Debug, Clone, Default)]
pub struct Geometry {
    pub points: Vec<Target>,
    pub segments: Vec<Segment>,
}
pub fn length(v: Point) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
pub fn dot(a: Point, b: Point) -> f64 {
    let mut sum = 0.;
    for i in 0..3 {
        sum += a[i] * b[i]
    }
    sum
}
impl Geometry {
    pub fn add(&mut self, point: Point, kind: Kind) {
        if point.iter().all(|v| v.is_finite())
            && !self
                .points
                .iter()
                .any(|p| p.kind == kind && length(math_core::sub(p.point, point)) < 1e-7)
        {
            self.points.push(Target { point, kind })
        }
    }
    pub fn bounds_center(&mut self, points: &[Point]) {
        if points.is_empty() {
            return;
        }
        let p = std::array::from_fn(|i| {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for p in points {
                min = min.min(p[i]);
                max = max.max(p[i]);
            }
            (min + max) / 2.
        });
        self.add(p, Kind::BoundsCenter)
    }
}
/// Legacy sampled circular-support hint; not an exact rational circle certificate.
pub fn sampled_circle_center(
    first: Point,
    middle: Point,
    last: Point,
    samples: &[Point],
) -> Option<Point> {
    let u = math_core::sub(middle, first);
    let v = math_core::sub(last, first);
    let n = math_core::cross(u, v);
    let n2 = dot(n, n);
    if n2 <= 1e-16 {
        return None;
    }
    let uu = dot(u, u);
    let vv = dot(v, v);
    let vn = math_core::cross(v, n);
    let nu = math_core::cross(n, u);
    let center = std::array::from_fn(|i| first[i] + (uu * vn[i] + vv * nu[i]) / (2. * n2));
    let r = length(math_core::sub(first, center));
    samples
        .iter()
        .all(|p| (length(math_core::sub(*p, center)) - r).abs() < 1e-7_f64.max(r * 1e-7))
        .then_some(center)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_deduplicate_by_kind_and_reject_nonfinite_points() {
        let mut g = Geometry::default();
        g.add([1., 2., 3.], Kind::Vertex);
        g.add([1. + 1e-8, 2., 3.], Kind::Vertex);
        g.add([1., 2., 3.], Kind::Midpoint);
        g.add([f64::NAN, 0., 0.], Kind::Center);
        assert_eq!(g.points.len(), 2);
    }
    #[test]
    fn circle_hint_rejects_non_circular_samples_and_collinear_points() {
        let a = [1., 0., 2.];
        let b = [0., 1., 2.];
        let c = [-1., 0., 2.];
        assert_eq!(
            sampled_circle_center(a, b, c, &[a, b, c]),
            Some([0., 0., 2.])
        );
        assert!(sampled_circle_center(a, b, c, &[a, b, c, [0., 2., 2.]]).is_none());
        assert!(sampled_circle_center(a, [0., 0., 2.], c, &[a, c]).is_none());
    }
}
