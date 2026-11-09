//! Volume queries and refit over the packed picking BVH.
//!
//! Like [`crate::bvh_query`], everything here borrows the caller's float32
//! vertex records and u32 indices. Node boxes are the exact float32 bounds of
//! their triangles, so pruning against them never drops a qualifying triangle.
use crate::bvh::{LEAF_BIT, MeshBvh, js_max, js_min};
use crate::bvh_query::QueryError;
use crate::proximity::closest_triangle;

type V3 = [f64; 3];

/// A triangle found by a distance query. `point` is the closest point on the
/// triangle and `distance` its Euclidean distance to the query point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbor {
    pub triangle: u32,
    pub distance: f64,
    pub point: V3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefitError {
    /// A child does not follow its parent, or a leaf range leaves the buffers.
    InvalidTree,
    /// The triangle's indices are out of range or a coordinate is not finite.
    InvalidTriangle(u32),
}

impl std::fmt::Display for RefitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTree => f.write_str("invalid BVH tree"),
            Self::InvalidTriangle(id) => write!(f, "triangle {id} cannot be bounded"),
        }
    }
}
impl std::error::Error for RefitError {}

fn node_count(bvh: &MeshBvh) -> usize {
    bvh.node_count
        .min(bvh.bounds.len() / 6)
        .min(bvh.nodes.len() / 2)
}

fn node_box(bvh: &MeshBvh, node: usize) -> Option<(V3, V3)> {
    let b = &bvh.bounds[node * 6..node * 6 + 6];
    let min = [b[0] as f64, b[1] as f64, b[2] as f64];
    let max = [b[3] as f64, b[4] as f64, b[5] as f64];
    // Written so that NaN bounds fail the test.
    (0..3).all(|k| min[k] <= max[k]).then_some((min, max))
}

fn triangle_points(vertices: &[f32], indices: &[u32], stride: usize, id: u32) -> Option<[V3; 3]> {
    let start = (id as usize).checked_mul(3)?;
    let ids = indices.get(start..start.checked_add(3)?)?;
    let mut points = [[0.; 3]; 3];
    for (point, &vertex) in points.iter_mut().zip(ids) {
        let offset = (vertex as usize).checked_mul(stride)?;
        let p = vertices.get(offset..offset.checked_add(3)?)?;
        *point = [p[0] as f64, p[1] as f64, p[2] as f64];
        if !point.iter().all(|v| v.is_finite()) {
            return None;
        }
    }
    Some(points)
}

fn box_distance_squared(p: V3, min: V3, max: V3) -> f64 {
    (0..3)
        .map(|k| {
            let d = (min[k] - p[k]).max(p[k] - max[k]).max(0.);
            d * d
        })
        .sum()
}

/// Depth-first walk. `accept` decides whether a node box is entered and may
/// tighten its own state between calls; `leaf` receives each triangle ID of
/// an accepted leaf. `order` returns the key used to visit the nearer child
/// first. Repeated nodes are refused rather than looped over.
fn walk(
    bvh: &MeshBvh,
    mut accept: impl FnMut(V3, V3) -> bool,
    order: impl Fn(V3, V3) -> f64,
    mut leaf: impl FnMut(u32),
) -> Result<(), QueryError> {
    let count = node_count(bvh);
    if count == 0 {
        return Ok(());
    }
    let mut visited = vec![false; count];
    let mut stack = vec![0usize];
    while let Some(node) = stack.pop() {
        if node >= count {
            continue;
        }
        if std::mem::replace(&mut visited[node], true) {
            return Err(QueryError::InvalidTree);
        }
        let Some((min, max)) = node_box(bvh, node) else {
            continue;
        };
        if !accept(min, max) {
            continue;
        }
        let first = bvh.nodes[node * 2] as usize;
        let metadata = bvh.nodes[node * 2 + 1];
        if metadata & LEAF_BIT != 0 {
            let end = first
                .saturating_add((metadata & !LEAF_BIT) as usize)
                .min(bvh.triangles.len());
            for &id in bvh.triangles.get(first..end).unwrap_or(&[]) {
                leaf(id);
            }
            continue;
        }
        let right = metadata as usize;
        let key = |child: usize| {
            (child < count)
                .then(|| node_box(bvh, child))
                .flatten()
                .map_or(f64::INFINITY, |(min, max)| order(min, max))
        };
        // The stack pops the last push first.
        if key(first) <= key(right) {
            stack.push(right);
            stack.push(first);
        } else {
            stack.push(first);
            stack.push(right);
        }
    }
    Ok(())
}

