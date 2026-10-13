//! Sufficient shared-boundary certificate using original control points.
//! One surface lies in a plane; the other has exactly one boundary control row
//! in that plane and remaining control points on the same side. When zeros
//! occur off that boundary, a complete opposite Bezier row must be strict.
//! Positive rational basis weights then forbid any off-boundary contact.
use crate::Model;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
use nurbs_core::{curve::Curve, surface::Surface};
mod monotone_coordinate;

mod certificate;
mod separates_surfaces;
#[cfg(test)]
mod tests;
pub use certificate::*;
pub use separates_surfaces::*;
