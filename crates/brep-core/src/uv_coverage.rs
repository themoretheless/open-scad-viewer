//! Independent coverage check of a certified lifted UV arrangement, kept beside
//! `uv_arrangement` because it reads the arrangement's DCEL directly.
use crate::uv_arrangement::{LiftedUvArrangement, WindingLabel};
use cad_predicates::ToleranceContext;
use nurbs_core::{Error, Result};
use nurbs_intersect::coverage_verifier::CoverageAudit;

fn refuse(message: &str) -> Error {
    Error::new("BREP_COVERAGE_VERIFIER_REFUSED", message)
}

pub fn verify_lifted_uv_arrangement_coverage(
    arrangement: &LiftedUvArrangement,
    context: &ToleranceContext,
) -> Result<CoverageAudit> {
    if arrangement.context != context.spec_identity()
        || arrangement.coverage.context != arrangement.context
    {
        return Err(refuse("Lifted UV arrangement tolerance context mismatch"));
    }
    if !arrangement.coverage.complete
        || arrangement.coverage.event_vertex_count != arrangement.vertices.len()
        || arrangement.coverage.halfedge_count != arrangement.halfedges.len()
        || arrangement.coverage.cell_count != arrangement.cells.len()
    {
        return Err(refuse(
            "Lifted UV coverage counts were mutated or are incomplete",
        ));
    }
    for (id, halfedge) in arrangement.halfedges.iter().enumerate() {
        if halfedge.origin >= arrangement.vertices.len()
            || halfedge.destination >= arrangement.vertices.len()
            || halfedge.twin >= arrangement.halfedges.len()
            || arrangement.halfedges[halfedge.twin].twin != id
            || halfedge.next >= arrangement.halfedges.len()
            || halfedge.cell >= arrangement.cells.len()
        {
            return Err(refuse("Lifted UV DCEL incidence is invalid"));
        }
    }
    if !arrangement
        .cells
        .iter()
        .any(|cell| matches!(cell.label, WindingLabel::Material(_)))
    {
        return Err(refuse(
            "Lifted UV arrangement has no certified material cell",
        ));
    }
    Ok(CoverageAudit {
        complete: true,
        component_count: arrangement.cells.len(),
        unresolved_count: 0,
        notes: vec!["lifted_uv_dcel_verified"],
    })
}