/// IDs, in ascending order, of the triangles whose axis-aligned bounding box
/// touches the inclusive box `[min, max]`. This is a broadphase answer: a
/// triangle may be reported although its surface misses the box. A query box
/// with a NaN or with `min > max` on some axis matches nothing.
pub fn overlap_aabb(
    bvh: &MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    min: V3,
    max: V3,
) -> Result<Vec<u32>, QueryError> {
    let mut found = Vec::new();
    if stride < 3 || !(0..3).all(|k| min[k] <= max[k]) {
        return Ok(found);
    }
    let overlaps = |lo: V3, hi: V3| (0..3).all(|k| lo[k] <= max[k] && hi[k] >= min[k]);
    walk(
        bvh,
        overlaps,
        |_, _| 0.,
        |id| {
            if let Some(p) = triangle_points(vertices, indices, stride, id) {
                let lo = std::array::from_fn(|k| p[0][k].min(p[1][k]).min(p[2][k]));
                let hi = std::array::from_fn(|k| p[0][k].max(p[1][k]).max(p[2][k]));
                if overlaps(lo, hi) {
                    found.push(id);
                }
            }
        },
    )?;
    found.sort_unstable();
    Ok(found)
}

fn by_distance_then_id(a: &Neighbor, b: &Neighbor) -> std::cmp::Ordering {
    a.distance
        .total_cmp(&b.distance)
        .then(a.triangle.cmp(&b.triangle))
}

/// Every triangle within `radius` (inclusive) of `center`, ordered by
/// distance and then by triangle ID. A negative or NaN radius, or a
/// non-finite center, matches nothing.
pub fn within_radius(
    bvh: &MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    center: V3,
    radius: f64,
) -> Result<Vec<Neighbor>, QueryError> {
    let mut found = Vec::new();
    if stride < 3 || radius.is_nan() || radius < 0. || !center.iter().all(|v| v.is_finite()) {
        return Ok(found);
    }
    // Squaring an infinite radius stays infinite, so "everything" works.
    let limit = radius * radius;
    walk(
        bvh,
        |min, max| box_distance_squared(center, min, max) <= limit,
        |_, _| 0.,
        |id| {
            if let Some(neighbor) = measure(vertices, indices, stride, id, center)
                && neighbor.distance <= radius
            {
                found.push(neighbor);
            }
        },
    )?;
    found.sort_unstable_by(by_distance_then_id);
    Ok(found)
}

fn measure(vertices: &[f32], indices: &[u32], stride: usize, id: u32, p: V3) -> Option<Neighbor> {
    let [a, b, c] = triangle_points(vertices, indices, stride, id)?;
    let point = closest_triangle(p, a, b, c);
    let distance = (0..3)
        .map(|k| (point[k] - p[k]) * (point[k] - p[k]))
        .sum::<f64>()
        .sqrt();
    distance.is_finite().then_some(Neighbor {
        triangle: id,
        distance,
        point,
    })
}

