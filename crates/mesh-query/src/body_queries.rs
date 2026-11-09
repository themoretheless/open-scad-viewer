//! Exact narrow-phase triangle queries accelerated by binary64 BVHs.
//! Distances apply to the supplied meshes, never to unmeshed source surfaces.
use crate::distance_bvh::{DistanceBvh, DistanceTriangle, distance_triangles};
use crate::proximity::closest_triangle;
use crate::{Error, MeshView, Result};
use math_core::{cross, dot, norm, sub};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub type Point = [f64; 3];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClosestPoint {
    pub point: Point,
    pub distance: f64,
    pub triangle: usize,
    pub nodes_visited: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDistance {
    pub a: Point,
    pub b: Point,
    pub distance: f64,
    pub triangle_a: usize,
    pub triangle_b: usize,
    pub node_pairs_visited: usize,
}
#[derive(Clone, Copy)]
struct Candidate<'a> {
    lower_squared: f64,
    a: &'a DistanceBvh,
    b: &'a DistanceBvh,
}
impl PartialEq for Candidate<'_> {
    fn eq(&self, o: &Self) -> bool {
        self.lower_squared == o.lower_squared
    }
}
impl Eq for Candidate<'_> {}
impl PartialOrd for Candidate<'_> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Candidate<'_> {
    fn cmp(&self, o: &Self) -> Ordering {
        o.lower_squared.total_cmp(&self.lower_squared)
    }
}
fn invalid(message: &str) -> Error {
    Error::new("MESH_QUERY_INVALID_INPUT", message)
}
fn bound_squared(min: Point, max: Point, p: Point) -> f64 {
    (0..3)
        .map(|k| (min[k] - p[k]).max(p[k] - max[k]).max(0.).powi(2))
        .sum()
}
fn pair_bound_squared(a: &DistanceBvh, b: &DistanceBvh) -> f64 {
    (0..3)
        .map(|k| {
            (a.min[k] - b.max[k])
                .max(b.min[k] - a.max[k])
                .max(0.)
                .powi(2)
        })
        .sum()
}

pub fn closest_point_bvh(
    mesh: MeshView<'_>,
    query: Point,
    max_nodes: usize,
) -> Result<ClosestPoint> {
    if query.iter().any(|v| !v.is_finite()) || max_nodes == 0 {
        return Err(invalid(
            "Closest point requires a finite point and nonzero node budget",
        ));
    }
    let triangles = distance_triangles(mesh)?;
    if triangles.is_empty() {
        return Err(invalid("Closest point requires a nonempty body"));
    }
    let tree = DistanceBvh::build(triangles);
    let mut stack = vec![&tree];
    let mut best = f64::INFINITY;
    let mut result = None;
    let mut visited = 0;
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > max_nodes {
            return Err(Error::new(
                "MESH_QUERY_BUDGET_EXCEEDED",
                "Point-body BVH node budget exhausted",
            ));
        }
        if bound_squared(node.min, node.max, query) > best {
            continue;
        }
        if let Some(c) = &node.children {
            stack.push(&c[0]);
            stack.push(&c[1]);
        } else {
            for t in &node.triangles {
                let point = closest_triangle(query, t.p[0], t.p[1], t.p[2]);
                let d2 = dot(sub(query, point), sub(query, point));
                if d2 < best
                    || (d2 == best
                        && result.is_none_or(|r: ClosestPoint| t.source_index < r.triangle))
                {
                    best = d2;
                    result = Some(ClosestPoint {
                        point,
                        distance: d2.sqrt(),
                        triangle: t.source_index,
                        nodes_visited: visited,
                    });
                }
            }
        }
    }
    result
        .map(|mut r| {
            r.nodes_visited = visited;
            r
        })
        .ok_or_else(|| invalid("BVH did not contain a triangle"))
}

