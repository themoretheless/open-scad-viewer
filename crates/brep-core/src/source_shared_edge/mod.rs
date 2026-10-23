//! Exact canonical world ownership for original source edges with qualified restrictions.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use cad_predicates::BezierIdentity;
use nurbs_core::{curve::Curve, curve_surface_agreement, Error, Result};

mod shared_edge;
mod same_linear_parameter;
#[cfg(test)]
mod tests;
pub use shared_edge::*;
use same_linear_parameter::*;
