//! Spatial primitives: quaternions (`Quat`), dual quaternions (`DualQuat`),
//! rays (`Ray3`), planes (`Plane3`), and view frustums (`Frustum`).

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

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    /// Construct a unit quaternion from a rotation axis and angle in radians.
    pub fn from_axis_angle(axis: V3, angle_rad: f64) -> Option<Self> {
        if !angle_rad.is_finite() || !axis.iter().all(|v| v.is_finite()) {
            return None;
        }
        let len = norm(axis);
        if !len.is_finite() || len == 0.0 {
            return None;
        }
        let u = unit(axis);
        let (s, c) = (0.5 * angle_rad).sin_cos();
        Some(Self {
            x: u[0] * s,
            y: u[1] * s,
            z: u[2] * s,
            w: c,
        })
    }

    /// Construct a unit quaternion from a 3x3 orthonormal rotation matrix.
    pub fn from_mat3(m: M3) -> Self {
        let trace = m[0][0] + m[1][1] + m[2][2];
        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            Self {
                w: 0.25 * s,
                x: (m[2][1] - m[1][2]) / s,
                y: (m[0][2] - m[2][0]) / s,
                z: (m[1][0] - m[0][1]) / s,
            }
        } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            Self {
                w: (m[2][1] - m[1][2]) / s,
                x: 0.25 * s,
                y: (m[0][1] + m[1][0]) / s,
                z: (m[0][2] + m[2][0]) / s,
            }
        } else if m[1][1] > m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            Self {
                w: (m[0][2] - m[2][0]) / s,
                x: (m[0][1] + m[1][0]) / s,
                y: 0.25 * s,
                z: (m[1][2] + m[2][1]) / s,
            }
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            Self {
                w: (m[1][0] - m[0][1]) / s,
                x: (m[0][2] + m[2][0]) / s,
                y: (m[1][2] + m[2][1]) / s,
                z: 0.25 * s,
            }
        }
        .normalize()
        .unwrap_or(Self::IDENTITY)
    }

    #[inline]
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline]
    pub fn norm_sq(self) -> f64 {
        self.dot(self)
    }

    #[inline]
    pub fn norm(self) -> f64 {
        self.norm_sq().sqrt()
    }

    #[inline]
    pub fn conjugate(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    pub fn normalize(self) -> Option<Self> {
        let len = self.norm();
        if !len.is_finite() || len == 0.0 {
            return None;
        }
        Some(Self {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
            w: self.w / len,
        })
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
        }
    }

    /// Rotate a 3D vector by this unit quaternion.
    #[inline]
    pub fn rotate_vec3(self, v: V3) -> V3 {
        let qv = [self.x, self.y, self.z];
        let uv = cross(qv, v);
        let uuv = cross(qv, uv);
        add(v, scale(add(scale(uv, self.w), uuv), 2.0))
    }

    /// Spherical linear interpolation along the shortest arc between unit quaternions.
    pub fn slerp(self, mut end: Self, t: f64) -> Self {
        let mut cos_theta = self.dot(end);
        if cos_theta < 0.0 {
            end = Self::new(-end.x, -end.y, -end.z, -end.w);
            cos_theta = -cos_theta;
        }
        if cos_theta > 0.9995 {
            let lerped = Self::new(
                self.x + t * (end.x - self.x),
                self.y + t * (end.y - self.y),
                self.z + t * (end.z - self.z),
                self.w + t * (end.w - self.w),
            );
            return lerped.normalize().unwrap_or(self);
        }
        let theta = cos_theta.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let w0 = ((1.0 - t) * theta).sin() / sin_theta;
        let w1 = (t * theta).sin() / sin_theta;
        Self::new(
            self.x * w0 + end.x * w1,
            self.y * w0 + end.y * w1,
            self.z * w0 + end.z * w1,
            self.w * w0 + end.w * w1,
        )
    }

    /// Convert unit quaternion to 3x3 row-major rotation matrix.
    pub fn to_mat3(self) -> M3 {
        let q = self.normalize().unwrap_or(Self::IDENTITY);
        let xx = q.x * q.x;
        let yy = q.y * q.y;
        let zz = q.z * q.z;
        let xy = q.x * q.y;
        let xz = q.x * q.z;
        let yz = q.y * q.z;
        let wx = q.w * q.x;
        let wy = q.w * q.y;
        let wz = q.w * q.z;
        [
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
        ]
    }

    /// Convert unit quaternion to 4x4 row-major affine matrix.
    pub fn to_mat4(self) -> AffineMatrix {
        let m3 = self.to_mat3();
        let mut out = IDENTITY;
        for r in 0..3 {
            for c in 0..3 {
                out[r][c] = m3[r][c];
            }
        }
        out
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

impl DualQuat {
    pub const IDENTITY: Self = Self {
        real: Quat::IDENTITY,
        dual: Quat::new(0.0, 0.0, 0.0, 0.0),
    };

    /// Construct a unit dual quaternion from a rotation quaternion and translation vector.
    pub fn from_rotation_translation(rotation: Quat, translation: V3) -> Self {
        let r = rotation.normalize().unwrap_or(Quat::IDENTITY);
        let t_quat = Quat::new(
            0.5 * translation[0],
            0.5 * translation[1],
            0.5 * translation[2],
            0.0,
        );
        let dual = t_quat.mul(r);
        Self { real: r, dual }
    }

    /// Normalize a dual quaternion so `|real| == 1` and `dot(real, dual) == 0`.
    pub fn normalize(self) -> Option<Self> {
        let mag = self.real.norm();
        if !mag.is_finite() || mag == 0.0 {
            return None;
        }
        let inv = 1.0 / mag;
        let real = Quat::new(
            self.real.x * inv,
            self.real.y * inv,
            self.real.z * inv,
            self.real.w * inv,
        );
        let dual_scaled = Quat::new(
            self.dual.x * inv,
            self.dual.y * inv,
            self.dual.z * inv,
            self.dual.w * inv,
        );
        let rd = real.dot(dual_scaled);
        let dual = Quat::new(
            dual_scaled.x - real.x * rd,
            dual_scaled.y - real.y * rd,
            dual_scaled.z - real.z * rd,
            dual_scaled.w - real.w * rd,
        );
        Some(Self { real, dual })
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        let r = self.real.mul(rhs.real);
        let d1 = self.real.mul(rhs.dual);
        let d2 = self.dual.mul(rhs.real);
        Self {
            real: r,
            dual: Quat::new(d1.x + d2.x, d1.y + d2.y, d1.z + d2.z, d1.w + d2.w),
        }
    }

    /// Extract the rotation quaternion and translation vector.
    pub fn to_rotation_translation(self) -> (Quat, V3) {
        let dq = self.normalize().unwrap_or(Self::IDENTITY);
        let t_quat = dq.dual.mul(dq.real.conjugate());
        (
            dq.real,
            [2.0 * t_quat.x, 2.0 * t_quat.y, 2.0 * t_quat.z],
        )
    }

    /// Transform a 3D point by this dual quaternion.
    #[inline]
    pub fn transform_point(self, point: V3) -> V3 {
        let (rot, trans) = self.to_rotation_translation();
        add(rot.rotate_vec3(point), trans)
    }

    /// Dual quaternion linear blending (DLB) with antipodal sign correction and normalization.
    pub fn blend(self, mut other: Self, t: f64) -> Self {
        if self.real.dot(other.real) < 0.0 {
            other.real = Quat::new(-other.real.x, -other.real.y, -other.real.z, -other.real.w);
            other.dual = Quat::new(-other.dual.x, -other.dual.y, -other.dual.z, -other.dual.w);
        }
        let w0 = 1.0 - t;
        let w1 = t;
        let raw = Self {
            real: Quat::new(
                self.real.x * w0 + other.real.x * w1,
                self.real.y * w0 + other.real.y * w1,
                self.real.z * w0 + other.real.z * w1,
                self.real.w * w0 + other.real.w * w1,
            ),
            dual: Quat::new(
                self.dual.x * w0 + other.dual.x * w1,
                self.dual.y * w0 + other.dual.y * w1,
                self.dual.z * w0 + other.dual.z * w1,
                self.dual.w * w0 + other.dual.w * w1,
            ),
        };
        raw.normalize().unwrap_or(self)
    }
}

/// Oriented 3D ray with origin and unit direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    pub origin: V3,
    pub direction: V3,
}

