//! Involute gears as trimmed NURBS solids: spur, helical and herringbone,
//! external with an optional bore or internal (ring) with a rim.
//!
//! The tooth outline is authored per tooth as exact rational arcs (tip and
//! root circles), radial lines below the base circle, and involute flanks
//! approximated by quintic Hermite spans with an analytic real-arithmetic
//! remainder at most `FLANK_FIT`. The report separately records inclusive
//! flank bounds against stored parameters and the complete nominal spec. Each outline
//! curve becomes one wall face: a ruled surface for a spur gear, or for a
//! helical gear a quintic Hermite sweep with a continuous real-arithmetic
//! remainder at most `SWEEP_FIT`; the report flags inclusive saved-wall and
//! nominal twist bounds separately from ideal profile accuracy.
//! A herringbone gear is two mirrored helical halves sharing the mid-plane
//! outline. Caps are planar faces bounded by the outline and its rotated
//! image, with affine pcurves. All edges are iso-curves of the wall
//! surfaces, so the body is self-consistent to the kernel tolerance; the
//! stated fit bounds describe how far the surfaces sit from the ideal
//! involute helicoid, which no polynomial surface represents exactly.
//! Line/arc endpoints are canonicalized to exact shared ownership; their
//! continuous displacement is reported, while all certified flank poles remain.
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::f64::consts::{PI, TAU};

/// Accepted flank deviation budget; report flags identify the certified scope.
const FLANK_FIT: f64 = 1e-6;
/// Largest deviation of a lofted helical wall from the exact helical sweep.
const SWEEP_FIT: f64 = 1e-4;
const ARC_WEIGHT_MAX_SWEEP: f64 = PI - 1e-6;


mod gear_spec;
mod gear_with_report;
#[cfg(test)]
mod tests;
pub use gear_spec::*;
pub use gear_with_report::*;
#[path = "../gear_profile_join.rs"]
mod profile_join;

#[path = "../gear_arc_radius.rs"]
mod arc_radius;

#[path = "../gear_profile_parameter.rs"]
mod profile_parameter;

#[path = "../gear_profile_embedding.rs"]
mod profile_embedding;
pub use profile_embedding::{
    ProfileEmbeddingCertificate, ProfileEmbeddingLimits, ProfileVerifiedGear, VerifiedGearProfile,
    certify_grouped_profile_embedding, certify_profile_embedding, gear_with_verified_profile,
    verify_profile_embedding,
};

#[path = "../bevel_profile.rs"]
mod bevel_profile;
pub use bevel_profile::{
    VerifiedBevelProfile, certify_bevel_profile, verify_bevel_profile_projection,
};

#[path = "../bevel_body.rs"]
mod bevel_body;
pub use bevel_body::{BevelBodyCandidate, bevel_body_candidate};
pub use bevel_body::{BevelBuildLimits, ProfileVerifiedBevelGear, bevel_with_verified_profile};
