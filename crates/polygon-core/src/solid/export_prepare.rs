//! Batch display-mesh preparation for STL/OBJ. File formatting is a host concern.
use crate::{Result, error};
pub struct Prepared {
    pub positions: Vec<f64>,
    pub indices: Vec<u32>,
    pub normals: Vec<f64>,
}
pub fn prepare(
    vertices: &[f32],
    indices: &[u32],
    matrix: &[f32],
    float32: bool,
) -> Result<Prepared> {
    if vertices.len() % 6 != 0
        || indices.len() % 3 != 0
        || matrix.len() != 16
        || !vertices.iter().chain(matrix).all(|v| v.is_finite())
        || matrix[12..] != [0., 0., 0., 1.]
    {
        return Err(error("Mesh export requires finite affine triangle data"));
    }
    let count = vertices.len() / 6;
    if indices.iter().any(|&i| i as usize >= count) {
        return Err(error("Mesh export contains an out-of-range vertex index"));
    }
    if indices.len() / 3 > 750_000 {
        return Err(error("Export exceeds 750000 triangles"));
    }
    let m: Vec<f64> = matrix.iter().map(|&v| v as f64).collect();
    let mirrored = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
        + m[2] * (m[4] * m[9] - m[5] * m[8])
        < 0.;
    let points: Vec<[f64; 3]> = vertices
        .chunks_exact(6)
        .map(|v| {
            std::array::from_fn(|row| {
                let k = row * 4;
                let value =
                    m[k] * v[0] as f64 + m[k + 1] * v[1] as f64 + m[k + 2] * v[2] as f64 + m[k + 3];
                if float32 { value as f32 as f64 } else { value }
            })
        })
        .collect();
    let mut valid = Vec::with_capacity(indices.len());
    let mut normals = Vec::with_capacity(indices.len());
    let mut used = vec![false; count];
    for t in indices.chunks_exact(3) {
        let ids = if mirrored {
            [t[0], t[2], t[1]]
        } else {
            [t[0], t[1], t[2]]
        };
        let [a, b, c] = ids.map(|i| points[i as usize]);
        if !a.iter().chain(&b).chain(&c).all(|v| v.is_finite()) {
            continue;
        }
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = n[0].hypot(n[1]).hypot(n[2]);
        if !length.is_finite() || length == 0. {
            continue;
        }
        for id in ids {
            used[id as usize] = true;
            valid.push(id)
        }
        normals.extend(n.map(|x| x / length));
    }
    let mut mapped = vec![0u32; count];
    let mut positions = Vec::new();
    for (id, p) in points.into_iter().enumerate() {
        if used[id] {
            mapped[id] = (positions.len() / 3) as u32;
            positions.extend(p)
        }
    }
    for id in &mut valid {
        *id = mapped[*id as usize]
    }
    Ok(Prepared {
        positions,
        indices: valid,
        normals,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    const ID: [f32; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    fn vertices() -> Vec<f32> {
        vec![
            0., 0., 0., 0., 0., 1., 1., 0., 0., 0., 0., 1., 0., 1., 0., 0., 0., 1., 999., 999.,
            999., 0., 0., 1.,
        ]
    }
    #[test]
    fn mirrored_winding_and_unused_vertices() {
        let mut m = ID;
        m[0] = -1.;
        let out = prepare(&vertices(), &[0, 1, 2, 0, 0, 1], &m, false).unwrap();
        assert_eq!(out.indices, [0, 2, 1]);
        assert_eq!(out.positions.len(), 9);
        assert_eq!(out.normals, [0., 0., 1.]);
    }
    #[test]
    fn stl_rounding_precedes_degeneracy_and_normals() {
        let mut m = ID;
        m[3] = 1e8;
        m[7] = 1e8;
        assert!(
            prepare(&vertices(), &[0, 1, 2], &m, true)
                .unwrap()
                .indices
                .is_empty()
        );
        assert_eq!(
            prepare(&vertices(), &[0, 1, 2], &m, false)
                .unwrap()
                .indices
                .len(),
            3
        );
    }
    #[test]
    fn malformed_mesh_is_not_silently_filtered() {
        assert!(prepare(&vertices(), &[0, 1, 99], &ID, false).is_err());
        let mut v = vertices();
        v[3] = f32::NAN;
        assert!(prepare(&v, &[0, 1, 2], &ID, false).is_err());
    }
}
