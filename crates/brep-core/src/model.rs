//! `model` module family (refactor plan M8).
pub(crate) mod body_edit;
pub mod boolean_robustness;
pub(crate) mod boolean_support;
pub mod closed_solid;
pub mod face_domain;
pub mod face_injectivity;
#[doc(hidden)]
pub mod face_senses;
pub mod face_window;
pub mod keypoints;
pub mod mixed_root_model;
pub(crate) mod model_identity;
pub(crate) mod model_report;
pub mod ray_parity;
pub mod root_model;
pub mod topology_reference;
pub mod transform;
pub(crate) mod transform_plane_charts;
