//! Private prepared winding strategies for arrangement side classification.
use super::{Rings, cross2, sub2};

#[derive(Clone, Copy)]
struct WindingEdge {
    a: [f64; 2],
    b: [f64; 2],
    operand: usize,
}

/// Interval index for horizontal ray queries. A typical smooth outline has
/// only two active edges at a queried height, so boundary classification should
/// not rescan thousands of unrelated source edges per atomic segment.
struct EdgeIndex {
    middle: f64,
    lower: Vec<WindingEdge>,
    upper: Vec<WindingEdge>,
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
}

impl EdgeIndex {
    fn new(a: &Rings, b: &Rings) -> Option<Box<Self>> {
        Self::build(
            [a, b]
                .into_iter()
                .enumerate()
                .flat_map(|(operand, rings)| {
                    rings.iter().flat_map(move |ring| {
                        (0..ring.len()).map(move |i| WindingEdge {
                            a: ring[i],
                            b: ring[(i + 1) % ring.len()],
                            operand,
                        })
                    })
                })
                .filter(|edge| edge.a[1] != edge.b[1])
                .collect(),
        )
    }

    fn build(edges: Vec<WindingEdge>) -> Option<Box<Self>> {
        if edges.is_empty() {
            return None;
        }
        // Split at a median lower endpoint, rather than an interpolated center:
        // the median interval necessarily spans this coordinate even if it is
        // only one ULP high. Both recursive sides contain at most half the edges.
        let mut starts: Vec<_> = edges.iter().map(|e| e.a[1].min(e.b[1])).collect();
        let median = starts.len() / 2;
        starts.select_nth_unstable_by(median, f64::total_cmp);
        Some(Box::new(Self::partition_at(edges, starts[median])))
    }

    fn partition_at(edges: Vec<WindingEdge>, middle: f64) -> Self {
        let (mut left, mut right, mut lower) = (Vec::new(), Vec::new(), Vec::new());
        for edge in edges {
            if edge.a[1].max(edge.b[1]) <= middle {
                left.push(edge);
            } else if edge.a[1].min(edge.b[1]) > middle {
                right.push(edge);
            } else {
                lower.push(edge);
            }
        }
        lower.sort_by(|a, b| a.a[1].min(a.b[1]).total_cmp(&b.a[1].min(b.b[1])));
        let mut upper = lower.clone();
        upper.sort_by(|a, b| b.a[1].max(b.b[1]).total_cmp(&a.a[1].max(a.b[1])));
        Self {
            middle,
            lower,
            upper,
            left: Self::build(left),
            right: Self::build(right),
        }
    }

    fn winding(&self, p: [f64; 2], output: &mut [i32; 2]) {
        let add = |edge: &WindingEdge, output: &mut [i32; 2]| {
            if edge.a[0].max(edge.b[0]) <= p[0] {
                return;
            }
            let cross = cross2(sub2(edge.b, edge.a), sub2(p, edge.a));
            if edge.b[1] > edge.a[1] && cross > 0. {
                output[edge.operand] += 1;
            }
            if edge.b[1] < edge.a[1] && cross < 0. {
                output[edge.operand] -= 1;
            }
        };
        if p[1] < self.middle {
            for edge in &self.lower {
                if edge.a[1].min(edge.b[1]) > p[1] {
                    break;
                }
                add(edge, output);
            }
            if let Some(left) = &self.left {
                left.winding(p, output);
            }
        } else {
            for edge in &self.upper {
                if edge.a[1].max(edge.b[1]) <= p[1] {
                    break;
                }
                add(edge, output);
            }
            if let Some(right) = &self.right {
                right.winding(p, output);
            }
        }
    }
}

