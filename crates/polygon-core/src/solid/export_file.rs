//! Compatibility adapter preserving the scene-export error contract.
use crate::Result;
pub use mesh_io::export_file::MAX_BYTES;
pub struct Builder(mesh_io::export_file::Builder);
impl Builder {
    pub fn new(binary: bool, name: &str) -> Self {
        Self(mesh_io::export_file::Builder::new(binary, name))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn poison(&mut self) {
        self.0.poison()
    }
    pub fn append(
        &mut self,
        vertices: &[f64],
        indices: &[u32],
        matrix: &[f64],
        limit: usize,
    ) -> Result<()> {
        self.0
            .append(vertices, indices, matrix, limit)
            .map_err(crate::mesh_error)
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        self.0.finish().map_err(crate::mesh_error)
    }
}
