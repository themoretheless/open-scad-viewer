//! World placement and exact-coordinate welding for scene exchange meshes.
use crate::{Mesh, Result, check};
use std::collections::BTreeMap;
pub struct Input {
    pub vertices: Vec<f64>,
    pub indices: Vec<usize>,
    pub transform: Vec<f64>,
}
/// Bounds of already placed exchange positions, including unreferenced vertices.
pub fn bounds(groups: &[Vec<f64>]) -> Result<([f64; 3], [f64; 3])> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for positions in groups {
        check(
            positions.len() % 3 == 0,
            "Body bounds require complete coordinate triples.",
        )?;
        for p in positions.as_chunks::<3>().0 {
            check(
                p.iter().all(|v| v.is_finite()),
                "Body bounds require finite coordinates.",
            )?;
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
    }
    check(min.iter().all(|v| v.is_finite()), "Select bodies.")?;
    Ok((min, max))
}
pub fn flatten(meshes: &[Input]) -> Result<Mesh> {
    let mut count = 0usize;
    for mesh in meshes {
        check(
            mesh.indices.len() % 3 == 0,
            "Scene flattening requires complete triangles.",
        )?;
        count = count
            .checked_add(mesh.indices.len() / 3)
            .ok_or_else(|| crate::error("Mesh export exceeds 100000 triangles."))?;
        check(count <= 100_000, "Mesh export exceeds 100000 triangles.")?;
    }
    let mut result = Mesh {
        positions: Vec::new(),
        indices: Vec::new(),
        uv: None,
    };
    let mut vertices = BTreeMap::new();
    for mesh in meshes {
        let m = &mesh.transform;
        check(
            m.len() == 16 && m.iter().all(|v| v.is_finite()) && m[12..] == [0., 0., 0., 1.],
            "Scene flattening requires a finite affine transform.",
        )?;
        check(
            mesh.vertices.len() % 6 == 0
                && mesh.indices.iter().all(|&i| i < mesh.vertices.len() / 6),
            "Malformed scene mesh.",
        )?;
        let mut map = Vec::with_capacity(mesh.vertices.len() / 6);
        for vertex in mesh.vertices.as_chunks::<6>().0 {
            // Keep the established translation-first binary64 evaluation order.
            let p: [f64; 3] = std::array::from_fn(|r| {
                m[r * 4 + 3]
                    + m[r * 4] * vertex[0]
                    + m[r * 4 + 1] * vertex[1]
                    + m[r * 4 + 2] * vertex[2]
            });
            check(
                p.iter().all(|v| v.is_finite()),
                "Nonfinite flattened scene coordinate.",
            )?;
            let key = p.map(|v| if v == 0. { 0 } else { v.to_bits() });
            let index = if let Some(&index) = vertices.get(&key) {
                index
            } else {
                let index = result.positions.len() / 3;
                check(
                    index < crate::MAX_VERTICES,
                    "Flattened scene exceeds 300000 vertices.",
                )?;
                result.positions.extend(p);
                vertices.insert(key, index);
                index
            };
            map.push(index);
        }
        let determinant = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
            + m[2] * (m[4] * m[9] - m[5] * m[8]);
        check(
            determinant.is_finite(),
            "Nonfinite scene transform determinant.",
        )?;
        for t in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [map[t[0]], map[t[1]], map[t[2]]];
            result.indices.extend(if determinant < 0. {
                [a, c, b]
            } else {
                [a, b, c]
            });
        }
    }
    Ok(result)
}
/// Resize a selection about its shared minimum corner.
pub fn resize_matrix(groups: &[Vec<f64>], desired: [f64; 3]) -> Result<[[f64; 4]; 4]> {
    let (min, max) = bounds(groups)?;
    let mut matrix = [[0.; 4]; 4];
    matrix[3][3] = 1.;
    for axis in 0..3 {
        let span = max[axis] - min[axis];
        check(
            desired[axis].is_finite() && desired[axis] > 0. && span.is_finite() && span >= 1e-8,
            "Dimensions must be positive.",
        )?;
        let scale = desired[axis] / span;
        let offset = min[axis] - min[axis] * scale;
        check(
            scale.is_finite() && scale > 0. && offset.is_finite(),
            "Resize transform exceeds finite numeric range.",
        )?;
        matrix[axis][axis] = scale;
        matrix[axis][3] = offset;
    }
    Ok(matrix)
}
pub fn mirror_matrix(origin: [f64; 3], axis: [f64; 3]) -> Result<[[f64; 4]; 4]> {
    check(
        origin.iter().chain(&axis).all(|v| v.is_finite()),
        "Mirror requires finite plane coordinates.",
    )?;
    let scale = axis.iter().map(|v| v.abs()).fold(0., f64::max);
    check(scale > 0., "Mirror requires a nonzero plane normal.")?;
    let n = axis.map(|v| v / scale);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let n = n.map(|v| v / length);
    let distance = origin.iter().zip(n).map(|(a, b)| a * b).sum::<f64>();
    let mut matrix = [[0.; 4]; 4];
    matrix[3][3] = 1.;
    for i in 0..3 {
        for j in 0..3 {
            matrix[i][j] = if i == j { 1. } else { 0. };
            matrix[i][j] -= 2. * n[i] * n[j];
        }
        matrix[i][3] = 2. * distance * n[i];
    }
    check(
        matrix.iter().flatten().all(|v| v.is_finite()),
        "Mirror transform exceeds finite numeric range.",
    )?;
    Ok(matrix)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mirror_plane_is_scale_invariant_and_rejects_zero_axis() {
        let expected = [
            [-1., 0., 0., 6.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        for scale in [f64::from_bits(1), 1., f64::MAX] {
            assert_eq!(
                mirror_matrix([3., 0., 0.], [scale, 0., 0.]).unwrap(),
                expected
            );
        }
        assert!(mirror_matrix([0.; 3], [0.; 3]).is_err());
    }
    #[test]
    fn resize_uses_one_selection_pivot_and_refuses_flat_axes() {
        assert_eq!(
            resize_matrix(&[vec![10., 20., 30., 20., 40., 60.]], [2., 4., 6.]).unwrap(),
            [
                [0.2, 0., 0., 8.],
                [0., 0.2, 0., 16.],
                [0., 0., 0.2, 24.],
                [0., 0., 0., 1.]
            ]
        );
        assert!(resize_matrix(&[vec![0., 0., 0., 1., 1., 0.]], [1.; 3]).is_err());
        assert!(resize_matrix(&[vec![0., 0., 0., 1., 1., 1.]], [0., 1., 1.]).is_err());
    }
    fn mesh() -> Input {
        Input {
            vertices: vec![
                0., 0., 0., 0., 0., 1., 1., 0., 0., 0., 0., 1., 0., 1., 0., 0., 0., 1.,
            ],
            indices: vec![0, 1, 2],
            transform: vec![
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
        }
    }
    #[test]
    fn exact_welding_and_reflected_winding() {
        let a = mesh();
        let mut b = mesh();
        b.transform[0] = -1.;
        let r = flatten(&[a, b]).unwrap();
        assert_eq!(
            r.positions,
            vec![0., 0., 0., 1., 0., 0., 0., 1., 0., -1., 0., 0.]
        );
        assert_eq!(r.indices, vec![0, 1, 2, 0, 2, 3]);
    }
    #[test]
    fn malformed_inputs_refuse_without_partial_output() {
        let mut b = mesh();
        b.indices[0] = 99;
        assert!(flatten(&[mesh(), b]).is_err());
        let mut b = mesh();
        b.transform[12] = 1.;
        assert!(flatten(&[b]).is_err());
        let mut b = mesh();
        b.indices.push(0);
        assert!(flatten(&[b]).is_err());
    }
    #[test]
    fn body_bounds_cover_all_groups_and_validate_coordinates() {
        assert_eq!(
            bounds(&[vec![], vec![1., 2., 3., -4., 5., 6.], vec![7., -8., 9.]]).unwrap(),
            ([-4., -8., 3.], [7., 5., 9.])
        );
        assert!(bounds(&[]).is_err());
        assert!(bounds(&[vec![0.; 4]]).is_err());
        assert!(bounds(&[vec![0., 0., 0., f64::INFINITY, 1., 2.]]).is_err());
    }
}
