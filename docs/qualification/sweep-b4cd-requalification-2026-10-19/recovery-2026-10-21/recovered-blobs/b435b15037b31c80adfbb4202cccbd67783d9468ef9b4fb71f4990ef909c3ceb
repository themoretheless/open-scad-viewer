//! Compatibility adapters for mesh-io.
use crate::{Mesh, Result};
pub use mesh_io::mesh_export::MAX_BYTES;
pub fn export(mesh: &Mesh, format: &str) -> Result<Vec<u8>> {
    mesh_io::export(&mesh.view(), format).map_err(crate::mesh_error)
}
pub fn export_print_mesh(mesh: &Mesh, format: &str) -> Result<Vec<u8>> {
    mesh_io::export_print_mesh(&mesh.view(), format).map_err(crate::mesh_error)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Mesh {
        Mesh {
            positions: vec![0., 0., 0., 2., 0., 0., 2., 3., 0., 0., 3., 0.],
            indices: vec![0, 1, 2, 0, 2, 3],
            uv: None,
        }
    }
    #[test]
    fn open_mesh_admission_and_index_conventions() {
        let mesh = square();
        for format in ["stl", "stl_binary", "amf"] {
            assert!(
                export(&mesh, format)
                    .unwrap_err()
                    .message
                    .contains("closed")
            );
        }
        let obj = String::from_utf8(export(&mesh, "obj").unwrap()).unwrap();
        assert!(obj.ends_with("f 1 2 3\nf 1 3 4\n"));
        for format in ["off", "ply"] {
            let text = String::from_utf8(export(&mesh, format).unwrap()).unwrap();
            assert!(text.ends_with("3 0 1 2\n3 0 2 3\n"));
        }
        let mut malformed = mesh.clone();
        malformed.indices[0] = 99;
        for format in ["stl", "stl_binary", "obj", "ply", "off", "amf"] {
            assert!(export(&malformed, format).is_err());
        }
        assert!(export(&mesh, "unknown").is_err());
    }
    #[test]
    fn binary_stl_records_and_float32_collapse() {
        let mesh = square().thicken([0., 0., 4.]).unwrap().mesh;
        let data = export(&mesh, "stl_binary").unwrap();
        assert_eq!(data.len(), 84 + 12 * 50);
        assert_eq!(&data[80..84], &12u32.to_le_bytes());
        // First face points downward, with source indexing preserved.
        assert_eq!(f32::from_le_bytes(data[92..96].try_into().unwrap()), -1.);
        assert_eq!(&data[132..134], &[0, 0]);
        let mut shifted = mesh.clone();
        for p in shifted.positions.as_chunks_mut::<3>().0 {
            p[0] += 100_000_000.;
        }
        assert!(
            export(&shifted, "stl_binary")
                .unwrap_err()
                .message
                .contains("collapse")
        );
        assert!(export(&shifted, "stl").is_ok());
    }
    #[test]
    fn print_mesh_trio_covers_stl_obj_and_3mf() {
        let mesh = square().thicken([0., 0., 4.]).unwrap().mesh;
        for format in ["stl", "obj", "3mf"] {
            let bytes = export_print_mesh(&mesh, format).unwrap();
            assert!(!bytes.is_empty());
        }
        assert!(
            export_print_mesh(&square(), "stl")
                .unwrap_err()
                .message
                .contains("closed")
        );
        assert!(export_print_mesh(&mesh, "ply").is_err());
    }
}
