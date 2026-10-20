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

fn prepare_scene_impl(
    meshes: Vec<Mesh>,
    scale_mode: ScaleMode,
    compute_normals: bool,
) -> Result<Vec<PreparedMeshWithNormals>, Error> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut result = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        if mesh.indices.is_empty() {
            continue;
        }
        if !mesh.vertices.len().is_multiple_of(6)
            || !mesh.indices.len().is_multiple_of(3)
            || !mesh
                .transform
                .iter()
                .chain(mesh.color.iter())
                .all(|x| x.is_finite())
        {
            return Err(Error::InvalidGeometry);
        }
        let mut positions = Vec::with_capacity(mesh.vertices.len() / 2);
        let mut normals = if compute_normals {
            Vec::with_capacity(mesh.vertices.len() / 2)
        } else {
            Vec::new()
        };
        let m = mesh.transform.map(f64::from);
        for p in mesh.vertices.chunks_exact(6) {
            let [x, y, z] = [p[0] as f64, p[1] as f64, p[2] as f64];
            positions.extend_from_slice(&[
                (m[0] * x + m[1] * y + m[2] * z + m[3]) as f32,
                (m[8] * x + m[9] * y + m[10] * z + m[11]) as f32,
                -(m[4] * x + m[5] * y + m[6] * z + m[7]) as f32,
            ]);
            if compute_normals {
                let [nx, ny, nz] = [p[3] as f64, p[4] as f64, p[5] as f64];
                let rx = m[0] * nx + m[1] * ny + m[2] * nz;
                let ry = m[8] * nx + m[9] * ny + m[10] * nz;
                let rz = -(m[4] * nx + m[5] * ny + m[6] * nz);
                let len = (rx * rx + ry * ry + rz * rz).sqrt();
                if len > 1e-12 {
                    normals.extend_from_slice(&[
                        (rx / len) as f32,
                        (ry / len) as f32,
                        (rz / len) as f32,
                    ]);
                } else {
                    normals.extend_from_slice(&[0.0, 1.0, 0.0]);
                }
            }
        }
        for &index in &mesh.indices {
            let offset = (index as usize)
                .checked_mul(3)
                .ok_or(Error::InvalidGeometry)?;
            let end = offset.checked_add(3).ok_or(Error::InvalidGeometry)?;
            let p = positions.get(offset..end).ok_or(Error::InvalidGeometry)?;
            for a in 0..3 {
                if !p[a].is_finite() {
                    return Err(Error::InvalidGeometry);
                }
                min[a] = min[a].min(p[a] as f64);
                max[a] = max[a].max(p[a] as f64);
            }
        }
        result.push(PreparedMeshWithNormals {
            positions,
            normals,
            indices: mesh.indices,
            color: mesh.color,
        });
    }
    if result.is_empty() {
        return Err(Error::EmptyScene);
    }
    let extent = (0..3).map(|a| max[a] - min[a]).fold(0.0_f64, f64::max);
    let scale = match scale_mode {
        ScaleMode::NormalizedFit(target) => {
            if extent > 0.0 && target.is_finite() && target > 0.0 {
                target / extent
            } else {
                0.001
            }
        }
        ScaleMode::TrueScaleMmToMeters => 0.001,
    };
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    for mesh in &mut result {
        for p in mesh.positions.chunks_exact_mut(3) {
            p[0] = ((p[0] as f64 - center[0]) * scale) as f32;
            p[1] = ((p[1] as f64 - center[1]) * scale) as f32;
            p[2] = ((p[2] as f64 - center[2]) * scale) as f32;
        }
    }
    Ok(result)
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
mod tests {
    use super::*;
    fn mesh(offset: f32) -> Mesh {
        Mesh {
            vertices: vec![
                0., 0., 0., 0., 0., 1., 100., 0., 0., 0., 0., 1., 0., 0., 100., 0., 0., 1.,
            ],
            indices: vec![0, 1, 2],
            transform: [
                1., 0., 0., offset, 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
            color: [1.; 4],
        }
    }
    #[test]
    fn scene_spacing_and_axis() {
        let result = prepare_scene(vec![mesh(0.), mesh(100.)]).unwrap();
        assert!((result[0].positions[0] + 0.3).abs() < 1e-6);
        assert!((result[1].positions[3] - 0.3).abs() < 1e-6);
        assert!((result[0].positions[7] - 0.15).abs() < 1e-6);
    }
    #[test]
    fn rejects_invalid_inputs() {
        assert!(matches!(prepare_scene(vec![]), Err(Error::EmptyScene)));
        let mut m = mesh(0.);
        m.indices[0] = u32::MAX;
        assert!(matches!(
            prepare_scene(vec![m]),
            Err(Error::InvalidGeometry)
        ));
        let mut m = mesh(0.);
        m.vertices[0] = f32::NAN;
        assert!(prepare_scene(vec![m]).is_err());
        for input in [&[][..], &[255, 255, 255, 255][..], &[1, 0, 0, 0][..]] {
            assert!(prepare_bytes(input).is_err());
        }
    }
    #[test]
    fn rotated_anchor() {
        let p = [
            0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 3., 2., 1., 1.,
        ];
        let a = anchor(p);
        assert!((a[12] - 1.8).abs() < 1e-6);
        assert_eq!(a[13], 2.);
        assert_eq!(a[14], 1.);
    }
    #[test]
    fn true_scale_normals_and_raycast() {
        let with_normals =
            prepare_scene_with_normals(vec![mesh(0.)], ScaleMode::TrueScaleMmToMeters).unwrap();
        assert_eq!(with_normals[0].normals.len(), 9);
        assert!((with_normals[0].normals[1] - 1.0).abs() < 1e-6);
        // 100 mm span in TrueScaleMmToMeters is 0.1 m (-0.05 to +0.05)
        assert!((with_normals[0].positions[0] + 0.05).abs() < 1e-6);
        assert!((with_normals[0].positions[3] - 0.05).abs() < 1e-6);

        let prepared = prepare_scene_with_scale(vec![mesh(0.)], ScaleMode::TrueScaleMmToMeters).unwrap();
        let hit = raycast_scene(&prepared, [-0.02, -0.02, 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert_eq!(hit.mesh_index, 0);
        assert_eq!(hit.triangle_index, 0);
        assert!((hit.distance_m - 1.0).abs() < 1e-5);
    }

    #[test]
    fn vr_measurements_section_clip_and_exploded_view() {
        let m = measure_vr_distance([0.0, 0.0, 0.0], [0.03, 0.04, 0.0], 0.001).unwrap();
        assert!((m.distance_m - 0.05).abs() < 1e-6);
        assert!((m.cad_distance_mm - 50.0).abs() < 1e-4);

        let deg = measure_angle_deg([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
        assert!((deg - 90.0).abs() < 1e-5);

        let scene = prepare_scene(vec![mesh(0.), mesh(200.)]).unwrap();
        let clipped = clip_scene_by_plane(&scene, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(clipped.len(), 1);

        let exploded = vr_exploded_view(&scene, 0.5);
        assert!(exploded[1].positions[0] > scene[1].positions[0]);
        assert!(exploded[0].positions[0] < scene[0].positions[0]);

        let pose = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1.6, 0., 1.,
        ];
        let grab = compute_ergonomic_grab_anchor(pose, 0.55, 0.25);
        assert!((grab[1] - 1.35).abs() < 1e-5);
        assert!((grab[2] + 0.55).abs() < 1e-5);
    }
}

