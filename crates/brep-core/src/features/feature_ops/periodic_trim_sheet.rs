//! Open B-rep faces from exact periodic UV component charts.
use crate::{
    Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, TopoId, TopologyIds, Vertex,
};
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::{
    curve::Curve,
    editing::exact_periodic_surface::{
        PeriodicTrimEdge, PeriodicTrimEdgeRefusal, PeriodicTrimEdgeSource, PeriodicTrimEdgesReport,
        PeriodicTrimInputEdgeSource, build_exact_periodic_surface_trim_edges,
        build_exact_periodic_surface_trim_input_edges,
    },
    editing::{exact_knot_insertion, rational_factor::Refusal as ExactRefusal},
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

/// One exact parameter on a persistent B-rep edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimEdgeParameter {
    pub edge_id: TopoId,
    pub parameter: BigRational,
}

/// One exact parameter on a segment of the original input contour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimInputParameter {
    pub contour: usize,
    pub segment: usize,
    pub parameter: BigRational,
}

/// A clipped section of one edge, ordered in the caller's traversal direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrimEdgeRange {
    pub edge_id: TopoId,
    pub input_parameters: [BigRational; 2],
    pub edge_parameters: [BigRational; 2],
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimCurvePiece {
    pub range: PeriodicTrimEdgeRange,
    /// Keeps the edge's native clock. Use the directed range to traverse it.
    pub curve: Curve,
}

#[derive(Clone, Debug)]
pub struct PeriodicTrimCurveExtraction {
    pub pieces: Option<Vec<PeriodicTrimCurvePiece>>,
    pub work_used: u64,
    pub refusal: Option<ExactRefusal>,
}

impl InputBoundPeriodicTrimSheet {
    /// Find the edges containing a parameter on an original contour segment.
    /// At a split inside the segment, return both incident edges in model order.
    pub fn locate_input_parameter(
        &self,
        contour: usize,
        segment: usize,
        parameter: &BigRational,
    ) -> Result<Vec<PeriodicTrimEdgeParameter>> {
        let Some(model) = &self.bound_sheet.sheet.model else {
            return Ok(Vec::new());
        };
        if self.edge_bindings.len() != model.edges.len() {
            return Err(edge_count_error());
        }

        let mut hits = Vec::new();
        for (index, binding) in self.edge_bindings.iter().enumerate() {
            let source = &binding.source;
            if source.contour != contour || source.segment != segment {
                continue;
            }
            let [from, to] = &source.parameters;
            if parameter < from.min(to) || parameter > from.max(to) {
                continue;
            }
            let [start, end] = bound_edge_domain(model, index, binding)?;
            let fraction = (parameter - from) / (to - from);
            let edge_parameter = &start + fraction * (end - &start);
            hits.push(PeriodicTrimEdgeParameter {
                edge_id: binding.edge_id,
                parameter: edge_parameter,
            });
        }
        Ok(hits)
    }

    /// Clip a directed input segment range to the generated edges.
    /// Pieces follow the input range direction. Point contacts are omitted.
    pub fn locate_input_range(
        &self,
        contour: usize,
        segment: usize,
        range: [&BigRational; 2],
    ) -> Result<Vec<PeriodicTrimEdgeRange>> {
        Ok(self
            .locate_indexed_input_range(contour, segment, range)?
            .into_iter()
            .map(|(_, range)| range)
            .collect())
    }

    /// Extract the exact NURBS pieces under one shared restriction budget.
    /// Refuse atomically if a parameter or restricted spline cannot be stored exactly.
    pub fn extract_input_range_curves(
        &self,
        contour: usize,
        segment: usize,
        range: [&BigRational; 2],
        max_work: u64,
    ) -> Result<PeriodicTrimCurveExtraction> {
        if !(1..=cad_predicates::MAX_WORK).contains(&max_work) {
            return Err(Error::new(
                "BREP_PERIODIC_TRIM_SHEET",
                "Invalid exact restriction work limit",
            ));
        }
        let refused = |reason, work_used| PeriodicTrimCurveExtraction {
            pieces: None,
            work_used,
            refusal: Some(reason),
        };
        let mut work_used = 0;
        let mut pieces = Vec::new();
        if let Some(model) = &self.bound_sheet.sheet.model {
            for (index, range) in self.locate_indexed_input_range(contour, segment, range)? {
                if work_used == max_work {
                    return Ok(refused(ExactRefusal::ResourceLimit, work_used));
                }
                let [lo, hi] = range.edge_parameters.each_ref().map(|parameter| {
                    parameter.to_f64().filter(|stored| {
                        BigRational::from_float(*stored).as_ref() == Some(parameter)
                    })
                });
                let (Some(lo), Some(hi)) = (lo, hi) else {
                    return Ok(refused(ExactRefusal::NotRepresentable, work_used));
                };
                let report = exact_knot_insertion::restrict(
                    &model.edges[index].curve,
                    [lo.min(hi), lo.max(hi)],
                    max_work - work_used,
                )?;
                work_used += report.work_used;
                let Some(curve) = report.curve else {
                    return Ok(refused(
                        report.refusal.unwrap_or(ExactRefusal::InvalidOutput),
                        work_used,
                    ));
                };
                pieces.push(PeriodicTrimCurvePiece { range, curve });
            }
        }
        Ok(PeriodicTrimCurveExtraction {
            pieces: Some(pieces),
            work_used,
            refusal: None,
        })
    }

