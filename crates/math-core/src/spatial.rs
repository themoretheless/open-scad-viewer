//! Spatial primitives: quaternions (`Quat`), dual quaternions (`DualQuat`),
//! rays (`Ray3`), planes (`Plane3`), view frustums (`Frustum`), 3D bounding
//! boxes (`Aabb3`, `Obb3`), and triangle barycentric / closest-point queries.

use crate::affine::{AffineMatrix, IDENTITY};
use crate::{Error, M3, Result, V3, add, cross, dot, norm, scale, sub, unit};

/// Unit or general quaternion `[x, y, z, w]` where `w` is the scalar part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}



/// Dual quaternion `real + eps * dual` representing a 3D rigid body transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DualQuat {
    pub real: Quat,
    pub dual: Quat,
}

impl Default for DualQuat {
    fn default() -> Self {
        Self::IDENTITY
    }
}



/// Oriented 3D ray with origin and unit direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    pub origin: V3,
    pub direction: V3,
}



/// 3D plane defined by `dot(normal, x) + d == 0`, with unit `normal`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane3 {
    pub normal: V3,
    pub d: f64,
}

impl Plane3 {
    /// Normalize `[a, b, c, d]` coefficients for `a*x + b*y + c*z + d = 0`.
    pub fn from_coeffs(a: f64, b: f64, c: f64, d: f64) -> Option<Self> {
        let len = norm([a, b, c]);
        if !len.is_finite() || len == 0.0 || !d.is_finite() {
            return None;
        }
        Some(Self {
            normal: [a / len, b / len, c / len],
            d: d / len,
        })
    }

    pub fn from_point_normal(point: V3, normal: V3) -> Option<Self> {
        let len = norm(normal);
        if !len.is_finite() || len == 0.0 {
            return None;
        }
        let n = unit(normal);
        let d = -dot(n, point);
        if !d.is_finite() {
            return None;
        }
        Some(Self { normal: n, d })
    }

    #[inline]
    pub fn signed_distance(&self, point: V3) -> f64 {
        dot(self.normal, point) + self.d
    }

    /// Intersect a ray with this plane, returning parameter `t >= 0.0`.
    #[inline]
    pub fn intersect_ray(&self, ray: &Ray3) -> Option<f64> {
        let denom = dot(self.normal, ray.direction);
        if denom.abs() < 1e-15 {
            return None;
        }
        let t = -self.signed_distance(ray.origin) / denom;
        if t >= 0.0 && t.is_finite() {
            Some(t)
        } else {
            None
        }
    }
}

/// Six-plane view frustum (inward-pointing normals: `signed_distance(p) >= 0` is inside).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frustum {
    /// Order: left, right, bottom, top, near, far.
    pub planes: [Plane3; 6],
}



/// 3D Axis-Aligned Bounding Box defined by inclusive `min` and `max` corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    pub min: V3,
    pub max: V3,
}

impl Default for Aabb3 {
    fn default() -> Self {
        Self::empty()
    }
}



/// 3D Oriented Bounding Box defined by `center`, orthonormal local `axes` (rows), and positive `half_extents`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb3 {
    pub center: V3,
    /// Orthonormal local axes `[u0, u1, u2]`.
    pub axes: M3,
    pub half_extents: V3,
}

impl Obb3 {
    #[inline]
    pub fn new(center: V3, axes: M3, half_extents: V3) -> Self {
        Self {
            center,
            axes,
            half_extents: [
                half_extents[0].abs(),
                half_extents[1].abs(),
                half_extents[2].abs(),
            ],
        }
    }

    /// Construct an axis-aligned OBB from an [`Aabb3`].
    #[inline]
    pub fn from_aabb(aabb: &Aabb3) -> Self {
        let e = aabb.extents();
        Self {
            center: aabb.center(),
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            half_extents: [0.5 * e[0], 0.5 * e[1], 0.5 * e[2]],
        }
    }

