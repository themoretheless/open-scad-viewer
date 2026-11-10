//! A moved seam through partial authored clocks retains an actual clock break.
//! The old seam stays as an intermediate vertex, never an erased relation.
use super::*;
pub(super) fn relocate(
    source: &Model,
    edge: usize,
    parameter: f64,
    key: &str,
    uv_tolerance: f64,
    limits: Limits,
    uses: &[(usize, usize, usize)],
) -> Result<Relocation> {
    if parameter == source.edges[edge].curve.domain()[0] {
        let evidence = qualification::verify_edge_refinement(source, limits)?;
        return Ok(Relocation {
            source: source.clone(),
            model: source.clone(),
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
            vertex: source.edges[edge].vertices[0],
            relations: Vec::new(),
        });
    }
    let split = split_with_uv_tolerance(source, edge, &[parameter], key, uv_tolerance, limits)?;
    let vertex = split.vertices()[0];
    let cycle = vec![split.edges()[1], split.edges()[0]];
    let relations = split.relations().to_vec();
    let mut model = split.model().clone();
    let mut wires = std::collections::BTreeSet::new();
    for &(_, wire, _) in uses {
        if !wires.insert(wire) {
            continue;
        }
        let start = model.loops[wire]
            .coedges
            .iter()
            .position(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .ok_or_else(|| error("Partitioned seam has no incident wire start"))?;
        model.loops[wire].coedges.rotate_left(start);
        let parent = model.1.loops[wire];
        let id = TopoId::derive(
            TopoKind::Loop,
            "edge-seam",
            key,
            &format!("clock-break-wire-{}", source.1.loops[wire]),
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
    model.refresh_change_set(&[split.model()]);
    model.validate()?;
    let prior = split.evidence();
    let work = Work {
        cells: prior.cells,
        pairs: prior.pairs,
        domain_cells: prior.domain_cells,
    };
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
        traversal: cycle
            .iter()
            .map(|&edge| CycleUse {
                edge,
                reversed: false,
            })
            .collect(),
        cycle,
        vertex,
        relations,
    })
}
