//! Relocate a seam on an actual shared multi-edge cycle, preserving clock breaks.
use super::*;
use std::collections::BTreeSet;
fn uses(source: &Model, edge: usize) -> Result<Vec<(usize, usize, usize)>> {
    let mut uses = Vec::new();
    for (face, f) in source.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (slot, c) in source.loops[wire].coedges.iter().enumerate() {
                if c.edge == edge {
                    uses.push((face, wire, slot));
                }
            }
        }
    }
    if uses.len() != 2 {
        return Err(error("Cycle seam needs exactly two selected edge uses"));
    }
    let edges = |w: usize| {
        source.loops[w]
            .coedges
            .iter()
            .map(|c| c.edge)
            .collect::<BTreeSet<_>>()
    };
    let expected = edges(uses[0].1);
    if expected.len() < 2
        || expected.len() > 256
        || expected.len() != source.loops[uses[0].1].coedges.len()
    {
        return Err(error("Cycle seam needs 2..256 distinct actual cycle edges"));
    }
    for &(_, wire, _) in &uses {
        if edges(wire) != expected || source.loops[wire].coedges.len() != expected.len() {
            return Err(error(
                "Both actual incident wires must own the same complete edge cycle",
            ));
        }
    }
    Ok(uses)
}
pub(super) fn relocate(
    source: &Model,
    edge: usize,
    parameter: f64,
    key: &str,
    uv_tolerance: f64,
    limits: Limits,
) -> Result<Relocation> {
    let original = &source.edges[edge];
    if original.degenerate {
        return Err(error("A degenerate edge cannot select a cycle seam"));
    }
    let d = original.curve.domain();
    if !parameter.is_finite() || parameter < d[0] || parameter > d[1] {
        return Err(error(
            "Cycle seam parameter must lie in the selected original edge domain",
        ));
    }
    let uses = uses(source, edge)?;
    let backwards = source.loops[uses[0].1].coedges[uses[0].2].reversed;
    let split = if parameter > d[0] && parameter < d[1] {
        Some(split_with_uv_tolerance(
            source,
            edge,
            &[parameter],
            key,
            uv_tolerance,
            limits,
        )?)
    } else {
        None
    };
    let (intermediate, vertex, relations, prior) = if let Some(s) = &split {
        (s.model(), s.vertices()[0], s.relations().to_vec(), None)
    } else {
        (
            source,
            original.vertices[usize::from(parameter == d[1])],
            Vec::new(),
            Some(qualification::verify_edge_refinement(source, limits)?),
        )
    };
    let prior = split
        .as_ref()
        .map(|s| s.evidence())
        .or(prior.as_ref())
        .unwrap();
    let work = Work {
        cells: prior.cells,
        pairs: prior.pairs,
        domain_cells: prior.domain_cells,
    };
    let mut model = intermediate.clone();
    let mut wires = BTreeSet::new();
    for &(_, wire, _) in &uses {
        if !wires.insert(wire) {
            continue;
        }
        let start = model.loops[wire]
            .coedges
            .iter()
            .position(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .ok_or_else(|| error("Requested cycle vertex has no actual wire start"))?;
        if start == 0 {
            continue;
        }
        model.loops[wire].coedges.rotate_left(start);
        let parent = model.1.loops[wire];
        let id = TopoId::derive(
            TopoKind::Loop,
            "edge-cycle-seam",
            key,
            &source.1.loops[wire].to_string(),
            source.1.edges[edge].to_string().as_bytes(),
        );
        model.1.loops[wire] = id;
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
    model.validate()?;
    let mut traversal = model.loops[uses[0].1]
        .coedges
        .iter()
        .map(|c| CycleUse {
            edge: c.edge,
            reversed: c.reversed,
        })
        .collect::<Vec<_>>();
    if backwards {
        traversal.reverse();
        for c in &mut traversal {
            c.reversed = !c.reversed;
        }
    }
    let cycle = traversal.iter().map(|c| c.edge).collect();
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
        cycle,
        traversal,
        vertex,
        relations,
    })
}
