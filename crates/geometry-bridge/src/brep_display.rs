//! Native display conversion; these mesh metrics are estimates, not B-rep certificates.
use super::{Mesh, Result, Value, field, input};
use std::collections::BTreeMap;
use value_codec::json;
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn prepare(mesh: &Mesh) -> Result<(Vec<f32>, Vec<usize>, f64)> {
    mesh.validate()?;
    if mesh.indices.len() / 3 > 20_000 {
        return Err(input("B-rep scene exceeds 20000 display triangles"));
    }
    let mut rounded = BTreeMap::new();
    for point in mesh.positions.as_chunks::<3>().0 {
        let float: [f32; 3] = std::array::from_fn(|i| point[i] as f32);
        if float.iter().any(|x| !x.is_finite()) {
            return Err(input("B-rep display exceeds finite Float32 coordinates"));
        }
        let key = float.map(|x| if x == 0. { 0 } else { x.to_bits() });
        let native: [u64; 3] = std::array::from_fn(|i| {
            if point[i] == 0. {
                0
            } else {
                point[i].to_bits()
            }
        });
        if rounded
            .insert(key, native)
            .is_some_and(|previous| previous != native)
        {
            return Err(input(
                "Distinct B-rep vertices merge in Float32 display coordinates",
            ));
        }
    }
    let mut vertices = Vec::with_capacity(mesh.indices.len() * 6);
    let mut area = 0.;
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let points: [[f64; 3]; 3] = std::array::from_fn(|corner| {
            std::array::from_fn(|axis| mesh.positions[triangle[corner] * 3 + axis])
        });
        let a = std::array::from_fn(|i| points[1][i] - points[0][i]);
        let b = std::array::from_fn(|i| points[2][i] - points[0][i]);
        let normal = cross(a, b);
        let length = normal[0].hypot(normal[1]).hypot(normal[2]);
        if !length.is_finite() || length <= 0. {
            return Err(input("B-rep display triangle is non-finite or degenerate"));
        }
        area += length / 2.;
        let unit = normal.map(|x| x / length);
        let offset = vertices.len();
        for point in points {
            vertices.extend(point.map(|x| x as f32));
            vertices.extend(unit.map(|x| x as f32));
        }
        let a = std::array::from_fn(|i| {
            f64::from(vertices[offset + 6 + i]) - f64::from(vertices[offset + i])
        });
        let b = std::array::from_fn(|i| {
            f64::from(vertices[offset + 12 + i]) - f64::from(vertices[offset + i])
        });
        let rounded_normal = cross(a, b);
        let orientation = (0..3).map(|i| rounded_normal[i] * unit[i]).sum::<f64>();
        if !orientation.is_finite() || orientation <= 0. {
            return Err(input(
                "B-rep display triangle collapses or reverses in Float32 coordinates",
            ));
        }
    }
    if !area.is_finite() {
        return Err(input("B-rep display metrics are non-finite"));
    }
    Ok((vertices, (0..mesh.indices.len()).collect(), area))
}
pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let tessellation = super::brep::nurbs(&field(&v, "model")?, field(&v, "segments")?)?;
    let (vertices, indices, area) = prepare(&tessellation.built.mesh)?;
    let mut value = json!({
        "report":tessellation.built.report,
        "faceIds":tessellation.face_ids,
        "displayVertices":vertices,
        "displayIndices":indices,
        "surfaceArea":area
    });
    if let Some(ids) = tessellation.topology_face_ids {
        value["topologyFaceIds"] = json!(ids);
    }
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn triangle(positions: Vec<f64>) -> Mesh {
        Mesh {
            positions,
            indices: vec![0, 1, 2],
            uv: None,
        }
    }
    #[test]
    fn prepares_flat_normals_and_area() {
        let (vertices, indices, area) =
            prepare(&triangle(vec![0., 0., 0., 2., 0., 0., 0., 3., 0.])).unwrap();
        assert_eq!(area, 3.);
        assert_eq!(indices, vec![0, 1, 2]);
        assert_eq!(&vertices[3..6], &[0., 0., 1.]);
        assert_eq!(vertices.len(), 18);
    }
    #[test]
    fn refuses_float32_merges_collapse_and_overflow() {
        for positions in [
            vec![1e8, 0., 0., 1e8 + 1., 0., 0., 1e8, 1., 0.],
            vec![0., 0., 0., 1., 1., 0., 2., 2. + 1e-9, 0.],
            vec![1e40, 0., 0., 0., 1., 0., 0., 0., 1.],
        ] {
            assert!(
                prepare(&triangle(positions))
                    .unwrap_err()
                    .message
                    .contains("Float32")
            );
        }
    }
}