    /// Construct an OBB from center, unit rotation quaternion, and half-extents.
    pub fn from_rotation(center: V3, rotation: Quat, half_extents: V3) -> Self {
        let m = rotation.to_mat3();
        // Columns of rotation matrix are the rotated local X, Y, Z axes.
        let axes = [
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ];
        Self::new(center, axes, half_extents)
    }

    /// True if `point` lies within (or on the boundary of) this OBB.
    #[inline]
    pub fn contains_point(&self, point: V3) -> bool {
        let d = sub(point, self.center);
        for i in 0..3 {
            let proj = dot(d, self.axes[i]).abs();
            if proj > self.half_extents[i] + 1e-12 {
                return false;
            }
        }
        true
    }

    /// 15-axis Separating Axis Theorem (SAT) intersection test between two 3D OBBs (Gottschalk / Ericson).
    pub fn intersects_obb(&self, other: &Self) -> bool {
        const EPS: f64 = 1e-12;
        let a = &self.half_extents;
        let b = &other.half_extents;

        // Rotation matrix R[i][j] = dot(self.axes[i], other.axes[j])
        let mut r = [[0.0_f64; 3]; 3];
        let mut abs_r = [[0.0_f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = dot(self.axes[i], other.axes[j]);
                abs_r[i][j] = r[i][j].abs() + EPS;
            }
        }

        // Translation vector t in `self`'s local frame
        let t_world = sub(other.center, self.center);
        let t = [
            dot(t_world, self.axes[0]),
            dot(t_world, self.axes[1]),
            dot(t_world, self.axes[2]),
        ];

        // Test axes L = A0, A1, A2
        for i in 0..3 {
            let ra = a[i];
            let rb = b[0] * abs_r[i][0] + b[1] * abs_r[i][1] + b[2] * abs_r[i][2];
            if t[i].abs() > ra + rb {
                return false;
            }
        }

        // Test axes L = B0, B1, B2
        for j in 0..3 {
            let ra = a[0] * abs_r[0][j] + a[1] * abs_r[1][j] + a[2] * abs_r[2][j];
            let rb = b[j];
            let proj = (t[0] * r[0][j] + t[1] * r[1][j] + t[2] * r[2][j]).abs();
            if proj > ra + rb {
                return false;
            }
        }

        // Test 9 cross-product axes L = Ai x Bj
        for i in 0..3 {
            let i1 = (i + 1) % 3;
            let i2 = (i + 2) % 3;
            for j in 0..3 {
                let j1 = (j + 1) % 3;
                let j2 = (j + 2) % 3;
                let ra = a[i1] * abs_r[i2][j] + a[i2] * abs_r[i1][j];
                let rb = b[j1] * abs_r[i][j2] + b[j2] * abs_r[i][j1];
                let proj = (t[i2] * r[i1][j] - t[i1] * r[i2][j]).abs();
                if proj > ra + rb {
                    return false;
                }
            }
        }

        true
    }
}

/// Compute the 3D barycentric coordinates `[u, v, w]` of `p` projected onto triangle `(a, b, c)`
/// such that the orthogonal projection is `u*a + v*b + w*c` with `u + v + w == 1.0`.
/// Returns `None` if the triangle is degenerate (zero area) or inputs are non-finite.
pub fn barycentric_coords_3d(p: V3, a: V3, b: V3, c: V3) -> Option<V3> {
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(p, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if !denom.is_finite() || denom.abs() <= 1e-24 {
        return None;
    }
    let inv = 1.0 / denom;
    let v = (d11 * d20 - d01 * d21) * inv;
    let w = (d00 * d21 - d01 * d20) * inv;
    let u = 1.0 - v - w;
    if u.is_finite() && v.is_finite() && w.is_finite() {
        Some([u, v, w])
    } else {
        None
    }
}

/// Find the closest point on 3D triangle `(a, b, c)` to query point `p` (Christer Ericson's Voronoi region algorithm).
pub fn closest_point_on_triangle_3d(p: V3, a: V3, b: V3, c: V3) -> V3 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let denom = d1 - d3;
        let v = if denom.abs() > 1e-24 { d1 / denom } else { 0.0 };
        return add(a, scale(ab, v));
    }

    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let denom = d2 - d6;
        let w = if denom.abs() > 1e-24 { d2 / denom } else { 0.0 };
        return add(a, scale(ac, w));
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let bc = sub(c, b);
        let denom = (d4 - d3) + (d5 - d6);
        let w = if denom.abs() > 1e-24 {
            (d4 - d3) / denom
        } else {
            0.0
        };
        return add(b, scale(bc, w));
    }

    let sum = va + vb + vc;
    if sum.abs() <= 1e-24 {
        return a;
    }
    let denom = 1.0 / sum;
    let v = vb * denom;
    let w = vc * denom;
    add(a, add(scale(ab, v), scale(ac, w)))
}

