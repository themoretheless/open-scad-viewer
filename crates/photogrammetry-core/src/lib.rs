//! Deterministic photo reconstruction. Algorithms use Rust std only; host owns codecs and UI.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod bundle;
pub mod calibration;
pub mod camera;
pub mod dense;
pub mod diagnostics;
pub mod evaluation;
pub mod features;
pub mod host_matching;
mod matching;
pub(crate) use math_core as math;
#[cfg(feature = "gpu")]
pub mod gpu;
mod model;
mod pipeline;
mod seeding;
/// Compute backend for the heavy stages. `Cpu` is the deterministic reference
/// and the default; `Gpu` is opt-in and requires the `gpu` crate feature.
pub use math_core::{Acceleration, Error, Result};
pub use model::{Image, Point, Reconstruction};
pub use pipeline::{
    ReconstructionOptions, ReconstructionOutcome, reconstruct, reconstruct_detailed,
    reconstruct_detailed_with_matches,
};
pub use seeding::SeedOptions;
pub const INVALID_INPUT: &str = "PHOTOGRAMMETRY_INVALID_INPUT";
pub fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}

/// Downsamples a 3D point cloud (and optional synchronized unit normals) over a regular spatial
/// voxel grid of edge length `voxel_size`, averaging positions and re-normalizing averaged normals
/// within each occupied voxel in deterministic lexicographical voxel order.
pub fn voxel_downsample_point_cloud(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    voxel_size: f64,
) -> (Vec<[f64; 3]>, Option<Vec<[f64; 3]>>) {
    if points.is_empty() || !voxel_size.is_finite() || voxel_size <= 1e-12 {
        return (
            points.to_vec(),
            normals.map(|n| n.to_vec()),
        );
    }
    let has_normals = normals.is_some_and(|n| n.len() == points.len());
    let inv = 1.0 / voxel_size;
    let mut map: std::collections::BTreeMap<(i64, i64, i64), ([f64; 3], [f64; 3], usize)> =
        std::collections::BTreeMap::new();
    for (idx, &p) in points.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            continue;
        }
        let key = (
            (p[0] * inv).floor() as i64,
            (p[1] * inv).floor() as i64,
            (p[2] * inv).floor() as i64,
        );
        let entry = map.entry(key).or_insert(([0.0; 3], [0.0; 3], 0));
        entry.0[0] += p[0];
        entry.0[1] += p[1];
        entry.0[2] += p[2];
        if has_normals
            && let Some(ns) = normals
        {
            let n = ns[idx];
            if n[0].is_finite() && n[1].is_finite() && n[2].is_finite() {
                entry.1[0] += n[0];
                entry.1[1] += n[1];
                entry.1[2] += n[2];
            }
        }
        entry.2 += 1;
    }
    let mut out_pts = Vec::with_capacity(map.len());
    let mut out_norms = if has_normals {
        Some(Vec::with_capacity(map.len()))
    } else {
        None
    };
    for (_, (sum_p, sum_n, count)) in map {
        let inv_c = 1.0 / (count as f64);
        out_pts.push([sum_p[0] * inv_c, sum_p[1] * inv_c, sum_p[2] * inv_c]);
        if let Some(ref mut ns) = out_norms {
            let len = (sum_n[0] * sum_n[0] + sum_n[1] * sum_n[1] + sum_n[2] * sum_n[2]).sqrt();
            if len > 1e-12 {
                ns.push([sum_n[0] / len, sum_n[1] / len, sum_n[2] / len]);
            } else {
                ns.push([0.0, 0.0, 1.0]);
            }
        }
    }
    (out_pts, out_norms)
}

