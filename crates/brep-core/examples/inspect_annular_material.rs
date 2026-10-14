//! Detailed read-only refusal receipt for an actual saved annular body.
use brep_core::{boundary_embedding, face_contacts, sweep_cap_contacts, volume_validity};
use std::collections::BTreeSet;
use value_codec::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let model: brep_core::Model = value_codec::from_str(&std::fs::read_to_string(&args[0])?)?;
    let limits = volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1024,
            contacts: face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        nesting_pairs: 10000,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 1024,
    };
    let result = volume_validity::inspect_sweep(
        &model,
        1e-8,
        limits,
        100000,
        &[],
        sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work: 1000000,
            max_chart_cells: 1000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        },
    )?;
    let contacts = &result.boundary.intersections.pairs;
    let pairs=contacts.pairs.iter().map(|p| {
        let edge_sets=p.faces.map(|f|std::iter::once(model.faces[f].outer).chain(model.faces[f].holes.iter().copied())
            .flat_map(|w|model.loops[w].coedges.iter().map(|c|c.edge)).collect::<BTreeSet<_>>());
        let shared=edge_sets[0].intersection(&edge_sets[1]).copied().collect::<Vec<_>>();
        json!({"faces":p.faces,"reason":p.reason,"sharedEdges":shared,
            "hullDisjoint":p.hull_disjoint,"boundaryCertified":p.boundary.is_some(),
            "boundaryCells":p.boundary_cells,"hullCells":p.hull_cells,
            "search":p.result.as_ref().map(|r|json!({"absenceProven":r.absence_proven,"contact":r.contact.is_some(),
                "cells":r.cells,"domainCells":r.domain_cells,"unresolved":r.unresolved}))})
    }).collect::<Vec<_>>();
    let record = json!({"scope":"Fresh actual-model volume proof; no source-error claim or enlarged budget",
        "solidGeometryCertified":result.proven,"boundaryEmbeddingCertified":result.boundary.proven,
        "allFacesInjective":result.boundary.intersections.faces.all_faces_injective,
        "allPairsClassified":contacts.all_pairs_classified,"totalPairs":contacts.total_pairs,
        "nextPair":contacts.next_pair,"cells":contacts.cells,"domainCells":contacts.domain_cells,
        "groupCells":contacts.group_cells,"groupedPairs":contacts.grouped_pairs,"pairs":pairs,
        "agreementAllEqual":result.boundary.agreement.all_equal,"joinsExact":result.boundary.agreement.all_joins_exact});
    std::fs::write(&args[1], value_codec::to_string(&record)?)?;
    Ok(())
}
