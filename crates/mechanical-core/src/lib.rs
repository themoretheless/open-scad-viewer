#![doc = include_str!("../README.md")]
//! Native mechanical profiles and meshes with optional SCAD/Value compatibility.
mod profiles;
pub mod standard_parts;
mod threads;
pub use profiles::{
    GearInstance, GearOptions, GearProfile, GearReport, PlanetaryOptions, PlanetaryProfiles,
    gear_profile, planetary_profiles,
};
pub use standard_parts::{
    BearingFitClass, BearingFitReport, BevelGearOptions, BevelGearReport, DiscSpringOptions,
    DiscSpringReport, HeatSetInsertSpec, HelicalGearOptions, HelicalGearReport,
    HelicalSpringOptions, HelicalSpringReport, KeywaySpec, MetricFastenerSpec, ORingGrooveSpec,
    PulleyStandard, RackOptions, RetainingRingGrooveSpec, SprocketOptions, TimingPulleyOptions,
    WormDriveOptions, WormDriveReport, bevel_gear_params, disc_spring_load, heat_set_insert_spec,
    helical_gear_params, helical_spring_analysis, hex_nut_profile, involute_rack_profile,
    iso_metric_fastener, iso286_shaft_housing_fit, o_ring_groove_spec, retaining_ring_groove,
    roller_chain_sprocket, shaft_keyway_din6885, timing_pulley_profile, timing_pulley_standard,
    worm_drive_params,
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
