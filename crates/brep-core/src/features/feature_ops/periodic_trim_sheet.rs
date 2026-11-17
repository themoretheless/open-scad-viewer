//! Open B-rep faces from exact periodic UV component charts.
use crate::{
    Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, TopoId, TopologyIds, Vertex,
};
use nurbs_core::{
    editing::exact_periodic_surface::{
        PeriodicTrimEdge, PeriodicTrimEdgeRefusal, PeriodicTrimEdgeSource, PeriodicTrimEdgesReport,
        PeriodicTrimInputEdgeSource, build_exact_periodic_surface_trim_edges,
        build_exact_periodic_surface_trim_input_edges,
    },
    surface::Surface,
};

/// Persistent identities of the face and wire produced by an input contour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimContourBinding {
    pub face_id: TopoId,
    pub loop_id: TopoId,
    pub is_hole: bool,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimSheetReport {
    pub model: Option<Model>,
    pub work_used: u64,
    pub refusal: Option<PeriodicTrimEdgeRefusal>,
}

#[derive(Clone, Debug)]
pub struct BoundPeriodicTrimSheet {
    pub sheet: PeriodicTrimSheetReport,
    /// Bindings in input contour order. Empty when construction is refused.
    pub contour_bindings: Vec<PeriodicTrimContourBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicTrimEdgeBinding {
    pub edge_id: TopoId,
    pub source: PeriodicTrimEdgeSource,
}

#[derive(Clone, Debug)]
pub struct EdgeBoundPeriodicTrimSheet {
    pub bound_sheet: BoundPeriodicTrimSheet,
    /// Bindings in model edge order. Empty when construction is refused.
    pub edge_bindings: Vec<PeriodicTrimEdgeBinding>,
}

/// Author one open shell per filled component, retaining exact world edges,
/// paired pcurves, and holes. Distinct components keep separate topology.
pub fn build_exact_periodic_trim_sheet(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<PeriodicTrimSheetReport> {
    Ok(
        build_exact_periodic_trim_sheet_with_bindings(source, contours, tolerance_mm, max_work)?
            .sheet,
    )
}

/// Build an exact open sheet and retain its input contour to B-rep wire mapping.
pub fn build_exact_periodic_trim_sheet_with_bindings(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<BoundPeriodicTrimSheet> {
    Ok(build_sheet(source, contours, tolerance_mm, max_work, false)?.bound_sheet)
}

/// Build an open sheet with the source segment intervals of its persistent edges.
pub fn build_exact_periodic_trim_sheet_with_edge_bindings(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<EdgeBoundPeriodicTrimSheet> {
    build_sheet(source, contours, tolerance_mm, max_work, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimInputEdgeBinding {
    pub edge_id: TopoId,
    pub source: PeriodicTrimInputEdgeSource,
}

#[derive(Clone, Debug)]
pub struct InputBoundPeriodicTrimSheet {
    pub bound_sheet: BoundPeriodicTrimSheet,
    /// Bindings in model edge order, expressed in the original sample order.
    pub edge_bindings: Vec<PeriodicTrimInputEdgeBinding>,
}

/// Build an exact sheet and trace persistent edges back to the caller's
/// original contour segments, including the direction of each edge.
pub fn build_exact_periodic_trim_sheet_with_input_bindings(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<InputBoundPeriodicTrimSheet> {
    check_tolerance(tolerance_mm)?;
    let input = build_exact_periodic_surface_trim_input_edges(source, contours, max_work)?;
    let bound_sheet =
        assemble_sheet(input.geometry, contours.len(), tolerance_mm, false)?.bound_sheet;
    let sources = input.sources.into_iter().flatten().flatten();
    let mut edge_bindings = Vec::new();
    if let Some(model) = &bound_sheet.sheet.model {
        let mut sources = sources;
        for &edge_id in &model.1.edges {
            let source = sources.next().ok_or_else(edge_count_error)?;
            edge_bindings.push(PeriodicTrimInputEdgeBinding { edge_id, source });
        }
        if sources.next().is_some() {
            return Err(edge_count_error());
        }
    }
    Ok(InputBoundPeriodicTrimSheet {
        bound_sheet,
        edge_bindings,
    })
}

fn edge_count_error() -> Error {
    Error::new(
        "BREP_PERIODIC_TRIM_SHEET",
        "Periodic trim source and edge counts differ",
    )
}

fn check_tolerance(tolerance_mm: f64) -> Result<()> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(Error::new(
            "BREP_PERIODIC_TRIM_SHEET",
            "Periodic trim sheet needs positive finite tolerance",
        ));
    }
    Ok(())
}

fn build_sheet(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
    bind_edges: bool,
) -> Result<EdgeBoundPeriodicTrimSheet> {
    check_tolerance(tolerance_mm)?;
    let report = build_exact_periodic_surface_trim_edges(source, contours, max_work)?;
    assemble_sheet(report, contours.len(), tolerance_mm, bind_edges)
}

fn assemble_sheet(
    report: PeriodicTrimEdgesReport,
    contour_count: usize,
    tolerance_mm: f64,
    bind_edges: bool,
) -> Result<EdgeBoundPeriodicTrimSheet> {
    let error = |message| Error::new("BREP_PERIODIC_TRIM_SHEET", message);
    let Some(patches) = report.patches else {
        return Ok(EdgeBoundPeriodicTrimSheet {
            bound_sheet: BoundPeriodicTrimSheet {
                sheet: PeriodicTrimSheetReport {
                    model: None,
                    work_used: report.work_used,
                    refusal: report.refusal,
                },
                contour_bindings: Vec::new(),
            },
            edge_bindings: Vec::new(),
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
    let mut wire_owners = vec![None; contour_count];
    let mut edge_sources = Vec::new();
    for patch in patches {
        if bind_edges {
            edge_sources.extend(patch.edge_sources()?.into_iter().flatten());
        }
        let face = topology.faces.len();
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
        let loop_count = topology.loops.len() - outer;
        if patch.patch.trim.contours.len() != loop_count {
            return Err(error("Periodic trim contour and loop counts differ"));
        }
        for (offset, contour) in patch.patch.trim.contours.into_iter().enumerate() {
            let owner = wire_owners
                .get_mut(contour)
                .ok_or_else(|| error("Periodic trim contour index is out of range"))?;
            if owner.replace((face, outer + offset, offset != 0)).is_some() {
                return Err(error("Periodic trim contour has multiple wire owners"));
            }
        }
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
    let contour_bindings = wire_owners
        .into_iter()
        .map(|owner| {
            let (face, wire, is_hole) =
                owner.ok_or_else(|| error("Periodic trim contour has no wire owner"))?;
            Ok(PeriodicTrimContourBinding {
                face_id: model.1.faces[face].clone(),
                loop_id: model.1.loops[wire].clone(),
                is_hole,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if bind_edges && edge_sources.len() != model.1.edges.len() {
        return Err(edge_count_error());
    }
    let edge_bindings = edge_sources
        .into_iter()
        .zip(&model.1.edges)
        .map(|(source, edge_id)| PeriodicTrimEdgeBinding {
            edge_id: edge_id.clone(),
            source,
        })
        .collect();
    Ok(EdgeBoundPeriodicTrimSheet {
        bound_sheet: BoundPeriodicTrimSheet {
            sheet: PeriodicTrimSheetReport {
                model: Some(model),
                work_used: report.work_used,
                refusal: None,
            },
            contour_bindings,
        },
        edge_bindings,
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
