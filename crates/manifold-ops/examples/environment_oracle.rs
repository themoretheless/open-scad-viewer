//! Native side of the shared native/WASM and f32/f64 manifold matrix.

use manifold_core::{ManifoldReport, MeshMetrics, RepairMode};
use manifold_ops::{BooleanOp, boolean_manifold};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
    #[serde(rename = "booleanPairs")]
    boolean_pairs: Vec<BooleanPair>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    positions: Vec<f64>,
    indices: Vec<usize>,
}

#[derive(Deserialize)]
struct BooleanPair {
    id: String,
    a: String,
    b: String,
    operations: Vec<String>,
}

fn flat_edges(edges: &[(usize, usize)]) -> Vec<usize> {
    edges.iter().flat_map(|&(a, b)| [a, b]).collect()
}

fn canonical_indices(indices: &[usize]) -> Vec<usize> {
    assert_eq!(indices.len() % 3, 0, "invalid triangle indices");
    let mut canonical = Vec::with_capacity(indices.len());
    for triangle in indices.chunks_exact(3) {
        let start = (0..3).min_by_key(|&index| triangle[index]).unwrap();
        canonical.extend((0..3).map(|index| triangle[(start + index) % 3]));
    }
    canonical
}

fn check_json(report: &ManifoldReport) -> Value {
    json!({
        "vertexCount": report.vertex_count,
        "triangleCount": report.triangle_count,
        "componentCount": report.component_count,
        "isManifold": report.is_manifold(),
        "isManifoldWithBoundary": report.is_manifold_with_boundary(),
        "boundaryEdges": flat_edges(&report.boundary_edges),
        "nonManifoldEdges": flat_edges(&report.non_manifold_edges),
        "orientationEdges": flat_edges(&report.orientation_edges),
        "degenerateTriangles": report.degenerate_triangles,
        "nonManifoldVertices": report.non_manifold_vertices,
        "isolatedVertices": report.isolated_vertices,
    })
}

fn metrics_json(metrics: &MeshMetrics) -> Value {
    json!({
        "vertexCount": metrics.vertex_count,
        "triangleCount": metrics.triangle_count,
        "edgeCount": metrics.edge_count,
        "eulerCharacteristic": metrics.euler_characteristic,
        "componentCount": metrics.components.len(),
        "signedVolume": metrics.signed_volume,
        "surfaceArea": metrics.surface_area,
        "watertight": metrics.watertight,
        "components": metrics.components.iter().map(|c| json!({
            "triangleCount": c.triangle_count,
            "vertexCount": c.vertex_count,
            "edgeCount": c.edge_count,
            "boundaryEdges": c.boundary_edges,
            "eulerCharacteristic": c.euler_characteristic,
            "genus": c.genus,
            "signedVolume": c.signed_volume,
            "surfaceArea": c.surface_area,
        })).collect::<Vec<_>>(),
    })
}

fn record(id: &str, operation: String, precision: &str, result: Value) -> Value {
    json!({"id": id, "operation": operation, "precision": precision, "result": result})
}

fn main() {
    let corpus: Corpus = serde_json::from_reader(std::io::stdin()).unwrap();
    let mut results = Vec::new();
    for precision in ["f64", "f32"] {
        let cast = |values: &[f64]| -> Vec<f64> {
            if precision == "f32" {
                values.iter().map(|&value| value as f32 as f64).collect()
            } else {
                values.to_vec()
            }
        };
        for case in &corpus.cases {
            let positions = cast(&case.positions);
            let check = manifold_core::check(&positions, &case.indices);
            results.push(record(
                &case.id,
                "check".into(),
                precision,
                check_json(&check),
            ));
            let metrics = manifold_core::metrics(&positions, &case.indices);
            results.push(record(
                &case.id,
                "metrics".into(),
                precision,
                metrics_json(&metrics),
            ));
            let repaired =
                manifold_core::repair_with_mode(&positions, &case.indices, 0.0, RepairMode::Full);
            let repair = json!({
                "positions": repaired.positions,
                "indices": canonical_indices(&repaired.indices),
                "weldedVertices": repaired.report.welded_vertices,
                "removedDegenerateTriangles": repaired.report.removed_degenerate_triangles,
                "flippedTriangles": repaired.report.flipped_triangles,
                "splitVertices": repaired.report.split_vertices,
                "filledHoles": repaired.report.filled_holes,
                "filledTriangles": repaired.report.filled_triangles,
                "mode": 1,
                "isManifold": repaired.is_manifold(),
                "isManifoldWithBoundary": repaired.report.residual.is_manifold_with_boundary(),
                "outputMetrics": metrics_json(&manifold_core::metrics(&repaired.positions, &repaired.indices)),
            });
            results.push(record(&case.id, "repair".into(), precision, repair));
        }
        for pair in &corpus.boolean_pairs {
            let a = corpus.cases.iter().find(|case| case.id == pair.a).unwrap();
            let b = corpus.cases.iter().find(|case| case.id == pair.b).unwrap();
            let ap = cast(&a.positions);
            let bp = cast(&b.positions);
            for operation in &pair.operations {
                let op = match operation.as_str() {
                    "union" => BooleanOp::Union,
                    "intersection" => BooleanOp::Intersection,
                    "difference" => BooleanOp::Difference,
                    _ => panic!("unknown boolean operation {operation}"),
                };
                let output = boolean_manifold(&ap, &a.indices, &bp, &b.indices, op).unwrap();
                let result = json!({
                    "positions": output.positions,
                    "indices": canonical_indices(&output.indices),
                    "operandRepaired": output.operand_repairs.map(|repair| repair.is_some()),
                    "kernel": {
                        "toleranceMm": output.kernel_report.tolerance_mm,
                        "work": output.kernel_report.work,
                        "fragments": output.kernel_report.fragments,
                        "inputTriangles": output.kernel_report.input_triangles,
                    },
                    "isManifold": output.output_report.is_manifold(),
                    "outputMetrics": metrics_json(&manifold_core::metrics(&output.positions, &output.indices)),
                });
                results.push(record(
                    &pair.id,
                    format!("boolean-{operation}"),
                    precision,
                    result,
                ));
            }
        }
    }
    serde_json::to_writer(
        std::io::stdout(),
        &json!({
            "schemaVersion": 1,
            "environment": {"kind": "native", "arch": std::env::consts::ARCH, "os": std::env::consts::OS},
            "results": results,
        }),
    )
    .unwrap();
}
