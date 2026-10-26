//! Assemble an immutable face fragment from audited UV loops and canonical
//! world edges. No welding, curve reversal, fitting or solid admission occurs.
use crate::{Coedge, Edge, Face, Loop, Vertex};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{
    Error, Result, curve::Curve, curve_surface_agreement as agreement, surface::Surface,
    trim_region_audit,
};

mod boundary;
#[cfg(feature = "codec")]
mod locate_source_contact_points;
#[cfg(test)]
mod tests;
pub use boundary::*;
#[cfg(feature = "codec")]
pub use locate_source_contact_points::*;
