//! Native display-mesh preparation: crease-smoothed triangle lists,
//! area-weighted indexed vertex normals, and flat-shaded triangle expansion.

use std::collections::HashMap;

pub struct SmoothTriangleList {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

pub struct ExpandedDisplayMesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub surface_area: f64,
}

#[inline]
fn quantize_coord(v: f64) -> i64 {
    if v.is_finite() {
        (v * 1e5).round() as i64
    } else {
        i64::MIN
    }
}

/// Expand an indexed mesh into a flat triangle list with crease-aware smoothed normals.
/// Normals of triangles meeting at the same quantized position are averaged when their
/// dot product is at least `cos_crease`.
pub fn smooth_triangle_list(
    positions: &[f64],
    indices: &[u32],
    cos_crease: f64,
) -> SmoothTriangleList {
    let triangle_count = indices.len() / 3;
    let vertex_count = positions.len() / 3;
    let mut face_normals = vec![[0.0f64; 3]; triangle_count];
    let mut by_position: HashMap<[i64; 3], usize> = HashMap::with_capacity(vertex_count);
    let mut buckets: Vec<Vec<usize>> = Vec::with_capacity(vertex_count);
    let mut bucket_by_vertex: Vec<Option<usize>> = vec![None; vertex_count];

    for (t, face_normal) in face_normals.iter_mut().enumerate() {
        let t3 = t * 3;
        let ia = indices[t3] as usize;
        let ib = indices[t3 + 1] as usize;
        let ic = indices[t3 + 2] as usize;
        if ia >= vertex_count || ib >= vertex_count || ic >= vertex_count {
            continue;
        }
        let ax = positions[ia * 3];
        let ay = positions[ia * 3 + 1];
        let az = positions[ia * 3 + 2];
        let ux = positions[ib * 3] - ax;
        let uy = positions[ib * 3 + 1] - ay;
        let uz = positions[ib * 3 + 2] - az;
        let wx = positions[ic * 3] - ax;
        let wy = positions[ic * 3 + 1] - ay;
        let wz = positions[ic * 3 + 2] - az;
        let nx = uy * wz - uz * wy;
        let ny = uz * wx - ux * wz;
        let nz = ux * wy - uy * wx;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        let scale = if len == 0.0 || !len.is_finite() {
            1.0
        } else {
            len
        };
        *face_normal = [nx / scale, ny / scale, nz / scale];

        for k in 0..3 {
            let vi = indices[t3 + k] as usize;
            let bucket_idx = match bucket_by_vertex[vi] {
                Some(idx) => idx,
                None => {
                    let v3 = vi * 3;
                    let key = [
                        quantize_coord(positions[v3]),
                        quantize_coord(positions[v3 + 1]),
                        quantize_coord(positions[v3 + 2]),
                    ];
                    let idx = *by_position.entry(key).or_insert_with(|| {
                        let new_idx = buckets.len();
                        buckets.push(Vec::new());
                        new_idx
                    });
                    bucket_by_vertex[vi] = Some(idx);
                    idx
                }
            };
            buckets[bucket_idx].push(t);
        }
    }

    let mut out_positions = vec![0.0f32; triangle_count * 9];
    let mut out_normals = vec![0.0f32; triangle_count * 9];
    for (t, &[fnx, fny, fnz]) in face_normals.iter().enumerate() {
        let t3 = t * 3;
        for k in 0..3 {
            let i = indices[t3 + k] as usize;
            if i >= vertex_count {
                continue;
            }
            let mut sum_x = 0.0f64;
            let mut sum_y = 0.0f64;
            let mut sum_z = 0.0f64;
            if let Some(bucket_idx) = bucket_by_vertex[i] {
                for &other_t in &buckets[bucket_idx] {
                    let [onx, ony, onz] = face_normals[other_t];
                    if fnx * onx + fny * ony + fnz * onz >= cos_crease {
                        sum_x += onx;
                        sum_y += ony;
                        sum_z += onz;
                    }
                }
            }
            let len = (sum_x * sum_x + sum_y * sum_y + sum_z * sum_z).sqrt();
            let o = (t3 + k) * 3;
            let i3 = i * 3;
            out_positions[o] = positions[i3] as f32;
            out_positions[o + 1] = positions[i3 + 1] as f32;
            out_positions[o + 2] = positions[i3 + 2] as f32;
            if len > 1e-9 {
                out_normals[o] = (sum_x / len) as f32;
                out_normals[o + 1] = (sum_y / len) as f32;
                out_normals[o + 2] = (sum_z / len) as f32;
            } else {
                out_normals[o] = fnx as f32;
                out_normals[o + 1] = fny as f32;
                out_normals[o + 2] = fnz as f32;
            }
        }
    }

    SmoothTriangleList {
        positions: out_positions,
        normals: out_normals,
    }
}

