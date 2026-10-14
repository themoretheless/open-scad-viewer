//! Native diagnostics for unresolved surface intersection regions.
use crate::intersection::UnresolvedReason;

pub enum UnresolvedClassification {
    PoleOrSingular,
    NearMissOrIllConditioned,
    BranchBudget,
}

pub struct UnresolvedSurfaceIntersection {
    pub parameter_box: Option<[f64; 8]>,
    pub reason: UnresolvedReason,
    pub classification: Option<UnresolvedClassification>,
}