impl Ray3 {
    pub fn new(origin: V3, direction: V3) -> Result<Self> {
        if !origin.iter().all(|v| v.is_finite()) || !direction.iter().all(|v| v.is_finite()) {
            return Err(Error::new(
                "invalid_ray",
                "Ray3 expects finite origin and direction coordinates",
            ));
        }
        let len = norm(direction);
        if !len.is_finite() || len == 0.0 {
            return Err(Error::new(
                "invalid_ray",
                "Ray3 direction must be non-zero",
            ));
        }
        let dir = unit(direction);
        Ok(Self {
            origin,
            direction: dir,
        })
    }

    #[inline]
    pub fn at(&self, t: f64) -> V3 {
        add(self.origin, scale(self.direction, t))
    }

    /// Ray-AABB slab intersection returning `Some((t_near, t_far))` when hit with `t_far >= 0`.
    pub fn intersect_aabb(&self, min: V3, max: V3) -> Option<(f64, f64)> {
        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;
        for axis in 0..3 {
            let d = self.direction[axis];
            let o = self.origin[axis];
            if d.abs() < 1e-15 {
                if o < min[axis] || o > max[axis] {
                    return None;
                }
            } else {
                let inv = 1.0 / d;
                let mut t1 = (min[axis] - o) * inv;
                let mut t2 = (max[axis] - o) * inv;
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    return None;
                }
            }
        }
        if t_max < 0.0 {
            return None;
        }
        Some((t_min.max(0.0), t_max))
    }

    /// Ray-sphere intersection returning `Some((t_enter, t_exit))` when hit with `t_exit >= 0`.
    pub fn intersect_sphere(&self, center: V3, radius: f64) -> Option<(f64, f64)> {
        if !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let oc = sub(self.origin, center);
        let b = dot(oc, self.direction);
        let c = dot(oc, oc) - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let t0 = -b - s;
        let t1 = -b + s;
        if t1 < 0.0 {
            return None;
        }
        Some((t0.max(0.0), t1))
    }
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

