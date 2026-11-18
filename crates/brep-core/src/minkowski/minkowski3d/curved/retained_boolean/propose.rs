//! All topology edits here are proposals, independently rechecked by partition.
use super::*;
use propose_arcs::ArcProposal;
mod arrange;
pub(super) fn construct(
    models: [&Model; 2],
    g: &QualifiedNurbsIntersectionGraph,
) -> Result<[NurbsFacePartition; 2]> {
    let arcs = propose_arcs::construct(models, g)?;
    Ok([operand(models, g, &arcs, 0)?, operand(models, g, &arcs, 1)?])
}
fn operand(
    models: [&Model; 2],
    g: &QualifiedNurbsIntersectionGraph,
    arcs: &[ArcProposal],
    side: usize,
) -> Result<NurbsFacePartition> {
    let source = models[side];
    let graph = g.certificate();
    let mut p = NurbsFacePartition {
        model: source.clone(),
        face_sources: (0..source.faces.len()).collect(),
        edge_sources: source
            .edges
            .iter()
            .enumerate()
            .map(|(edge, e)| RegionEdgeOrigin::Original {
                edge,
                range: e.curve.domain(),
            })
            .collect(),
    };
    let mut roots = BTreeMap::new();
    // Split each original edge once per ordered original root, updating ALL
    // incident uses. This preserves the original inter-face seam ownership.
    let mut ordered = graph
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, k)| k.operand == side)
        .map(|(i, key)| {
            let event = graph
                .pairs
                .iter()
                .flat_map(|p| &p.events)
                .find(|e| e.key == *key)
                .unwrap();
            Ok((key.edge, propose_arcs::dyadic(event.edge_parameter)?, i))
        })
        .collect::<Result<Vec<_>>>()?;
    ordered.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    for (original, t, root) in ordered {
        let slots = p
            .edge_sources
            .iter()
            .enumerate()
            .filter_map(|(e, o)| match o {
                RegionEdgeOrigin::Original { edge, range }
                    if *edge == original && range[0] < t && t < range[1] =>
                {
                    Some(e)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if slots.len() != 1 {
            return Err(fail(
                "original root does not own one strict edge restriction",
            ));
        }
        let edge = slots[0];
        let d = p.model.edges[edge].curve.domain();
        let mut cuts = Vec::new();
        for (wire, l) in p.model.loops.iter().enumerate() {
            for (coedge, c) in l.coedges.iter().enumerate().filter(|(_, c)| c.edge == edge) {
                let pc = c.pcurve.domain();
                let u = (exact::q(t) - exact::q(d[0])) / (exact::q(d[1]) - exact::q(d[0]));
                let u = if c.reversed { Q::one() - u } else { u };
                let parameter =
                    exact::float(&(exact::q(pc[0]) + u * (exact::q(pc[1]) - exact::q(pc[0]))))?;
                cuts.push(crate::edge_partition::PcurveCut {
                    wire,
                    coedge,
                    parameter,
                });
            }
        }
        let result = crate::edge_partition::partition(&p.model, edge, t, &cuts, 100_000)?;
        let RegionEdgeOrigin::Original { range, .. } = p.edge_sources[edge] else {
            unreachable!()
        };
        p.edge_sources.resize(
            result.model.edges.len(),
            RegionEdgeOrigin::Original {
                edge: original,
                range,
            },
        );
        p.edge_sources[result.child_edges[0]] = RegionEdgeOrigin::Original {
            edge: original,
            range: [range[0], t],
        };
        p.edge_sources[result.child_edges[1]] = RegionEdgeOrigin::Original {
            edge: original,
            range: [t, range[1]],
        };
        roots.insert(root, result.vertex);
        p.model = result.model;
    }
    arrange::construct(&mut p, source, g, arcs, side, roots)?;
    Ok(p)
}
