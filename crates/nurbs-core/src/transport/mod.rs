//! Optional bounded JSON request boundary.
mod sweep_requests;
mod loft_requests;
mod certificate_requests;
mod offset_requests;
mod exact_primitive_requests;
mod parametric_primitives;
mod surface_offset_requests;
mod editing_and_continuity;
mod core_queries;
mod periodic_surface_edits;
mod periodic_surface_assembly;
mod motion_jumps;
mod trajectory_clearance;
mod radius_clearance;
mod chain_clearance;
mod synchronized_clearance;
mod scheduled_clearance;
mod tool_pair_clearance;
mod tool_workspaces;
mod trajectory_ranges;
mod motion_turns;
mod trimmed_surface_integrals;
mod periodic_trim_components;
mod trim_contour_measurements;
mod periodic_chart_measurements;
use super::*;
use crate::surface::Surface;
#[path="../retained_body_coverage.rs"]
mod retained_body_coverage;
#[path="../profile_bridge_chord_transport.rs"]
mod profile_bridge_chord_transport;
use crate::retained_wall_domain;
use value_codec::{Deserialize, Serialize, Value, json};

mod dispatch;
mod dispatch_other;
mod execute;
#[cfg(test)]
mod scalar_dispatch_regression;
pub use dispatch::*;
use dispatch_other::*;
pub use execute::*;

#[cfg(test)]
#[path = "../surface_offset_transport_tests.rs"]
mod surface_offset_transport_tests;