/// Fill stride-6 vertex normal slots (`[3..6]` per vertex) with normalized
/// area-weighted triangle normals.
pub fn fill_indexed_normals(vertices: &mut [f32], indices: &[u32]) {
    let vertex_count = vertices.len() / 6;
    for face in indices.chunks_exact(3) {
        let ia = face[0] as usize;
        let ib = face[1] as usize;
        let ic = face[2] as usize;
        if ia >= vertex_count || ib >= vertex_count || ic >= vertex_count {
            continue;
        }
        let a = ia * 6;
        let b = ib * 6;
        let c = ic * 6;
        let ux = vertices[b] as f64 - vertices[a] as f64;
        let uy = vertices[b + 1] as f64 - vertices[a + 1] as f64;
        let uz = vertices[b + 2] as f64 - vertices[a + 2] as f64;
        let vx = vertices[c] as f64 - vertices[a] as f64;
        let vy = vertices[c + 1] as f64 - vertices[a + 1] as f64;
        let vz = vertices[c + 2] as f64 - vertices[a + 2] as f64;
        let normal = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
        for start in [a, b, c] {
            for axis in 0..3 {
                vertices[start + 3 + axis] =
                    (vertices[start + 3 + axis] as f64 + normal[axis]) as f32;
            }
        }
    }
    for vertex in vertices.chunks_exact_mut(6) {
        let nx = vertex[3] as f64;
        let ny = vertex[4] as f64;
        let nz = vertex[5] as f64;
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        if length > 0.0 {
            vertex[3] = (nx / length) as f32;
            vertex[4] = (ny / length) as f32;
            vertex[5] = (nz / length) as f32;
        }
    }
}

