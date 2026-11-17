//! Open B-rep faces from exact periodic UV component charts.
use crate::{Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, TopologyIds, Vertex};
use nurbs_core::{
    editing::exact_periodic_surface::{
        PeriodicTrimEdge, PeriodicTrimEdgeRefusal, build_exact_periodic_surface_trim_edges,
    },
    surface::Surface,
};

#[derive(Clone, Debug)]
pub struct PeriodicTrimSheetReport {
    pub model: Option<Model>,
    pub work_used: u64,
    pub refusal: Option<PeriodicTrimEdgeRefusal>,
}

/// Author one open shell per filled component, retaining exact world edges,
/// paired pcurves, and holes. Distinct components keep separate topology.
pub fn build_exact_periodic_trim_sheet(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<PeriodicTrimSheetReport> {
    let error = |message| Error::new("BREP_PERIODIC_TRIM_SHEET", message);
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(error("Periodic trim sheet needs positive finite tolerance"));
    }
    let report = build_exact_periodic_surface_trim_edges(source, contours, max_work)?;
    let Some(patches) = report.patches else {
        return Ok(PeriodicTrimSheetReport {
            model: None,
            work_used: report.work_used,
            refusal: report.refusal,
        });
    };
    let mut topology = brep_topology::Model {
        vertices: Vec::new(),
        edges: Vec::new(),
        loops: Vec::new(),
        faces: Vec::new(),
        shells: Vec::new(),
        bodies: Vec::new(),
        tolerance_mm,
    };
    for patch in patches {
        let outer = topology.loops.len();
        for path in patch.loops {
            if path.is_empty() {
                return Err(error("Periodic trim loop has no edges"));
            }
            let first_vertex = topology.vertices.len();
            let edge_endpoints = path
                .iter()
                .map(edge_endpoints)
                .collect::<Result<Vec<_>>>()?;
            for ([start, end], [next, _]) in edge_endpoints
                .iter()
                .zip(edge_endpoints.iter().cycle().skip(1))
            {
                if end != next {
                    return Err(error(
                        "Periodic trim spatial endpoints do not match exactly",
                    ));
                }
                topology.vertices.push(Vertex { point: *start });
            }
            let len = path.len();
            let mut coedges = Vec::with_capacity(len);
            for (index, edge) in path.into_iter().enumerate() {
                let degenerate = edge
                    .curve
                    .control_points
                    .iter()
                    .all(|p| Some(p) == edge.curve.control_points.first());
                let vertices = [first_vertex + index, first_vertex + (index + 1) % len];
                if degenerate {
                    return Err(error("Periodic trim edge collapses to one spatial point"));
                }
                coedges.push(Coedge::new(topology.edges.len(), false, edge.pcurve));
                topology.edges.push(Edge {
                    vertices,
                    curve: edge.curve,
                    degenerate,
                });
            }
            topology.loops.push(Loop { coedges });
        }
        let face = topology.faces.len();
        topology.faces.push(Face {
            surface: patch.patch.surface,
            outer,
            holes: (outer + 1..topology.loops.len()).collect(),
        });
        topology.shells.push(Shell {
            faces: vec![FaceUse {
                face,
                reversed: false,
            }],
            closed: false,
        });
    }
    let mut model = Model(topology, TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(PeriodicTrimSheetReport {
        model: Some(model),
        work_used: report.work_used,
        refusal: None,
    })
}

fn edge_endpoints(edge: &PeriodicTrimEdge) -> Result<[[f64; 3]; 2]> {
    let point = |p: Option<&Vec<f64>>| {
        p.and_then(|p| <[f64; 3]>::try_from(p.as_slice()).ok())
            .ok_or_else(|| {
                Error::new(
                    "BREP_PERIODIC_TRIM_SHEET",
                    "Periodic trim edge has no 3D endpoint",
                )
            })
    };
    Ok([
        point(edge.curve.control_points.first())?,
        point(edge.curve.control_points.last())?,
    ])
}
