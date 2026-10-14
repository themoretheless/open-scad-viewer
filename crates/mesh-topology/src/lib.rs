#![doc = include_str!("../README.md")]
//! Indexed edge incidence, boundary loops and display edge extraction.
//! Full manifoldness (including vertex links) and repair live in manifold-core.
mod boundary;
pub mod edges;
pub mod keypoints;
pub mod soup;
pub mod planar;
mod indexed;
mod inspection;
pub mod measure;
pub mod weld;
mod view;
pub use edges::{
    SemanticEdgeDiagnostics, SemanticEdges, extract_semantic_edges,
    extract_semantic_edges_cooperative,
};
pub use indexed::{EdgeCounts, EdgeUses};
pub use inspection::Inspection;
pub use math_core::{Error, Result};
pub use view::MeshView;

pub const MAX_MESH_TRIANGLES: usize = 100_000;
pub const MAX_VERTICES: usize = 300_000;
fn error(message: impl Into<String>) -> Error {
    Error::new("MESH_INVALID_INPUT", message)
}
fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, "MESH_INVALID_INPUT", message)
}
