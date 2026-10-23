//! Saved mapped-corner B-rep control; not a prescribed-radius fillet fixture.
#[path = "support/corner_close_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory =
        std::path::PathBuf::from(std::env::args().nth(1).ok_or("Need output directory")?);
    let (open, uses, options) = fixture::fixture();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )?;
    let mut options = options;
    options.reverse_along = [false, true, true, false];
    options.gains = [4.; 4];
    let result = brep_core::region_replacement::replace_g2_mapped(
        &closed.model,
        &[closed.face],
        uses,
        [[-0.25, 0.], [0.25, 0.], [0., -0.25], [0., 0.25]],
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )?;
    let json = value_codec::to_string(&result.replacement.model)?;
    let restored: brep_core::Model = value_codec::from_str(&json)?;
    restored.validate()?;
    let closure = brep_core::watertight::inspect_with_exact(&restored, 100000, 1000000)?;
    if closure.closure.status != brep_core::watertight::Closure::WithinTolerance {
        return Err("Reload closure unproven".into());
    }
    let (step, _, _) = brep_core::export_step_v5(&restored)?;
    let manifest = value_codec::json!({"admitted":result.corner.admitted,"directOriginalAdmitted":result.corner.direct_admitted,
        "directOriginalUpper":result.corner.direct_upper,"cells":result.cells,"exactWork":result.exact_work,"coefficientWork":result.corner.coefficient_work,"originalSupportUpper":result.corner.upper,"bodies":restored.bodies.len(),"faces":restored.faces.len(),"edges":restored.edges.len(),"vertices":restored.vertices.len(),"nativeReloadClosed":true,"radiusClaim":false,"globalEmbeddingClaim":false,"independentStepMeasurement":false});
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("model.json"), json)?;
    std::fs::write(directory.join("mapped-corner.step"), step)?;
    std::fs::write(
        directory.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
