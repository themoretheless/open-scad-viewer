//! Selected opposite-face clearance; not a whole-solid thickness certificate.
#[path = "support/corner_close_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (source, uses, options) = fixture::fixture_extended();
    let out = brep_core::corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )?;
    let automatic = brep_core::wall_clearance::inspect_shell(
        &out.model,
        0,
        out.face,
        -0.5,
        2.9,
        1e-7,
        1e-8,
        100000 - out.cells,
        1000000 - out.cells,
    )?;
    let targets = &automatic.selection.candidate_faces;
    let report = automatic
        .candidate_clearance
        .as_ref()
        .ok_or("No candidate clearance proof")?;
    let witness = report.witness_pair.map(|i| &report.pairs[i]);
    println!(
        "scope=selected-opposing-face-set face={} targets={} lower={} upper={:?} decision={:?} converged={} cells={} domains={} whole_wall_claim=false",
        out.face,
        targets.len(),
        report.lower_bound_mm,
        report.upper_bound_mm,
        report.decision,
        report.converged,
        report.cells,
        report.domain_cells
    );
    if let Some(path) = std::env::args().nth(1) {
        use sha2::{Digest, Sha256};
        let model = value_codec::to_string(&out.model)?;
        let hash = Sha256::digest(model.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let decision = match report.decision {
            brep_core::wall_clearance::Decision::MeetsRequiredClearance => {
                "meets-required-clearance"
            }
            brep_core::wall_clearance::Decision::BelowRequiredClearance => {
                "below-required-clearance"
            }
            brep_core::wall_clearance::Decision::Unresolved => "unresolved",
        };
        let data = value_codec::json!({"schema":"selected-wall-clearance/1","modelSha256":hash,
            "sourceFace":out.face,"targetFaces":targets,"requiredMm":report.required_mm,
            "candidateDiscovery":"original-surface-normal-opposition","normalDotCutoff":automatic.selection.cutoff,
            "candidateSelectionComplete":automatic.selection.selection_complete,
            "oppositionGeometryCells":automatic.selection.cells,"automaticGeometryCells":automatic.cells,
            "excludedFaces":automatic.selection.excluded_faces,
            "lowerMm":report.lower_bound_mm,"upperMm":report.upper_bound_mm,"converged":report.converged,
            "decision":decision,"coverageComplete":report.coverage_complete,"pendingFaces":report.pending_faces,
            "geometryCells":report.cells,"domainCells":report.domain_cells,"wholeWallClaim":false,
            "witnessTargetFace":witness.map(|p|p.target_face),"witnessParameters":witness.and_then(|p|p.distance.parameters),
            "witnessPoints":witness.and_then(|p|p.distance.points),"witnessEnclosures":witness.and_then(|p|p.distance.point_enclosures.as_ref())});
        std::fs::write(path, value_codec::to_string_pretty(&data)?)?;
    }
    Ok(())
}
