//! Adjacent curved face cuts with shared original roots; both sectors retained.
#[path = "support/curved_trim_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use brep_core::{face_cut, watertight};
    use sha2::{Digest, Sha256};
    let path = std::env::args().nth(1).ok_or("Need output directory")?;
    let source = fixture::build();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75])?;
    let out = face_cut::partition_ruled_network(
        &source,
        &[1, 3, 4, 5],
        &cutter,
        0,
        1e-8,
        100000,
        1000000,
    )?;
    let closure = watertight::inspect(&out.model, 100000)?;
    let trims =
        brep_core::face_domain::audit_trim_regions(&out.model, 1e-8, 10000, 100000, 100000)?;
    let (step, _, _) = brep_core::export_step_v5(&out.model)?;
    let model = value_codec::to_string(&out.model)?;
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(model.as_bytes()),
        "assemblyCells":out.cells,"exactWork":out.exact_work,"nativeClosureWithinTolerance":closure.status==watertight::Closure::WithinTolerance,
        "trimRegionsValid":trims.all_valid,"sharedRootVertices":out.root_vertices.iter().map(|r|r.2).collect::<Vec<_>>(),
        "sectionFaces":out.sections.iter().map(|r|r.faces).collect::<Vec<_>>(),
        "expected":{"solids":1,"faces":10,"edges":20,"vertices":12,"boundsMm":[[0.,0.,-1.],[1.,1.,1.25]],"volumeMm3":19./12.},
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
