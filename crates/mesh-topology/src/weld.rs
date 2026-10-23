//! Exact-coordinate welding in first-use order; nearby points stay distinct.
use crate::{MeshView, Result};
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub struct Welded {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
}
impl Welded {
    pub fn view(&self) -> MeshView<'_> {
        MeshView::new(&self.positions, &self.indices)
    }
}
/// Validate all input buffers, discard unused vertices and UVs, and share only
/// identical coordinates. Signed zeros share identity; first-use values survive.
pub fn exact(mesh: &MeshView<'_>) -> Result<Welded> {
    mesh.validate()?;
    let mut positions = Vec::new();
    let mut map = BTreeMap::new();
    let mut indices = Vec::with_capacity(mesh.indices.len());
    for &index in mesh.indices {
        let point = &mesh.positions[index * 3..index * 3 + 3];
        let key = std::array::from_fn::<_, 3, _>(|axis| {
            if point[axis] == 0. {
                0
            } else {
                point[axis].to_bits()
            }
        });
        let next = positions.len() / 3;
        let id = *map.entry(key).or_insert_with(|| {
            positions.extend_from_slice(point);
            next
        });
        indices.push(id);
    }
    let result = Welded { positions, indices };
    result.view().validate()?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_first_use_signed_zero_and_nearby_points_without_mutation() {
        let positions = [99., 99., 99., -0., 0., 0., 0., -0., 0., 1e-15, 0., 0.];
        let indices = [2, 1, 3];
        let result = exact(&MeshView::new(&positions, &indices)).unwrap();
        assert_eq!(result.indices, [0, 0, 1]);
        assert_eq!(result.positions.len(), 6);
        assert_eq!(result.positions[1].to_bits(), (-0_f64).to_bits());
        assert_eq!(positions[0], 99.);
        let mut view = MeshView::new(&positions, &indices);
        view.uv = Some(&[0.]);
        assert!(exact(&view).is_err());
    }
}