/// Möller–Trumbore ray-triangle intersection returning `Some((t, u, v))` when `ray` hits
/// triangle `(v0, v1, v2)` at distance `t >= 0.0` with barycentric weights `u >= 0.0`, `v >= 0.0`,
/// `u + v <= 1.0` such that the intersection point is `(1 - u - v) * v0 + u * v1 + v * v2`.
#[inline]
pub fn ray_triangle_intersect_moller_trumbore(
    ray: &Ray3,
    v0: V3,
    v1: V3,
    v2: V3,
) -> Option<(f64, f64, f64)> {
    const EPS: f64 = 1e-12;
    let edge1 = sub(v1, v0);
    let edge2 = sub(v2, v0);
    let h = cross(ray.direction, edge2);
    let a = dot(edge1, h);
    if !a.is_finite() || a.abs() < EPS {
        return None;
    }
    let f = 1.0 / a;
    let s = sub(ray.origin, v0);
    let u = f * dot(s, h);
    if !(-EPS..=1.0 + EPS).contains(&u) {
        return None;
    }
    let q = cross(s, edge1);
    let v = f * dot(ray.direction, q);
    if v < -EPS || u + v > 1.0 + EPS {
        return None;
    }
    let t = f * dot(edge2, q);
    if t >= 0.0 && t.is_finite() {
        Some((t, u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)))
    } else {
        None
    }
}

/// Solve a 3x3 real symmetric eigenvalue problem `A * V = V * diag(lambda)` using exact
/// Jacobi rotations, returning `(eigenvalues, eigenvectors_columns)` sorted in descending
/// order of eigenvalue (`lambda[0] >= lambda[1] >= lambda[2]`).
///
/// The returned `eigenvectors` matrix `V` has orthonormal columns (`V[row][col]` is the `row`-th
/// component of the `col`-th unit eigenvector) with right-handed orientation (`det(V) == +1`).
pub fn symmetric_eigen3(a: M3) -> (V3, M3) {
    // Symmetrize input defensively in case of tiny numerical asymmetry.
    let sym = [
        [a[0][0], 0.5 * (a[0][1] + a[1][0]), 0.5 * (a[0][2] + a[2][0])],
        [0.5 * (a[1][0] + a[0][1]), a[1][1], 0.5 * (a[1][2] + a[2][1])],
        [0.5 * (a[2][0] + a[0][2]), 0.5 * (a[2][1] + a[1][2]), a[2][2]],
    ];
    let (raw_d, raw_v) = crate::linalg::eigen(sym);
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| raw_d[j].total_cmp(&raw_d[i]));
    let eigenvalues = [raw_d[order[0]], raw_d[order[1]], raw_d[order[2]]];
    let c0 = unit([raw_v[0][order[0]], raw_v[1][order[0]], raw_v[2][order[0]]]);
    let c1 = unit([raw_v[0][order[1]], raw_v[1][order[1]], raw_v[2][order[1]]]);
    let mut c2 = unit([raw_v[0][order[2]], raw_v[1][order[2]], raw_v[2][order[2]]]);
    if dot(cross(c0, c1), c2) < 0.0 {
        c2 = scale(c2, -1.0);
    }
    let eigenvectors = [
        [c0[0], c1[0], c2[0]],
        [c0[1], c1[1], c2[1]],
        [c0[2], c1[2], c2[2]],
    ];
    (eigenvalues, eigenvectors)
}

