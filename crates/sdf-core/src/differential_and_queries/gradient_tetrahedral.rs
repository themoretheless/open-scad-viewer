//! differential and queries operations: gradient_tetrahedral.
use super::*;
impl Field {
/// Computes the unit outward gradient $\nabla f(p) / \|\nabla f(p)\|$ using a 4-point
    /// tetrahedral finite-difference stencil (only 4 SDF evaluations instead of 6 axial evaluations).
    #[inline]
    pub fn gradient_tetrahedral(&self, p: Point, eps: f64) -> Point {
        let h = eps.max(1e-7);
        let d0 = self.sample([p[0] + h, p[1] - h, p[2] - h]);
        let d1 = self.sample([p[0] - h, p[1] - h, p[2] + h]);
        let d2 = self.sample([p[0] - h, p[1] + h, p[2] - h]);
        let d3 = self.sample([p[0] + h, p[1] + h, p[2] + h]);
        let g = [
            d0 - d1 - d2 + d3,
            -d0 - d1 + d2 + d3,
            -d0 + d1 - d2 + d3,
        ];
        let n = length(g);
        if n > 1e-15 {
            [g[0] / n, g[1] / n, g[2] / n]
        } else {
            [0.0, 0.0, 1.0]
        }
    }
/// Computes the unit outward gradient $\nabla f(p) / \|\nabla f(p)\|$ using a 4-point
    /// tetrahedral finite-difference stencil.
    #[inline]
    pub fn gradient(&self, p: Point, eps: f64) -> Point {
        self.gradient_tetrahedral(p, eps)
    }
/// Estimates ambient occlusion in $[0, 1]$ at surface point `p` along outward `normal`
    /// using an $N$-step exponentially weighted SDF cone/normal march ($k = 1 \dots \text{steps}$
    /// at distances $d_k = k \cdot \text{step\_dist}$). Returns $1.0$ for unoccluded convex/flat
    /// surfaces and approaches $0.0$ inside deep cavities or crevices.
    pub fn ambient_occlusion(&self, p: Point, normal: Point, steps: usize, step_dist: f64) -> f64 {
        if !finite(p) || !finite(normal) || !step_dist.is_finite() || step_dist <= 0.0 || steps == 0 {
            return 1.0;
        }
        let n_len = length(normal);
        if n_len <= 1e-12 {
            return 1.0;
        }
        let n = [normal[0] / n_len, normal[1] / n_len, normal[2] / n_len];
        let count = steps.min(32);
        let mut occlusion = 0.0;
        let mut weight = 0.5;
        let mut norm_sum = 0.0;
        for i in 1..=count {
            let expected_dist = (i as f64) * step_dist;
            let q = [
                p[0] + n[0] * expected_dist,
                p[1] + n[1] * expected_dist,
                p[2] + n[2] * expected_dist,
            ];
            let actual_dist = self.sample(q);
            if actual_dist.is_finite() {
                let deficit = (expected_dist - actual_dist).max(0.0) / expected_dist;
                occlusion += weight * deficit.clamp(0.0, 1.0);
            }
            norm_sum += weight;
            weight *= 0.5;
        }
        if norm_sum > 0.0 {
            (1.0 - occlusion / norm_sum).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
/// Computes the mean curvature $H = \frac{1}{2} \nabla \cdot \left(\frac{\nabla f}{\|\nabla f\|}\right)$
    /// at point `p` via central differences of the normalized SDF gradient with step `eps`.
    /// For a sphere of radius $R$, $H = 1 / R$ on the surface.
    pub fn mean_curvature(&self, p: Point, eps: f64) -> f64 {
        if !finite(p) || !eps.is_finite() {
            return 0.0;
        }
        let h = eps.abs().max(1e-5);
        let g_eps = h * 0.5;
        let nx_pos = self.gradient_tetrahedral([p[0] + h, p[1], p[2]], g_eps)[0];
        let nx_neg = self.gradient_tetrahedral([p[0] - h, p[1], p[2]], g_eps)[0];
        let ny_pos = self.gradient_tetrahedral([p[0], p[1] + h, p[2]], g_eps)[1];
        let ny_neg = self.gradient_tetrahedral([p[0], p[1] - h, p[2]], g_eps)[1];
        let nz_pos = self.gradient_tetrahedral([p[0], p[1], p[2] + h], g_eps)[2];
        let nz_neg = self.gradient_tetrahedral([p[0], p[1], p[2] - h], g_eps)[2];
        let div = ((nx_pos - nx_neg) + (ny_pos - ny_neg) + (nz_pos - nz_neg)) / (2.0 * h);
        0.5 * div
    }
/// Estimates the principal curvatures, Gaussian curvature, and mean curvature
    /// `(k1, k2, gaussian_k, mean_h)` (with $k_1 \ge k_2$) at `p` from the $3 \times 3$
    /// finite-difference Hessian of the SDF projected onto the $2 \times 2$ tangent plane
    /// orthogonal to the unit normal $\mathbf{n} = \nabla f / \|\nabla f\|$.
    pub fn principal_curvatures(&self, p: Point, eps: f64) -> (f64, f64, f64, f64) {
        if !finite(p) || !eps.is_finite() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let h = eps.abs().max(1e-4);
        let f0 = self.sample(p);
        if !f0.is_finite() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let fx_p = self.sample([p[0] + h, p[1], p[2]]);
        let fx_m = self.sample([p[0] - h, p[1], p[2]]);
        let fy_p = self.sample([p[0], p[1] + h, p[2]]);
        let fy_m = self.sample([p[0], p[1] - h, p[2]]);
        let fz_p = self.sample([p[0], p[1], p[2] + h]);
        let fz_m = self.sample([p[0], p[1], p[2] - h]);

        let inv_2h = 0.5 / h;
        let gx = (fx_p - fx_m) * inv_2h;
        let gy = (fy_p - fy_m) * inv_2h;
        let gz = (fz_p - fz_m) * inv_2h;
        let g_norm = (gx * gx + gy * gy + gz * gz).sqrt();
        if !g_norm.is_finite() || g_norm <= 1e-12 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let n = [gx / g_norm, gy / g_norm, gz / g_norm];

        let inv_h2 = 1.0 / (h * h);
        let fxx = (fx_p - 2.0 * f0 + fx_m) * inv_h2;
        let fyy = (fy_p - 2.0 * f0 + fy_m) * inv_h2;
        let fzz = (fz_p - 2.0 * f0 + fz_m) * inv_h2;

        let inv_4h2 = 0.25 * inv_h2;
        let fxy = (self.sample([p[0] + h, p[1] + h, p[2]])
            - self.sample([p[0] + h, p[1] - h, p[2]])
            - self.sample([p[0] - h, p[1] + h, p[2]])
            + self.sample([p[0] - h, p[1] - h, p[2]]))
            * inv_4h2;
        let fxz = (self.sample([p[0] + h, p[1], p[2] + h])
            - self.sample([p[0] + h, p[1], p[2] - h])
            - self.sample([p[0] - h, p[1], p[2] + h])
            + self.sample([p[0] - h, p[1], p[2] - h]))
            * inv_4h2;
        let fyz = (self.sample([p[0], p[1] + h, p[2] + h])
            - self.sample([p[0], p[1] + h, p[2] - h])
            - self.sample([p[0], p[1] - h, p[2] + h])
            + self.sample([p[0], p[1] - h, p[2] - h]))
            * inv_4h2;

        let hess = [
            [fxx, fxy, fxz],
            [fxy, fyy, fyz],
            [fxz, fyz, fzz],
        ];

        // Construct orthonormal tangent basis (t1, t2) orthogonal to n
        let helper = if n[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let u = cross(n, helper);
        let u_len = length(u);
        if u_len <= 1e-12 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let t1 = [u[0] / u_len, u[1] / u_len, u[2] / u_len];
        let t2 = cross(n, t1);

        let apply_hess = |v: Point| -> Point {
            [
                hess[0][0] * v[0] + hess[0][1] * v[1] + hess[0][2] * v[2],
                hess[1][0] * v[0] + hess[1][1] * v[1] + hess[1][2] * v[2],
                hess[2][0] * v[0] + hess[2][1] * v[1] + hess[2][2] * v[2],
            ]
        };
        let ht1 = apply_hess(t1);
        let ht2 = apply_hess(t2);
        let s11 = dot(t1, ht1) / g_norm;
        let s12 = dot(t1, ht2) / g_norm;
        let s22 = dot(t2, ht2) / g_norm;

        let mean_h = 0.5 * (s11 + s22);
        let gaussian_k = s11 * s22 - s12 * s12;
        let disc = (0.25 * (s11 - s22) * (s11 - s22) + s12 * s12).max(0.0).sqrt();
        let k1 = mean_h + disc;
        let k2 = mean_h - disc;
        (k1, k2, gaussian_k, mean_h)
    }
/// Projects an arbitrary `query` point onto the zero-isosurface ($f(p) = 0$) using
    /// iterative Newton / gradient-descent steps $p_{k+1} = p_k - f(p_k) \frac{\nabla f(p_k)}{\|\nabla f(p_k)\|}$
    /// for up to `max_iters` iterations until $|f(p_k)| \le \text{tol}$.
    pub fn closest_surface_point(&self, query: Point, max_iters: usize, tol: f64) -> Point {
        if !finite(query) {
            return query;
        }
        let threshold = if tol.is_finite() && tol > 0.0 {
            tol
        } else {
            1e-6
        };
        let eps = threshold.clamp(1e-6, 1e-3);
        let mut p = query;
        let iters = max_iters.min(256);
        for _ in 0..iters {
            let d = self.sample(p);
            if !d.is_finite() || d.abs() <= threshold {
                break;
            }
            let n = self.gradient_tetrahedral(p, eps);
            let n_len = length(n);
            if n_len <= 1e-12 {
                break;
            }
            let next = [
                p[0] - d * (n[0] / n_len),
                p[1] - d * (n[1] / n_len),
                p[2] - d * (n[2] / n_len),
            ];
            if !finite(next) {
                break;
            }
            p = next;
        }
        p
    }
/// Sphere-tracing raymarcher along a ray from `origin` in direction `dir` up to `max_dist`
    /// with surface hit tolerance `tol`. Returns `Some((hit_position, hit_normal, distance))`
    /// on convergence to the zero-level surface ($|f(p)| \le \text{tol}$).
    pub fn raycast(
        &self,
        origin: Point,
        dir: Point,
        max_dist: f64,
        tol: f64,
    ) -> Option<(Point, Point, f64)> {
        if !finite(origin) || !finite(dir) || !max_dist.is_finite() || max_dist <= 0.0 || !tol.is_finite() || tol <= 0.0 {
            return None;
        }
        let dir_len = length(dir);
        if dir_len <= 1e-12 {
            return None;
        }
        let d = [dir[0] / dir_len, dir[1] / dir_len, dir[2] / dir_len];
        let mut t = 0.0;
        let eps = tol.max(1e-6);
        for _ in 0..256 {
            let p = [origin[0] + d[0] * t, origin[1] + d[1] * t, origin[2] + d[2] * t];
            let dist = self.sample(p);
            if !dist.is_finite() {
                return None;
            }
            if dist.abs() <= tol {
                let normal = self.gradient_tetrahedral(p, eps);
                return Some((p, normal, t));
            }
            t += dist.max(tol * 0.5);
            if t > max_dist {
                break;
            }
        }
        None
    }
/// Estimates the enclosed interior volume ($f(p) \le 0$) within `bounds_min..=bounds_max`
    /// using stratified voxel midpoint integration with `samples_per_axis` subdivisions per axis.
    pub fn estimate_volume_in_bounds(
        &self,
        bounds_min: Point,
        bounds_max: Point,
        samples_per_axis: usize,
    ) -> f64 {
        let n = samples_per_axis.clamp(1, 128);
        let dx = (bounds_max[0] - bounds_min[0]) / n as f64;
        let dy = (bounds_max[1] - bounds_min[1]) / n as f64;
        let dz = (bounds_max[2] - bounds_min[2]) / n as f64;
        if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 || !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
            return 0.0;
        }
        let voxel_vol = dx * dy * dz;
        let mut inside = 0usize;
        // Coarse 2x2x2 block Lipschitz skip: a 2x2x2 block of sub-voxels has center-to-subvoxel-center
        // offset (±0.5*dx, ±0.5*dy, ±0.5*dz), whose Euclidean distance is 0.5 * sqrt(dx^2 + dy^2 + dz^2).
        // Use a conservative Lipschitz safety margin (voxel_diag = sqrt(dx^2 + dy^2 + dz^2)).
        let voxel_diag = (dx * dx + dy * dy + dz * dz).sqrt();
        let can_lipschitz_skip = self.is_lipschitz_bounded();
        let n_even = if can_lipschitz_skip { n & !1 } else { 0 };

        for iz in (0..n_even).step_by(2) {
            let z0 = bounds_min[2] + (iz as f64 + 0.5) * dz;
            let z1 = z0 + dz;
            let zc = 0.5 * (z0 + z1);
            for iy in (0..n_even).step_by(2) {
                let y0 = bounds_min[1] + (iy as f64 + 0.5) * dy;
                let y1 = y0 + dy;
                let yc = 0.5 * (y0 + y1);
                for ix in (0..n_even).step_by(2) {
                    let x0 = bounds_min[0] + (ix as f64 + 0.5) * dx;
                    let x1 = x0 + dx;
                    let xc = 0.5 * (x0 + x1);
                    let dc = self.sample([xc, yc, zc]);
                    if dc > voxel_diag {
                        // All 8 sub-voxel centers are strictly outside
                        continue;
                    }
                    if dc < -voxel_diag {
                        // All 8 sub-voxel centers are strictly inside
                        inside += 8;
                        continue;
                    }
                    for z in [z0, z1] {
                        for y in [y0, y1] {
                            for x in [x0, x1] {
                                if self.sample([x, y, z]) <= 0.0 {
                                    inside += 1;
                                }
                            }
                        }
                    }
                }
                for ix in n_even..n {
                    let x = bounds_min[0] + (ix as f64 + 0.5) * dx;
                    for z in [z0, z1] {
                        for y in [y0, y1] {
                            if self.sample([x, y, z]) <= 0.0 {
                                inside += 1;
                            }
                        }
                    }
                }
            }
            for iy in n_even..n {
                let y = bounds_min[1] + (iy as f64 + 0.5) * dy;
                for z in [z0, z1] {
                    for ix in 0..n {
                        let x = bounds_min[0] + (ix as f64 + 0.5) * dx;
                        if self.sample([x, y, z]) <= 0.0 {
                            inside += 1;
                        }
                    }
                }
            }
        }
        for iz in n_even..n {
            let z = bounds_min[2] + (iz as f64 + 0.5) * dz;
            for iy in 0..n {
                let y = bounds_min[1] + (iy as f64 + 0.5) * dy;
                for ix in 0..n {
                    let x = bounds_min[0] + (ix as f64 + 0.5) * dx;
                    if self.sample([x, y, z]) <= 0.0 {
                        inside += 1;
                    }
                }
            }
        }
        inside as f64 * voxel_vol
    }
pub(super) fn is_lipschitz_bounded(&self) -> bool {
        match self {
            Self::Sphere { .. }
            | Self::Box { .. }
            | Self::Torus { .. }
            | Self::Cylinder { .. }
            | Self::Capsule { .. }
            | Self::Octahedron { .. }
            | Self::HexPrism { .. }
            | Self::TriangularPrism { .. }
            | Self::Extrude { .. }
            | Self::Revolve { .. }
            | Self::MeshDistance { .. } => true,
            Self::Union { a, b }
            | Self::Intersection { a, b }
            | Self::Difference { a, b }
            | Self::SmoothUnion { a, b, .. }
            | Self::SmoothIntersection { a, b, .. }
            | Self::SmoothDifference { a, b, .. }
            | Self::ChamferUnion { a, b, .. }
            | Self::ChamferIntersection { a, b, .. }
            | Self::ChamferDifference { a, b, .. } => {
                a.is_lipschitz_bounded() && b.is_lipschitz_bounded()
            }
            Self::Offset { input, .. }
            | Self::Onion { input, .. }
            | Self::Elongate { input, .. }
            | Self::Mirror { input, .. }
            | Self::Translate { input, .. } => input.is_lipschitz_bounded(),
            _ => false,
        }
    }
/// Estimates the enclosed interior volume ($f(p) \le 0$) using an automatically inferred bounding box
    /// and `samples_per_axis` subdivisions along each axis.
    pub fn estimate_volume(&self, samples_per_axis: usize) -> f64 {
        let (bmin, bmax) = self.infer_bounds();
        self.estimate_volume_in_bounds(bmin, bmax, samples_per_axis)
    }
pub(super) fn infer_bounds(&self) -> (Point, Point) {
        match self {
            Self::Sphere { center, radius } => (
                [center[0] - radius, center[1] - radius, center[2] - radius],
                [center[0] + radius, center[1] + radius, center[2] + radius],
            ),
            Self::Box { center, half_size } => (
                [
                    center[0] - half_size[0],
                    center[1] - half_size[1],
                    center[2] - half_size[2],
                ],
                [
                    center[0] + half_size[0],
                    center[1] + half_size[1],
                    center[2] + half_size[2],
                ],
            ),
            Self::Cylinder {
                center,
                radius,
                half_height,
            }
            | Self::HexPrism {
                center,
                radius,
                half_height,
            }
            | Self::TriangularPrism {
                center,
                radius,
                half_height,
            } => (
                [
                    center[0] - radius,
                    center[1] - radius,
                    center[2] - half_height,
                ],
                [
                    center[0] + radius,
                    center[1] + radius,
                    center[2] + half_height,
                ],
            ),
            Self::Octahedron { center, size } => (
                [center[0] - size, center[1] - size, center[2] - size],
                [center[0] + size, center[1] + size, center[2] + size],
            ),
            Self::Torus {
                center,
                major_radius,
                minor_radius,
            } => {
                let r = major_radius + minor_radius;
                (
                    [center[0] - r, center[1] - r, center[2] - minor_radius],
                    [center[0] + r, center[1] + r, center[2] + minor_radius],
                )
            }
            Self::Capsule { a, b, radius } => (
                [
                    a[0].min(b[0]) - radius,
                    a[1].min(b[1]) - radius,
                    a[2].min(b[2]) - radius,
                ],
                [
                    a[0].max(b[0]) + radius,
                    a[1].max(b[1]) + radius,
                    a[2].max(b[2]) + radius,
                ],
            ),
            Self::Union { a, b }
            | Self::SmoothUnion { a, b, .. }
            | Self::ChamferUnion { a, b, .. } => {
                let (min_a, max_a) = a.infer_bounds();
                let (min_b, max_b) = b.infer_bounds();
                (
                    [
                        min_a[0].min(min_b[0]),
                        min_a[1].min(min_b[1]),
                        min_a[2].min(min_b[2]),
                    ],
                    [
                        max_a[0].max(max_b[0]),
                        max_a[1].max(max_b[1]),
                        max_a[2].max(max_b[2]),
                    ],
                )
            }
            Self::Intersection { a, b }
            | Self::SmoothIntersection { a, b, .. }
            | Self::ChamferIntersection { a, b, .. } => {
                let (min_a, max_a) = a.infer_bounds();
                let (min_b, max_b) = b.infer_bounds();
                (
                    [
                        min_a[0].max(min_b[0]),
                        min_a[1].max(min_b[1]),
                        min_a[2].max(min_b[2]),
                    ],
                    [
                        max_a[0].min(max_b[0]),
                        max_a[1].min(max_b[1]),
                        max_a[2].min(max_b[2]),
                    ],
                )
            }
            Self::Difference { a, .. }
            | Self::SmoothDifference { a, .. }
            | Self::ChamferDifference { a, .. } => a.infer_bounds(),
            Self::Translate { input, vector } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [bmin[0] + vector[0], bmin[1] + vector[1], bmin[2] + vector[2]],
                    [bmax[0] + vector[0], bmax[1] + vector[1], bmax[2] + vector[2]],
                )
            }
            Self::Offset { input, distance } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [bmin[0] - distance, bmin[1] - distance, bmin[2] - distance],
                    [bmax[0] + distance, bmax[1] + distance, bmax[2] + distance],
                )
            }
            Self::Onion { input, thickness } => {
                let (bmin, bmax) = input.infer_bounds();
                let h = 0.5 * thickness;
                (
                    [bmin[0] - h, bmin[1] - h, bmin[2] - h],
                    [bmax[0] + h, bmax[1] + h, bmax[2] + h],
                )
            }
            Self::Elongate {
                input,
                half_extents,
            } => {
                let (bmin, bmax) = input.infer_bounds();
                (
                    [
                        bmin[0] - half_extents[0],
                        bmin[1] - half_extents[1],
                        bmin[2] - half_extents[2],
                    ],
                    [
                        bmax[0] + half_extents[0],
                        bmax[1] + half_extents[1],
                        bmax[2] + half_extents[2],
                    ],
                )
            }
            Self::Mirror { input, axis } => {
                let (bmin, bmax) = input.infer_bounds();
                let bound = |i: usize| {
                    if axis[i] {
                        let m = bmin[i].abs().max(bmax[i].abs());
                        (-m, m)
                    } else {
                        (bmin[i], bmax[i])
                    }
                };
                let (x0, x1) = bound(0);
                let (y0, y1) = bound(1);
                let (z0, z1) = bound(2);
                ([x0, y0, z0], [x1, y1, z1])
            }
            Self::Taper { input, factor, .. } => {
                let (bmin, bmax) = input.infer_bounds();
                let s = factor.max(1.0);
                (
                    [bmin[0] * s, bmin[1] * s, bmin[2]],
                    [bmax[0] * s, bmax[1] * s, bmax[2]],
                )
            }
            _ => ([-2.0, -2.0, -2.0], [2.0, 2.0, 2.0]),
        }
    }
}
