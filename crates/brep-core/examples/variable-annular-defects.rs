//! Bounded self-intersection diagnostics for new unequal-radius fixtures.
//! A pending classification is never interpreted as an absence certificate.
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        let model = brep_core::circular_blend::partial_annular_variable_arc(
            20.,
            5.,
            6.,
            radii,
            [0.3, 0.7, 0.4],
            1e-7,
        )
        .unwrap();
        let before = value_codec::to_string(&model).unwrap();
        let report = brep_core::self_intersection::inspect(
            &model,
            1e-8,
            4096,
            brep_core::face_contacts::Limits {
                pairs: 1000,
                cells: 2000,
                domain_cells: 20000,
                cells_per_pair: 32,
                domain_cells_per_pair: 1000,
            },
        )
        .unwrap();
        assert_eq!(before, value_codec::to_string(&model).unwrap());
        cases.push(value_codec::json!({
            "radii": radii, "sweeps": [0.3, 0.7, 0.4],
            "absenceProven": report.absence_proven,
            "allFacesInjective": report.faces.all_faces_injective,
            "allPairsClassified": report.pairs.all_pairs_classified,
            "totalPairs": report.pairs.total_pairs,
            "visitedPairs": report.pairs.pairs.len(),
            "nextPair": report.pairs.next_pair,
            "faces": report.faces.faces.iter().map(|f| value_codec::json!({
                "face": f.face, "proven": f.result.as_ref().is_some_and(|r| r.proven),
                "reason": f.result.as_ref().map(|r| r.reason),
            })).collect::<Vec<_>>(),
            "pairs": report.pairs.pairs.iter().map(|p| value_codec::json!({
                "faces": p.faces, "reason": p.reason,
            })).collect::<Vec<_>>(),
        }));
    }
    std::fs::write(output, value_codec::to_string_pretty(&value_codec::json!({
        "schema": "cad-variable-annular-defects/1",
        "scope": "Bounded original-face self-intersection diagnostics. Unproven does not mean defective. No thickness, pole G1 or geometric solid admission.",
        "limits": {"faceSpans": 4096, "pairs": 1000, "cells": 2000, "domainCells": 20000, "cellsPerPair": 32, "domainCellsPerPair": 1000},
        "cases": cases,
    })).unwrap()).unwrap();
}
