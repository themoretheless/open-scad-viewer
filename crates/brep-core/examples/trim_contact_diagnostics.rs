#[path = "support/nurbs_contact_stock.rs"]
mod fixture;
#[path = "support/contact_qualification.rs"]
mod qualification;
use brep_core::region_replacement as r;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&dir)?;
    let f = fixture::build();
    let original = f.prepared.clone();
    let cuts = f
        .cuts
        .iter()
        .map(|c| r::ContactCut {
            face: c.face,
            endpoints: c.endpoints,
            segments: &c.segments,
            replacement_pcurves: &c.replacement,
            remove: c.anchor,
            smooth: c.smooth,
        })
        .collect::<Vec<_>>();
    let out =
        r::trim_and_replace_contacts(&f.prepared, &cuts, &f.blend, true, 1e-8, 100_000, 1_000_000)?;
    assert_eq!(f.prepared, original);
    assert!(f.source.faces.iter().all(|f| {
        out.replacement
            .model
            .faces
            .iter()
            .any(|g| g.surface == f.surface)
    }));
    std::fs::write(dir.join("source.json"), value_codec::to_string(&f.source)?)?;
    std::fs::write(
        dir.join("construction.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"sourceUnchanged":true,"allSixOriginalSupportDefinitionsPreserved":true,"trimmedSupports":out.retained_supports,"contactEdges":out.contact_edges,"constructionCells":out.cells,"constructionExact":out.exact_work,"scope":"Original rational stock, four shared edge roots, two original curved support cuts and two original end-cap cuts. Original surface definitions are retained; this does not claim automatic contact-curve generation on arbitrary B-rep."}),
        )?,
    )?;
    assert!(qualification::qualify(
        &out.replacement.model,
        &out.smooth_edges,
        &dir
    )?);
    Ok(())
}
