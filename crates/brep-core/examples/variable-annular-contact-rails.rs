//! Full original contact rails and explicitly bounded regular subintervals.
use brep_core::{circular_blend::partial_annular_variable_arc, edge_tangency};
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let model =
        partial_annular_variable_arc(20., 5., 6., [0.5, 1.25], [0.3, 0.7, 0.4], 1e-7).unwrap();
    let before = value_codec::to_string(&model).unwrap();
    let poles = model
        .edges
        .iter()
        .filter(|e| e.degenerate)
        .map(|e| model.vertices[e.vertices[0]].point)
        .collect::<Vec<_>>();
    let mut cases = Vec::new();
    for base in [0, 5, 10] {
        for neighbor in [base + 1, base + 2] {
            let edge = model.loops[model.faces[base].outer]
                .coedges
                .iter()
                .find(|a| {
                    model.loops[model.faces[neighbor].outer]
                        .coedges
                        .iter()
                        .any(|b| a.edge == b.edge)
                })
                .unwrap()
                .edge;
            let ends = model.edges[edge].vertices.map(|v| model.vertices[v].point);
            let touches = ends.map(|p| poles.contains(&p));
            let mut intervals = vec![[0., 1.]];
            if touches[0] {
                intervals.push([0.05, 1.]);
            }
            if touches[1] {
                intervals.push([0., 0.95]);
            }
            for interval in intervals {
                let full_pole = interval == [0., 1.] && touches.iter().any(|p| *p);
                let r = edge_tangency::inspect_interval(
                    &model,
                    edge,
                    interval,
                    edge_tangency::Limits {
                        tolerance_mm: 1e-7,
                        max_sine_squared: 1e-4,
                        agreement_cells: 256,
                        normal_cells: if full_pole { 512 } else { 5000 },
                        normal_spans: if full_pole { 1024 } else { 10000 },
                    },
                )
                .unwrap();
                cases.push(value_codec::json!({
                    "edge": edge, "faces": r.faces, "poleAtEnds": touches,
                    "normalInterval": r.contact.normal_interval,
                    "positionsProven": r.contact.positions_proven,
                    "tangentPlanesProven": r.contact.tangent_planes_proven,
                    "visited": r.contact.visited, "spans": r.contact.spans,
                    "cells": r.contact.cells.iter().map(|c| value_codec::json!({
                        "interval": c.interval,
                        "aligned": c.normals.as_ref().and_then(|n| n.aligned),
                        "sineSquaredInterval": c.normals.as_ref().and_then(|n| n.sine_squared_interval),
                        "reason": c.normals.as_ref().map(|n| n.reason),
                    })).collect::<Vec<_>>(),
                }));
            }
        }
    }
    assert_eq!(value_codec::to_string(&model).unwrap(), before);
    std::fs::write(output, value_codec::to_string_pretty(&value_codec::json!({
        "schema": "cad-variable-annular-contact-rails/1", "radii": [0.5, 1.25], "sweeps": [0.3, 0.7, 0.4],
        "scope": "Full original position agreement; normal bounds on the explicitly recorded interval only. Excluded pole neighborhoods remain unqualified. No pole G1, thickness or body admission.",
        "toleranceMm": 1e-7, "maxSineSquared": 1e-4, "cases": cases,
    })).unwrap()).unwrap();
}
