//! Screen-space post-processing kernels (SSAO ambient occlusion, Sobel edge
//! silhouette detection) and frustum/backface culling telemetry for `raster-core`.

/// Telemetry summary for frustum and backface triangle culling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CullingStats {
    pub total_triangles: usize,
    pub frustum_culled_triangles: usize,
    pub backface_culled_triangles: usize,
    pub surviving_triangles: usize,
}

impl CullingStats {
    /// Ratio of culled triangles (`0.0..=1.0`).
    #[inline]
    pub fn cull_ratio(&self) -> f32 {
        if self.total_triangles == 0 {
            0.0
        } else {
            let culled = self.frustum_culled_triangles + self.backface_culled_triangles;
            (culled as f32) / (self.total_triangles as f32)
        }
    }
}

/// Evaluate frustum and backface culling statistics for an indexed triangle mesh.
///
/// - `positions`: flat `[x, y, z, ...]` vertex coordinates (3 floats per vertex).
/// - `indices`: triangle vertex indices (3 `u32`s per triangle).
/// - `vp_col_major`: 4x4 column-major view-projection matrix (WebGPU `[0, 1]` depth clip convention).
/// - `eye`: camera eye position `[x, y, z]`.
pub fn evaluate_culling_stats(
    positions: &[f32],
    indices: &[u32],
    vp_col_major: &[f32; 16],
    eye: [f32; 3],
) -> CullingStats {
    let vertex_count = positions.len() / 3;
    let total_triangles = indices.len() / 3;
    let mut stats = CullingStats {
        total_triangles,
        ..Default::default()
    };

    let project = |p: [f32; 3]| -> [f32; 4] {
        [
            vp_col_major[0] * p[0]
                + vp_col_major[4] * p[1]
                + vp_col_major[8] * p[2]
                + vp_col_major[12],
            vp_col_major[1] * p[0]
                + vp_col_major[5] * p[1]
                + vp_col_major[9] * p[2]
                + vp_col_major[13],
            vp_col_major[2] * p[0]
                + vp_col_major[6] * p[1]
                + vp_col_major[10] * p[2]
                + vp_col_major[14],
            vp_col_major[3] * p[0]
                + vp_col_major[7] * p[1]
                + vp_col_major[11] * p[2]
                + vp_col_major[15],
        ]
    };

    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
            stats.frustum_culled_triangles += 1;
            continue;
        }
        let p0 = [
            positions[i0 * 3],
            positions[i0 * 3 + 1],
            positions[i0 * 3 + 2],
        ];
        let p1 = [
            positions[i1 * 3],
            positions[i1 * 3 + 1],
            positions[i1 * 3 + 2],
        ];
        let p2 = [
            positions[i2 * 3],
            positions[i2 * 3 + 1],
            positions[i2 * 3 + 2],
        ];
        let c0 = project(p0);
        let c1 = project(p1);
        let c2 = project(p2);

        // Frustum outcode check: all 3 vertices outside any single clip plane
        let out_left = c0[0] < -c0[3] && c1[0] < -c1[3] && c2[0] < -c2[3];
        let out_right = c0[0] > c0[3] && c1[0] > c1[3] && c2[0] > c2[3];
        let out_bottom = c0[1] < -c0[3] && c1[1] < -c1[3] && c2[1] < -c2[3];
        let out_top = c0[1] > c0[3] && c1[1] > c1[3] && c2[1] > c2[3];
        let out_near = c0[2] < -c0[3] && c1[2] < -c1[3] && c2[2] < -c2[3];
        let out_far = c0[2] > c0[3] && c1[2] > c1[3] && c2[2] > c2[3];
        if out_left || out_right || out_bottom || out_top || out_near || out_far {
            stats.frustum_culled_triangles += 1;
            continue;
        }

        // World-space geometric normal and view vector backface check
        let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let view_vec = [eye[0] - p0[0], eye[1] - p0[1], eye[2] - p0[2]];
        let facing = n[0] * view_vec[0] + n[1] * view_vec[1] + n[2] * view_vec[2];
        if facing <= 0.0 {
            stats.backface_culled_triangles += 1;
            continue;
        }
        stats.surviving_triangles += 1;
    }

    stats
}