/// Expand an indexed mesh into flat-shaded stride-6 display vertices, sequential
/// triangle indices, and total surface area.
pub fn expand_flat_triangles(positions: &[f64], indices: &[u32]) -> ExpandedDisplayMesh {
    let tri_count = (indices.len() / 3) * 3;
    let vertex_count = positions.len() / 3;
    let mut out_vertices = vec![0.0f32; tri_count * 6];
    let mut out_indices = vec![0u32; tri_count];
    let mut surface_area = 0.0f64;

    for t in (0..tri_count).step_by(3) {
        let v0 = indices[t] as usize;
        let v1 = indices[t + 1] as usize;
        let v2 = indices[t + 2] as usize;
        if v0 >= vertex_count || v1 >= vertex_count || v2 >= vertex_count {
            continue;
        }
        let i0 = v0 * 3;
        let i1 = v1 * 3;
        let i2 = v2 * 3;
        let p0x = positions[i0];
        let p0y = positions[i0 + 1];
        let p0z = positions[i0 + 2];
        let p1x = positions[i1];
        let p1y = positions[i1 + 1];
        let p1z = positions[i1 + 2];
        let p2x = positions[i2];
        let p2y = positions[i2 + 1];
        let p2z = positions[i2 + 2];
        let ax = p1x - p0x;
        let ay = p1y - p0y;
        let az = p1z - p0z;
        let bx = p2x - p0x;
        let by = p2y - p0y;
        let bz = p2z - p0z;
        let nx = ay * bz - az * by;
        let ny = az * bx - ax * bz;
        let nz = ax * by - ay * bx;
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        surface_area += length * 0.5;
        let inv_len = if length != 0.0 && length.is_finite() {
            1.0 / length
        } else {
            0.0
        };
        let ux = (nx * inv_len) as f32;
        let uy = (ny * inv_len) as f32;
        let uz = (nz * inv_len) as f32;

        let base0 = t * 6;
        let base1 = (t + 1) * 6;
        let base2 = (t + 2) * 6;
        out_indices[t] = t as u32;
        out_indices[t + 1] = (t + 1) as u32;
        out_indices[t + 2] = (t + 2) as u32;

        out_vertices[base0] = p0x as f32;
        out_vertices[base0 + 1] = p0y as f32;
        out_vertices[base0 + 2] = p0z as f32;
        out_vertices[base0 + 3] = ux;
        out_vertices[base0 + 4] = uy;
        out_vertices[base0 + 5] = uz;

        out_vertices[base1] = p1x as f32;
        out_vertices[base1 + 1] = p1y as f32;
        out_vertices[base1 + 2] = p1z as f32;
        out_vertices[base1 + 3] = ux;
        out_vertices[base1 + 4] = uy;
        out_vertices[base1 + 5] = uz;

        out_vertices[base2] = p2x as f32;
        out_vertices[base2 + 1] = p2y as f32;
        out_vertices[base2 + 2] = p2z as f32;
        out_vertices[base2 + 3] = ux;
        out_vertices[base2 + 4] = uy;
        out_vertices[base2 + 5] = uz;
    }

    ExpandedDisplayMesh {
        vertices: out_vertices,
        indices: out_indices,
        surface_area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_triangle_list_splits_sharp_crease_and_averages_coplanar_quad() {
        let positions = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            1.0, 1.0, 0.0, // 2
            0.0, 1.0, 0.0, // 3
            0.0, 0.0, 1.0, // 4 (90-degree wall on x=0)
        ];
        let indices = vec![0, 1, 2, 0, 2, 3, 0, 3, 4];
        let cos_crease = (40.0f64.to_radians()).cos();
        let out = smooth_triangle_list(&positions, &indices, cos_crease);
        assert_eq!(out.positions.len(), 27);
        assert_eq!(out.normals.len(), 27);
        // First two triangles are coplanar with +Z normal
        assert!((out.normals[2] - 1.0).abs() < 1e-6);
        assert!((out.normals[11] - 1.0).abs() < 1e-6);
        // Third triangle is perpendicular (+X normal) and does not blend across 90 deg crease
        assert!((out.normals[18] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fill_indexed_normals_computes_unit_normals() {
        let mut vertices = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
            2.0, 0.0, 0.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, 0.0, 0.0, //
        ];
        let indices = vec![0, 1, 2];
        fill_indexed_normals(&mut vertices, &indices);
        assert_eq!(&vertices[3..6], &[0.0, 0.0, 1.0]);
        assert_eq!(&vertices[9..12], &[0.0, 0.0, 1.0]);
        assert_eq!(&vertices[15..18], &[0.0, 0.0, 1.0]);
    }

    #[test]
    fn expand_flat_triangles_computes_soup_and_surface_area() {
        let positions = vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0];
        let indices = vec![0, 1, 2];
        let out = expand_flat_triangles(&positions, &indices);
        assert_eq!(out.indices, vec![0, 1, 2]);
        assert_eq!(out.vertices.len(), 18);
        assert!((out.surface_area - 3.0).abs() < 1e-12);
        assert_eq!(&out.vertices[3..6], &[0.0, 0.0, 1.0]);
    }
}
