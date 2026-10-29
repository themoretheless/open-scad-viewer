//! Optional bounded JSON request boundary.
mod sweep_requests;
mod loft_requests;
mod certificate_requests;
mod offset_requests;
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
