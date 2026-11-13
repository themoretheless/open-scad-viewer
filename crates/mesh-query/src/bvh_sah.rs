//! Binned surface-area-heuristic builder for the packed picking BVH.
//!
//! Produces the same buffer layout as [`crate::bvh::build_mesh_bvh`]
//! (preorder nodes, explicit child indices, leaf ranges into `triangles`,
//! exact float32 node bounds), so every query in [`crate::bvh_query`] and
//! [`crate::bvh_range`] works unchanged. The median builder stays the
//! default and the only one covered by the TypeScript parity fixture; this
//! builder trades that byte identity for cheaper traversal on uneven
//! meshes. Deterministic: ties break on bin order, then axis order, and
//! the in-place partition is stable in record order.

use crate::bvh::{LEAF_BIT, MeshBvh};

const BINS: usize = 16;
/// Relative cost of one node visit against one triangle test.
const TRAVERSAL_COST: f64 = 1.;

type Aabb = ([f32; 3], [f32; 3]);

fn empty() -> Aabb {
    ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3])
}

fn grow(a: &mut Aabb, b: &Aabb) {
    for k in 0..3 {
        a.0[k] = a.0[k].min(b.0[k]);
        a.1[k] = a.1[k].max(b.1[k]);
    }
}

fn half_area(a: &Aabb) -> f64 {
    let e: [f64; 3] = std::array::from_fn(|k| (a.1[k] as f64 - a.0[k] as f64).max(0.));
    e[0] * e[1] + e[1] * e[2] + e[2] * e[0]
}

#[derive(Clone)]
struct Record {
    bounds: Aabb,
    centroid: [f64; 3],
    triangle: u32,
}

struct Builder {
    records: Vec<Record>,
    leaf_size: usize,
    bounds: Vec<f32>,
    nodes: Vec<u32>,
}

impl Builder {
    /// Best binned split of `start..end` as (axis, bin boundary) or None
    /// when a leaf is cheaper or every centroid coincides.
    fn split(&self, start: usize, end: usize, node: &Aabb) -> Option<(usize, f64, f64, usize)> {
        let count = end - start;
        let mut cmin = [f64::INFINITY; 3];
        let mut cmax = [f64::NEG_INFINITY; 3];
        for r in &self.records[start..end] {
            for k in 0..3 {
                cmin[k] = cmin[k].min(r.centroid[k]);
                cmax[k] = cmax[k].max(r.centroid[k]);
            }
        }
        let mut best: Option<(f64, usize, usize)> = None;
        for axis in 0..3 {
            let extent = cmax[axis] - cmin[axis];
            if extent.is_nan() || extent <= 0. {
                continue;
            }
            let mut boxes = [empty(); BINS];
            let mut counts = [0usize; BINS];
            for r in &self.records[start..end] {
                let b = bin(r.centroid[axis], cmin[axis], extent);
                counts[b] += 1;
                grow(&mut boxes[b], &r.bounds);
            }
            // Suffix sweep, then prefix sweep scoring each boundary.
            let mut right_area = [0.; BINS];
            let mut right_count = [0usize; BINS];
            let (mut acc, mut n) = (empty(), 0);
            for b in (1..BINS).rev() {
                grow(&mut acc, &boxes[b]);
                n += counts[b];
                right_area[b] = half_area(&acc);
                right_count[b] = n;
            }
            let (mut acc, mut n) = (empty(), 0);
            for b in 1..BINS {
                grow(&mut acc, &boxes[b - 1]);
                n += counts[b - 1];
                if n == 0 || right_count[b] == 0 {
                    continue;
                }
                let cost = n as f64 * half_area(&acc) + right_count[b] as f64 * right_area[b];
                if best.is_none_or(|(c, _, _)| cost < c) {
                    best = Some((cost, axis, b));
                }
            }
        }
        let (cost, axis, boundary) = best?;
        let parent = half_area(node);
        let leaf_cost = count as f64;
        let split_cost = if parent > 0. {
            TRAVERSAL_COST + cost / parent
        } else {
            leaf_cost
        };
        (count > self.leaf_size || split_cost < leaf_cost)
            .then(|| (axis, cmin[axis], cmax[axis] - cmin[axis], boundary))
    }

