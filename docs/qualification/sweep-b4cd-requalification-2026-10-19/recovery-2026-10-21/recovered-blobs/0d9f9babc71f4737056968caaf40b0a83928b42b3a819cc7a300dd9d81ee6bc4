//! Independent STEP target: integral of 3 + x²/4 + y²/8 on [-1,2]².
#[path = "support/corner_close_fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&directory)?;
    let extended = std::env::args().nth(2).as_deref() == Some("extended");
    let (source, uses, options) = if extended {
        fixture::fixture_extended()
    } else {
        fixture::fixture()
    };
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
    let (step, _, _) = brep_core::export_step_v5(&out.model)?;
    let sha = Sha256::digest(step.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(directory.join("closed-g2-corner.step"), step)?;
    std::fs::write(
        directory.join("model.json"),
        value_codec::to_string(&out.model)?,
    )?;
    let manifest = value_codec::json!({"schema":"cad-roadmap-step/1","toleranceMm":1e-6,"relativeVolumeTolerance":1e-10,
        "parts":[{"name":"four-owned-boundary-g2-corner","file":"closed-g2-corner.step","sha256":sha,
            "nativeProven":false,"nativeClosureWithinTolerance":true,"nativeExactBoundary":out.closure.exact.all_equal,
            "nativeExactJoins":out.closure.exact.all_joins_exact,"wholeRadiusClaim":false,"globalEmbeddingClaim":false,
            "cells":out.cells,"exactWork":out.exact_work,"coefficientWork":out.corner.coefficient_work,
            "g2AllFourSeams":out.corner.within_tolerance,
            "nativePatchInteriorCertified":out.corner.interior.certified,
            "nativeCornerAdmitted":out.corner.admitted,
            "rimInsideOriginalSupportCharts":extended,
            "expected":{"boundsMm":[[-1.,-1.,-3.],[2.,2.,1.5]],"volumeMm3":30.375,"solids":1,
                "edges":60,"vertices":32}}]});
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!(
        "faces={} edges={} vertices={} cells={} coefficients={} exact_work={}",
        out.model.faces.len(),
        out.model.edges.len(),
        out.model.vertices.len(),
        out.cells,
        out.corner.coefficient_work,
        out.exact_work
    );
    Ok(())
}
