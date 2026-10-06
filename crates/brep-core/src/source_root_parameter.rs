//! Exact candidate parameters of an immutable certified UV root.
//! Candidates never replace the original root expression or its enclosure.
use crate::source_contact_point::SourcePoint;
use nurbs_core::{Error, Result};
pub struct Report {
    pub parameters: Option<[f64; 2]>,
    pub work_used: u64,
    pub reason: &'static str,
}
pub fn verify(point: &SourcePoint, candidate: [f64; 2], max_work: u64) -> Result<Report> {
    if candidate.iter().any(|t| !t.is_finite())
        || !(1..=cad_predicates::MAX_WORK).contains(&max_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_ROOT_PARAMETER_INPUT",
            "Choose finite original parameters and bounded exact work",
        ));
    }
    let mut out = Report {
        parameters: None,
        work_used: 0,
        reason: "source-root-candidate-outside-selector",
    };
    let selector = point.selector();
    if (0..2).any(|i| candidate[i] <= selector[i][0] || candidate[i] >= selector[i][1]) {
        return Ok(out);
    }
    // SourcePoint privately owns a unique certified root in this original box.
    // Exact equality inside that box therefore identifies that root.
    out.reason = "source-root-candidate-identity-unproven";
    if let Some(r) = nurbs_core::curve_point_identity::verify(
        [point.boundary(), point.contact()],
        candidate,
        max_work,
    )? {
        out.work_used = r.work_used;
        if r.outcome == cad_predicates::BezierIdentity::Equal {
            out.parameters = Some(candidate);
            out.reason = "source-root-candidate-qualified";
        }
    }
    Ok(out)
}
