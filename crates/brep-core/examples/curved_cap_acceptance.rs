//! Material removal and cap on an analytic curved control, not a fillet.
#[path = "support/curved_trim_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use brep_core::{face_cut::{self, CapSelection}, watertight};
    use sha2::{Digest, Sha256};
    let path = std::env::args().nth(1).ok_or("Need output directory")?;
    let mut source = fixture::build();
    let transpose = std::env::args().nth(2).as_deref() == Some("transpose");
    if transpose { fixture::transpose_charts(&mut source); }
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75])?;
    let mut selections = [
        CapSelection { face: 1, remove_uv: [0.5, 0.8] },
        CapSelection { face: 3, remove_uv: [0.75, 0.95] },
        CapSelection { face: 4, remove_uv: [0.5, 0.9] },
        CapSelection { face: 5, remove_uv: [0.25, 0.95] },
    ];
    if transpose { for request in &mut selections { request.remove_uv.swap(0, 1); } }
    let result = face_cut::trim_ruled_and_cap(
        &source, &selections, &[], &cutter, 0, 1e-8, 100000, 1000000,
    )?;
    let cells = result.cells;
    let exact_work = result.exact_work;
    let cap = result.cap;
    let saved = value_codec::to_string(&cap.replacement.model)?;
    let restored: brep_core::Model = value_codec::from_str(&saved)?;
    let closure = watertight::inspect(&restored, 100000)?;
    let (step, _, _) = brep_core::export_step_v5(&restored)?;
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(saved.as_bytes()),
        "assemblyCells":cells,"exactWork":exact_work,"nativeClosureWithinTolerance":closure.status==watertight::Closure::WithinTolerance,
        "trimRegionsValid":cap.replacement.trims.all_valid,"capInjective":cap.interior.certified,
        "originalVolumeMm3":19./12.,"expectedRemovedVolumeMm3":83./480.,
        "expected":{"solids":1,"faces":7,"edges":15,"vertices":10,"boundsMm":[[0.,0.,-1.],[1.,1.,0.5]],"volumeMm3":677./480.},
        "transposedOriginalCharts":transpose,"atomicTrimAndCap":true,"materialRemovalClaim":true,"automaticMaterialSideClaim":false,"filletClaim":false,"wholeBodyG2Claim":false});
    std::fs::create_dir_all(&path)?;
    let directory = std::path::Path::new(&path);
    std::fs::write(directory.join("trimmed.step"), step)?;
    std::fs::write(directory.join("model.json"), saved)?;
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
