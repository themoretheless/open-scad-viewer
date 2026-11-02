//! Prepared polyline measurements. Geometry-only, immutable and reusable across
//! samples; callers control lifetime instead of an implicit global cache.
use crate::{Result, check};

pub struct ArcLengthIndex {
    segments: Vec<Segment>,
    order: Vec<usize>,
    nodes: Vec<Node>,
    total: f64,
}
struct Segment {
    a: [f64; 2],
    b: [f64; 2],
    start: f64,
    end: f64,
}
struct Node {
    min: [f64; 2],
    max: [f64; 2],
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
impl ArcLengthIndex {
    pub fn new(points: &[[f64; 2]]) -> Result<Self> {
        check(points.len() >= 2, "Path too short for arc table")?;
        check(
            points.len() <= crate::limits::FLATTEN_POINTS,
            "Arc table exceeds point budget",
        )?;
        crate::rings::coordinate_metrics(points.iter())?;
        let mut total = 0.;
        let mut segments = Vec::with_capacity(points.len() - 1);
        for pair in points.windows(2) {
            let start = total;
            total += (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
            segments.push(Segment {
                a: pair[0],
                b: pair[1],
                start,
                end: total,
            });
        }
        check(total.is_finite(), "Non-finite arc length")?;
        let mut index = Self {
            order: (0..segments.len()).collect(),
            segments,
            nodes: vec![],
            total,
        };
        index.build(0, index.order.len());
        Ok(index)
    }
    pub fn total_length(&self) -> f64 {
        self.total
    }
    /// Distance along the nearest segment. Equal-distance ties choose the first
    /// segment in path order, preserving retracing and self-intersection colors.
    pub fn nearest_length(&self, p: [f64; 2]) -> Result<f64> {
        check(p.iter().all(|v| v.is_finite()), "Invalid arc query point")?;
        let mut best = (f64::INFINITY, usize::MAX, 0.);
        self.search(0, p, &mut best);
        check(best.0.is_finite(), "Arc query exceeds numeric range")?;
        Ok(best.2)
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for &i in &self.order[start..end] {
            for p in [self.segments[i].a, self.segments[i].b] {
                for k in 0..2 {
                    min[k] = min[k].min(p[k]);
                    max[k] = max[k].max(p[k]);
                }
            }
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            min,
            max,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let axis = usize::from(max[1] - min[1] > max[0] - min[0]);
            let mid = (start + end) / 2;
            self.order[start..end].select_nth_unstable_by(mid - start, |&a, &b| {
                let center = |i: usize| {
                    let s = &self.segments[i];
                    s.a[axis] + (s.b[axis] - s.a[axis]) * 0.5
                };
                center(a).total_cmp(&center(b)).then(a.cmp(&b))
            });
            let left = self.build(start, mid);
            let right = self.build(mid, end);
            self.nodes[id].children = Some([left, right]);
        }
        id
    }
    fn lower_bound(&self, id: usize, p: [f64; 2]) -> f64 {
        let n = &self.nodes[id];
        let d = |k: usize| (n.min[k] - p[k]).max(p[k] - n.max[k]).max(0.);
        // Subtract a rounding allowance: bounds may only underestimate distance,
        // especially when a nearest projection lands exactly on a shared end.
        let scale = n
            .min
            .iter()
            .chain(&n.max)
            .chain(&p)
            .fold(0_f64, |a, b| a.max(b.abs()));
        (d(0).hypot(d(1)) - scale * f64::EPSILON * 8.).max(0.)
    }
    fn search(&self, id: usize, p: [f64; 2], best: &mut (f64, usize, f64)) {
        if self.lower_bound(id, p) > best.0 {
            return;
        }
        let node = &self.nodes[id];
        if let Some([a, b]) = node.children {
            let order = if self.lower_bound(a, p) <= self.lower_bound(b, p) {
                [a, b]
            } else {
                [b, a]
            };
            for child in order {
                self.search(child, p, best)
            }
        } else {
            for &i in &self.order[node.start..node.end] {
                let s = &self.segments[i];
                let d = [s.b[0] - s.a[0], s.b[1] - s.a[1]];
                let ap = [p[0] - s.a[0], p[1] - s.a[1]];
                let l2 = d[0] * d[0] + d[1] * d[1];
                let t = if l2 < 1e-18 {
                    0.
                } else {
                    ((ap[0] * d[0] + ap[1] * d[1]) / l2).clamp(0., 1.)
                };
                let q = [s.a[0] + d[0] * t, s.a[1] + d[1] * t];
                let distance = (p[0] - q[0]).hypot(p[1] - q[1]);
                if distance < best.0 || (distance == best.0 && i < best.1) {
                    *best = (distance, i, s.start + (s.end - s.start) * t);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_projection_matches_exhaustive_search_and_retracing_ties() {
        let points: Vec<_> = (0..257)
            .map(|i| {
                let t = i as f64 * 0.17;
                [t.cos() * t, t.sin() * t]
            })
            .collect();
        let index = ArcLengthIndex::new(&points).unwrap();
        let mut exhaustive = ArcLengthIndex::new(&points).unwrap();
        exhaustive.nodes[0].children = None;
        for i in 0..1000 {
            let p = [(i % 37) as f64 * 3. - 50., (i % 41) as f64 * 2.5 - 50.];
            let mut reference = (f64::INFINITY, usize::MAX, 0.);
            // A single leaf is the exhaustive oracle, with the same tie contract.
            exhaustive.search(0, p, &mut reference);
            assert_eq!(index.nearest_length(p).unwrap(), reference.2);
        }
        let retrace = ArcLengthIndex::new(&[[0., 0.], [10., 0.], [0., 0.], [10., 0.]]).unwrap();
        assert_eq!(retrace.nearest_length([5., 1.]).unwrap(), 5.);
        assert_eq!(retrace.total_length(), 30.);
        assert!(retrace.nearest_length([f64::NAN, 0.]).is_err());
    }
}
