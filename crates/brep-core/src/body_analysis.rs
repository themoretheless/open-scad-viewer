//! `body_analysis` module family (refactor plan M8).
pub mod analysis;
pub mod face_curvature;
pub mod face_injectivity;
pub mod watertight;
#[cfg(feature = "codec")]
pub mod defects;
pub mod ray_parity;
pub mod shell_distance;
pub mod shell_nesting;
pub mod shell_orientation;
pub mod shell_relation;
pub mod shell_thickness;
pub mod solid_audit;
pub mod solid_distance;
pub mod volume_validity;
pub(crate) mod model_validation;
