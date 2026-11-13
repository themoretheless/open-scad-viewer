#![doc = include_str!("../README.md")]
//! Triangle BVH construction, borrowed-buffer ray picking and mesh distances.
//! Does not depend on polygon-core, serialization, WASM or an executor.
pub mod body_queries;
pub mod bvh;
pub mod bvh_codec;
pub mod bvh_query;
pub mod bvh_range;
pub mod bvh_sah;
pub mod display;
pub mod distance_bvh;
pub mod proximity;
pub mod scene_bvh;
pub mod semantic_pick;
pub use bvh::{MeshBvh, build_mesh_bvh, build_mesh_bvh_cooperative};
pub use bvh_sah::build_mesh_bvh_sah;
pub use bvh_query::{Hit, Query, QueryError, raycast};
pub use bvh_range::{
    Neighbor, OverlapIter, RefitError, intersect_aabb, nearest, overlap_aabb, overlap_aabb_iter,
    refit, within_radius,
};
pub use math_core::{Error, Result};
pub use mesh_topology::MeshView;
pub use proximity::{
    Deviation, HausdorffReport, closest_point, closest_triangle, hausdorff_distance_approx,
    sample_deviation, signed_distance,
};
fn error(message: impl Into<String>) -> Error {
    Error::new("MESH_QUERY_INVALID_INPUT", message)
}
