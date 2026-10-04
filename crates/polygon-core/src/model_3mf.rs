//! Compatibility adapters for mesh-io.
use crate::{Mesh, Result};
pub use mesh_io::model_3mf::MAX_MODEL_BYTES;
pub fn export(mesh: &Mesh, parts: &[Mesh]) -> Result<Vec<u8>> {
    let parts: Vec<_> = parts.iter().map(Mesh::view).collect();
    mesh_io::model_3mf::export(&mesh.view(), &parts).map_err(crate::mesh_error)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn solid() -> Mesh {
        Mesh {
            positions: vec![0., 0., 0., 2., 0., 0., 2., 3., 0., 0., 3., 0.],
            indices: vec![0, 1, 2, 0, 2, 3],
            uv: None,
        }
        .thicken([0., 0., 4.])
        .unwrap()
        .mesh
    }
    #[test]
    fn preserves_parts_and_rejects_invalid_part_without_partial_document() {
        let a = solid();
        let mut b = a.clone();
        for p in b.positions.as_chunks_mut::<3>().0 {
            p[0] += 10.;
        }
        let xml = String::from_utf8(export(&a, &[a.clone(), b.clone()]).unwrap()).unwrap();
        assert_eq!(xml.matches("<object id=").count(), 2);
        assert!(xml.contains("<vertex x=\"12\""));
        assert!(xml.ends_with("<item objectid=\"1\"/><item objectid=\"2\"/></build></model>"));
        b.indices.truncate(3);
        assert!(
            export(&a, &[a.clone(), b])
                .unwrap_err()
                .message
                .contains("closed")
        );
        assert!(export(&a, &[]).is_ok());
    }
    #[test]
    fn exported_parts_cannot_bypass_the_aggregate_budget() {
        let a = solid();
        let mut b = a.clone();
        b.positions.resize(900_000, 0.);
        assert!(
            export(&a, &[b, a.clone()])
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
}
