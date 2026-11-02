//! Relocate a free closed-edge seam, retaining the original closed curve image.
//! Both actual face pcurves are rotated and freshly qualified independently.
use super::*;
use nurbs_core::{curve::Curve, periodic_seam, section_phase};
mod cycle;
pub mod compact;
mod partial;

/// Direction of an actual edge in the requested cycle traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleUse {
    pub edge: usize,
    pub reversed: bool,
}
pub struct Relocation {
    source: Model,
    model: Model,
    edge: usize,
    parameter: f64,
    key: String,
    limits: Limits,
    uv_tolerance: f64,
    evidence: Verification,
    cycle: Vec<usize>,
    traversal: Vec<CycleUse>,
    vertex: usize,
    relations: Vec<AuthoredRelation>,
}
impl Relocation {
    /// Actual cyclic edge order, oriented with the selected source edge.
    /// `traversal` supplies each edge's direction. Authored clock breaks remain
    /// explicit intermediate vertices.
    pub fn cycle(&self) -> &[usize] {
        &self.cycle
    }
    pub fn traversal(&self) -> &[CycleUse] {
        &self.traversal
    }
    pub fn seam_vertex(&self) -> usize {
        self.vertex
    }
    /// Exact original authored-relation ownership when clock breaks are retained.
    pub fn relations(&self) -> &[AuthoredRelation] {
        &self.relations
    }
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    /// Selected SOURCE edge index; cycle/traversal identify the actual output.
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn parameter(&self) -> f64 {
        self.parameter
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = relocate_with_uv_tolerance(
            &self.source,
            self.edge,
            self.parameter,
            &self.key,
            self.uv_tolerance,
            self.limits,
        )?;
        if actual.model != self.model
            || actual.cycle != self.cycle
            || actual.traversal != self.traversal
            || actual.vertex != self.vertex
            || actual.relations != self.relations
        {
            return Err(error("Saved seam differs from original reconstruction"));
        }
        Ok(actual.evidence)
    }
}
fn candidate(c: &Curve, parameter: f64) -> Result<Curve> {
    if !partition::verify_source_closure(c)? {
        return Err(error("Seam needs exact original C0 closure"));
    }
    let clamped;
    let input = if c.periodic {
        c
    } else {
        let d = c.domain();
        let mut proposal = c.trim(d[0], d[1])?;
        let last = proposal.control_points.len() - 1;
        proposal.control_points[last] = proposal.control_points[0].clone();
        clamped = proposal;
        &clamped
    };
    let proposed = section_phase::curve_candidate(input, parameter)?;
    let d = proposed.domain();
    let mut actual = proposed.trim(d[0], d[1])?;
    // Explicit closed candidate proposal. Fresh whole-original phase admission
    // below includes clamping and this endpoint coupling, never an exact claim.
    let last = actual.control_points.len() - 1;
    actual.control_points[last] = actual.control_points[0].clone();
    actual.validate()?;
    Ok(actual)
}
fn prove(
    source: &Curve,
    actual: &Curve,
    tolerance: f64,
    limits: Limits,
    work: &mut Work,
) -> Result<()> {
    let r =
        periodic_seam::verify_curve(source, actual, tolerance, work.remaining(limits)?.max_cells)?;
    work.cells += r.cells;
    if !r.accepted {
        return Err(error(
            "Whole original seam phase correspondence remains unproven",
        ));
    }
    Ok(())
}
/// Closed singleton edges may move a free vertex or retain an authored clock
/// break. An open edge selects a shared multi-edge cycle: interior requests
/// split that edge; endpoint requests reuse a vertex. Both wires rotate to the
/// requested seam without moving existing junctions. Sources are immutable.
pub fn relocate(
    source: &Model,
    edge: usize,
    parameter: f64,
    key: &str,
    limits: Limits,
) -> Result<Relocation> {
    relocate_with_uv_tolerance(source, edge, parameter, key, 1e-8, limits)
}
pub fn relocate_with_uv_tolerance(
    source: &Model,
    edge: usize,
    parameter: f64,
    key: &str,
    uv_tolerance: f64,
    limits: Limits,
) -> Result<Relocation> {
    source.validate()?;
    if key.is_empty() || key.len() > 128 || !uv_tolerance.is_finite() || uv_tolerance <= 0. {
        return Err(error(
            "Seam needs a bounded key and positive finite UV tolerance",
        ));
    }
    let original = source
        .edges
        .get(edge)
        .ok_or_else(|| error("Unknown seam edge"))?;
    if original.vertices[0] != original.vertices[1] {
        return cycle::relocate(source, edge, parameter, key, uv_tolerance, limits);
    }
    let vertex = original.vertices[0];
    if original.degenerate
        || vertex != original.vertices[1]
        || source
            .edges
            .iter()
            .enumerate()
            .any(|(i, e)| i != edge && e.vertices.contains(&vertex))
    {
        return Err(error(
            "Seam relocation needs one free regular closed-edge vertex",
        ));
    }
    let d = original.curve.domain();
    if !parameter.is_finite() || parameter < d[0] || parameter >= d[1] {
        return Err(error(
            "Seam parameter must be in the original half-open domain",
        ));
    }
    let mut uses = Vec::new();
    let mut partial_relation = false;
    for (f, face) in source.faces.iter().enumerate() {
        for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (slot, c) in source.loops[w].coedges.iter().enumerate() {
                if c.edge == edge {
                    if source.loops[w].coedges.len() != 1 {
                        return Err(error("A free closed seam needs a singleton incident wire"));
                    }
                    if let Some(t) = c.trim() {
                        let full = |a: [f64; 2], b: [f64; 2]| a == b || a == [b[1], b[0]];
                        if !full(t.curve_parameter, d)
                            || !full(t.pcurve_parameter, c.pcurve.domain())
                        {
                            partial_relation = true;
                        }
                    }
                    uses.push((f, w, slot));
                }
            }
        }
    }
    if uses.len() != 2 {
        return Err(error(
            "Seam relocation needs exactly two original face uses",
        ));
    }
    if partial_relation {
        return partial::relocate(source, edge, parameter, key, uv_tolerance, limits, &uses);
    }
    let mut work = Work::default();
    let before = qualification::verify_edge_refinement(source, work.remaining(limits)?)?;
    work.cells += before.cells;
    work.pairs += before.pairs;
    work.domain_cells += before.domain_cells;
    let tol = source.tolerance_mm.min(
        source
            .1
            .local_tolerances_mm
            .get(&source.1.edges[edge])
            .copied()
            .unwrap_or(source.tolerance_mm),
    );
    let actual = candidate(&original.curve, parameter)?;
    prove(&original.curve, &actual, tol, limits, &mut work)?;
    let mut model = source.clone();
    model.edges[edge].curve = actual;
    let start = &model.edges[edge].curve.control_points[0];
    model.vertices[vertex].point = [start[0], start[1], start[2]];
    let mut changed = std::collections::BTreeSet::new();
    for (face, wire, slot) in uses {
        let old = &source.loops[wire].coedges[slot];
        let pd = old.pcurve.domain();
        let t = (parameter - d[0]) / (d[1] - d[0]);
        let pc_parameter = pd[0] + (pd[1] - pd[0]) * if old.reversed { 1. - t } else { t };
        // The original upper endpoint is the same C0 seam as the lower one.
        let pc_parameter = if pc_parameter == pd[1] {
            pd[0]
        } else {
            pc_parameter
        };
        let pc = candidate(&old.pcurve, pc_parameter)?;
        prove(&old.pcurve, &pc, uv_tolerance, limits, &mut work)?;
        let mut coedge = crate::Coedge::new(edge, old.reversed, pc);
        if let Some(t) = old.trim() {
            let orient =
                |d: [f64; 2], old: [f64; 2]| if old[0] < old[1] { d } else { [d[1], d[0]] };
            let pcurve_parameter = orient(coedge.pcurve.domain(), t.pcurve_parameter);
            let periodic_lift = crate::boundary_agreement::periodic_lifts(
                &source.faces[face].surface,
                &coedge.pcurve,
                pcurve_parameter,
            )?;
            coedge = coedge.with_trim(brep_topology::CoedgeTrim {
                curve_parameter: orient(model.edges[edge].curve.domain(), t.curve_parameter),
                pcurve_parameter,
                periodic_lift,
            })?;
        }
        model.loops[wire].coedges[slot] = coedge;
        changed.insert(wire);
    }
    assign(source, &mut model, edge, vertex, &changed, key)?;
    checks::endpoints(&model, &[edge], limits, &mut work)?;
    let mut evidence = qualification::verify_edge_refinement(&model, work.remaining(limits)?)?;
    work.include(&mut evidence);
    Ok(Relocation {
        source: source.clone(),
        model,
        edge,
        parameter,
        key: key.into(),
        limits,
        uv_tolerance,
        evidence,
        cycle: vec![edge],
        traversal: vec![CycleUse {
            edge,
            reversed: false,
        }],
        vertex,
        relations: Vec::new(),
    })
}
fn assign(
    source: &Model,
    model: &mut Model,
    edge: usize,
    vertex: usize,
    wires: &std::collections::BTreeSet<usize>,
    key: &str,
) -> Result<()> {
    let old = source.1.edges[edge];
    let signature = old.to_string();
    let mut replace = |kind: TopoKind, parent: TopoId, role: &str| {
        let id = TopoId::derive(kind, "edge-seam", key, role, signature.as_bytes());
        if let Some(t) = model.1.local_tolerances_mm.remove(&parent) {
            model.1.local_tolerances_mm.insert(id, t);
        }
        model.1.lineage.push(crate::TopologyLineageRecord {
            operation: "persist".into(),
            entity_kind: match kind {
                TopoKind::Edge => "edge",
                TopoKind::Vertex => "vertex",
                _ => "loop",
            }
            .into(),
            parents: vec![parent],
            children: vec![id],
        });
        id
    };
    model.1.edges[edge] = replace(TopoKind::Edge, old, "closed-edge");
    model.1.vertices[vertex] = replace(TopoKind::Vertex, source.1.vertices[vertex], "seam-vertex");
    for &w in wires {
        model.1.loops[w] = replace(
            TopoKind::Loop,
            source.1.loops[w],
            &format!("wire-{}", source.1.loops[w]),
        );
    }
    model.1.sewed_edges.remove(&old);
    model.refresh_change_set(&[source]);
    model.validate()?;
    Ok(())
}