/// Filters sparse/dense 3D point clouds using Statistical Outlier Removal (SOR):
/// computes each point's mean distance to its `k_neighbors` nearest neighbors and retains
/// only points whose mean k-NN distance is within `global_mean + std_dev_multiplier * global_std_dev`.
pub fn statistical_outlier_removal(
    points: &[[f64; 3]],
    k_neighbors: usize,
    std_dev_multiplier: f64,
) -> Vec<[f64; 3]> {
    let finite_pts: Vec<[f64; 3]> = points
        .iter()
        .copied()
        .filter(|p| p[0].is_finite() && p[1].is_finite() && p[2].is_finite())
        .collect();
    let n = finite_pts.len();
    if n <= 1 || k_neighbors == 0 || !std_dev_multiplier.is_finite() || std_dev_multiplier < 0.0 {
        return finite_pts;
    }
    let k = k_neighbors.min(n - 1);
    let inv_k = 1.0 / (k as f64);
    let mut mean_dists = Vec::with_capacity(n);

    if k <= 32 && n > 256 {
        struct KdNode {
            point: [f64; 3],
            index: usize,
            left: usize,
            right: usize,
            axis: usize,
        }
        fn build_kdtree(
            items: &mut [([f64; 3], usize)],
            level: usize,
            nodes: &mut Vec<KdNode>,
        ) -> usize {
            if items.is_empty() {
                return usize::MAX;
            }
            let axis = level % 3;
            let mid = items.len() / 2;
            items.select_nth_unstable_by(mid, |a, b| {
                a.0[axis]
                    .total_cmp(&b.0[axis])
                    .then(a.0[(axis + 1) % 3].total_cmp(&b.0[(axis + 1) % 3]))
                    .then(a.0[(axis + 2) % 3].total_cmp(&b.0[(axis + 2) % 3]))
                    .then(a.1.cmp(&b.1))
            });
            let node_idx = nodes.len();
            nodes.push(KdNode {
                point: items[mid].0,
                index: items[mid].1,
                left: usize::MAX,
                right: usize::MAX,
                axis,
            });
            let (left_slice, right_slice) = items.split_at_mut(mid);
            let left = build_kdtree(left_slice, level + 1, nodes);
            let right = build_kdtree(&mut right_slice[1..], level + 1, nodes);
            nodes[node_idx].left = left;
            nodes[node_idx].right = right;
            node_idx
        }
        fn knn_search(
            nodes: &[KdNode],
            node_idx: usize,
            query: [f64; 3],
            self_idx: usize,
            k: usize,
            best: &mut [f64; 32],
        ) {
            if node_idx == usize::MAX {
                return;
            }
            let node = &nodes[node_idx];
            if node.index != self_idx {
                let dx = query[0] - node.point[0];
                let dy = query[1] - node.point[1];
                let dz = query[2] - node.point[2];
                let d2 = dx * dx + dy * dy + dz * dz;
                if d2 < best[k - 1] {
                    let mut pos = k - 1;
                    while pos > 0 && d2 < best[pos - 1] {
                        best[pos] = best[pos - 1];
                        pos -= 1;
                    }
                    best[pos] = d2;
                }
            }
            let delta = query[node.axis] - node.point[node.axis];
            let (near, far) = if delta <= 0.0 {
                (node.left, node.right)
            } else {
                (node.right, node.left)
            };
            knn_search(nodes, near, query, self_idx, k, best);
            if delta * delta < best[k - 1] {
                knn_search(nodes, far, query, self_idx, k, best);
            }
        }
        let mut items: Vec<([f64; 3], usize)> = finite_pts
            .iter()
            .copied()
            .enumerate()
            .map(|(i, p)| (p, i))
            .collect();
        let mut nodes = Vec::with_capacity(n);
        let root = build_kdtree(&mut items, 0, &mut nodes);
        for (i, &pi) in finite_pts.iter().enumerate() {
            let mut best = [f64::INFINITY; 32];
            knn_search(&nodes, root, pi, i, k, &mut best);
            let sum_d: f64 = best[..k].iter().map(|d2| d2.sqrt()).sum();
            mean_dists.push(sum_d * inv_k);
        }
    } else if k <= 32 {
        // Maintain a bounded sorted stack buffer of the `k` smallest squared distances:
        // O(1) stack memory and fast early-reject when d2 >= best[k - 1].
        for i in 0..n {
            let mut best = [f64::INFINITY; 32];
            let pi = finite_pts[i];
            for (j, &pj) in finite_pts.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = pi[0] - pj[0];
                let dy = pi[1] - pj[1];
                let dz = pi[2] - pj[2];
                let d2 = dx * dx + dy * dy + dz * dz;
                if d2 < best[k - 1] {
                    let mut pos = k - 1;
                    while pos > 0 && d2 < best[pos - 1] {
                        best[pos] = best[pos - 1];
                        pos -= 1;
                    }
                    best[pos] = d2;
                }
            }
            let sum_d: f64 = best[..k].iter().map(|d2| d2.sqrt()).sum();
            mean_dists.push(sum_d * inv_k);
        }
    } else {
        let mut sq_dists = Vec::with_capacity(n - 1);
        for i in 0..n {
            sq_dists.clear();
            let pi = finite_pts[i];
            for (j, &pj) in finite_pts.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = pi[0] - pj[0];
                let dy = pi[1] - pj[1];
                let dz = pi[2] - pj[2];
                sq_dists.push(dx * dx + dy * dy + dz * dz);
            }
            sq_dists.select_nth_unstable_by(k - 1, |a, b| a.total_cmp(b));
            let sum_d: f64 = sq_dists[..k].iter().map(|d2| d2.sqrt()).sum();
            mean_dists.push(sum_d * inv_k);
        }
    }

    let inv_n = 1.0 / (n as f64);
    let global_mean = mean_dists.iter().sum::<f64>() * inv_n;
    let variance = mean_dists
        .iter()
        .map(|&d| {
            let diff = d - global_mean;
            diff * diff
        })
        .sum::<f64>()
        * inv_n;
    let threshold = global_mean + std_dev_multiplier * variance.sqrt();
    let mut out = Vec::with_capacity(n);
    for (pt, d) in finite_pts.into_iter().zip(mean_dists) {
        if d <= threshold + 1e-12 {
            out.push(pt);
        }
    }
    out
}