/// The `k` triangles closest to `point`, ordered by distance and then by
/// triangle ID; equal distances keep the smaller ID. Fewer than `k` entries
/// are returned when the tree holds fewer triangles. A non-finite point or
/// `k == 0` matches nothing.
pub fn nearest(
    bvh: &MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    point: V3,
    k: usize,
) -> Result<Vec<Neighbor>, QueryError> {
    if stride < 3 || k == 0 || !point.iter().all(|v| v.is_finite()) {
        return Ok(Vec::new());
    }
    // Sorted best-first. `worst` is shared between the pruning test and the
    // leaf visitor, which both run inside one `walk`.
    let best = std::cell::RefCell::new(Vec::<Neighbor>::with_capacity(k.min(1024) + 1));
    let worst = std::cell::Cell::new(f64::INFINITY);
    walk(
        bvh,
        // `<=` keeps boxes at exactly the worst distance: they may hold a
        // triangle that ties on distance and wins on ID.
        |min, max| box_distance_squared(point, min, max).sqrt() <= worst.get(),
        |min, max| box_distance_squared(point, min, max),
        |id| {
            let Some(neighbor) = measure(vertices, indices, stride, id, point) else {
                return;
            };
            let mut best = best.borrow_mut();
            let at = best.partition_point(|old| by_distance_then_id(old, &neighbor).is_lt());
            if at >= k {
                return;
            }
            best.insert(at, neighbor);
            best.truncate(k);
            if best.len() == k {
                worst.set(best[k - 1].distance);
            }
        },
    )?;
    Ok(best.into_inner())
}

