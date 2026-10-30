//! Headset-independent VR scene preparation. Browser APIs live in the host.
//! CAD matrices are row-major; XR poses are column-major and measured in metres.

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidGeometry,
    EmptyScene,
}

pub struct Mesh {
    pub vertices: Vec<f32>, // interleaved XYZ + normal
    pub indices: Vec<u32>,
    pub transform: [f32; 16],
    pub color: [f32; 4],
}
pub struct PreparedMesh {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleMode {
    /// Normalize the scene bounding box to fit inside `target_extent_m` (default `0.6` m) centered at the origin.
    NormalizedFit(f64),
    /// Preserve true 1:1 CAD dimensions by converting millimeters directly to meters (`0.001`) centered at the origin.
    TrueScaleMmToMeters,
}

pub struct PreparedMeshWithNormals {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VrRayHit {
    pub mesh_index: usize,
    pub triangle_index: usize,
    pub distance_m: f32,
    pub point: [f32; 3],
}

pub fn prepare_scene(meshes: Vec<Mesh>) -> Result<Vec<PreparedMesh>, Error> {
    prepare_scene_with_scale(meshes, ScaleMode::NormalizedFit(0.6))
}

pub fn prepare_scene_with_scale(
    meshes: Vec<Mesh>,
    scale_mode: ScaleMode,
) -> Result<Vec<PreparedMesh>, Error> {
    let with_normals = prepare_scene_impl(meshes, scale_mode, false)?;
    Ok(with_normals
        .into_iter()
        .map(|m| PreparedMesh {
            positions: m.positions,
            indices: m.indices,
            color: m.color,
        })
        .collect())
}

pub fn prepare_scene_with_normals(
    meshes: Vec<Mesh>,
    scale_mode: ScaleMode,
) -> Result<Vec<PreparedMeshWithNormals>, Error> {
    prepare_scene_impl(meshes, scale_mode, true)
}



/// Möller–Trumbore XR controller raycast against prepared VR scene meshes.
pub fn raycast_scene(
    scene: &[PreparedMesh],
    origin: [f32; 3],
    direction: [f32; 3],
) -> Option<VrRayHit> {
    let dir_len =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
            .sqrt();
    if !dir_len.is_finite() || dir_len <= 1e-12 {
        return None;
    }
    let dir = [
        direction[0] / dir_len,
        direction[1] / dir_len,
        direction[2] / dir_len,
    ];
    let mut best: Option<VrRayHit> = None;
    for (mi, mesh) in scene.iter().enumerate() {
        for (ti, tri) in mesh.indices.chunks_exact(3).enumerate() {
            let i0 = (tri[0] as usize) * 3;
            let i1 = (tri[1] as usize) * 3;
            let i2 = (tri[2] as usize) * 3;
            if i0 + 2 >= mesh.positions.len()
                || i1 + 2 >= mesh.positions.len()
                || i2 + 2 >= mesh.positions.len()
            {
                continue;
            }
            let v0 = [
                mesh.positions[i0],
                mesh.positions[i0 + 1],
                mesh.positions[i0 + 2],
            ];
            let v1 = [
                mesh.positions[i1],
                mesh.positions[i1 + 1],
                mesh.positions[i1 + 2],
            ];
            let v2 = [
                mesh.positions[i2],
                mesh.positions[i2 + 1],
                mesh.positions[i2 + 2],
            ];
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let h = [
                dir[1] * e2[2] - dir[2] * e2[1],
                dir[2] * e2[0] - dir[0] * e2[2],
                dir[0] * e2[1] - dir[1] * e2[0],
            ];
            let a = e1[0] * h[0] + e1[1] * h[1] + e1[2] * h[2];
            if a.abs() < 1e-7 {
                continue;
            }
            let f = 1.0 / a;
            let s = [origin[0] - v0[0], origin[1] - v0[1], origin[2] - v0[2]];
            let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = [
                s[1] * e1[2] - s[2] * e1[1],
                s[2] * e1[0] - s[0] * e1[2],
                s[0] * e1[1] - s[1] * e1[0],
            ];
            let v = f * (dir[0] * q[0] + dir[1] * q[1] + dir[2] * q[2]);
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let t = f * (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]);
            if t > 1e-6 && best.is_none_or(|b| t < b.distance_m) {
                best = Some(VrRayHit {
                    mesh_index: mi,
                    triangle_index: ti,
                    distance_m: t,
                    point: [
                        origin[0] + dir[0] * t,
                        origin[1] + dir[1] * t,
                        origin[2] + dir[2] * t,
                    ],
                });
            }
        }
    }
    best
}

/// Lock a scene 1.2 m along the viewer's forward direction.
pub fn anchor(mut pose: [f32; 16]) -> [f32; 16] {
    for a in 0..3 {
        pose[12 + a] -= pose[8 + a] * 1.2;
    }
    pose
}

/// 3D point-to-point measurement in XR space with true millimeter conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VrMeasurement {
    pub point_a: [f32; 3],
    pub point_b: [f32; 3],
    pub delta_m: [f32; 3],
    pub distance_m: f32,
    pub cad_distance_mm: f64,
}

