use super::*;
/// Complete native seam scope. A request cannot exclude cap faces or closure.
pub fn diagnose_complete(v: Value) -> Result<Value> {
    use brep_core::sweep_smoothness::complete;
    let model = field(&v, "model")?;
    let order = field(&v, "order")?;
    let r = complete::inspect(
        &model,
        complete::Limits {
            order,
            exact_work: field(&v, "maxExactWork")?,
            agreement_work: field(&v, "maxAgreementWork")?,
            regularity_cells: field(&v, "maxRegularityCells")?,
        },
    )?;
    Ok(
        json!({"scope":"all-owned-retained-boundary-joins","order":order,
        "extractionComplete":r.extraction_complete,"vertexLinksComplete":r.vertex_links_complete,
        "edgeIds":r.edge_ids,"unresolvedEdges":r.unresolved_edges,"exactWork":r.exact_work,
        "edgeCorrespondenceCertified":r.edge_correspondence.as_ref().is_some_and(|p|p.all_equal&&p.all_joins_exact),
        "agreementWork":r.edge_correspondence.as_ref().map_or(0,|p|p.work),
        "allFacesRegular":r.regularity.all_faces_regular,"regularityCells":r.regularity.cells,
        "fullBoundaryG1Certified":r.g1_certified,"fullBoundaryG2Certified":r.g2_certified,
        "proof":collection(&r.proof,order),"idealSourceSmoothnessCertified":false,"solidCertified":false}),
    )
}
