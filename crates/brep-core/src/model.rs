//! `model` module family (refactor plan M8).
#[doc(hidden)]
pub mod face_senses;
pub mod feature_family;
pub mod feature_graph;
pub mod feature_history;
pub mod history_snapshots;
pub mod journal;
pub mod keypoints;
pub mod face_domain;
pub mod face_window;
pub mod closed_solid;
#[cfg(feature = "codec")]
pub mod import_policy;
pub mod mixed_root_model;
pub mod root_model;
pub mod transactions;
pub mod transform;
pub(crate) mod transform_plane_charts;
pub(crate) mod model_identity;
pub(crate) mod model_report;
pub mod topology_reference;
pub mod algebraic_model;
