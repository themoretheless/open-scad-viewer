//! Compatibility adapters for mesh-io.
use crate::{Mesh, Result};
pub fn export(mesh: &Mesh, parts: &[Mesh], compressed: bool) -> Result<Vec<u8>> {
    let parts: Vec<_> = parts.iter().map(Mesh::view).collect();
    mesh_io::package_3mf::export(&mesh.view(), &parts, compressed).map_err(crate::mesh_error)
}
