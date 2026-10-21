//! Joint exact-domain boundary audit of the actual partitioned Boolean cavity.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outer = brep_core::cuboid([0.; 3], [10.; 3])?;
    let inner = brep_core::cuboid([2.; 3], [8.; 3])?;
    let model = brep_core::operations::boolean(&outer, &inner, "difference")?;
    let r = brep_core::boundary_embedding::inspect_with_linear(
        &model,
        1e-8,
        brep_core::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 1000,
            trim_domain_cells: 1000,
            spans: 1024,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10000,
                cells: 50000,
                domain_cells: 500000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        1000,
    )?;
    let unknown = r
        .intersections
        .pairs
        .pairs
        .iter()
        .filter(|p| p.reason == "pair-unresolved")
        .map(|p| p.faces)
        .collect::<Vec<_>>();
    println!(
        "proven={} exact={} joins={} trims={} faces={} pairs={} next={:?} unknown={unknown:?} hulls={} cells={} domains={}",
        r.proven,
        r.agreement.all_equal,
        r.agreement.all_joins_exact,
        r.trim.all_valid,
        r.intersections.faces.all_faces_injective,
        r.intersections.pairs.total_pairs,
        r.intersections.pairs.next_pair,
        r.hull_contacts.len(),
        r.intersections.pairs.cells,
        r.intersections.pairs.domain_cells
    );
    if let Some(path) = std::env::args().nth(1) {
        let audit = brep_core::solid_audit::audit_solid(&model)?;
        if !r.proven || !audit.ok {
            return Err("Cavity boundary or volume roles remain unproven".into());
        }
        let root = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&root)?;
        let (step, _, _) = brep_core::export_step_v5(&model)?;
        std::fs::write(root.join("cavity.step"), step)?;
        std::fs::write(root.join("model.json"), value_codec::to_string(&model)?)?;
        let receipt = value_codec::json!({"boundaryProven":r.proven,"exactAgreement":r.agreement.all_equal,"exactJoins":r.agreement.all_joins_exact,"trimRegionsValid":r.trim.all_valid,"facesInjective":r.intersections.faces.all_faces_injective,"allPairsClassified":r.intersections.pairs.all_pairs_classified,"totalPairs":r.intersections.pairs.total_pairs,"nextPair":r.intersections.pairs.next_pair,"hullContacts":r.hull_contacts.len(),"contactCells":r.intersections.pairs.cells,"contactDomainCells":r.intersections.pairs.domain_cells,"nativeVolumeRoles":audit.ok,"faces":model.faces.len(),"wholeWallThicknessClaim":false});
        std::fs::write(
            root.join("manifest.json"),
            value_codec::to_string_pretty(&receipt)?,
        )?;
    }
    Ok(())
}
