//! Exact rational surfaces of revolution. Quarter patches avoid periodic seams
//! and preserve ordinary manifold incidence without collapsed pole edges.
use super::*;
mod loft;
pub(crate) use loft::piecewise_ruled_loft;
pub use loft::ruled_loft;

const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;


#[cfg(test)]
#[path="../tests/analytic_tests.rs"]
mod tests;

mod rational_loft;

pub use rational_loft_legacy::{capped_loft_with_caps, capped_loft_with_caps_checked, LoftCap};
pub use rational_loft::{SmoothStationWalls, smooth_station_walls, section_loft_surfaces, PolygonStationWalls, smooth_polygon_station_walls};
pub use rational_loft::{rational_section_loft, natural_section_loft, capped_loft_surfaces, periodic_section_loft};
pub use rational_loft::section_loft_source_matches;

mod rational_loft_legacy;

mod cylinder;
mod revolve_region_angle;
mod sphere_poles;
pub use cylinder::*;
pub use sphere_poles::{sphere_exact, sphere_pole_charts};
pub use revolve_region_angle::*;

pub use rational_loft::{progressive_guided_profile_body,progressive_authored_profile_body,progressive_affine_profile_body,progressive_profile_body};

pub use rational_loft::{progressive_profile_body_with_evidence_and_correction,EndpointCapCorrection,progressive_profile_body_with_rmf_policy};
