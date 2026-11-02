//! All topology edits here are proposals, independently rechecked by partition.
use super::*;
use propose_arcs::ArcProposal;
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
    for original in 0..source.faces.len() {
        let ids = graph
            .arcs
            .iter()
            .enumerate()
            .filter(|(_, a)| graph.pairs[a.pair].faces[side] == original)
            .map(|(i, _)| i)
            .collect::<BTreeSet<_>>();
        let mut remaining = ids;
        while !remaining.is_empty() {
            let first = remaining
                .iter()
                .copied()
                .find(|&a| {
                    graph.arcs[a]
                        .vertices
                        .iter()
                        .any(|&v| graph.vertices[v].operand == side)
                })
                .unwrap_or(*remaining.first().unwrap());
            let a = &graph.arcs[first];
            let start = if graph.vertices[a.vertices[0]].operand == side {
                a.vertices[0]
            } else if graph.vertices[a.vertices[1]].operand == side {
                a.vertices[1]
            } else {
                a.vertices[0]
            };
            let mut at = start;
            let mut walk = Vec::new();
            loop {
                let next = remaining
                    .iter()
                    .copied()
                    .filter(|&i| graph.arcs[i].vertices.contains(&at))
                    .collect::<Vec<_>>();
                let i = if walk.is_empty() {
                    first
                } else {
                    if next.len() != 1 {
                        return Err(fail("original-face cut chain branches or stops"));
                    }
                    next[0]
                };
                remaining.remove(&i);
                let vs = graph.arcs[i].vertices;
                let reverse = vs[1] == at;
                walk.push((i, reverse));
                at = vs[usize::from(!reverse)];
                if at == start || graph.vertices[at].operand == side {
                    break;
                }
                if walk.len() > 256 {
                    return Err(fail("original face cut exceeds path budget"));
                }
            }
            let closed = at == start;
            let mut path = walk
                .iter()
                .map(|&(i, r)| {
                    Ok(crate::face_cut::Segment {
                        curve: if r {
                            arcs[i].curve.reverse()?
                        } else {
                            arcs[i].curve.clone()
                        },
                        pcurve: if r {
                            arcs[i].pc[side].reverse()?
                        } else {
                            arcs[i].pc[side].clone()
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            if closed {
                let loops = vec![path.iter().map(|s| s.pcurve.clone()).collect()];
                // The profile constructor admits material-left orientation
                // only. Reversal is another PROPOSAL; closed partitioning must
                // still prove the complete positive domain independently.
                if crate::profile_certificate::certify(&loops, 1e-10).is_err() {
                    path = path
                        .into_iter()
                        .rev()
                        .map(|s| {
                            Ok(crate::face_cut::Segment {
                                curve: s.curve.reverse()?,
                                pcurve: s.pcurve.reverse()?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    walk = walk.into_iter().rev().map(|(i, r)| (i, !r)).collect();
                }
            }
            let seed = path[0]
                .pcurve
                .evaluate((path[0].pcurve.domain()[0] + path[0].pcurve.domain()[1]) * 0.5)?
                .point;
            let candidates = p
                .face_sources
                .iter()
                .enumerate()
                .filter(|(_, f)| **f == original)
                .filter_map(|(f, _)| {
                    let d = crate::face_domain::FaceDomain::new(&p.model, f, 1e-10).ok()?;
                    let r = d.classify([[seed[0]; 2], [seed[1]; 2]], 4096).ok()?;
                    (r.location == nurbs_core::trim_domain::Location::Inside
                        && r.winding == Some(1))
                    .then_some(f)
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return Err(fail("cut path has no unique current original-face sector"));
            }
            let result = if closed {
                crate::face_cut::closed_with_budget(&p.model, candidates[0], &path, 1e-10, 100_000, 1_000_000)
                    .map_err(|e| {
                        fail(&format!(
                            "closed cut operand={side} original={original}: {}",
                            e.message
                        ))
                    })?
            } else {
                let ends = [
                    *roots
                        .get(&start)
                        .ok_or_else(|| fail("missing original start root vertex"))?,
                    *roots
                        .get(&at)
                        .ok_or_else(|| fail("missing original end root vertex"))?,
                ];
                crate::face_cut::partition(&p.model, candidates[0], ends, &path, 1e-10, 100_000)
                    .map_err(|e| {
                        fail(&format!(
                            "open cut operand={side} original={original}: {}",
                            e.message
                        ))
                    })?
            };
            p.face_sources.resize(result.model.faces.len(), original);
            p.edge_sources.resize(
                result.model.edges.len(),
                RegionEdgeOrigin::Intersection {
                    arc: 0,
                    reversed: false,
                },
            );
            if result.edges.len() != walk.len() {
                return Err(fail("saved path/graph edge census changed"));
            }
            for (e, (arc, reversed)) in result.edges.into_iter().zip(walk) {
                p.edge_sources[e] = RegionEdgeOrigin::Intersection { arc, reversed };
            }
            p.model = result.model;
        }
    }
    Ok(p)
}
