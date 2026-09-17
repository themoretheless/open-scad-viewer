//! Small dense linear algebra and accelerated geometry math kernels.

mod acceleration;
mod distance_pairs;
mod error;
mod linalg;
mod nearest_neighbor;
mod registration;
mod types;

pub use acceleration::Acceleration;
pub use distance_pairs::{
    DISTANCE_PAIRS_WGSL, squared_distance_pairs, squared_distance_pairs_accelerated,
};
pub use error::{Error, Result, ensure};
pub use linalg::{
    add, add2, cross, cross2, det, dot, dot2, eigen, finite, mm, mv, norm, norm2, rotation, scale,
    scale2, smallest, solve, sub, sub2, svd, tr, transform_points, unit, unit2,
};
pub use nearest_neighbor::{NEAREST_NEIGHBOR_WGSL, nearest_neighbor, nearest_neighbor_accelerated};
pub use registration::{IcpOptions, IcpReport, RigidTransform, icp_register, rigid_transform};
pub use types::{ID, M3, V2, V3};

#[cfg(feature = "cuda")]
pub mod cuda;
#[cfg(feature = "gpu")]
pub mod gpu;

pub mod camera_gestures;
pub mod orbit_camera;
pub mod viewport;
