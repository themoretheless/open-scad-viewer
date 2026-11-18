//! Actual shared-cycle compaction, with unchanged-original segment admission.
use super::*;
use nurbs_core::curve_partition_agreement::concatenate::{self, Concatenation, Input};
use std::collections::{BTreeMap, BTreeSet};
mod clocks;
pub use clocks::Relation;
pub struct Compaction {
    source: Model,
    model: Model,
    selected: usize,
    key: String,
    limits: Limits,
    uv_tolerance: f64,
    evidence: Verification,
    edge: usize,
    vertex: usize,
    cycle: Vec<CycleUse>,
    world: Concatenation,
    relations: Vec<Relation>,
}
impl Compaction {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn seam_vertex(&self) -> usize {
        self.vertex
    }
    pub fn source_cycle(&self) -> &[CycleUse] {
        &self.cycle
    }
    pub fn segments(&self) -> &[concatenate::Segment] {
        self.world.segments()
    }
    /// Every original authored request and its independently admitted output clocks.
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    pub fn reverify(&self) -> Result<Verification> {
        let r = compact_with_uv_tolerance(
            &self.source,
            self.selected,
            &self.key,
            self.uv_tolerance,
            self.limits,
        )?;
        if r.model != self.model
            || r.cycle != self.cycle
            || r.edge != self.edge
            || r.vertex != self.vertex
            || r.relations != self.relations
        {
            return Err(error(
                "Saved cycle compaction differs from original reconstruction",
            ));
        }
        Ok(r.evidence)
    }
}
pub fn compact(source: &Model, selected: usize, key: &str, limits: Limits) -> Result<Compaction> {
    compact_with_uv_tolerance(source, selected, key, 1e-8, limits)
}
/// Retain authored requests when their exact output images form one connected
/// affine relation per face use. Disconnected/nonaffine unions require an
/// explicit compound relation and are refused rather than erased.
pub fn compact_with_uv_tolerance(
    source: &Model,
    selected: usize,
    key: &str,
    uv_tolerance: f64,
    limits: Limits,
) -> Result<Compaction> {
    let e = source
        .edges
        .get(selected)
        .ok_or_else(|| error("Unknown compaction edge"))?;
    if e.vertices[0] == e.vertices[1] {
        return Err(error("Compaction requires an actual multi-edge cycle"));
    }
    let relocation = super::relocate_with_uv_tolerance(
        source,
        selected,
        e.curve.domain()[0],
        key,
        uv_tolerance,
        limits,
    )?;
    let intermediate = relocation.model();
    let cycle = relocation.traversal().to_vec();
    let set = cycle.iter().map(|c| c.edge).collect::<BTreeSet<_>>();
    let cycle_vertices = set
        .iter()
        .flat_map(|i| source.edges[*i].vertices)
        .collect::<BTreeSet<_>>();
    for (i, e) in source.edges.iter().enumerate() {
        if !set.contains(&i)
            && e.vertices
                .iter()
                .any(|v| *v != relocation.seam_vertex() && cycle_vertices.contains(v))
        {
            return Err(error(
                "Compaction cannot erase a connected intermediate junction",
            ));
        }
    }
    let mut uses = Vec::new();
    for (face, f) in intermediate.faces.iter().enumerate() {
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            if intermediate.loops[w]
                .coedges
                .iter()
                .any(|c| set.contains(&c.edge))
            {
                uses.push((face, w));
            }
        }
    }
    if uses.len() != 2 || uses[0].1 == uses[1].1 {
        return Err(error(
            "Compaction requires two independently owned face wires",
        ));
    }
    let prior = relocation.evidence();
    let mut work = Work {
        cells: prior.cells,
        pairs: prior.pairs,
        domain_cells: prior.domain_cells,
    };
    let tol = cycle
        .iter()
        .map(|c| {
            source
                .1
                .local_tolerances_mm
                .get(&source.1.edges[c.edge])
                .copied()
                .unwrap_or(source.tolerance_mm)
        })
        .fold(source.tolerance_mm, f64::min);
    let tol = cycle_vertices
        .iter()
        .map(|v| {
            source
                .1
                .local_tolerances_mm
                .get(&source.1.vertices[*v])
                .copied()
                .unwrap_or(source.tolerance_mm)
        })
        .fold(tol, f64::min);
    let input = cycle
        .iter()
        .map(|c| Input {
            curve: &intermediate.edges[c.edge].curve,
            reversed: c.reversed,
        })
        .collect::<Vec<_>>();
    let world = concatenate::concatenate(&input, true, tol, work.remaining(limits)?.max_cells)?;
    work.cells += world.visited();
    check_junctions(source, &cycle, world.curve(), limits, &mut work)?;
    let mut uv = BTreeMap::new();
    let mut relations = Vec::new();
    for &(face, w) in &uses {
        let coedges = &intermediate.loops[w].coedges;
        let backwards = coedges
            .iter()
            .find(|c| c.edge == selected)
            .unwrap()
            .reversed;
        let input = coedges
            .iter()
            .map(|c| Input {
                curve: &c.pcurve,
                reversed: false,
            })
            .collect::<Vec<_>>();
        let joined = concatenate::concatenate(
            &input,
            true,
            uv_tolerance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += joined.visited();
        let (trim, transferred) = clocks::transfer(
            clocks::WireTransfer {
                source, intermediate, face, wire: w, cycle: &cycle,
                world: &world, uv: &joined, reversed: backwards,
                tolerance: tol, uv_tolerance, limits,
            },
            &mut work,
        )?;
        relations.extend(transferred);
        uv.insert(w, (joined.curve().clone(), backwards, trim));
    }
    let mut model = intermediate.clone();
    let parents = cycle
        .iter()
        .map(|c| source.1.edges[c.edge])
        .collect::<Vec<_>>();
    let signature = parents
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/");
    let new_id = TopoId::derive(
        TopoKind::Edge,
        "edge-cycle-compact",
        key,
        "closed-edge",
        signature.as_bytes(),
    );
    let mut remap = vec![None; model.edges.len()];
    let mut edges = Vec::new();
    let mut ids = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if !set.contains(&i) {
            remap[i] = Some(edges.len());
            edges.push(e.clone());
            ids.push(model.1.edges[i]);
        }
    }
    let edge = edges.len();
    let old_vertex = relocation.seam_vertex();
    edges.push(crate::Edge {
        curve: world.curve().clone(),
        vertices: [old_vertex; 2],
        degenerate: false,
    });
    ids.push(new_id);
    model.0.edges = edges;
    model.1.edges = ids;
    for (w, l) in model.0.loops.iter_mut().enumerate() {
        if let Some((pc, reversed, trim)) = uv.get(&w) {
            let mut coedge = crate::Coedge::new(edge, *reversed, pc.clone());
            if let Some(trim) = trim {
                coedge = coedge.with_trim(*trim)?;
            }
            l.coedges = vec![coedge];
        } else {
            for c in &mut l.coedges {
                c.edge = remap[c.edge]
                    .ok_or_else(|| error("Cycle edge has an unrelated retained use"))?;
            }
        }
    }
    let cycle_vertices = set
        .iter()
        .flat_map(|i| source.edges[*i].vertices)
        .collect::<BTreeSet<_>>();
    let used = model
        .edges
        .iter()
        .flat_map(|e| e.vertices)
        .collect::<BTreeSet<_>>();
    let mut vertex_map = vec![None; model.vertices.len()];
    let mut vertices = Vec::new();
    let mut vertex_ids = Vec::new();
    let mut deleted = Vec::new();
    for (i, v) in model.vertices.iter().enumerate() {
        if !cycle_vertices.contains(&i) || used.contains(&i) {
            vertex_map[i] = Some(vertices.len());
            vertices.push(v.clone());
            vertex_ids.push(model.1.vertices[i]);
        } else {
            deleted.push(model.1.vertices[i]);
        }
    }
    for e in &mut model.0.edges {
        e.vertices = e.vertices.map(|v| vertex_map[v].unwrap());
    }
    let vertex = vertex_map[old_vertex].unwrap();
    model.0.vertices = vertices;
    model.1.vertices = vertex_ids;
    for parent in &parents {
        model.1.local_tolerances_mm.remove(parent);
        model.1.sewed_edges.remove(parent);
    }
    for id in &deleted {
        model.1.local_tolerances_mm.remove(id);
    }
    model.1.local_tolerances_mm.insert(new_id, tol);
    model.1.lineage.push(crate::TopologyLineageRecord {
        operation: "merge".into(),
        entity_kind: "edge".into(),
        parents,
        children: vec![new_id],
    });
    for &(_, w) in &uses {
        let parent = model.1.loops[w];
        let id = TopoId::derive(
            TopoKind::Loop,
            "edge-cycle-compact",
            key,
            &source.1.loops[w].to_string(),
            signature.as_bytes(),
        );
        model.1.loops[w] = id;
        if let Some(t) = model.1.local_tolerances_mm.remove(&parent) {
            model.1.local_tolerances_mm.insert(id, t);
        }
        model.1.lineage.push(crate::TopologyLineageRecord {
            operation: "persist".into(),
            entity_kind: "loop".into(),
            parents: vec![parent],
            children: vec![id],
        });
    }
    model.refresh_change_set(&[intermediate]);
    for parent in deleted {
        model.1.change_set.changes.push(crate::TopologyChange {
            kind: crate::ChangeKind::Deleted,
            topo_kind: TopoKind::Vertex,
            parents: vec![parent],
            children: vec![],
            provenance: crate::ChangeProvenance {
                operation: "edge-cycle-compact".into(),
                operand: None,
                occurrence: key.into(),
            },
            role: "retired-clock-break".into(),
            anchor: None,
        });
    }
    model.validate()?;
    checks::endpoints(&model, &[edge], limits, &mut work)?;
    let mut evidence = qualification::verify_edge_refinement(&model, work.remaining(limits)?)?;
    work.include(&mut evidence);
    Ok(Compaction {
        source: source.clone(),
        model,
        selected,
        key: key.into(),
        limits,
        uv_tolerance,
        evidence,
        edge,
        vertex,
        cycle,
        world,
        relations,
    })
}

