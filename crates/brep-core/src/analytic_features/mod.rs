//! Analytic fillet / chamfer / shell / solid loft / IGES (P3/P5/F5/F6).
//!
//! Faceted blends in `operations` remain available but must not be relabeled analytic.
//! Each capability publishes a `FeatureCertificate` only on the frozen positive matrix.
//! STEP AP214/AP242 topology roundtrip lives in `crate::step_interchange`.

#[cfg(feature = "codec")]
mod serialization;
use crate::analytic::ruled_loft;
#[cfg(test)]
use crate::cylinder;
use crate::operations::{boolean, extrude_polygon};
use crate::predicate_evidence::{ComposedEvidence, PredicateEvidence, compose_predicate_evidence};
use crate::solid_audit::{SolidAuditCertificate, audit_solid};
use crate::{ChangeSet, Model, cuboid, tube};
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::{Error, Result};

/// Finite successor cell: exact, constant-radius construction on one or more
/// vertical edges of an audited axis-aligned cuboid.
pub const AUDITED_MULTI_EDGE_FILLET_CAPABILITY: &str = "analytic-multi-edge-fillet/1";
/// Exact equal-distance support-plane chamfer on a connected selection in an
/// audited convex planar-faced polyhedron.
pub const EXACT_CONVEX_CHAMFER_CAPABILITY: &str = "exact-convex-straight-edge-chamfer/1";
/// Exact constant-radius cylindrical rounds on arbitrary selected longitudinal
/// edges of an audited strictly-convex planar prism under rigid placement.
pub const EXACT_CONVEX_PRISM_FILLET_CAPABILITY: &str = "exact-convex-prism-edge-fillet/1";
/// Convex longitudinal corners of a simple (possibly concave) planar prism.
pub const EXACT_SIMPLE_PRISM_FILLET_CAPABILITY: &str = "exact-simple-prism-convex-edge-fillet/1";
pub const EXACT_ANNULAR_FILLET_CAPABILITY: &str = "exact-annular-circular-edge-fillet/1";
/// Finite successor cell: a straight translation sweep. Bent Frenet/RMF paths
/// remain refused because this author does not construct their exact frame law.
pub const EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY: &str = "exact-parallel-frame-sweep/1";
/// Exact finite multi-section successor with explicit positional section
/// correspondence and rational bilinear side patches.
pub const EXACT_MULTI_SECTION_LOFT_CAPABILITY: &str = "analytic-solid-loft/2";
/// Exact finite piecewise-linear (degree-1 Bezier) path successor with
/// certified discrete rotation-minimizing frames and bounded section laws.
pub const EXACT_BENT_RMF_SWEEP_CAPABILITY: &str = "exact-parallel-frame-sweep/2";
/// Qualified finite shell/offset successor. The admitted cells are audited
/// convex planar-faced bodies and exact finite cylinders under rigid placement.
pub const EXACT_ANALYTIC_SHELL_CAPABILITY: &str = "analytic-shell/2";
/// Exact linear cross-section radius law on one edge of an audited cuboid
/// under rigid placement. This is not a general rolling-ball envelope.
pub const EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY: &str = "exact-variable-radius-fillet/1";
/// Exact equal-radius sphere/cylinder valence-3 corner on an audited AA cuboid.
pub const EXACT_VALENCE3_CORNER_BLEND_CAPABILITY: &str = "exact-valence3-corner-blend/1";

#[cfg(test)]
#[path = "../tests/analytic_features.rs"]
mod tests;

mod analytic_fillet;
mod audited_bent_rmf_sweep;
mod audited_feature_result;
mod exact_analytic_shell;
mod exact_variable_radius_fillet;
mod frame_law_ruled_sweep;
mod import_iges;
pub use analytic_fillet::*;
pub use audited_bent_rmf_sweep::*;
pub use audited_feature_result::*;
pub use exact_analytic_shell::*;
pub use exact_variable_radius_fillet::*;
pub use frame_law_ruled_sweep::*;

mod independent_corners;
pub use import_iges::*;
pub use independent_corners::*;
