//! Mesh reconstruction helpers. Exact-coordinate welding never guesses a tolerance.
#[cfg(feature = "codec")]
#[path = "proximity/serialization.rs"]
mod serialization;
use crate::{Mesh, Result};
pub type Point = math_core::V3;
pub fn weld_exact(mesh: &Mesh) -> Result<Mesh> {
    let result = mesh_topology::weld::exact(&mesh.view()).map_err(crate::mesh_error)?;
    Ok(Mesh {
        positions: result.positions,
        indices: result.indices,
        uv: None,
    })
}
pub fn valid_source(mesh: &Mesh, max_triangles: usize) -> Result<Mesh> {
    let result = mesh_query::proximity::valid_source(&mesh.view(), max_triangles)
        .map_err(crate::mesh_error)?;
    Ok(Mesh {
        positions: result.positions,
        indices: result.indices,
        uv: None,
    })
}
pub use mesh_query::proximity::closest_triangle;
pub fn closest_point(mesh: &Mesh, p: Point) -> (Point, f64) {
    mesh_query::closest_point(&mesh.view(), p)
}
pub fn signed_distance(mesh: &Mesh, p: Point) -> f64 {
    mesh_query::signed_distance(&mesh.view(), p)
}
#[derive(Clone, Debug)]
pub struct Deviation {
    pub sampled_max_mm: f64,
    pub sampled_rms_mm: f64,
    pub sample_count: usize,
    pub error_bound_certified: bool,
}

/// Bidirectional samples at vertices and triangle centroids, not Hausdorff proof.
pub fn sample_deviation(a: &Mesh, b: &Mesh) -> Result<Deviation> {
    let report = mesh_query::sample_deviation(&a.view(), &b.view()).map_err(crate::mesh_error)?;
    Ok(Deviation {
        sampled_max_mm: report.sampled_max_mm,
        sampled_rms_mm: report.sampled_rms_mm,
        sample_count: report.sample_count,
        error_bound_certified: report.error_bound_certified,
    })
}
