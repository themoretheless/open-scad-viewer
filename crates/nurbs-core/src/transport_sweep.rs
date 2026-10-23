use super::*;
pub(super) fn encode_wall_audit(report: sweep_wall_audit::Report) -> Result<Value> {
    Ok(json!({
            "chartsAndPairsCertified": report.charts_and_pairs_certified,
            "globalEmbeddingCertified": false,
            "injectivityCells": report.injectivity_cells,
            "unresolvedCharts": report.unresolved_charts,
            "charts": report.charts.iter().map(|chart| json!({
                "certified": chart.certified, "cells": chart.cells,
                "projection": chart.projection, "reason": chart.reason,
            })).collect::<Vec<_>>(),
            "declaredBoundariesC0": report.declared_boundaries_c0,
            "c0Boundaries": report.c0_boundaries,
            "unresolvedBoundaries": report.unresolved_boundaries,
            "pairs": {
                "allPairsSeparated": report.pairs.all_pairs_separated,
                "allPairsCompatible": report.pairs.all_pairs_compatible,
                "boundaryOnlyPairs": report.pairs.boundary_only_pairs,
                "separatedPairs": report.pairs.separated_pairs,
                "pairs": report.pairs.pairs, "cells": report.pairs.cells,
                "unresolved": report.pairs.unresolved.iter().map(|pair| json!({
                    "patches": pair.patches, "reason": pair.reason,
                })).collect::<Vec<_>>(),
            },
    }))
}


pub(super) fn miter_constant_vector_law(value:[f64;3])->Result<curve::Curve> {
    let curve=curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![value.to_vec();2],weights:vec![1.,1.],periodic:false};
    curve.validate()?;Ok(curve)
}
