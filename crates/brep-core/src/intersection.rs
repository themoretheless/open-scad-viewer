//! `intersection` module family (refactor plan M8).
pub mod face_crossings;
pub mod intersections;
pub mod nurbs_ss_g6;
pub mod nurbs_ss_general;
pub mod ss_face_membership;
pub mod ss_trimmed_face;
#[cfg(feature = "codec")]
pub mod profile_intersections;
pub mod self_intersection;