// At each p-fold integer join, the outgoing active endpoint pole is the exact
// rational curve value. Removed vertices keep their original point tolerance.
fn check_junctions(
    source: &Model,
    cycle: &[CycleUse],
    curve: &Curve,
    limits: Limits,
    work: &mut Work,
) -> Result<()> {
    use nurbs_core::interval_eval::Interval as I;
    for (i, c) in cycle.iter().enumerate() {
        work.remaining(limits)?;
        work.cells += 1;
        let t = i as f64;
        if curve.knots.iter().filter(|k| **k == t).count() < curve.degree {
            return Err(error(
                "Compacted junction has no exact endpoint-pole authority",
            ));
        }
        let span = (curve.degree..curve.control_points.len())
            .find(|&j| curve.knots[j] == t && curve.knots[j] < curve.knots[j + 1])
            .ok_or_else(|| error("Compacted junction has no active span"))?;
        let p = &curve.control_points[span - curve.degree];
        let vertex = source.edges[c.edge].vertices[usize::from(c.reversed)];
        let tol = source
            .1
            .local_tolerances_mm
            .get(&source.1.vertices[vertex])
            .copied()
            .unwrap_or(source.tolerance_mm)
            .min(source.tolerance_mm);
        let mut bound = I::point(0.);
        for k in 0..3 {
            if p[k] != source.vertices[vertex].point[k] {
                let e = I::point(p[k]).sub(I::point(source.vertices[vertex].point[k]))?;
                bound = bound.add(I::point(e.lo.abs().max(e.hi.abs())))?;
            }
        }
        if bound.hi > tol {
            return Err(error(
                "Compacted curve exceeds an original junction vertex tolerance",
            ));
        }
    }
    Ok(())
}