#[cfg(test)]
mod point_cloud_filter_tests {
    use super::*;

    #[test]
    fn voxel_downsample_and_statistical_outlier_removal_clean_point_cloud() {
        let pts = vec![
            [0.1, 0.1, 0.1],
            [0.3, 0.3, 0.3],
            [1.1, 0.1, 0.1],
            [1.3, 0.3, 0.3],
        ];
        let norms = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let (down_pts, down_norms) = voxel_downsample_point_cloud(&pts, Some(&norms), 1.0);
        assert_eq!(down_pts.len(), 2);
        let down_norms = down_norms.unwrap();
        assert_eq!(down_norms.len(), 2);
        assert!((down_pts[0][0] - 0.2).abs() < 1e-9);
        assert!((down_pts[1][0] - 1.2).abs() < 1e-9);
        assert!((down_norms[0][2] - 1.0).abs() < 1e-9);

        // Cluster of 9 points near origin + 1 far outlier at [100, 100, 100]
        let mut cloud = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [-0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            [0.0, -0.1, 0.0],
            [0.0, 0.0, 0.1],
            [0.0, 0.0, -0.1],
            [0.1, 0.1, 0.0],
            [-0.1, -0.1, 0.0],
            [100.0, 100.0, 100.0],
        ];
        let filtered = statistical_outlier_removal(&cloud, 4, 1.0);
        assert_eq!(filtered.len(), 9);
        cloud.pop();
        assert_eq!(filtered, cloud);

        // Large point cloud (N = 400 > 256) exercises the exact KD-tree k-NN path
        let mut large_cloud: Vec<[f64; 3]> = (0..400)
            .map(|i| {
                [
                    (i % 20) as f64 * 0.05,
                    (i / 20) as f64 * 0.05,
                    ((i * 13) % 7) as f64 * 0.01,
                ]
            })
            .collect();
        large_cloud.push([50.0, 50.0, 50.0]);
        let filtered_large = statistical_outlier_removal(&large_cloud, 8, 2.0);
        assert_eq!(filtered_large.len(), 400);
        assert!(!filtered_large.contains(&[50.0, 50.0, 50.0]));
    }
}

#[cfg(test)]
mod integration_tests;
