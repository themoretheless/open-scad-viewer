//! Exact rational surfaces of revolution. Quarter patches avoid periodic seams
//! and preserve ordinary manifold incidence without collapsed pole edges.
use super::*;
mod loft;
pub(crate) use loft::piecewise_ruled_loft;
pub use loft::ruled_loft;

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[cfg(test)]
#[path = "../tests/analytic_tests.rs"]
mod tests;

mod rational_loft;

pub use rational_loft::section_loft_source_matches;
pub use rational_loft::{
    PolygonStationWalls, SmoothStationWalls, section_loft_surfaces, smooth_polygon_station_walls,
    smooth_station_walls,
};
pub use rational_loft::{
    capped_loft_surfaces, natural_section_loft, periodic_section_loft, rational_section_loft,
};
pub use rational_loft_legacy::{LoftCap, capped_loft_with_caps, capped_loft_with_caps_checked};

mod rational_loft_legacy;

mod cylinder;
mod fibered_body;
mod parametric_edges;
mod revolve_region_angle;
mod sphere_poles;
mod sphere_regular;
pub use cylinder::*;
pub use fibered_body::{
    FiberedBody, FiberedBodyAttempt, FiberedBodyLimits, FiberedBodyReport, construct_fibered_body,
    verify_fibered_body,
};
pub use rational_loft::{
    AffinePose, AffineStationLimits, AffineStationWalls, affine_station_walls,
};
pub(crate) use rational_loft::{
    periodic_material_loft_surfaces, periodic_material_section_loft, retained_section_partition,
};
pub use revolve_region_angle::*;
pub use sphere_poles::{sphere_exact, sphere_pole_charts};

pub use rational_loft::{
    progressive_affine_profile_body, progressive_authored_profile_body,
    progressive_guided_profile_body, progressive_profile_body,
};

pub use rational_loft::{
    EndpointCapCorrection, progressive_profile_body_with_evidence_and_correction,
    progressive_profile_body_with_rmf_policy,
};

pub mod g2_annular;
pub use rational_loft::{
    AnnularCaps, AnnularProgressiveBody, ProgressiveBodyEvidence,
    progressive_profile_body_with_annular_caps, progressive_profile_body_with_automatic_caps,
    progressive_profile_body_with_evidence,
    progressive_profile_body_with_rmf_policy_and_corrections,
};
use revolve_region_angle::revolve_g2_annular_meridian;