fn segment_segment(a: Point, b: Point, c: Point, d: Point) -> (Point, Point) {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    let aa = dot(u, u);
    let bb = dot(u, v);
    let cc = dot(v, v);
    let dd = dot(u, w);
    let ee = dot(v, w);
    let den = aa * cc - bb * bb;
    let mut s = 0.;
    let mut t = 0.;
    if aa == 0. && cc == 0. {
        return (a, c);
    }
    if aa == 0. {
        t = (ee / cc).clamp(0., 1.);
    } else if cc == 0. {
        s = (-dd / aa).clamp(0., 1.);
    } else {
        if den > 0. {
            s = ((bb * ee - cc * dd) / den).clamp(0., 1.);
        }
        t = (bb * s + ee) / cc;
        if !(0.0..=1.0).contains(&t) {
            t = t.clamp(0., 1.);
            s = ((bb * t - dd) / aa).clamp(0., 1.);
        }
    }
    (
        std::array::from_fn(|k| a[k] + s * u[k]),
        std::array::from_fn(|k| c[k] + t * v[k]),
    )
}
fn triangle_pair(a: &DistanceTriangle, b: &DistanceTriangle) -> (Point, Point, f64) {
    let mut best = (a.p[0], b.p[0], f64::INFINITY);
    let mut offer = |p: Point, q: Point| {
        let d = dot(sub(p, q), sub(p, q));
        if d < best.2 {
            best = (p, q, d);
        }
    };
    for p in a.p {
        offer(p, closest_triangle(p, b.p[0], b.p[1], b.p[2]));
    }
    for p in b.p {
        offer(closest_triangle(p, a.p[0], a.p[1], a.p[2]), p);
    }
    let ae = [(a.p[0], a.p[1]), (a.p[1], a.p[2]), (a.p[2], a.p[0])];
    let be = [(b.p[0], b.p[1]), (b.p[1], b.p[2]), (b.p[2], b.p[0])];
    for x in ae {
        for y in be {
            let (p, q) = segment_segment(x.0, x.1, y.0, y.1);
            offer(p, q);
        }
    }
    best
}

