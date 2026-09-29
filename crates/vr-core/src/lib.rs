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

pub fn prepare_scene(meshes: Vec<Mesh>) -> Result<Vec<PreparedMesh>, Error> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut result = Vec::new();
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
        let m = mesh.transform.map(f64::from);
        for p in mesh.vertices.chunks_exact(6) {
            let [x, y, z] = [p[0] as f64, p[1] as f64, p[2] as f64];
            positions.extend([
                (m[0] * x + m[1] * y + m[2] * z + m[3]) as f32,
                (m[8] * x + m[9] * y + m[10] * z + m[11]) as f32,
                -(m[4] * x + m[5] * y + m[6] * z + m[7]) as f32,
            ]);
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
        result.push(PreparedMesh {
            positions,
            indices: mesh.indices,
            color: mesh.color,
        });
    }
    if result.is_empty() {
        return Err(Error::EmptyScene);
    }
    let extent = (0..3).map(|a| max[a] - min[a]).fold(0.0_f64, f64::max);
    let scale = if extent > 0.0 { 0.6 / extent } else { 0.001 };
    for mesh in &mut result {
        for (i, value) in mesh.positions.iter_mut().enumerate() {
            *value = ((*value as f64 - (min[i % 3] + max[i % 3]) / 2.0) * scale) as f32;
        }
    }
    Ok(result)
}

/// Lock a scene 1.2 m along the viewer's forward direction.
pub fn anchor(mut pose: [f32; 16]) -> [f32; 16] {
    for a in 0..3 {
        pose[12 + a] -= pose[8 + a] * 1.2;
    }
    pose
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
    let mut meshes = Vec::new();
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
    let mut words = vec![0, scene.len() as u32];
    for mesh in scene {
        words.extend([mesh.positions.len() as u32, mesh.indices.len() as u32]);
        words.extend(mesh.color.map(f32::to_bits));
        words.extend(mesh.positions.into_iter().map(f32::to_bits));
        words.extend(mesh.indices);
    }
    words[0] = u32::try_from(words.len() * 4).map_err(|_| Error::InvalidGeometry)?;
    Ok(words.into_iter().flat_map(u32::to_le_bytes).collect())
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
}
