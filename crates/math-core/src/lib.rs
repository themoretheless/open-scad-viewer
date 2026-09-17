//! Small dense linear algebra and accelerated geometry math kernels.

mod acceleration;
mod chamfer;
mod distance_pairs;
mod error;
mod linalg;
mod nearest_neighbor;
mod nearest_two;
mod registration;
mod types;

pub use acceleration::Acceleration;
pub use chamfer::{
    ChamferDistance, DirectedChamfer, chamfer_distance, directed_chamfer_distance,
    directed_hausdorff_distance, hausdorff_distance,
};
pub use distance_pairs::{
    DISTANCE_PAIR_SUM_WGSL, DISTANCE_PAIRS_WGSL, squared_distance_pair_sum,
    squared_distance_pair_sum_accelerated, squared_distance_pairs,
    squared_distance_pairs_accelerated,
};
pub use error::{Error, Result, ensure};
pub use linalg::{
    add, add2, cross, cross2, det, dot, dot2, eigen, finite, mm, mv, norm, norm2, rotation, scale,
    scale2, smallest, solve, sub, sub2, svd, tr, transform_points, unit, unit2,
};
pub use nearest_neighbor::{NEAREST_NEIGHBOR_WGSL, nearest_neighbor, nearest_neighbor_accelerated};
pub use nearest_two::{
    NEAREST_TWO_WGSL, Neighbor, TwoNearest, nearest_two, nearest_two_accelerated,
    nearest_two_first_only, nearest_two_ratios_accelerated,
};
pub use registration::{IcpOptions, IcpReport, RigidTransform, icp_register, rigid_transform};
pub use types::{ID, M3, V2, V3};

#[cfg(feature = "cuda")]
pub mod cuda;
#[cfg(feature = "gpu")]
pub mod gpu;

pub mod camera_gestures;
pub mod orbit_camera;
pub mod viewport;
