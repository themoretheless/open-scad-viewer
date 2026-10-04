//! Wire input validation for the existing surface coverage API.
use super::*;

/// Verify a nurbs-ss/1 Value report: empty unresolved when Complete, ToleranceContext
/// bind, missed-branch proof, and Boolean mutation authority stays false.
pub fn verify_general_ss_report_coverage(report: &value_codec::Value) -> Result<CoverageAudit> {
    let audit = nurbs_core::ss_intersection::verify_ss_coverage(report)?;
    Ok(CoverageAudit {
        complete: audit["complete"].as_bool().unwrap_or(false),
        component_count: audit["componentCount"].as_u64().unwrap_or(0) as usize,
        unresolved_count: audit["unresolvedCount"].as_u64().unwrap_or(0) as usize,
        notes: vec![
            "general_ss_coverage_verified",
            "no_graph_patch_iso_fixture",
            "boolean_mutation_deferred",
        ],
    })
}