/// Solve the 3x3 linear system `A * x = b` using Gaussian elimination with partial pivoting
/// and condition/finiteness checks. Returns `None` if `A` is singular, nearly singular, or contains
/// non-finite entries.
pub fn solve_linear_3x3(a: M3, b: V3) -> Option<V3> {
    if !a.iter().flatten().all(|v| v.is_finite()) || !b.iter().all(|v| v.is_finite()) {
        return None;
    }
    let max_entry = a
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    if max_entry <= 1e-30 {
        return None;
    }
    let det_a = crate::linalg::det(a);
    let scale3 = max_entry * max_entry * max_entry;
    if !det_a.is_finite() || det_a.abs() <= 1e-14 * scale3 {
        return None;
    }
    crate::linalg::solve(a, b)
}

/// 3D Catmull-Rom spline segment defined by four control points `(p0, p1, p2, p3)` and
/// chordal exponent `alpha` (`0.0` = uniform, `0.5` = centripetal, `1.0` = chordal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatmullRomSpline3 {
    pub p0: V3,
    pub p1: V3,
    pub p2: V3,
    pub p3: V3,
    pub alpha: f64,
}

impl CatmullRomSpline3 {
    #[inline]
    pub const fn new(p0: V3, p1: V3, p2: V3, p3: V3, alpha: f64) -> Self {
        Self {
            p0,
            p1,
            p2,
            p3,
            alpha,
        }
    }

    /// Construct a centripetal (`alpha = 0.5`) 3D Catmull-Rom segment, which guarantees no cusps or self-intersections.
    #[inline]
    pub const fn centripetal(p0: V3, p1: V3, p2: V3, p3: V3) -> Self {
        Self::new(p0, p1, p2, p3, 0.5)
    }

    #[inline]
    pub fn evaluate(&self, t: f64) -> V3 {
        evaluate_catmull_rom_3d(self.p0, self.p1, self.p2, self.p3, t, self.alpha)
    }

    #[inline]
    pub fn tangent(&self, t: f64) -> V3 {
        evaluate_catmull_rom_tangent_3d(self.p0, self.p1, self.p2, self.p3, t, self.alpha)
    }
}

/// Evaluate a 3D Barry–Goldman parameterized Catmull-Rom spline between `p1` (`t = 0`) and `p2` (`t = 1`)
/// with neighbor controls `p0`, `p3` and parameterization exponent `alpha` (`0.0` = uniform, `0.5` = centripetal).
pub fn evaluate_catmull_rom_3d(p0: V3, p1: V3, p2: V3, p3: V3, t: f64, alpha: f64) -> V3 {
    let u = t.clamp(0.0, 1.0);
    let a = alpha.clamp(0.0, 1.0);
    let knot_delta = |pa: V3, pb: V3| -> f64 {
        let d = norm(sub(pb, pa));
        let dt = d.powf(a);
        if dt.is_finite() && dt > 1e-9 {
            dt
        } else {
            1.0
        }
    };
    let t0 = 0.0;
    let t1 = t0 + knot_delta(p0, p1);
    let t2 = t1 + knot_delta(p1, p2);
    let t3 = t2 + knot_delta(p2, p3);
    let tau = t1 + u * (t2 - t1);

    let lerp_pt = |pa: V3, pb: V3, ta: f64, tb: f64| -> V3 {
        let denom = tb - ta;
        if denom.abs() <= 1e-15 {
            pa
        } else {
            let w = (tau - ta) / denom;
            add(scale(pa, 1.0 - w), scale(pb, w))
        }
    };

    let a1 = lerp_pt(p0, p1, t0, t1);
    let a2 = lerp_pt(p1, p2, t1, t2);
    let a3 = lerp_pt(p2, p3, t2, t3);

    let b1 = lerp_pt(a1, a2, t0, t2);
    let b2 = lerp_pt(a2, a3, t1, t3);

    lerp_pt(b1, b2, t1, t2)
}

