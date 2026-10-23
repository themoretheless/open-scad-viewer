//! Compatibility adapter for mesh-io.
use crate::Result;
pub use mesh_io::export_prepare::Prepared;
pub fn prepare(
    vertices: &[f64],
    indices: &[u32],
    matrix: &[f64],
    float32: bool,
) -> Result<Prepared> {
    mesh_io::export_prepare::prepare(vertices, indices, matrix, float32).map_err(crate::mesh_error)
}