    fn build(&mut self, start: usize, end: usize) -> usize {
        let node = self.nodes.len() / 2;
        self.nodes.extend([0, 0]);
        let mut aabb = empty();
        for r in &self.records[start..end] {
            grow(&mut aabb, &r.bounds);
        }
        self.bounds.extend(aabb.0);
        self.bounds.extend(aabb.1);
        let count = end - start;
        let middle = if count <= self.leaf_size {
            None
        } else {
            Some(match self.split(start, end, &aabb) {
                Some((axis, lo, extent, boundary)) => {
                    let left = |r: &Record| bin(r.centroid[axis], lo, extent) < boundary;
                    stable_partition(&mut self.records[start..end], left) + start
                }
                // Coincident centroids: split by count in record order.
                None => start + count / 2,
            })
        };
        match middle {
            None => {
                self.nodes[node * 2] = start as u32;
                self.nodes[node * 2 + 1] = LEAF_BIT | count as u32;
            }
            Some(middle) => {
                let left = self.build(start, middle);
                let right = self.build(middle, end);
                self.nodes[node * 2] = left as u32;
                self.nodes[node * 2 + 1] = right as u32;
            }
        }
        node
    }
}

fn bin(value: f64, lo: f64, extent: f64) -> usize {
    (((value - lo) / extent * BINS as f64) as usize).min(BINS - 1)
}

/// Stable partition: records satisfying `left` first, both groups keep
/// their order. Returns the size of the left group.
fn stable_partition(records: &mut [Record], left: impl Fn(&Record) -> bool) -> usize {
    let (mut l, mut r): (Vec<_>, Vec<_>) = records.iter().cloned().partition(|x| left(x));
    let n = l.len();
    l.append(&mut r);
    for (dst, src) in records.iter_mut().zip(l) {
        *dst = src;
    }
    n
}

