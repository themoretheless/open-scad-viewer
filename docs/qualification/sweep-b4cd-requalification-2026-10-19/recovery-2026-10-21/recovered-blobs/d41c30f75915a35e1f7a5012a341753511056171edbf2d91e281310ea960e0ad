//! Compatibility adapters for mesh-section.
use crate::{Mesh, Result};
pub use mesh_section::{MeshSection, SectionContour};
use planar_geometry::rings::Rings;
#[derive(Debug)]
pub struct MeshSectionIndex(mesh_section::MeshSectionIndex);
impl MeshSectionIndex {
    pub fn new(mesh: &Mesh) -> Result<Self> {
        mesh_section::MeshSectionIndex::new(&mesh.view())
            .map(Self)
            .map_err(crate::mesh_error)
    }
    pub fn section(&self, z: f64) -> Result<MeshSection> {
        self.0.section(z).map_err(crate::mesh_error)
    }
    pub fn section_for_display(&self, z: f64) -> Result<(MeshSection, Vec<usize>)> {
        self.0.section_for_display(z).map_err(crate::mesh_error)
    }
}
pub fn project(mesh: &Mesh) -> Result<Rings> {
    mesh_section::project(&mesh.view()).map_err(crate::mesh_error)
}
pub fn slice(mesh: &Mesh, z: f64) -> Result<Rings> {
    mesh_section::slice(&mesh.view(), z).map_err(crate::mesh_error)
}
