//! Bounded, geometry-only intersection queries on rational curves and surfaces,
//! with the predicate evidence and coverage certificates that accompany their
//! reports. Nothing here knows about B-rep models: `brep-core` builds on this
//! crate, and `brep-intersect` adds the analytic pair intersections on models.
// `unwrap`/`expect` in production code are counted per crate (`clippyUnwrap` pins, plan L6)
// and may only go down; prefer typed errors.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
pub mod analytic_ss;
pub mod coverage_verifier;
pub mod nurbs_ss;
pub mod predicate_evidence;
mod queries;
mod rational_curve;
pub mod vec3;
pub use queries::*;
pub use rational_curve::RationalCurveDefinition;
