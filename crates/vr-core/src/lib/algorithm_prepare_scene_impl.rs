//! lib algorithm: prepare_scene_impl.
use super::*;
pub fn prepare_scene_impl(
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