/// Evaluate the first derivative $dC/dt$ (with respect to normalized segment parameter $t \in [0, 1]$)
/// of a 3D Barry–Goldman Catmull-Rom spline between `p1` and `p2`.
pub fn evaluate_catmull_rom_tangent_3d(
    p0: V3,
    p1: V3,
    p2: V3,
    p3: V3,
    t: f64,
    alpha: f64,
) -> V3 {
    let u = t.clamp(0.0, 1.0);
    let a = alpha.clamp(0.0, 1.0);
    let knot_delta = |pa: V3, pb: V3| -> f64 {
        let d = norm(sub(pb, pa));
        let dt = d.powf(a);
        if dt.is_finite() && dt > 1e-9 {
            dt
        } else {
            1.0
        }
    };
    let t0 = 0.0;
    let t1 = t0 + knot_delta(p0, p1);
    let t2 = t1 + knot_delta(p1, p2);
    let t3 = t2 + knot_delta(p2, p3);
    let dt12 = t2 - t1;
    let tau = t1 + u * dt12;

    let lerp_pt = |pa: V3, pb: V3, ta: f64, tb: f64| -> (V3, V3) {
        let denom = tb - ta;
        if denom.abs() <= 1e-15 {
            (pa, [0.0, 0.0, 0.0])
        } else {
            let inv = 1.0 / denom;
            let w = (tau - ta) * inv;
            let val = add(scale(pa, 1.0 - w), scale(pb, w));
            let dval = scale(sub(pb, pa), inv);
            (val, dval)
        }
    };

    let (a1, da1) = lerp_pt(p0, p1, t0, t1);
    let (a2, da2) = lerp_pt(p1, p2, t1, t2);
    let (a3, da3) = lerp_pt(p2, p3, t2, t3);

    let combine = |va: V3, dva: V3, vb: V3, dvb: V3, ta: f64, tb: f64| -> (V3, V3) {
        let denom = tb - ta;
        if denom.abs() <= 1e-15 {
            (va, dva)
        } else {
            let inv = 1.0 / denom;
            let w = (tau - ta) * inv;
            let val = add(scale(va, 1.0 - w), scale(vb, w));
            let term1 = scale(sub(vb, va), inv);
            let term2 = add(scale(dva, 1.0 - w), scale(dvb, w));
            (val, add(term1, term2))
        }
    };

    let (b1, db1) = combine(a1, da1, a2, da2, t0, t2);
    let (b2, db2) = combine(a2, da2, a3, da3, t1, t3);
    let (_c, dc_dtau) = combine(b1, db1, b2, db2, t1, t2);
    scale(dc_dtau, dt12)
}

/// Fit a best-fit 3D plane `(centroid, unit_normal)` to a point cloud via Principal Component
/// Analysis (PCA) using [`symmetric_eigen3`] on the 3x3 covariance matrix.
/// Returns `None` if fewer than 3 finite points are provided or if the point cloud is collinear/degenerate.
pub fn plane_from_points_pca(points: &[V3]) -> Option<(V3, V3)> {
    if points.len() < 3 || !points.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let inv_n = 1.0 / (points.len() as f64);
    let mut centroid = [0.0_f64; 3];
    for &p in points {
        centroid[0] += p[0];
        centroid[1] += p[1];
        centroid[2] += p[2];
    }
    centroid[0] *= inv_n;
    centroid[1] *= inv_n;
    centroid[2] *= inv_n;

    let mut cov = [[0.0_f64; 3]; 3];
    for &p in points {
        let dx = p[0] - centroid[0];
        let dy = p[1] - centroid[1];
        let dz = p[2] - centroid[2];
        cov[0][0] += dx * dx;
        cov[0][1] += dx * dy;
        cov[0][2] += dx * dz;
        cov[1][1] += dy * dy;
        cov[1][2] += dy * dz;
        cov[2][2] += dz * dz;
    }
    cov[1][0] = cov[0][1];
    cov[2][0] = cov[0][2];
    cov[2][1] = cov[1][2];

    let (evals, evecs) = symmetric_eigen3(cov);
    // Require at least two non-degenerate principal variance directions (non-collinear points)
    if evals[0] <= 1e-24 || evals[1] <= 1e-20 * evals[0] {
        return None;
    }
    let normal = unit([evecs[0][2], evecs[1][2], evecs[2][2]]);
    let len = norm(normal);
    if !len.is_finite() || len <= 0.5 {
        return None;
    }
    Some((centroid, normal))
}

