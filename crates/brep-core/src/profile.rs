//! `profile` module family (refactor plan M8).
pub(crate) mod pcurve_trim;
pub mod bezier_profile;
pub(crate) mod exact_affine_source;
pub use crate::planar_cap;
pub(crate) mod section_caps;
pub mod planar_trim;
pub mod prism;
pub mod prism_frame;
pub mod profile_certificate;
pub mod profile_region;
pub(crate) mod rational_identity;
pub mod sketch_wire;
pub(crate) mod stepped_prism;
