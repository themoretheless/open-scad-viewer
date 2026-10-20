//! Actual rolling-ball saved B-rep radius acceptance. No G2 join claim.
#[path = "support/contact_graft_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Need output directory")?);
    let variable = std::env::args().nth(2).is_some_and(|s| s == "variable");
    let f = if variable {
        fixture::build_variable_rolling_automatic()
    } else {
        fixture::build_rolling_automatic()
    };
    let field = f.radius_field;
    let center = [field, [0., 1., 0., 0.], field];
    let combined = brep_core::contact_graft::attach_radius_tangent_qualified(
        &f.source,
        &f.patch,
        &f.mapping,
        &f.targets,
        &[1, 3],
        center,
        field,
        1e-8,
        1e-7,
        1e-5,
        100000,
        1000000,
        2000000,
    )?;
    let result = combined.qualified;
    let json = value_codec::to_string(&result.graft.model)?;
    let restored: brep_core::Model = value_codec::from_str(&json)?;
    let fresh = brep_core::fillet_radius::inspect_saved_skin(
        &restored,
        result.graft.patch_face,
        center,
        field,
        1e-7,
        2000000,
    )?;
    if !fresh.within_tolerance {
        return Err("Saved skin radius unproven after reload".into());
    }
    let (step, _, _) = brep_core::export_step_v5(&restored)?;
    let report = value_codec::json!({"savedSkinRadiusWithinTolerance":true,"radialErrorUpperMm":fresh.radial_error_upper_mm,"coefficientWork":fresh.coefficient_work,"cells":combined.cells,"exactWork":combined.exact_work,"contactG1WithinTolerance":combined.tangency.within_tolerance,"contactNormalUpper":combined.tangency.seams.iter().map(|s|s.proof.normal_chord_upper).collect::<Vec<_>>(),"radiusField":field,"variable":variable,"trimEndpointRadiiMm":[2.*field[1]+field[3],8.*field[1]+field[3]],"patchFace":result.graft.patch_face,"g2Claim":false,"globalEmbeddingClaim":false});
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("rolling.step"), step)?;
    std::fs::write(root.join("model.json"), json)?;
    std::fs::write(
        root.join("manifest.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&report)?);
    Ok(())
}