impl Frustum {
    /// Extract a 6-plane frustum (Gribb-Hartmann) from a row-major 4x4 view-projection matrix.
    /// Near plane uses standard OpenGL/clip `[-w, w]` (`r3 + r2`); see [`Self::from_view_proj_zero_to_one`] for WebGPU `[0, w]`.
    pub fn from_view_proj_row_major(m: AffineMatrix) -> Option<Self> {
        let r0 = m[0];
        let r1 = m[1];
        let r2 = m[2];
        let r3 = m[3];
        let left = Plane3::from_coeffs(r3[0] + r0[0], r3[1] + r0[1], r3[2] + r0[2], r3[3] + r0[3])?;
        let right =
            Plane3::from_coeffs(r3[0] - r0[0], r3[1] - r0[1], r3[2] - r0[2], r3[3] - r0[3])?;
        let bottom =
            Plane3::from_coeffs(r3[0] + r1[0], r3[1] + r1[1], r3[2] + r1[2], r3[3] + r1[3])?;
        let top = Plane3::from_coeffs(r3[0] - r1[0], r3[1] - r1[1], r3[2] - r1[2], r3[3] - r1[3])?;
        let near = Plane3::from_coeffs(r3[0] + r2[0], r3[1] + r2[1], r3[2] + r2[2], r3[3] + r2[3])?;
        let far = Plane3::from_coeffs(r3[0] - r2[0], r3[1] - r2[1], r3[2] - r2[2], r3[3] - r2[3])?;
        Some(Self {
            planes: [left, right, bottom, top, near, far],
        })
    }

    /// Extract a 6-plane frustum from a column-major 16-element view-projection matrix (WebGPU/wgpu layout).
    pub fn from_view_proj_col_major(col_major: &[f64; 16]) -> Option<Self> {
        let m = crate::affine::solid_column_major(*col_major);
        Self::from_view_proj_row_major(m)
    }