/// Build a SAH BVH. Same input contract as the median builder: degenerate,
/// non-finite and out-of-range triangles are omitted, a stride below three
/// yields an empty tree, `leaf_size` is clamped to at least one.
pub fn build_mesh_bvh_sah(
    vertices: &[f32],
    indices: &[u32],
    vertex_stride: usize,
    leaf_size: usize,
) -> MeshBvh {
    let vertex_count = if vertex_stride >= 3 {
        vertices.len() / vertex_stride
    } else {
        0
    };
    let mut records = Vec::with_capacity(indices.len() / 3);
    for (triangle, ids) in indices.as_chunks::<3>().0.iter().enumerate() {
        if ids.iter().any(|&i| i as usize >= vertex_count) {
            continue;
        }
        let p = ids.map(|i| {
            let o = i as usize * vertex_stride;
            [vertices[o], vertices[o + 1], vertices[o + 2]]
        });
        if !p.iter().flatten().all(|v| v.is_finite()) {
            continue;
        }
        let e1: [f64; 3] = std::array::from_fn(|k| p[1][k] as f64 - p[0][k] as f64);
        let e2: [f64; 3] = std::array::from_fn(|k| p[2][k] as f64 - p[0][k] as f64);
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let area = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        if area.is_nan() || area <= 0. || !area.is_finite() {
            continue;
        }
        let lo: [f32; 3] = std::array::from_fn(|k| p[0][k].min(p[1][k]).min(p[2][k]));
        let hi: [f32; 3] = std::array::from_fn(|k| p[0][k].max(p[1][k]).max(p[2][k]));
        records.push(Record {
            bounds: (lo, hi),
            centroid: std::array::from_fn(|k| lo[k] as f64 * 0.5 + hi[k] as f64 * 0.5),
            triangle: triangle as u32,
        });
    }
    if records.is_empty() {
        return MeshBvh {
            node_count: 0,
            bounds: Vec::new(),
            nodes: Vec::new(),
            triangles: Vec::new(),
        };
    }
    let count = records.len();
    let mut builder = Builder {
        records,
        leaf_size: leaf_size.max(1),
        bounds: Vec::new(),
        nodes: Vec::new(),
    };
    builder.build(0, count);
    MeshBvh {
        node_count: builder.nodes.len() / 2,
        bounds: builder.bounds,
        nodes: builder.nodes,
        triangles: builder.records.iter().map(|r| r.triangle).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Query, build_mesh_bvh, overlap_aabb, raycast, refit};
    use std::collections::BTreeSet;

    /// A deliberately uneven mesh: a dense cluster plus sparse far triangles.
    fn mesh() -> (Vec<f32>, Vec<u32>) {
        let mut v = Vec::new();
        let mut idx = Vec::new();
        let mut seed = 12345_u32;
        let mut rand = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        for i in 0..600 {
            let s = if i < 500 { 1. } else { 50. };
            let base = [rand() * s, rand() * s, rand() * s];
            let first = (v.len() / 3) as u32;
            for _ in 0..3 {
                v.extend([base[0] + rand() * 0.3, base[1] + rand() * 0.3, base[2] + rand() * 0.3]);
            }
            idx.extend([first, first + 1, first + 2]);
        }
        (v, idx)
    }

    #[test]
    fn sah_tree_answers_like_the_median_tree() {
        let (v, idx) = mesh();
        let sah = build_mesh_bvh_sah(&v, &idx, 3, 4);
        let median = build_mesh_bvh(&v, &idx, 3, 4);
        let mut all = sah.triangles.clone();
        all.sort_unstable();
        assert_eq!(all, (0..600).collect::<Vec<u32>>());
        assert_eq!(sah.bounds.len(), sah.node_count * 6);
        // Bounds are exact: refitting reproduces them.
        let mut refitted = sah.clone();
        refit(&mut refitted, &v, &idx, 3).unwrap();
        assert_eq!(refitted.bounds, sah.bounds);
        for (min, max) in [
            ([0.2, 0.2, 0.2], [0.6, 0.7, 0.5]),
            ([10., 0., 0.], [40., 50., 50.]),
            ([-1., -1., -1.], [100., 100., 100.]),
        ] {
            let eager = overlap_aabb(&median, &v, &idx, 3, min, max).unwrap();
            assert_eq!(overlap_aabb(&sah, &v, &idx, 3, min, max).unwrap(), eager);
            // The lazy iterator yields the same set on either tree.
            for tree in [&sah, &median] {
                let mut lazy = crate::overlap_aabb_iter(tree, &v, &idx, 3, min, max)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                lazy.sort_unstable();
                assert_eq!(lazy, eager);
            }
        }
        let excluded = BTreeSet::new();
        for k in 0..50 {
            let t = k as f64 * 0.37;
            let query = Query {
                origin: [-5., t.sin() * 0.5 + 0.5, t.cos() * 0.5 + 0.5],
                direction: [1., 0.01 * t.cos(), 0.01 * t.sin()],
                min_t: 0.,
                max_t: f64::INFINITY,
                local_from_world: None,
                excluded: &excluded,
            };
            let a = raycast(&sah, &v, &idx, 3, &query).unwrap();
            let b = raycast(&median, &v, &idx, 3, &query).unwrap();
            assert_eq!(a.map(|h| h.triangle), b.map(|h| h.triangle));
        }
        // On this uneven mesh the SAH tree is cheaper by its own measure:
        // the sum over nodes of box area times triangles below it.
        let cost = |t: &MeshBvh| {
            fn below(t: &MeshBvh, n: usize) -> usize {
                let m = t.nodes[n * 2 + 1];
                if m & LEAF_BIT != 0 {
                    (m & !LEAF_BIT) as usize
                } else {
                    below(t, t.nodes[n * 2] as usize) + below(t, m as usize)
                }
            }
            (0..t.node_count)
                .map(|n| {
                    let b = &t.bounds[n * 6..n * 6 + 6];
                    half_area(&([b[0], b[1], b[2]], [b[3], b[4], b[5]])) * below(t, n) as f64
                })
                .sum::<f64>()
        };
        assert!(cost(&sah) < cost(&median), "{} vs {}", cost(&sah), cost(&median));
        // Deterministic.
        let again = build_mesh_bvh_sah(&v, &idx, 3, 4);
        assert_eq!((again.nodes, again.triangles), (sah.nodes, sah.triangles));
    }

    #[test]
    fn sah_handles_empty_and_coincident_input() {
        assert_eq!(build_mesh_bvh_sah(&[], &[], 3, 4).node_count, 0);
        let v = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        let idx: Vec<u32> = (0..40).flat_map(|_| [0, 1, 2]).collect();
        let bvh = build_mesh_bvh_sah(&v, &idx, 3, 2);
        assert_eq!(bvh.triangles.len(), 40);
        assert!(bvh.node_count > 1);
    }
}
