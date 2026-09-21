//! Bake display placement into a Solid exchange mesh without f32 output loss.
use crate::{Result, error};
pub struct PlacedMesh {
    pub positions: Vec<f64>,
    pub indices: Vec<u32>,
}
pub fn place(vertices: &[f32], indices: &[u32], matrix: &[f32]) -> Result<Option<PlacedMesh>> {
    if !vertices.len().is_multiple_of(6) || vertices.len() / 6 < 3 || indices.len() < 3 {
        return Ok(None);
    }
    if matrix.len() != 16
        || !matrix.iter().all(|v| v.is_finite())
        || matrix[12..] != [0., 0., 0., 1.]
    {
        return Err(error(
            "Solid conversion requires a finite affine scene transform.",
        ));
    }
    let m: Vec<f64> = matrix.iter().map(|&v| v as f64).collect();
    let determinant = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
        + m[2] * (m[4] * m[9] - m[5] * m[8]);
    if !determinant.is_finite() || determinant == 0. {
        return Err(error(
            "Solid conversion cannot apply a singular scene transform.",
        ));
    }
    if !indices.len().is_multiple_of(3) || indices.iter().any(|&i| i as usize >= vertices.len() / 6) {
        return Err(error(
            "Solid conversion requires complete triangles with valid vertex indices.",
        ));
    }
    let mut positions = Vec::with_capacity(vertices.len() / 2);
    for vertex in vertices.as_chunks::<6>().0 {
        let [x, y, z] = [vertex[0] as f64, vertex[1] as f64, vertex[2] as f64];
        for row in 0..3 {
            let o = row * 4;
            let p = m[o] * x + m[o + 1] * y + m[o + 2] * z + m[o + 3];
            if !p.is_finite() || p.abs() > 1e6 {
                return Err(error(
                    "Solid scene placement exceeds its coordinate bounds.",
                ));
            }
            positions.push(p);
        }
    }
    let mut indices = indices.to_vec();
    if determinant < 0. {
        for triangle in indices.as_chunks_mut::<3>().0 {
            triangle.swap(1, 2)
        }
    }
    Ok(Some(PlacedMesh { positions, indices }))
}
#[cfg(test)]
mod tests {
    use super::*;
    const ID: [f32; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    fn vertices() -> [f32; 18] {
        [
            0., 0., 0., 0., 0., 1., 1., 0., 0., 0., 0., 1., 0., 1., 0., 0., 0., 1.,
        ]
    }
    #[test]
    fn reflection_and_f64_coordinate_retention() {
        let mut m = ID;
        m[0] = -2.;
        m[3] = 0.1;
        let out = place(&vertices(), &[0, 1, 2], &m).unwrap().unwrap();
        assert_eq!(out.indices, [0, 2, 1]);
        assert_eq!(out.positions[3], -2. + 0.1_f32 as f64);
        assert_ne!(out.positions[3], out.positions[3] as f32 as f64);
    }
    #[test]
    fn malformed_placement_refuses_and_empty_input_stays_empty() {
        assert!(place(&[], &[], &[]).unwrap().is_none());
        assert!(place(&vertices(), &[0, 1, 99], &ID).is_err());
        assert!(place(&vertices(), &[0, 1, 2, 0], &ID).is_err());
        let mut singular = ID;
        singular[0] = 0.;
        assert!(place(&vertices(), &[0, 1, 2], &singular).is_err());
        let mut outside = ID;
        outside[3] = 1e7;
        assert!(place(&vertices(), &[0, 1, 2], &outside).is_err());
    }
}
