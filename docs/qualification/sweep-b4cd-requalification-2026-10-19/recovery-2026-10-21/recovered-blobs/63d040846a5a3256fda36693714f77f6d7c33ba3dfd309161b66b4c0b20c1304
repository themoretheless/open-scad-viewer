//! Generated nonlinear cut on a closed analytic control; no material removal.
#[path = "support/curved_trim_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use brep_core::{face_cut, watertight};
    use sha2::{Digest, Sha256};
    let path = std::env::args().nth(1).ok_or("Need output directory")?;
    let source = fixture::build();
    let top = std::env::args()
        .nth(2)
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75])?;
    let plane = cutter
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.5)
        })
        .ok_or("Missing cutter face")?;
    let automatic =
        face_cut::partition_ruled_at_face(&source, top, &cutter, plane, 1e-8, 100000, 1000000)?;
    let vertices = automatic.endpoints;
    let cells = automatic.cells;
    let split = automatic.partition;
    let closure = watertight::inspect(&split.model, 100000)?;
    let trims =
        brep_core::face_domain::audit_trim_regions(&split.model, 1e-8, 10000, 100000, 100000)?;
    let (step, _, _) = brep_core::export_step_v5(&split.model)?;
    let model = value_codec::to_string(&split.model)?;
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(model.as_bytes()),
        "sourceFace":top,"assemblyCells":cells,"nativeClosureWithinTolerance":closure.status==watertight::Closure::WithinTolerance,
        "trimRegionsValid":trims.all_valid,"sharedRootVertices":vertices,
        "expected":{"solids":1,"faces":7,"edges":15,"vertices":10,"boundsMm":[[0.,0.,-1.],[1.,1.,1.25]],"volumeMm3":19./12.},
        "materialRemovalClaim":false,"filletClaim":false,"wholeBodyG2Claim":false});
    std::fs::create_dir_all(&path)?;
    let directory = std::path::Path::new(&path);
    std::fs::write(directory.join("trimmed.step"), step)?;
    std::fs::write(directory.join("model.json"), model)?;
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