    fn locate_indexed_input_range(
        &self,
        contour: usize,
        segment: usize,
        range: [&BigRational; 2],
    ) -> Result<Vec<(usize, PeriodicTrimEdgeRange)>> {
        let Some(model) = &self.bound_sheet.sheet.model else {
            return Ok(Vec::new());
        };
        if self.edge_bindings.len() != model.edges.len() {
            return Err(edge_count_error());
        }
        let [from, to] = range;
        let (low, high) = (from.min(to), from.max(to));
        let mut pieces = Vec::new();
        for (index, binding) in self.edge_bindings.iter().enumerate() {
            let source = &binding.source;
            if source.contour != contour || source.segment != segment {
                continue;
            }
            let [start, end] = &source.parameters;
            let lo = low.max(start.min(end));
            let hi = high.min(start.max(end));
            if lo >= hi {
                continue;
            }
            let [edge_start, edge_end] = bound_edge_domain(model, index, binding)?;
            let input_parameters = if from < to { [lo, hi] } else { [hi, lo] };
            let scale = (edge_end - &edge_start) / (end - start);
            let edge_parameters =
                input_parameters.map(|parameter| &edge_start + (parameter - start) * &scale);
            pieces.push((
                index,
                PeriodicTrimEdgeRange {
                    edge_id: binding.edge_id,
                    input_parameters: input_parameters.map(Clone::clone),
                    edge_parameters,
                },
            ));
        }
        pieces.sort_by(|(_, a), (_, b)| a.input_parameters[0].cmp(&b.input_parameters[0]));
        if from > to {
            pieces.reverse();
        }
        Ok(pieces)
    }

    /// Map an edge parameter back to the original input contour segment.
    /// An unknown edge ID or a parameter outside the edge returns no hit.
    pub fn locate_edge_parameter(
        &self,
        edge_id: TopoId,
        parameter: &BigRational,
    ) -> Result<Option<PeriodicTrimInputParameter>> {
        let Some(model) = &self.bound_sheet.sheet.model else {
            return Ok(None);
        };
        if self.edge_bindings.len() != model.edges.len() {
            return Err(edge_count_error());
        }
        let Some(index) = model.1.edges.iter().position(|id| *id == edge_id) else {
            return Ok(None);
        };
        let binding = self.edge_bindings.get(index).ok_or_else(edge_count_error)?;
        let [start, end] = bound_edge_domain(model, index, binding)?;
        if parameter < &start || parameter > &end {
            return Ok(None);
        }
        let fraction = (parameter - &start) / (end - start);
        let [from, to] = &binding.source.parameters;
        Ok(Some(PeriodicTrimInputParameter {
            contour: binding.source.contour,
            segment: binding.source.segment,
            parameter: from + fraction * (to - from),
        }))
    }
}

fn bound_edge_domain(
    model: &Model,
    index: usize,
    binding: &PeriodicTrimInputEdgeBinding,
) -> Result<[BigRational; 2]> {
    let invalid = || {
        Error::new(
            "BREP_PERIODIC_TRIM_SHEET",
            "Periodic trim edge binding has an invalid identity or parameter domain",
        )
    };
    let [from, to] = &binding.source.parameters;
    if model.1.edges.get(index) != Some(&binding.edge_id) || from == to {
        return Err(invalid());
    }
    let curve = &model.edges.get(index).ok_or_else(invalid)?.curve;
    curve.validate()?;
    let [start, end] = curve.domain().map(BigRational::from_float);
    Ok([start.ok_or_else(invalid)?, end.ok_or_else(invalid)?])
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