    #[inline]
    pub fn contains_point(&self, point: V3) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.signed_distance(point) >= -1e-9)
    }

    #[inline]
    pub fn intersects_sphere(&self, center: V3, radius: f64) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.signed_distance(center) >= -radius)
    }

    /// Conservative frustum-AABB intersection test using the positive-vertex (p-vertex) test.
    #[inline]
    pub fn intersects_aabb(&self, min: V3, max: V3) -> bool {
        for plane in &self.planes {
            let p_vertex = [
                if plane.normal[0] >= 0.0 {
                    max[0]
                } else {
                    min[0]
                },
                if plane.normal[1] >= 0.0 {
                    max[1]
                } else {
                    min[1]
                },
                if plane.normal[2] >= 0.0 {
                    max[2]
                } else {
                    min[2]
                },
            ];
            if plane.signed_distance(p_vertex) < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quat_axis_angle_slerp_and_mat3_roundtrip() {
        let q = Quat::from_axis_angle([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2).unwrap();
        let v = q.rotate_vec3([1.0, 0.0, 0.0]);
        assert!((v[0] - 0.0).abs() < 1e-12);
        assert!((v[1] - 1.0).abs() < 1e-12);
        assert!((v[2] - 0.0).abs() < 1e-12);

        let m = q.to_mat3();
        let recovered = Quat::from_mat3(m);
        assert!((q.dot(recovered).abs() - 1.0).abs() < 1e-12);

        let mid = Quat::IDENTITY.slerp(q, 0.5);
        let v_mid = mid.rotate_vec3([1.0, 0.0, 0.0]);
        let s45 = std::f64::consts::FRAC_1_SQRT_2;
        assert!((v_mid[0] - s45).abs() < 1e-12);
        assert!((v_mid[1] - s45).abs() < 1e-12);
    }

    #[test]
    fn dual_quat_transforms_and_blends_rigid_motions() {
        let q = Quat::from_axis_angle([0.0, 1.0, 0.0], std::f64::consts::FRAC_PI_2).unwrap();
        let dq = DualQuat::from_rotation_translation(q, [10.0, -5.0, 2.0]);
        let p = dq.transform_point([1.0, 2.0, 3.0]);
        // Rotating [1, 2, 3] by +90 deg around Y gives [3, 2, -1], plus [10, -5, 2] = [13, -3, 1].
        assert!((p[0] - 13.0).abs() < 1e-12);
        assert!((p[1] - (-3.0)).abs() < 1e-12);
        assert!((p[2] - 1.0).abs() < 1e-12);

        let mid = DualQuat::IDENTITY.blend(dq, 0.5);
        let (mid_r, mid_t) = mid.to_rotation_translation();
        assert!((mid_r.norm() - 1.0).abs() < 1e-12);
        assert!(mid_t.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn ray_aabb_sphere_and_plane_intersections() {
        let ray = Ray3::new([-5.0, 0.5, 0.5], [1.0, 0.0, 0.0]).unwrap();
        let hit = ray.intersect_aabb([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]).unwrap();
        assert!((hit.0 - 5.0).abs() < 1e-12);
        assert!((hit.1 - 7.0).abs() < 1e-12);

        let sphere_hit = ray.intersect_sphere([0.0, 0.5, 0.5], 1.5).unwrap();
        assert!((sphere_hit.0 - 3.5).abs() < 1e-12);
        assert!((sphere_hit.1 - 6.5).abs() < 1e-12);

        let plane = Plane3::from_point_normal([3.0, 0.0, 0.0], [-1.0, 0.0, 0.0]).unwrap();
        let t_plane = plane.intersect_ray(&ray).unwrap();
        assert!((t_plane - 8.0).abs() < 1e-12);
    }

    #[test]
    fn frustum_culling_identifies_inside_and_outside_aabbs() {
        // Identity VP matrix defines the canonical [-1, 1]^3 clip cube.
        let frustum = Frustum::from_view_proj_row_major(IDENTITY).unwrap();
        assert!(frustum.contains_point([0.0, 0.0, 0.0]));
        assert!(!frustum.contains_point([2.0, 0.0, 0.0]));
        assert!(frustum.intersects_aabb([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]));
        assert!(frustum.intersects_aabb([0.8, -0.2, -0.2], [2.0, 0.2, 0.2]));
        assert!(!frustum.intersects_aabb([1.5, 1.5, 1.5], [2.5, 2.5, 2.5]));
        assert!(frustum.intersects_sphere([1.25, 0.0, 0.0], 0.5));
        assert!(!frustum.intersects_sphere([2.0, 0.0, 0.0], 0.5));
    }
}