/// Recompute every node box after the vertex positions moved, keeping the
/// tree topology. Indices and triangle membership must be the ones the tree
/// was built from. Queries stay correct after a refit, but large deformations
/// make boxes overlap and queries slower; rebuild when that matters.
///
/// Only trees whose children follow their parents in the node buffer are
/// accepted, which holds for every tree from [`crate::build_mesh_bvh`]. On
/// error the tree is left unchanged. Refitting with unmoved vertices
/// reproduces the built bounds bit for bit.
pub fn refit(
    bvh: &mut MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
) -> Result<(), RefitError> {
    let count = node_count(bvh);
    if stride < 3 && count > 0 {
        return Err(RefitError::InvalidTree);
    }
    let mut bounds = vec![0f32; count * 6];
    for node in (0..count).rev() {
        let first = bvh.nodes[node * 2] as usize;
        let metadata = bvh.nodes[node * 2 + 1];
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        if metadata & LEAF_BIT != 0 {
            let end = first
                .checked_add((metadata & !LEAF_BIT) as usize)
                .ok_or(RefitError::InvalidTree)?;
            let ids = bvh
                .triangles
                .get(first..end)
                .ok_or(RefitError::InvalidTree)?;
            for &id in ids {
                let points = triangle_points(vertices, indices, stride, id)
                    .ok_or(RefitError::InvalidTriangle(id))?;
                for p in points {
                    for k in 0..3 {
                        min[k] = js_min(min[k], p[k]);
                        max[k] = js_max(max[k], p[k]);
                    }
                }
            }
        } else {
            for child in [first, metadata as usize] {
                if child <= node || child >= count {
                    return Err(RefitError::InvalidTree);
                }
                for k in 0..3 {
                    min[k] = js_min(min[k], bounds[child * 6 + k] as f64);
                    max[k] = js_max(max[k], bounds[child * 6 + 3 + k] as f64);
                }
            }
        }
        for k in 0..3 {
            bounds[node * 6 + k] = min[k] as f32;
            bounds[node * 6 + 3 + k] = max[k] as f32;
        }
    }
    bvh.bounds[..count * 6].copy_from_slice(&bounds);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bvh::build_mesh_bvh;

    /// Deterministic generator: the crate keeps its test dependencies minimal.
    struct Lcg(u64);
    impl Lcg {
        fn unit(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn point(&mut self, scale: f64) -> V3 {
            std::array::from_fn(|_| (self.unit() * 2. - 1.) * scale)
        }
    }

    /// A triangle soup of small triangles scattered through `[-10, 10]^3`.
    fn soup(rng: &mut Lcg, triangles: usize) -> (Vec<f32>, Vec<u32>) {
        let mut vertices = Vec::new();
        for _ in 0..triangles {
            let base = rng.point(10.);
            for _ in 0..3 {
                let offset = rng.point(0.75);
                vertices.extend((0..3).map(|k| (base[k] + offset[k]) as f32));
            }
        }
        (vertices, (0..triangles as u32 * 3).collect())
    }

    fn all_neighbors(vertices: &[f32], indices: &[u32], p: V3) -> Vec<Neighbor> {
        let mut all: Vec<_> = (0..indices.len() as u32 / 3)
            .filter_map(|id| measure(vertices, indices, 3, id, p))
            .collect();
        all.sort_unstable_by(by_distance_then_id);
        all
    }

    #[test]
    fn volume_queries_agree_with_linear_scans() {
        let mut rng = Lcg(0x5eed);
        for (triangles, leaf_size) in [(1, 1), (7, 2), (64, 1), (300, 4), (1000, 8)] {
            let (vertices, indices) = soup(&mut rng, triangles);
            let tree = build_mesh_bvh(&vertices, &indices, 3, leaf_size);
            for _ in 0..40 {
                let p = rng.point(12.);
                let all = all_neighbors(&vertices, &indices, p);
                for k in [1, 3, 17, triangles + 5] {
                    let got = nearest(&tree, &vertices, &indices, 3, p, k).unwrap();
                    assert_eq!(got, all[..k.min(all.len())]);
                }
                let radius = rng.unit() * 6.;
                let got = within_radius(&tree, &vertices, &indices, 3, p, radius).unwrap();
                let want: Vec<_> = all
                    .iter()
                    .copied()
                    .filter(|n| n.distance <= radius)
                    .collect();
                assert_eq!(got, want);

                let half = rng.point(3.).map(f64::abs);
                let (lo, hi): (V3, V3) = (
                    std::array::from_fn(|k| p[k] - half[k]),
                    std::array::from_fn(|k| p[k] + half[k]),
                );
                let got = overlap_aabb(&tree, &vertices, &indices, 3, lo, hi).unwrap();
                let want: Vec<u32> = (0..triangles as u32)
                    .filter(|&id| {
                        let t = triangle_points(&vertices, &indices, 3, id).unwrap();
                        (0..3).all(|k| {
                            t.iter().map(|q| q[k]).fold(f64::INFINITY, f64::min) <= hi[k]
                                && t.iter().map(|q| q[k]).fold(f64::NEG_INFINITY, f64::max) >= lo[k]
                        })
                    })
                    .collect();
                assert_eq!(got, want);
            }
        }
    }

    #[test]
    fn equal_distances_keep_the_smaller_triangle_id() {
        // Three coincident triangles: every query ties on distance.
        let vertices = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        let indices = [0, 1, 2, 0, 1, 2, 0, 1, 2];
        let tree = build_mesh_bvh(&vertices, &indices, 3, 1);
        let got = nearest(&tree, &vertices, &indices, 3, [0.25, 0.25, 2.], 2).unwrap();
        assert_eq!(got.iter().map(|n| n.triangle).collect::<Vec<_>>(), [0, 1]);
        assert!(got.iter().all(|n| n.distance == 2.));
    }

    #[test]
    fn degenerate_queries_match_nothing() {
        let mut rng = Lcg(7);
        let (vertices, indices) = soup(&mut rng, 20);
        let tree = build_mesh_bvh(&vertices, &indices, 3, 2);
        let nan = [f64::NAN, 0., 0.];
        assert!(
            nearest(&tree, &vertices, &indices, 3, nan, 3)
                .unwrap()
                .is_empty()
        );
        assert!(
            nearest(&tree, &vertices, &indices, 3, [0.; 3], 0)
                .unwrap()
                .is_empty()
        );
        for radius in [-1., f64::NAN] {
            assert!(
                within_radius(&tree, &vertices, &indices, 3, [0.; 3], radius)
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(
            overlap_aabb(&tree, &vertices, &indices, 3, [1.; 3], [0.; 3])
                .unwrap()
                .is_empty()
        );
        assert!(
            overlap_aabb(&tree, &vertices, &indices, 3, nan, [1.; 3])
                .unwrap()
                .is_empty()
        );
        let everything =
            within_radius(&tree, &vertices, &indices, 3, [0.; 3], f64::INFINITY).unwrap();
        assert_eq!(everything.len(), 20);
        let empty = build_mesh_bvh(&[], &[], 3, 2);
        assert!(nearest(&empty, &[], &[], 3, [0.; 3], 1).unwrap().is_empty());
    }

    #[test]
    fn malformed_trees_are_refused_without_looping_or_panicking() {
        let cyclic = MeshBvh {
            node_count: 1,
            bounds: vec![-1., -1., -1., 1., 1., 1.],
            nodes: vec![0, 0],
            triangles: vec![0],
        };
        assert_eq!(
            nearest(&cyclic, &[], &[], 3, [0.; 3], 1),
            Err(QueryError::InvalidTree)
        );
        assert_eq!(
            overlap_aabb(&cyclic, &[], &[], 3, [-1.; 3], [1.; 3]),
            Err(QueryError::InvalidTree)
        );
        let mut cyclic = cyclic;
        assert_eq!(
            refit(&mut cyclic, &[], &[], 3),
            Err(RefitError::InvalidTree)
        );
        let mut leaf = MeshBvh {
            nodes: vec![0, LEAF_BIT | 1],
            ..cyclic
        };
        assert!(
            nearest(&leaf, &[], &[u32::MAX; 3], 3, [0.; 3], 1)
                .unwrap()
                .is_empty()
        );
        let before = leaf.bounds.clone();
        assert_eq!(
            refit(&mut leaf, &[], &[u32::MAX; 3], 3),
            Err(RefitError::InvalidTriangle(0))
        );
        assert_eq!(leaf.bounds, before);
        leaf.nodes = vec![0, LEAF_BIT | 9];
        assert_eq!(refit(&mut leaf, &[], &[], 3), Err(RefitError::InvalidTree));
    }

    #[test]
    fn refit_of_unmoved_vertices_reproduces_the_built_bounds() {
        let mut rng = Lcg(11);
        for leaf_size in [1, 3, 8] {
            let (mut vertices, indices) = soup(&mut rng, 257);
            // Signed zeros exercise the JavaScript min/max rule of the build.
            vertices[0] = -0.;
            vertices[3] = 0.;
            let mut tree = build_mesh_bvh(&vertices, &indices, 3, leaf_size);
            let built: Vec<u32> = tree.bounds.iter().map(|v| v.to_bits()).collect();
            refit(&mut tree, &vertices, &indices, 3).unwrap();
            let refitted: Vec<u32> = tree.bounds.iter().map(|v| v.to_bits()).collect();
            assert_eq!(built, refitted);
        }
    }

    #[test]
    fn refit_keeps_queries_exact_after_deformation() {
        let mut rng = Lcg(23);
        let (mut vertices, indices) = soup(&mut rng, 400);
        let mut tree = build_mesh_bvh(&vertices, &indices, 3, 4);
        for round in 0..3 {
            for v in &mut vertices {
                *v += ((rng.unit() * 2. - 1.) * (1. + round as f64)) as f32;
            }
            refit(&mut tree, &vertices, &indices, 3).unwrap();
            for _ in 0..40 {
                let p = rng.point(14.);
                let all = all_neighbors(&vertices, &indices, p);
                let got = nearest(&tree, &vertices, &indices, 3, p, 5).unwrap();
                assert_eq!(got, all[..5]);
            }
            // A ray through a known triangle's centroid still finds a hit no
            // farther than that triangle.
            let excluded = std::collections::BTreeSet::new();
            for id in (0..400).step_by(37) {
                let t = triangle_points(&vertices, &indices, 3, id).unwrap();
                let c: V3 = std::array::from_fn(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.);
                let query = crate::Query {
                    origin: [c[0], c[1], 100.],
                    direction: [0., 0., -1.],
                    min_t: 0.,
                    max_t: f64::INFINITY,
                    local_from_world: None,
                    excluded: &excluded,
                };
                let hit = crate::raycast(&tree, &vertices, &indices, 3, &query).unwrap();
                // A triangle edge-on to the ray has no stable hit to compare.
                let e1: V3 = std::array::from_fn(|k| t[1][k] - t[0][k]);
                let e2: V3 = std::array::from_fn(|k| t[2][k] - t[0][k]);
                let nz = e1[0] * e2[1] - e1[1] * e2[0];
                if nz.abs() > 1e-3 {
                    assert!(hit.expect("refit lost a triangle").t <= 100. - c[2] + 1e-6);
                }
            }
        }
    }
}