/// Use closed-contour bounds for many small rings (stroke strips/joins), and
/// the edge interval index for a few long contours. Both compute signed counts;
/// fill rules and boolean selection remain the arrangement's responsibility.
pub(super) struct WindingIndex<'a>(Strategy<'a>);
enum Strategy<'a> {
    Edges(Option<Box<EdgeIndex>>),
    Contours(ContourNode<'a>),
}
impl<'a> WindingIndex<'a> {
    pub(super) fn new(a: &'a Rings, b: &'a Rings) -> Self {
        let count = a.len() + b.len();
        let edges: usize = a.iter().chain(b).map(Vec::len).sum();
        if count >= 8 && edges <= count * 64 {
            let contours: Vec<_> = [a, b]
                .into_iter()
                .enumerate()
                .flat_map(|(operand, rings)| {
                    rings
                        .iter()
                        .filter(|r| r.len() >= 3)
                        .map(move |ring| Contour {
                            bounds: Bounds::of(ring),
                            ring,
                            operand,
                        })
                })
                .collect();
            if let Some(first) = contours.first() {
                let bounds = contours
                    .iter()
                    .skip(1)
                    .fold(first.bounds, |b, r| b.union(r.bounds));
                let area = |b: Bounds| (b.max[0] - b.min[0]) * (b.max[1] - b.min[1]);
                let covered: f64 = contours.iter().map(|r| area(r.bounds)).sum();
                // Dense retracing/nesting defeats point-box pruning. Keep the
                // interval strategy when projected average overlap is high.
                if covered <= area(bounds) * 8. {
                    return Self(Strategy::Contours(ContourNode::build(contours)));
                }
            }
        }
        Self(Strategy::Edges(EdgeIndex::new(a, b)))
    }
    pub(super) fn winding(&self, p: [f64; 2], counts: &mut [i32; 2]) {
        match &self.0 {
            Strategy::Edges(Some(index)) => index.winding(p, counts),
            Strategy::Edges(None) => {}
            Strategy::Contours(index) => index.winding(p, counts),
        }
    }
}
#[derive(Clone, Copy)]
struct Bounds {
    min: [f64; 2],
    max: [f64; 2],
}
impl Bounds {
    fn of(ring: &[[f64; 2]]) -> Self {
        let mut bounds = Self {
            min: ring[0],
            max: ring[0],
        };
        for p in &ring[1..] {
            for k in 0..2 {
                bounds.min[k] = bounds.min[k].min(p[k]);
                bounds.max[k] = bounds.max[k].max(p[k]);
            }
        }
        bounds
    }
    fn contains(self, p: [f64; 2]) -> bool {
        (0..2).all(|k| p[k] >= self.min[k] && p[k] <= self.max[k])
    }
    fn union(self, other: Self) -> Self {
        Self {
            min: [self.min[0].min(other.min[0]), self.min[1].min(other.min[1])],
            max: [self.max[0].max(other.max[0]), self.max[1].max(other.max[1])],
        }
    }
}
struct Contour<'a> {
    ring: &'a [[f64; 2]],
    operand: usize,
    bounds: Bounds,
}
struct ContourNode<'a> {
    bounds: Bounds,
    contents: Contents<'a>,
}
enum Contents<'a> {
    Leaf(Vec<Contour<'a>>),
    Branch(Box<ContourNode<'a>>, Box<ContourNode<'a>>),
}
impl<'a> ContourNode<'a> {
    fn build(mut rings: Vec<Contour<'a>>) -> Self {
        let bounds = rings
            .iter()
            .skip(1)
            .fold(rings[0].bounds, |b, r| b.union(r.bounds));
        let contents = if rings.len() <= 4 {
            Contents::Leaf(rings)
        } else {
            let axis = usize::from(bounds.max[1] - bounds.min[1] > bounds.max[0] - bounds.min[0]);
            let middle = rings.len() / 2;
            rings.select_nth_unstable_by(middle, |a, b| {
                let center = |r: &Contour<'_>| {
                    r.bounds.min[axis] + (r.bounds.max[axis] - r.bounds.min[axis]) * 0.5
                };
                center(a).total_cmp(&center(b))
            });
            let right = rings.split_off(middle);
            Contents::Branch(Box::new(Self::build(rings)), Box::new(Self::build(right)))
        };
        Self { bounds, contents }
    }
    fn winding(&self, p: [f64; 2], counts: &mut [i32; 2]) {
        if !self.bounds.contains(p) {
            return;
        }
        match &self.contents {
            Contents::Branch(a, b) => {
                a.winding(p, counts);
                b.winding(p, counts);
            }
            Contents::Leaf(rings) => {
                for r in rings {
                    if !r.bounds.contains(p) {
                        continue;
                    }
                    // A closed ring contributes zero outside its bounds. Inside,
                    // use the original signed crossing predicate verbatim.
                    for i in 0..r.ring.len() {
                        let a = r.ring[i];
                        let b = r.ring[(i + 1) % r.ring.len()];
                        if a[1] <= p[1] && b[1] > p[1] {
                            if cross2(sub2(b, a), sub2(p, a)) > 0. {
                                counts[r.operand] += 1;
                            }
                        } else if a[1] > p[1] && b[1] <= p[1] && cross2(sub2(b, a), sub2(p, a)) < 0.
                        {
                            counts[r.operand] -= 1;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contour_index_matches_signed_exhaustive_winding_for_both_operands() {
        let mut a = vec![];
        let mut b = vec![];
        for i in 0..48 {
            let x = (i % 8) as f64 * 3.;
            let y = (i / 8) as f64 * 3.;
            let mut ring = if i % 5 == 0 {
                vec![[x, y], [x + 8., y + 8.], [x, y + 8.], [x + 8., y]]
            } else {
                vec![[x, y], [x + 8., y], [x + 8., y + 8.], [x, y + 8.]]
            };
            if i % 3 == 0 {
                ring.reverse();
            }
            if i % 2 == 0 {
                a.push(ring)
            } else {
                b.push(ring)
            }
        }
        let index = WindingIndex::new(&a, &b);
        assert!(matches!(index.0, Strategy::Contours(_)));
        for x in -3..65 {
            for y in -3..50 {
                let point = [x as f64 * 0.5, y as f64 * 0.5];
                let mut counts = [0, 0];
                index.winding(point, &mut counts);
                assert_eq!(
                    counts,
                    [
                        super::super::winding(point, &a),
                        super::super::winding(point, &b)
                    ],
                    "{point:?}"
                );
            }
        }
        let dense = vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]; 32];
        let none = vec![];
        let fallback = WindingIndex::new(&dense, &none);
        assert!(matches!(fallback.0, Strategy::Edges(_)));
        let mut counts = [0, 0];
        fallback.winding([5., 5.], &mut counts);
        assert_eq!(counts, [32, 0]);
        let rings = vec![];
        let empty = WindingIndex::new(&rings, &rings);
        let mut counts = [0, 0];
        empty.winding([0., 0.], &mut counts);
        assert_eq!(counts, [0, 0]);
    }
}
