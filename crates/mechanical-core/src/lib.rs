#![doc = include_str!("../README.md")]
//! Native mechanical profiles and meshes with optional SCAD/Value compatibility.
mod profiles;
mod threads;
pub use profiles::{
    GearInstance, GearOptions, GearProfile, GearReport, PlanetaryOptions, PlanetaryProfiles,
    gear_profile, planetary_profiles,
};
pub use threads::{ThreadGeometry, ThreadOptions, thread_mesh, thread_radius_at};
pub type Point = [f64; 2];
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub path: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}
impl std::error::Error for Error {}
fn err(path: &str, message: impl Into<String>) -> Error {
    Error {
        path: path.into(),
        message: message.into(),
    }
}
#[cfg(feature = "codec")]
mod codec;
#[cfg(feature = "codec")]
pub use codec::{
    Generated, append_number, extrude, gear, number, planetary, thread, thread_geometry,
    thread_radius,
};
