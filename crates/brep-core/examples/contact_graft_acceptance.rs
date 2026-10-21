//! Closed retained-source G2 transition with independent polynomial volume target.
#[path = "support/contact_graft_fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&directory)?;
    let fixture = fixture::build();
    let out = brep_core::contact_graft::attach(
        &fixture.source,
        &fixture.patch,
        &fixture.mapping,
        &fixture.targets,
        1e-8,
        100000,
        1000000,
    )?;
    let (step, _, _) = brep_core::export_step_v5(&out.model)?;
    let sha = Sha256::digest(step.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(directory.join("grafted-g2-transition.step"), step)?;
    std::fs::write(
        directory.join("model.json"),
        value_codec::to_string(&out.model)?,
    )?;
    // Independent DESIGN quintic section: G2 plane contact jets, setback r,
    // tangent handle h=0.35*r*sqrt(2). No native mass or measured skin poles.
    // Integral X*dZ uses exact Bernstein product-integral coefficients.
    let r = 1.5;
    let h = 0.35 * r * 2_f64.sqrt();
    let x = [r, r - h, r - 2. * h, 0., 0., 0.];
    let z = [0., 0., 0., r - 2. * h, r - h, r];
    let mut removed_area = 0.;
    for i in 0..6 {
        for j in 0..5 {
            removed_area += x[i] * 5. * (z[j + 1] - z[j]) * choose(5, i) * choose(4, j)
                / (10. * choose(9, i + j));
        }
    }
    let expected_volume = 600. - 6. * removed_area;
    let manifest = value_codec::json!({"schema":"cad-roadmap-step/1","toleranceMm":1e-6,"relativeVolumeTolerance":1e-10,
        "parts":[{"name":"retained-source-g2-transition","file":"grafted-g2-transition.step","sha256":sha,
            "nativeProven":false,"nativeClosureWithinTolerance":true,"nativeExactBoundary":out.closure.exact.all_equal,"nativeExactJoins":out.closure.exact.all_joins_exact,"trimRegionsValid":out.trims.all_valid,
            "cells":out.cells,"exactWork":out.exact_work,"wholeRadiusClaim":false,
            "expected":{"boundsMm":[[0.,2.,0.],[10.,8.,10.]],"volumeMm3":expected_volume,"solids":1,"edges":15,"vertices":10}}]});
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!(
        "closed_candidate=true vertices={} edges={} faces={} cells={} exact_work={} design_volume_mm3={expected_volume}",
        out.model.vertices.len(),
        out.model.edges.len(),
        out.model.faces.len(),
        out.cells,
        out.exact_work
    );
    Ok(())
}
