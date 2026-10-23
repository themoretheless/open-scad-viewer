//! All pairs incident to the actual inserted corner; not a full-body verdict.
#[path = "support/corner_close_fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (source, uses, options) = fixture::fixture_extended();
    let out = brep_core::corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )?;
    let limits = brep_core::face_contacts::Limits {
        pairs: 10000,
        cells: 100000 - out.cells,
        domain_cells: 1000000 - out.cells,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10000,
    };
    let r = brep_core::face_contacts::inspect_incident_with_vertices(
        &out.model,
        out.face,
        1e-8,
        limits,
        1000000 - out.exact_work,
    )?;
    println!(
        "scope=inserted-corner face={} total={} checked={} next={:?} classified={} geometry_cells={} domain_cells={} global_body_claim=false",
        r.face,
        r.total_pairs,
        r.pairs.len(),
        r.next_pair,
        r.all_incident_pairs_classified,
        r.cells,
        r.domain_cells
    );
    let mut rows = Vec::new();
    let mut locality_cells = 0;
    let mut locality_exact = 0;
    let mut edge_localized = 0;
    let mut vertex_localized = 0;
    let mut exact_vertices = 0;
    for pair in &r.pairs {
        let edges = |face: usize| {
            std::iter::once(out.model.faces[face].outer)
                .chain(out.model.faces[face].holes.iter().copied())
                .flat_map(|w| out.model.loops[w].coedges.iter().map(|c| c.edge))
                .collect::<std::collections::BTreeSet<_>>()
        };
        let left = edges(pair.faces[0]);
        let right = edges(pair.faces[1]);
        let shared_edges = left.intersection(&right).copied().collect::<Vec<_>>();
        let vertices = |edges: &std::collections::BTreeSet<usize>| {
            edges
                .iter()
                .flat_map(|&e| out.model.edges[e].vertices)
                .collect::<std::collections::BTreeSet<_>>()
        };
        let left = vertices(&left);
        let right = vertices(&right);
        let shared_vertices = left.intersection(&right).copied().collect::<Vec<_>>();
        println!(
            "pair={:?} reason={} cells={} domains={} shared_edges={:?} shared_vertices={:?}",
            pair.faces,
            pair.reason,
            pair.result.as_ref().map_or(0, |p| p.cells),
            pair.result.as_ref().map_or(0, |p| p.domain_cells),
            shared_edges,
            shared_vertices
        );
        let mut locality = value_codec::Value::Null;
        let exact_vertex = match &pair.boundary {
            Some(brep_core::face_contacts::SharedBoundary::OwnedVertex(c)) => {
                exact_vertices += 1;
                Some(c.vertex)
            }
            _ => None,
        };
        if pair.reason == "pair-unresolved" {
            let attempt = brep_core::trimmed_edge_contact::inspect(
                &out.model,
                pair.faces,
                (100000 - out.cells - r.cells - locality_cells)
                    .min(1000000 - out.cells - r.domain_cells - locality_cells),
                1000000 - out.exact_work - r.vertex_exact_work - locality_exact,
            )?;
            locality_cells += attempt.cells;
            locality_exact += attempt.exact_work;
            if let Some(c) = attempt.certificate {
                edge_localized += 1;
                println!(
                    "metric_edge_contact={:?} edge={} distance_upper={} penetration_upper={}",
                    pair.faces, c.edge, c.distance_upper, c.hull_penetration_upper
                );
                locality = value_codec::json!({"kind":"owned-edge-tolerance-tube","edge":c.edge,
                    "distanceUpperMm":c.distance_upper,"hullPenetrationUpperMm":c.hull_penetration_upper,
                    "exactAbsenceClaim":false});
            } else {
                let attempt = brep_core::trimmed_vertex_contact::inspect(
                    &out.model,
                    pair.faces,
                    (100000 - out.cells - r.cells - locality_cells)
                        .min(1000000 - out.cells - r.domain_cells - locality_cells),
                    1000000 - out.exact_work - r.vertex_exact_work - locality_exact,
                )?;
                locality_cells += attempt.cells;
                locality_exact += attempt.exact_work;
                if let Some(c) = attempt.certificate {
                    vertex_localized += 1;
                    println!(
                        "metric_vertex_contact={:?} vertex={} distance_upper={}",
                        pair.faces, c.vertex, c.distance_upper
                    );
                    locality = value_codec::json!({"kind":"owned-vertex-tolerance-ball","vertex":c.vertex,
                        "distanceUpperMm":c.distance_upper,"hullPenetrationsUpperMm":c.hull_penetrations,
                        "exactAbsenceClaim":false});
                }
            }
        }
        rows.push(value_codec::json!({"faces":pair.faces,"reason":pair.reason,
            "sharedEdges":shared_edges,"sharedVertices":shared_vertices,
            "geometryCells":pair.result.as_ref().map_or(0,|p|p.cells),
            "domainCells":pair.result.as_ref().map_or(0,|p|p.domain_cells),
            "unresolvedCells":pair.result.as_ref().map_or(0,|p|p.unresolved.len()),"metricLocality":locality,
            "exactOwnedVertex":exact_vertex}));
    }
    let remaining = (100000 - out.cells - r.cells - locality_cells)
        .min(1000000 - out.cells - r.domain_cells - locality_cells);
    if remaining == 0 {
        return Err("No shared face-injectivity work remains".into());
    }
    let span_budget = remaining.min(1024);
    let faces = brep_core::face_injectivity::inspect_with_linear(
        &out.model,
        span_budget,
        remaining - span_budget,
    )?;
    println!(
        "all_faces_injective={} face_count={} added_face_cells={} face_spans={}",
        faces.all_faces_injective,
        faces.faces.len(),
        faces.linear_cells,
        faces.spans
    );
    println!(
        "metric_edge_pairs={edge_localized} metric_vertex_pairs={vertex_localized} exact_vertex_pairs={exact_vertices} locality_cells={locality_cells} locality_exact={locality_exact} vertex_exact_work={}",
        r.vertex_exact_work
    );
    if let Some(path) = std::env::args().nth(1) {
        let model = value_codec::to_string(&out.model)?;
        let hash = Sha256::digest(model.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let report = value_codec::json!({"schema":"corner-incident-diagnostics/1","modelSha256":hash,
            "scope":"all-pairs-incident-to-inserted-corner","face":out.face,"totalPairs":r.total_pairs,
            "checkedPairs":r.pairs.len(),"nextPair":r.next_pair,"allIncidentPairsClassified":r.all_incident_pairs_classified,
            "geometryCells":r.cells,"constructionCells":out.cells,"domainCells":r.domain_cells,
            "metricEdgePairs":edge_localized,"metricVertexPairs":vertex_localized,"localityCells":locality_cells,"localityExactWork":locality_exact,
            "exactVertexPairs":exact_vertices,"vertexExactWork":r.vertex_exact_work,
            "allFacesInjective":faces.all_faces_injective,"faceCount":faces.faces.len(),
            "faceSpans":faces.spans,"faceLinearCells":faces.linear_cells,"globalBodyClaim":false,"pairs":rows});
        std::fs::write(path, value_codec::to_string_pretty(&report)?)?;
    }
    Ok(())
}
