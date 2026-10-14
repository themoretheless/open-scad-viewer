#![doc = include_str!("../README.md")]
//! Triangle BVH construction, borrowed-buffer ray picking and mesh distances.
//! Does not depend on polygon-core, serialization, WASM or an executor.
pub mod bvh;
pub mod bvh_query;
pub mod display;
pub mod distance_bvh;
pub mod proximity;
pub mod scene_bvh;
pub use bvh::{MeshBvh, build_mesh_bvh, build_mesh_bvh_cooperative};
pub use bvh_query::{Hit, Query, QueryError, raycast};
pub use math_core::{Error, Result};
pub use mesh_topology::MeshView;
pub use proximity::{
    Deviation, closest_point, closest_triangle, sample_deviation, signed_distance,
};
fn error(message: impl Into<String>) -> Error {
    Error::new("MESH_QUERY_INVALID_INPUT", message)
}
