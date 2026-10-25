//! Pack normalized face polygons into the existing triangle-fan mesh convention.
//! Concave faces retain the caller's fan semantics; topology validation belongs to the mesh kernel.
#[derive(Debug, Clone, PartialEq)]
pub struct PackedFaces {
    pub vertices: Vec<f64>,
    pub indices: Vec<u32>,
    pub usable: bool,
}
/// Reverse authored face winding while preserving polygon and corner order.
pub fn reversed_fans(polygons: &[Vec<[f64; 3]>]) -> crate::Result<PackedFaces> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for polygon in polygons {
        if polygon.len() < 3 {
            continue;
        }
        let base = u32::try_from(vertices.len() / 3)
            .map_err(|_| crate::fail("Polygon mesh index budget exceeded"))?;
        let count = u32::try_from(polygon.len())
            .map_err(|_| crate::fail("Polygon mesh index budget exceeded"))?;
        base.checked_add(count)
            .ok_or_else(|| crate::fail("Polygon mesh index budget exceeded"))?;
        vertices.extend(polygon.iter().flatten().copied());
        for index in 1..count - 1 {
            indices.extend([base, base + index + 1, base + index]);
        }
    }
    let usable = !indices.is_empty() && vertices.iter().all(|value| (*value as f32).is_finite());
    Ok(PackedFaces {
        vertices,
        indices,
        usable,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fans_keep_face_order_and_reverse_winding() {
        let packed = reversed_fans(&[
            vec![[0.; 3]; 2],
            vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            vec![[2., 0., 0.], [3., 0., 0.], [2., 1., 0.]],
        ])
        .unwrap();
        assert_eq!(packed.indices, vec![0, 2, 1, 0, 3, 2, 4, 6, 5]);
        assert_eq!(packed.vertices.len(), 21);
        assert!(packed.usable);
        for invalid in [f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(!reversed_fans(&[vec![[invalid, 0., 0.]; 3]]).unwrap().usable);
        }
        assert!(!reversed_fans(&[]).unwrap().usable);
    }
}
