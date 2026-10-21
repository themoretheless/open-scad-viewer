//! Prepared curved corner-sector assembly; no prescribed-radius corner claim.
#[path = "support/region_network_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let path = std::env::args().nth(1).ok_or("Need an output directory")?;
    let directory = std::path::Path::new(&path);
    let f = fixture::build();
    let out = brep_core::region_replacement::replace_g2_network(
        &f.source,
        &f.plans(),
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )?;
    let (step, _, _) = brep_core::export_step_v5(&out.model)?;
    let model = value_codec::to_string(&out.model)?;
    let digest = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(model.as_bytes()),
        "cells":out.cells,"exactWork":out.exact_work,"patchFaces":out.patch_faces,"seamEdges":out.seams.seams.iter().map(|s|s.edge).collect::<Vec<_>>(),
        "allSavedNewSeamsG2":out.seams.complete&&out.seams.within_tolerance,"allPatchChartsInjective":out.interiors.iter().all(|r|r.certified),
        "nativeClosureWithinTolerance":out.closure.closure.status==brep_core::watertight::Closure::WithinTolerance,"trimRegionsValid":out.trims.all_valid,
        "expected":{"solids":1,"faces":31,"edges":61,"vertices":32,"boundsMm":[[-1.,-1.,-3.],[2.,2.,1.5]],"volumeMm3":30.375},
        "prescribedRadiusClaim":false,"generalCornerConstructionClaim":false,"wholeBodyG2Claim":false,"globalEmbeddingClaim":false});
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join("network.step"), step)?;
    std::fs::write(directory.join("model.json"), model)?;
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
