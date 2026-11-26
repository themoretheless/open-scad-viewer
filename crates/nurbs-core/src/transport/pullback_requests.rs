use super::*;
use crate::{curve_pullback, curve_surface_agreement::Status};

pub(super) fn inspect(v: &Value) -> Result<Value> {
    let report = curve_pullback::project(
        &field(v, "curve")?,
        &field(v, "surface")?,
        curve_pullback::Limits {
            tolerance: field(v, "tolerance_mm")?,
            max_segments: field(v, "max_segments")?,
            max_iterations: field(v, "max_iterations")?,
            agreement_cells: field(v, "agreement_cells")?,
        },
    )?;
    let status = match report.agreement.as_ref().map(|proof| proof.status) {
        Some(Status::WithinTolerance) => "within_tolerance",
        Some(Status::Mismatch) => "mismatch",
        Some(Status::Unresolved) | None => "unresolved",
    };
    Ok(json!({
        "pcurve":report.pcurve,
        "status":status,
        "withinTolerance":report.within_tolerance,
        "agreementCells":report.agreement_cells,
        "evaluations":report.evaluations,
    }))
}
