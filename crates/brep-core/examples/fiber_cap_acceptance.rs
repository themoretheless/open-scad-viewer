#[path = "support/fiber_cap_fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("Need output directory")?;
    let automatic = std::env::args().nth(2).as_deref() == Some("--automatic");
    let (source, cutter, requests) = fixture::build();
    let out = if automatic {
        let automatic_requests = requests
            .iter()
            .map(|r| brep_core::face_cut::AutomaticFiberCapRequest {
                face: r.section.face,
                remove_uv: r.remove_uv,
            })
            .collect::<Vec<_>>();
        brep_core::face_cut::trim_fiber_and_cap_automatic(
            &source,
            &automatic_requests,
            &[1],
            &cutter,
            0,
            1e-8,
            100000,
            1000000,
        )?
    } else {
        brep_core::face_cut::trim_fiber_and_cap(
            &source,
            &requests,
            &[1],
            &cutter,
            0,
            1e-8,
            100000,
            1000000,
        )?
    };
    let saved = value_codec::to_string(&out.cap.replacement.model)?;
    let restored: brep_core::Model = value_codec::from_str(&saved)?;
    if restored != out.cap.replacement.model {
        return Err("Reload changed the body".into());
    }
    let closure = brep_core::watertight::inspect(&restored, 100000)?;
    let (step, _, _) = brep_core::export_step_v5(&restored)?;
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(saved.as_bytes()),
        "assemblyCells":out.cells,"exactWork":out.exact_work,"evaluations":out.evaluations,
        "rootErrorUpperMm":out.root_error_upper_mm,"sharedOriginalRoots":out.root_vertices.len(),
        "nativeClosureWithinTolerance":closure.status==brep_core::watertight::Closure::WithinTolerance,
        "trimRegionsValid":out.cap.replacement.trims.all_valid,"capInjective":out.cap.interior.certified,
        "expected":{"solids":1,"faces":6,"edges":14,"vertices":10,"boundsMm":[[0.,0.,-1.],[1.,1.,-0.055]],"volumeMm3":509./600.},
        "originalVolumeMm3":19./12.,"expectedRemovedVolumeMm3":19./12.-509./600.,
        "automaticSectionCharts":automatic,
        "materialRemovalClaim":true,"automaticMaterialSideClaim":false,"filletClaim":false,"wholeBodyG2Claim":false});
    std::fs::create_dir_all(&directory)?;
    let path = std::path::Path::new(&directory);
    std::fs::write(path.join("trimmed.step"), step)?;
    std::fs::write(path.join("model.json"), saved)?;
    std::fs::write(
        path.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
