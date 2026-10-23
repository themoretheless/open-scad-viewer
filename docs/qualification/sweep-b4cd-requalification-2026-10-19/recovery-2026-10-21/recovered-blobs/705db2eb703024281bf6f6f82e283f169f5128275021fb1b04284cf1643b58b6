//! Fresh whole-boundary diagnostics of a saved model, with explicit coverage.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Need qualification directory")?,
    );
    let model: brep_core::Model =
        value_codec::from_str(&std::fs::read_to_string(root.join("model.json"))?)?;
    let r = brep_core::self_intersection::inspect_qualified(
        &model,
        1e-8,
        1024,
        brep_core::face_contacts::Limits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 1000000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10000,
        },
        1000000,
    )?;
    let face_status=r.faces.faces.iter().map(|f|value_codec::json!({"face":f.face,"contraction":f.result.as_ref().is_some_and(|r|r.proven),"linear":f.linear.as_ref().is_some_and(|r|r.certified)})).collect::<Vec<_>>();
    let pair_status=r.pairs.pairs.iter().map(|p|value_codec::json!({"faces":p.faces,"reason":p.reason,"hullDisjoint":p.hull_disjoint,"sharedBoundary":p.boundary.is_some()})).collect::<Vec<_>>();
    let localized=r.localized.iter().map(|p|value_codec::json!({"faces":p.faces,"edges":p.owned_edges,"vertex":p.owned_vertex,"distanceUpperMm":p.distance_upper_mm})).collect::<Vec<_>>();
    let vertices = r
        .exact_vertices
        .iter()
        .map(|p| value_codec::json!({"faces":p.faces,"vertex":p.vertex}))
        .collect::<Vec<_>>();
    let report = value_codec::json!({"allFacesInjective":r.faces.all_faces_injective,"initialAllPairsClassified":r.pairs.all_pairs_classified,"totalPairs":r.pairs.total_pairs,"nextPair":r.pairs.next_pair,"spans":r.faces.spans,"cells":r.cells,"domainCells":r.domain_cells,"additionalExactWork":r.fallback_exact_work,"faces":face_status,"pairs":pair_status,"exactVertexContacts":vertices,"localized":localized,"unresolved":r.unresolved,"interiorContacts":r.interior_contacts,"absenceProven":r.absence_proven,"toleranceContactLocalityProven":r.tolerance_contact_locality_proven,"thicknessClaim":false});
    std::fs::write(
        root.join("qualified-diagnostics.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&report)?);
    Ok(())
}