/// Spreads the lowest 10 bits of `v` so there are 2 zero bits between each bit, producing a 30-bit
/// mask ready for 3D Morton (Z-order curve) interleaving.
#[inline]
pub const fn expand_bits_10_to_30(v: u32) -> u32 {
    let mut x = v & 0x0000_03FF;
    x = (x.wrapping_mul(0x0001_0001)) & 0xFF00_00FF;
    x = (x.wrapping_mul(0x0000_0101)) & 0x0F00_F00F;
    x = (x.wrapping_mul(0x0000_0011)) & 0xC30C_30C3;
    x = (x.wrapping_mul(0x0000_0005)) & 0x4924_9249;
    x
}

/// Compacts every third bit of a 30-bit Morton word back into a 10-bit integer in `0..=1023`.
#[inline]
const fn compact_bits_30_to_10(v: u32) -> u32 {
    let mut x = v & 0x4924_9249;
    x = (x ^ (x >> 2)) & 0xC30C_30C3;
    x = (x ^ (x >> 4)) & 0x0F00_F00F;
    x = (x ^ (x >> 8)) & 0xFF00_00FF;
    x = (x ^ (x >> 16)) & 0x0000_03FF;
    x
}

/// Encodes 10-bit integer coordinates `(x, y, z)` (clamped/masked to `0..=1023`) into a 30-bit
/// 3D Z-order Morton code (`x` at bit 0, `y` at bit 1, `z` at bit 2).
#[inline]
pub const fn morton3d_encode(x: u32, y: u32, z: u32) -> u32 {
    expand_bits_10_to_30(x) | (expand_bits_10_to_30(y) << 1) | (expand_bits_10_to_30(z) << 2)
}

/// Decodes a 30-bit 3D Z-order Morton code back into its 10-bit `(x, y, z)` integer coordinates.
#[inline]
pub const fn morton3d_decode(code: u32) -> (u32, u32, u32) {
    (
        compact_bits_30_to_10(code),
        compact_bits_30_to_10(code >> 1),
        compact_bits_30_to_10(code >> 2),
    )
}

/// Quantizes a normalized 3D point `p` in $[0, 1]^3$ onto a $1024^3$ lattice and returns its
/// 30-bit 3D Z-order Morton code for GPU/CPU Linear BVH (`LBVH`) construction.
#[inline]
pub fn morton3d_from_unit_point(p: V3) -> u32 {
    let quantize = |c: f64| -> u32 {
        if !c.is_finite() {
            0
        } else {
            (c.clamp(0.0, 1.0) * 1023.0).round().clamp(0.0, 1023.0) as u32
        }
    };
    morton3d_encode(quantize(p[0]), quantize(p[1]), quantize(p[2]))
}

#[cfg(test)]
#[path = "spatial/tests/mod.rs"]
mod tests;

#[path="spatial/methods_Quat_IDENTITY.rs"]
mod methods_Quat_IDENTITY;
#[path="spatial/methods_DualQuat_IDENTITY.rs"]
mod methods_DualQuat_IDENTITY;
#[path="spatial/methods_Ray3_new.rs"]
mod methods_Ray3_new;
#[path="spatial/methods_Frustum_from_view_proj_row_major.rs"]
mod methods_Frustum_from_view_proj_row_major;
#[path="spatial/methods_Aabb3_empty.rs"]
mod methods_Aabb3_empty;