pub fn body_distance_bvh(
    a: MeshView<'_>,
    b: MeshView<'_>,
    max_node_pairs: usize,
) -> Result<BodyDistance> {
    if max_node_pairs == 0 {
        return Err(invalid("Body distance requires a nonzero pair budget"));
    }
    let ta = distance_triangles(a)?;
    let tb = distance_triangles(b)?;
    if ta.is_empty() || tb.is_empty() {
        return Err(invalid("Body distance requires nonempty bodies"));
    }
    let a = DistanceBvh::build(ta);
    let b = DistanceBvh::build(tb);
    let mut heap = BinaryHeap::new();
    heap.push(Candidate {
        lower_squared: 0.,
        a: &a,
        b: &b,
    });
    let mut best = f64::INFINITY;
    let mut answer = None;
    let mut visited = 0;
    while let Some(Candidate {
        lower_squared,
        a,
        b,
    }) = heap.pop()
    {
        if lower_squared > best {
            continue;
        }
        visited += 1;
        if visited > max_node_pairs {
            return Err(Error::new(
                "MESH_QUERY_BUDGET_EXCEEDED",
                "Body-body BVH pair budget exhausted",
            ));
        }
        match (&a.children, &b.children) {
            (Some(ac), Some(bc)) => {
                for x in ac.iter() {
                    for y in bc.iter() {
                        let d = pair_bound_squared(x, y);
                        if d <= best {
                            heap.push(Candidate {
                                lower_squared: d,
                                a: x,
                                b: y,
                            });
                        }
                    }
                }
            }
            (Some(ac), None) => {
                for x in ac.iter() {
                    let d = pair_bound_squared(x, b);
                    if d <= best {
                        heap.push(Candidate {
                            lower_squared: d,
                            a: x,
                            b,
                        });
                    }
                }
            }
            (None, Some(bc)) => {
                for y in bc.iter() {
                    let d = pair_bound_squared(a, y);
                    if d <= best {
                        heap.push(Candidate {
                            lower_squared: d,
                            a,
                            b: y,
                        });
                    }
                }
            }
            (None, None) => {
                for x in &a.triangles {
                    for y in &b.triangles {
                        let (pa, pb, d) = triangle_pair(x, y);
                        if d < best {
                            best = d;
                            answer = Some(BodyDistance {
                                a: pa,
                                b: pb,
                                distance: d.sqrt(),
                                triangle_a: x.source_index,
                                triangle_b: y.source_index,
                                node_pairs_visited: visited,
                            });
                        }
                    }
                }
            }
        }
    }
    answer
        .map(|mut r| {
            r.node_pairs_visited = visited;
            r
        })
        .ok_or_else(|| invalid("No finite body-distance candidate"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DihedralReport {
    pub min_radians: f64,
    pub max_radians: f64,
    pub mean_radians: f64,
    pub endpoint_radians: [f64; 2],
    pub edges: usize,
}
/// Length-weighted dihedral statistics over shared mesh edges. `endpoint_radians`
/// reports the two endpoint samples of the first edge; linear mesh faces have
/// constant normals, so endpoint and interior angles agree exactly.
pub fn dihedral_angles(mesh: MeshView<'_>) -> Result<DihedralReport> {
    mesh.validate()?;
    let chunks = mesh.indices.as_chunks::<3>().0;
    let mut edges: std::collections::BTreeMap<(usize, usize), Vec<usize>> = Default::default();
    for (i, t) in chunks.iter().enumerate() {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            edges.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    let p = |id: usize| std::array::from_fn(|k| mesh.positions[id * 3 + k]);
    let mut min = f64::INFINITY;
    let mut max: f64 = 0.;
    let mut sum: f64 = 0.;
    let mut weights: f64 = 0.;
    let mut endpoints = None;
    let mut count = 0;
    for ((a, b), ts) in edges {
        if ts.len() != 2 {
            continue;
        }
        let normal = |i: usize| {
            let t = chunks[i];
            cross(sub(p(t[1]), p(t[0])), sub(p(t[2]), p(t[0])))
        };
        let n0 = normal(ts[0]);
        let n1 = normal(ts[1]);
        if norm(n0) == 0. || norm(n1) == 0. {
            return Err(invalid(
                "Dihedral angle is undefined at a collapsed triangle",
            ));
        }
        let angle = (dot(n0, n1) / (norm(n0) * norm(n1))).clamp(-1., 1.).acos();
        let length = norm(sub(p(a), p(b)));
        if !length.is_finite() {
            return Err(invalid("Non-finite edge length"));
        }
        min = min.min(angle);
        max = max.max(angle);
        sum += angle * length;
        weights += length;
        endpoints.get_or_insert([angle; 2]);
        count += 1;
    }
    if count == 0 {
        return Err(invalid(
            "Dihedral statistics require a shared nondegenerate edge",
        ));
    }
    Ok(DihedralReport {
        min_radians: min,
        max_radians: max,
        mean_radians: sum / weights,
        endpoint_radians: endpoints.unwrap(),
        edges: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grid() -> (Vec<f64>, Vec<usize>) {
        let mut p = vec![];
        let mut ix = vec![];
        for i in 0..20 {
            let n = p.len() / 3;
            p.extend([i as f64, 0., 0., i as f64, 1., 0., i as f64, 0., 1.]);
            ix.extend([n, n + 1, n + 2]);
        }
        (p, ix)
    }
    #[test]
    fn point_body_matches_bruteforce_and_honours_budget() {
        let (p, i) = grid();
        let m = MeshView::new(&p, &i);
        for q in [[2.2, 0.3, 4.], [-3., 0.2, 0.2], [30., 2., 2.]] {
            let r = closest_point_bvh(m, q, 1000).unwrap();
            assert!((r.distance - crate::proximity::closest_point(&m, q).1).abs() < 1e-12);
        }
        assert_eq!(
            closest_point_bvh(m, [0.; 3], 1).unwrap_err().code,
            "MESH_QUERY_BUDGET_EXCEEDED"
        );
    }
    #[test]
    fn body_distance_handles_separated_touching_and_intersecting() {
        let p = vec![0., 0., 0., 1., 0., 0., 0., 1., 0.];
        let q = vec![3., 0., 0., 4., 0., 0., 3., 1., 0.];
        let r = body_distance_bvh(
            MeshView::new(&p, &[0, 1, 2]),
            MeshView::new(&q, &[0, 1, 2]),
            100,
        )
        .unwrap();
        assert!((r.distance - 2.).abs() < 1e-12);
        let crossing = vec![0.5, 0., -1., 0.5, 0., 1., 0.5, 1., 0.];
        let r = body_distance_bvh(
            MeshView::new(&p, &[0, 1, 2]),
            MeshView::new(&crossing, &[0, 1, 2]),
            100,
        )
        .unwrap();
        assert!(r.distance < 1e-12);
    }
    #[test]
    fn orthogonal_faces_have_constant_dihedral() {
        let p = vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
        let r = dihedral_angles(MeshView::new(&p, &[0, 1, 2, 0, 3, 1])).unwrap();
        assert!(
            (r.mean_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
            "{r:?}"
        );
        assert_eq!(r.endpoint_radians, [r.mean_radians; 2]);
    }
}
