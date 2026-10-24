//! Reproducible native fixtures; no general fillet/body validity certificate.
use brep_core::{
    circular_blend::{partial_annular_variable_arc, plane_cylinder_transition},
    export_step_v9,
};

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    let diagnostics = std::env::args().nth(2).as_deref() == Some("diagnostics");
    std::fs::create_dir_all(&directory).unwrap();
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for sign in [-1., 1.] {
            let sweeps = [sign * 0.3, sign * 0.7, sign * 0.4];
            let model = partial_annular_variable_arc(20., 5., 6., radii, sweeps, 1e-7).unwrap();
            let topology = model.validate().unwrap();
            let mut radius_checks = Vec::new();
            for (start, sweep, a, b) in [
                (0., sweeps[0], 0., radii[0]),
                (sweeps[0], sweeps[1], radii[0], radii[1]),
                (sweeps[0] + sweeps[1], sweeps[2], radii[1], 0.),
            ] {
                let span = plane_cylinder_transition(20., 6., a, b, start, sweep).unwrap();
                let tolerance = if a == 0. || b == 0. { 1e-6 } else { 1e-9 };
                let report = span.qualify_radius(tolerance, 100, 100_000_000).unwrap();
                assert!(
                    report.certificate.is_some(),
                    "{} {:?}",
                    report.reason,
                    report.error_upper
                );
                radius_checks.push(value_codec::json!({
                    "reason": report.reason, "errorUpperMm": report.error_upper,
                    "toleranceMm": tolerance, "work": report.work,
                }));
            }
            let mut seam_checks = Vec::new();
            if diagnostics {
                for faces in [[0, 5], [5, 10]] {
                    let edge = model.loops[model.faces[faces[0]].outer]
                        .coedges
                        .iter()
                        .find(|a| {
                            model.loops[model.faces[faces[1]].outer]
                                .coedges
                                .iter()
                                .any(|b| a.edge == b.edge)
                        })
                        .unwrap()
                        .edge;
                    let r = brep_core::edge_tangency::inspect(
                        &model,
                        edge,
                        brep_core::edge_tangency::Limits {
                            tolerance_mm: 1e-7,
                            max_sine_squared: 1e-4,
                            agreement_cells: 256,
                            normal_cells: 10000,
                            normal_spans: 20000,
                        },
                    )
                    .unwrap();
                    seam_checks.push(value_codec::json!({
                        "edge": edge, "faces": r.faces,
                        "positionsProven": r.contact.positions_proven,
                        "tangentPlanesProven": r.contact.tangent_planes_proven,
                        "visited": r.contact.visited, "spans": r.contact.spans,
                        "toleranceMm": 1e-7, "maxSineSquared": 1e-4,
                        "cells": r.contact.cells.iter().map(|c| value_codec::json!({
                            "interval": c.interval,
                            "aligned": c.normals.as_ref().and_then(|n| n.aligned),
                            "sineSquaredInterval": c.normals.as_ref().and_then(|n| n.sine_squared_interval),
                            "reason": c.normals.as_ref().map(|n| n.reason),
                        })).collect::<Vec<_>>(),
                    }));
                }
            }
            let name = format!("case-{}", cases.len());
            let (step, _, _) = export_step_v9(&model).unwrap();
            std::fs::write(format!("{directory}/{name}.step"), step).unwrap();
            std::fs::write(
                format!("{directory}/{name}.json"),
                value_codec::to_string(&model).unwrap(),
            )
            .unwrap();
            cases.push(value_codec::json!({
                "name": name, "radii": radii, "sweeps": sweeps,
                "parameters": [20., 5., 6.], "bodyCount": topology.body_count,
                "boundaryEdgeCount": topology.boundary_edge_count,
                "faces": model.faces.len(), "radiusChecks": radius_checks,
                "seamChecks": seam_checks,
            }));
        }
    }
    std::fs::write(format!("{directory}/manifest.json"), value_codec::to_string_pretty(&value_codec::json!({
        "schema": "cad-variable-annular-transitions/1",
        "scope": "Native combinatorial closure, original whole-patch section radius bounds and STEP export. No independent STEP measurements, pole G1, global thickness, self-intersection or general rolling-ball envelope certificate.",
        "cases": cases,
    })).unwrap()).unwrap();
    println!("Exported and radius-qualified {} fixtures", cases.len());
}
