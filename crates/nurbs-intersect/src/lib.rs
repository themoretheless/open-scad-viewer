//! Bounded, geometry-only intersection queries on rational curves and surfaces,
//! with the predicate evidence and coverage certificates that accompany their
//! reports. Nothing here knows about B-rep models: `brep-core` builds on this
//! crate, and `brep-intersect` adds the analytic pair intersections on models.
pub mod analytic_ss;
pub mod coverage_verifier;
pub mod nurbs_ss;
pub mod predicate_evidence;
mod queries;
mod rational_curve;
pub use queries::*;
pub use rational_curve::RationalCurveDefinition;
