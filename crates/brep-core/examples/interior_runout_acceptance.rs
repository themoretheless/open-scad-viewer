//! Export the original-support, 0.1 mm setback control after fresh admission.
#[path = "support/interior_runout_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Need an output directory")?;
    let directory = std::path::Path::new(&path);
    let (_, out) = fixture::build();
    let (step, _, _) = brep_core::export_step_v5(&out.model)?;
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join("runout.step"), step)?;
    std::fs::write(
        directory.join("model.json"),
        value_codec::to_string(&out.model)?,
    )?;
    println!(
        "closed runout: faces={} edges={} vertices={} contact_edges={:?}",
        out.model.faces.len(),
        out.model.edges.len(),
        out.model.vertices.len(),
        [out.patch_edges[1], out.patch_edges[3]]
    );
    Ok(())
}
