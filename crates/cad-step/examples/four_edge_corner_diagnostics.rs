#[path = "support/pyramid_material.rs"]
mod material;
#[path = "support/contact_qualification.rs"]
mod qualification;
use brep_core::region_replacement as r;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&dir)?;
    let source = r::four_edge_corner_stock()?;
    let before = source.clone();
    let p = r::propose_four_edge_corner(&source, [2.6, 2.8, 3., 3.2])?;
    assert_eq!(source, before);
    std::fs::write(dir.join("source.json"), value_codec::to_string(&source)?)?;
    std::fs::write(
        dir.join("construction.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"sourceUnchanged":true,"sourceHasFourValentVertex":true,"originalFiveSupportDefinitionsPreserved":p.retained_faces.iter().enumerate().all(|(i,f)|p.model.faces[*f].surface==source.faces[i].surface),"requestedMidNormalRadii":p.requested_mid_radii,"midNormalRadiusIntervals":p.mid_radius_intervals,"authoredHandles":p.authored_handles,"centralFace":p.central_face,"stripFaces":p.strip_faces,"scope":"Four prescribed mid-section normal radii on the four extruded arms, not constant circular curvature through the G2 plane transitions. Bounded pyramid stock; arbitrary high-valence source geometry remains separate."}),
        )?,
    )?;
    assert!(material::qualify(
        &p.model,
        p.strip_faces,
        p.requested_mid_radii,
        &dir
    )?);
    assert!(qualification::qualify(&p.model, &p.smooth_edges, &dir)?);
    Ok(())
}