/// Screen-space ambient occlusion (SSAO) estimation from a normalized depth buffer (`0.0..=1.0`,
/// where `1.0` is the far background clear value).
///
/// Returns one occlusion visibility factor in `[0.0, 1.0]` per pixel (`1.0` = fully unoccluded,
/// `< 1.0` = darkened crease/cavity).
pub fn compute_ssao_ao(
    depth: &[f32],
    width: u32,
    height: u32,
    radius_px: u32,
    bias: f32,
    intensity: f32,
) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let total = w.saturating_mul(h);
    if depth.len() < total || w == 0 || h == 0 || radius_px == 0 {
        return vec![1.0; depth.len().max(total)];
    }

    // 8-tap radial spiral offsets in [-1, 1]^2
    const OFFSETS: [(f32, f32); 8] = [
        (1.0, 0.0),
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (0.7071, 0.7071),
        (-0.7071, 0.7071),
        (0.7071, -0.7071),
        (-0.7071, -0.7071),
    ];

    let r = radius_px as f32;
    let range_limit = (bias * 32.0).max(0.05);
    let mut ao = vec![1.0_f32; total];

    for y in 0..h {
        for x in 0..w {
            let center_d = depth[y * w + x];
            if !center_d.is_finite() || center_d >= 1.0 {
                continue;
            }
            let mut occluded = 0.0_f32;
            for &(ox, oy) in &OFFSETS {
                let sx = (x as f32 + ox * r).round().clamp(0.0, (w - 1) as f32) as usize;
                let sy = (y as f32 + oy * r).round().clamp(0.0, (h - 1) as f32) as usize;
                let sample_d = depth[sy * w + sx];
                let diff = center_d - sample_d;
                if diff > bias && diff < range_limit {
                    let weight = 1.0 - (diff / range_limit);
                    occluded += weight;
                }
            }
            let occ_norm = (occluded / OFFSETS.len() as f32) * intensity.max(0.0);
            ao[y * w + x] = (1.0 - occ_norm).clamp(0.0, 1.0);
        }
    }
    ao
}

/// Apply a 3x3 Sobel edge silhouette detector to a single-channel depth or luminance image.
///
/// Returns edge magnitude normalized to `[0.0, 1.0]` per pixel (`1.0` where gradient >= `threshold`).
pub fn sobel_edge_outline(
    channel: &[f32],
    width: u32,
    height: u32,
    threshold: f32,
) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let total = w.saturating_mul(h);
    if channel.len() < total || w < 3 || h < 3 {
        return vec![0.0; channel.len().max(total)];
    }
    let thresh = threshold.max(1e-6);
    let mut out = vec![0.0_f32; total];
    let sample = |x: usize, y: usize| -> f32 { channel[y * w + x] };

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let tl = sample(x - 1, y - 1);
            let tc = sample(x, y - 1);
            let tr = sample(x + 1, y - 1);
            let ml = sample(x - 1, y);
            let mr = sample(x + 1, y);
            let bl = sample(x - 1, y + 1);
            let bc = sample(x, y + 1);
            let br = sample(x + 1, y + 1);

            let gx = (tr + 2.0 * mr + br) - (tl + 2.0 * ml + bl);
            let gy = (bl + 2.0 * bc + br) - (tl + 2.0 * tc + tr);
            let mag = (gx * gx + gy * gy).sqrt();
            out[y * w + x] = (mag / thresh).clamp(0.0, 1.0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_culling_stats_classifies_frustum_and_backface_triangles() {
        let vp_identity: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, // col 0
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 1.0, 0.0, // col 2
            0.0, 0.0, 0.0, 1.0, // col 3
        ];
        // Tri 0: inside frustum, front-facing to eye at (0, 0, 5)
        // Tri 1: inside frustum, back-facing (reversed winding)
        // Tri 2: outside frustum (x > 2)
        let positions: [f32; 27] = [
            // Tri 0
            0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0, // Tri 1 (reversed)
            0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.5, 0.0, 0.0, // Tri 2 (outside)
            3.0, 0.0, 0.0, 3.5, 0.0, 0.0, 3.0, 0.5, 0.0,
        ];
        let indices: [u32; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let stats = evaluate_culling_stats(&positions, &indices, &vp_identity, [0.0, 0.0, 5.0]);
        assert_eq!(stats.total_triangles, 3);
        assert_eq!(stats.surviving_triangles, 1);
        assert_eq!(stats.backface_culled_triangles, 1);
        assert_eq!(stats.frustum_culled_triangles, 1);
        assert!((stats.cull_ratio() - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn ssao_darkens_concave_depth_pockets_and_preserves_flat_planes() {
        let mut depth = vec![0.5_f32; 25];
        let flat_ao = compute_ssao_ao(&depth, 5, 5, 1, 0.001, 1.0);
        assert!(flat_ao.iter().all(|&v| (v - 1.0).abs() < 1e-6));

        // Center pixel is recessed (deeper = larger depth 0.52 surrounded by 0.50)
        depth[2 * 5 + 2] = 0.52;
        let cavity_ao = compute_ssao_ao(&depth, 5, 5, 1, 0.001, 1.0);
        assert!(cavity_ao[2 * 5 + 2] < 0.7);
    }

    #[test]
    fn sobel_edge_outline_detects_step_boundaries() {
        let mut img = vec![0.0_f32; 25];
        for y in 0..5 {
            for x in 3..5 {
                img[y * 5 + x] = 1.0;
            }
        }
        let edges = sobel_edge_outline(&img, 5, 5, 0.5);
        // Column 2 straddles the 0->1 step
        assert_eq!(edges[2 * 5 + 2], 1.0);
        // Column 0 is inside uniform region
        assert_eq!(edges[2 * 5 + 0], 0.0);
    }
}
