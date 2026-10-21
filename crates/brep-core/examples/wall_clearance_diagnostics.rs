//! Automatic angular candidate clearance, not whole-solid thickness.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Need qualification directory")?,
    );
    let face = std::env::args()
        .nth(2)
        .ok_or("Need source face")?
        .parse::<usize>()?;
    let model: brep_core::Model =
        value_codec::from_str(&std::fs::read_to_string(root.join("model.json"))?)?;
    let r = brep_core::wall_clearance::inspect_shell(
        &model, 0, face, -0.5, 1., 1e-7, 1e-8, 100000, 1000000,
    )?;
    let selection=r.selection.faces.iter().map(|f|value_codec::json!({"face":f.face,"decision":format!("{:?}",f.decision),"dotBounds":f.dot_bounds})).collect::<Vec<_>>();
    let distance=r.candidate_clearance.as_ref().map(|c|value_codec::json!({"lowerMm":c.lower_bound_mm,"upperMm":c.upper_bound_mm,"decision":format!("{:?}",c.decision),"converged":c.converged,"coverageComplete":c.coverage_complete,"pendingFaces":c.pending_faces}));
    let report = value_codec::json!({"sourceFace":face,"selectionCells":r.selection.cells,"selectionComplete":r.selection.selection_complete,"candidates":r.selection.candidate_faces,"excluded":r.selection.excluded_faces,"selection":selection,"candidateClearance":distance,"cells":r.cells,"domainCells":r.domain_cells,"materialChordClaim":false,"wholeSolidThicknessClaim":false});
    std::fs::write(
        root.join(
            std::env::args()
                .nth(3)
                .unwrap_or_else(|| "wall-clearance-diagnostics.json".into()),
        ),
        value_codec::to_string_pretty(&report)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&report)?);
    Ok(())
}