/// Computes Euclidean distance and axis deltas between two picked XR points.
/// `scene_scale_m_per_mm` converts VR meters back into CAD millimeters (use `0.001` for `TrueScaleMmToMeters`).
pub fn measure_vr_distance(
    point_a: [f32; 3],
    point_b: [f32; 3],
    scene_scale_m_per_mm: f64,
) -> Option<VrMeasurement> {
    if !point_a
        .iter()
        .chain(point_b.iter())
        .all(|v| v.is_finite())
        || !scene_scale_m_per_mm.is_finite()
        || scene_scale_m_per_mm <= 0.0
    {
        return None;
    }
    let dx = point_b[0] - point_a[0];
    let dy = point_b[1] - point_a[1];
    let dz = point_b[2] - point_a[2];
    let dist_m = (dx * dx + dy * dy + dz * dz).sqrt();
    Some(VrMeasurement {
        point_a,
        point_b,
        delta_m: [dx, dy, dz],
        distance_m: dist_m,
        cad_distance_mm: (dist_m as f64) / scene_scale_m_per_mm,
    })
}

/// Computes the angle in degrees ($[0, 180]$) at `vertex` between rays to `point_a` and `point_b`.
pub fn measure_angle_deg(point_a: [f32; 3], vertex: [f32; 3], point_b: [f32; 3]) -> Option<f32> {
    let u = [
        point_a[0] - vertex[0],
        point_a[1] - vertex[1],
        point_a[2] - vertex[2],
    ];
    let v = [
        point_b[0] - vertex[0],
        point_b[1] - vertex[1],
        point_b[2] - vertex[2],
    ];
    let lu = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let lv = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if !lu.is_finite() || !lv.is_finite() || lu <= 1e-7 || lv <= 1e-7 {
        return None;
    }
    let cos_a = ((u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (lu * lv)).clamp(-1.0, 1.0);
    Some(cos_a.acos().to_degrees())
}

/// Clips a VR scene against a section plane `plane = [nx, ny, nz, d]`, keeping triangles whose
/// centroid satisfies $n_x x + n_y y + n_z z + d \le 0$.
pub fn clip_scene_by_plane(scene: &[PreparedMesh], plane: [f32; 4]) -> Vec<PreparedMesh> {
    if !plane.iter().all(|v| v.is_finite()) {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(scene.len());
    for mesh in scene {
        let mut kept_indices = Vec::with_capacity(mesh.indices.len());
        for tri in mesh.indices.chunks_exact(3) {
            let (i0, i1, i2) = (
                (tri[0] as usize) * 3,
                (tri[1] as usize) * 3,
                (tri[2] as usize) * 3,
            );
            if i0 + 2 >= mesh.positions.len()
                || i1 + 2 >= mesh.positions.len()
                || i2 + 2 >= mesh.positions.len()
            {
                continue;
            }
            let cx = (mesh.positions[i0] + mesh.positions[i1] + mesh.positions[i2]) / 3.0;
            let cy =
                (mesh.positions[i0 + 1] + mesh.positions[i1 + 1] + mesh.positions[i2 + 1]) / 3.0;
            let cz =
                (mesh.positions[i0 + 2] + mesh.positions[i1 + 2] + mesh.positions[i2 + 2]) / 3.0;
            let signed_dist = plane[0] * cx + plane[1] * cy + plane[2] * cz + plane[3];
            if signed_dist <= 0.0 {
                kept_indices.extend_from_slice(tri);
            }
        }
        if !kept_indices.is_empty() {
            out.push(PreparedMesh {
                positions: mesh.positions.clone(),
                indices: kept_indices,
                color: mesh.color,
            });
        }
    }
    out
}

/// Generates an interactive exploded VR assembly view by translating each mesh radially outward
/// from the overall scene centroid by `explosion_factor` ($\ge 0$).
pub fn vr_exploded_view(scene: &[PreparedMesh], explosion_factor: f32) -> Vec<PreparedMesh> {
    if scene.is_empty() || !explosion_factor.is_finite() || explosion_factor <= 0.0 {
        return scene
            .iter()
            .map(|m| PreparedMesh {
                positions: m.positions.clone(),
                indices: m.indices.clone(),
                color: m.color,
            })
            .collect();
    }
    let mut part_centers = Vec::with_capacity(scene.len());
    let mut global_sum = [0.0_f64; 3];
    let mut total_verts = 0usize;
    for mesh in scene {
        let mut sum = [0.0_f64; 3];
        let v_count = mesh.positions.len() / 3;
        for p in mesh.positions.chunks_exact(3) {
            sum[0] += p[0] as f64;
            sum[1] += p[1] as f64;
            sum[2] += p[2] as f64;
        }
        global_sum[0] += sum[0];
        global_sum[1] += sum[1];
        global_sum[2] += sum[2];
        total_verts += v_count;
        if v_count > 0 {
            part_centers.push([
                (sum[0] / v_count as f64) as f32,
                (sum[1] / v_count as f64) as f32,
                (sum[2] / v_count as f64) as f32,
            ]);
        } else {
            part_centers.push([0.0; 3]);
        }
    }
    let gc = if total_verts > 0 {
        [
            (global_sum[0] / total_verts as f64) as f32,
            (global_sum[1] / total_verts as f64) as f32,
            (global_sum[2] / total_verts as f64) as f32,
        ]
    } else {
        [0.0; 3]
    };
    scene
        .iter()
        .zip(part_centers)
        .map(|(mesh, pc)| {
            let offset = [
                (pc[0] - gc[0]) * explosion_factor,
                (pc[1] - gc[1]) * explosion_factor,
                (pc[2] - gc[2]) * explosion_factor,
            ];
            let mut positions = mesh.positions.clone();
            for p in positions.chunks_exact_mut(3) {
                p[0] += offset[0];
                p[1] += offset[1];
                p[2] += offset[2];
            }
            PreparedMesh {
                positions,
                indices: mesh.indices.clone(),
                color: mesh.color,
            }
        })
        .collect()
}

/// Computes an ergonomic grab/turntable anchor point (`[x, y, z]` in meters) at `distance_m`
/// in front of the viewer pose and `vertical_drop_m` below eye level.
pub fn compute_ergonomic_grab_anchor(
    head_pose: [f32; 16],
    distance_m: f32,
    vertical_drop_m: f32,
) -> [f32; 3] {
    [
        head_pose[12] - head_pose[8] * distance_m,
        head_pose[13] - head_pose[9] * distance_m - vertical_drop_m,
        head_pose[14] - head_pose[10] * distance_m,
    ]
}

/// Snaps a 3D VR placement `position` (in meters) to the nearest spatial lattice step `grid_step_m`
/// and snaps the horizontal orientation angle `yaw_rad` (in radians) to the nearest angular increment
/// `angle_step_rad` (if positive; otherwise leaves the coordinate or angle unchanged).
pub fn snap_pose_to_grid(
    position: [f64; 3],
    yaw_rad: f64,
    grid_step_m: f64,
    angle_step_rad: f64,
) -> ([f64; 3], f64) {
    let snapped_pos = if grid_step_m.is_finite() && grid_step_m > 1e-9 {
        [
            if position[0].is_finite() {
                (position[0] / grid_step_m).round() * grid_step_m
            } else {
                position[0]
            },
            if position[1].is_finite() {
                (position[1] / grid_step_m).round() * grid_step_m
            } else {
                position[1]
            },
            if position[2].is_finite() {
                (position[2] / grid_step_m).round() * grid_step_m
            } else {
                position[2]
            },
        ]
    } else {
        position
    };
    let snapped_yaw = if angle_step_rad.is_finite() && angle_step_rad > 1e-9 && yaw_rad.is_finite()
    {
        (yaw_rad / angle_step_rad).round() * angle_step_rad
    } else {
        yaw_rad
    };
    (snapped_pos, snapped_yaw)
}

/// Two-controller gesture transform (scale + rotation + translation) for VR CAD manipulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoHandTransform {
    /// Uniform scale ratio between current and previous inter-controller distances.
    pub scale: f32,
    /// Translation of the midpoint from previous to current frame (`[dx, dy, dz]` in meters).
    pub translation_m: [f32; 3],
    /// Unit axis of rotation mapping the previous hand-to-hand vector onto the current one.
    pub rotation_axis: [f32; 3],
    /// Signed rotation angle in radians around `rotation_axis`.
    pub rotation_angle_rad: f32,
    /// Yaw rotation in radians in the horizontal ($XZ$) plane.
    pub yaw_rad: f32,
}



/// Checks whether a spherical VR controller/hand volume (`hand_pos`, `hand_radius_m`)
/// collides with any triangle in `scene`.
pub fn check_hand_collision(
    hand_pos: [f32; 3],
    hand_radius_m: f32,
    scene: &[PreparedMesh],
) -> bool {
    if !hand_pos.iter().all(|v| v.is_finite()) || !hand_radius_m.is_finite() || hand_radius_m < 0.0
    {
        return false;
    }
    let r2 = hand_radius_m * hand_radius_m;
    for mesh in scene {
        for tri in mesh.indices.chunks_exact(3) {
            let (i0, i1, i2) = (
                (tri[0] as usize) * 3,
                (tri[1] as usize) * 3,
                (tri[2] as usize) * 3,
            );
            if i0 + 2 >= mesh.positions.len()
                || i1 + 2 >= mesh.positions.len()
                || i2 + 2 >= mesh.positions.len()
            {
                continue;
            }
            let a = [
                mesh.positions[i0],
                mesh.positions[i0 + 1],
                mesh.positions[i0 + 2],
            ];
            let b = [
                mesh.positions[i1],
                mesh.positions[i1 + 1],
                mesh.positions[i1 + 2],
            ];
            let c = [
                mesh.positions[i2],
                mesh.positions[i2 + 1],
                mesh.positions[i2 + 2],
            ];
            if point_triangle_dist_sq(hand_pos, a, b, c) <= r2 {
                return true;
            }
        }
    }
    false
}

fn point_triangle_dist_sq(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let d1 = ab[0] * ap[0] + ab[1] * ap[1] + ab[2] * ap[2];
    let d2 = ac[0] * ap[0] + ac[1] * ap[1] + ac[2] * ap[2];
    if d1 <= 0.0 && d2 <= 0.0 {
        return ap[0] * ap[0] + ap[1] * ap[1] + ap[2] * ap[2];
    }
    let bp = [p[0] - b[0], p[1] - b[1], p[2] - b[2]];
    let d3 = ab[0] * bp[0] + ab[1] * bp[1] + ab[2] * bp[2];
    let d4 = ac[0] * bp[0] + ac[1] * bp[1] + ac[2] * bp[2];
    if d3 >= 0.0 && d4 <= d3 {
        return bp[0] * bp[0] + bp[1] * bp[1] + bp[2] * bp[2];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        let q = [a[0] + v * ab[0], a[1] + v * ab[1], a[2] + v * ab[2]];
        return (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
    }
    let cp = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let d5 = ab[0] * cp[0] + ab[1] * cp[1] + ab[2] * cp[2];
    let d6 = ac[0] * cp[0] + ac[1] * cp[1] + ac[2] * cp[2];
    if d6 >= 0.0 && d5 <= d6 {
        return cp[0] * cp[0] + cp[1] * cp[1] + cp[2] * cp[2];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        let q = [a[0] + w * ac[0], a[1] + w * ac[1], a[2] + w * ac[2]];
        return (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let q = [
            b[0] + w * (c[0] - b[0]),
            b[1] + w * (c[1] - b[1]),
            b[2] + w * (c[2] - b[2]),
        ];
        return (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let q = [
        a[0] + ab[0] * v + ac[0] * w,
        a[1] + ab[1] * v + ac[1] * w,
        a[2] + ab[2] * v + ac[2] * w,
    ];
    (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)
}

// Compact little-endian transport, shared by native tests and the WASM host.
// Input: mesh count; each mesh: vertex-float count, index count, matrix16,
// color4, interleaved vertices, indices. Output: total byte length, mesh count;
// each mesh: position-float count, index count, color4, positions, indices.
struct Reader<'a> {
    bytes: &'a [u8],
}
impl Reader<'_> {
    fn u32(&mut self) -> Result<u32, Error> {
        let bytes = self.bytes.get(..4).ok_or(Error::InvalidGeometry)?;
        let value = u32::from_le_bytes(bytes.try_into().unwrap());
        self.bytes = &self.bytes[4..];
        Ok(value)
    }
    fn float(&mut self) -> Result<f32, Error> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn floats(&mut self, count: usize) -> Result<Vec<f32>, Error> {
        if count > self.bytes.len() / 4 {
            return Err(Error::InvalidGeometry);
        }
        (0..count).map(|_| self.float()).collect()
    }
}
pub fn prepare_bytes(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut r = Reader { bytes };
    let count = r.u32()? as usize;
    if count > r.bytes.len() / 88 {
        return Err(Error::InvalidGeometry);
    }
    let mut meshes = Vec::with_capacity(count);
    for _ in 0..count {
        let nv = r.u32()? as usize;
        let ni = r.u32()? as usize;
        let transform = r.floats(16)?.try_into().unwrap();
        let color = r.floats(4)?.try_into().unwrap();
        let vertices = r.floats(nv)?;
        if ni > r.bytes.len() / 4 {
            return Err(Error::InvalidGeometry);
        }
        let indices = (0..ni).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?;
        meshes.push(Mesh {
            vertices,
            indices,
            transform,
            color,
        });
    }
    if !r.bytes.is_empty() {
        return Err(Error::InvalidGeometry);
    }
    let scene = prepare_scene(meshes)?;
    let total_words: usize = 2 + scene
        .iter()
        .map(|m| 6 + m.positions.len() + m.indices.len())
        .sum::<usize>();
    let total_bytes = u32::try_from(total_words.checked_mul(4).ok_or(Error::InvalidGeometry)?)
        .map_err(|_| Error::InvalidGeometry)?;
    let mut out = Vec::with_capacity(total_bytes as usize);
    out.extend_from_slice(&total_bytes.to_le_bytes());
    out.extend_from_slice(&(scene.len() as u32).to_le_bytes());
    for mesh in scene {
        out.extend_from_slice(&(mesh.positions.len() as u32).to_le_bytes());
        out.extend_from_slice(&(mesh.indices.len() as u32).to_le_bytes());
        for c in mesh.color {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for p in mesh.positions {
            out.extend_from_slice(&p.to_le_bytes());
        }
        for i in mesh.indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    #[unsafe(no_mangle)]
    pub extern "C" fn vr_alloc(len: usize) -> *mut u8 {
        Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
    }
    /// Host must pass an allocation returned by this module with its exact size.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vr_free(ptr: *mut u8, len: usize) {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vr_prepare(ptr: *const u8, len: usize) -> *mut u8 {
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        match prepare_bytes(bytes) {
            Ok(output) => Box::into_raw(output.into_boxed_slice()) as *mut u8,
            Err(_) => std::ptr::null_mut(),
        }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vr_anchor(ptr: *mut u8) {
        let bytes = unsafe { std::slice::from_raw_parts_mut(ptr, 64) };
        let mut pose = [0.0; 16];
        for (i, p) in pose.iter_mut().enumerate() {
            *p = f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for (i, p) in anchor(pose).iter().enumerate() {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&p.to_le_bytes());
        }
    }
}

#[cfg(test)]
#[path = "lib/tests/mod.rs"]
mod tests;


#[path="lib/methods_TwoHandTransform_solve.rs"]
mod methods_TwoHandTransform_solve;
#[path="lib/algorithm_prepare_scene_impl.rs"]
mod algorithm_prepare_scene_impl;
use algorithm_prepare_scene_impl::prepare_scene_impl;
