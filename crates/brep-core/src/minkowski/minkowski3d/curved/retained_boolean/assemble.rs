use super::*;
use crate::{Body, Edge, FaceUse, Shell, Vertex};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VertexKey {
    Original(usize, usize),
    Split(usize, usize, u64),
    Root(usize),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EdgeKey {
    Original(usize, usize, u64, u64),
    Cut(usize),
}
fn keys(
    models: [&Model; 2],
    g: &QualifiedNurbsIntersectionGraph,
    p: &NurbsFacePartition,
    side: usize,
) -> Result<Vec<VertexKey>> {
    let mut keys = vec![None; p.model.vertices.len()];
    let graph = g.certificate();
    for (e, origin) in p.edge_sources.iter().enumerate() {
        for end in 0..2 {
            let key = match *origin {
                RegionEdgeOrigin::Original { edge, range } => {
                    let t = range[end];
                    let original = &models[side].edges[edge];
                    let d = original.curve.domain();
                    let hits = graph
                        .vertices
                        .iter()
                        .enumerate()
                        .filter(|(_, k)| k.operand == side && k.edge == edge)
                        .filter(|(_, k)| {
                            graph.pairs.iter().flat_map(|p| &p.events).any(|e| {
                                e.key == **k && e.edge_parameter[0] <= t && t <= e.edge_parameter[1]
                            })
                        })
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    if hits.len() > 1 {
                        return Err(fail(
                            "saved original endpoint has ambiguous algebraic ownership",
                        ));
                    }
                    if let Some(&i) = hits.first() {
                        VertexKey::Root(i)
                    } else if t == d[0] {
                        VertexKey::Original(side, original.vertices[0])
                    } else if t == d[1] {
                        VertexKey::Original(side, original.vertices[1])
                    } else {
                        VertexKey::Split(side, edge, t.to_bits())
                    }
                }
                RegionEdgeOrigin::Intersection { arc, reversed } => {
                    VertexKey::Root(graph.arcs[arc].vertices[if reversed { 1 - end } else { end }])
                }
            };
            let v = p.model.edges[e].vertices[end];
            if keys[v].as_ref().is_some_and(|old| old != &key) {
                return Err(fail(
                    "partition vertex has inconsistent ORIGINAL root ownership",
                ));
            }
            keys[v] = Some(key);
        }
    }
    keys.into_iter()
        .map(|k| k.ok_or_else(|| fail("unowned partition vertex")))
        .collect()
}
pub(super) fn build(
    models: [&Model; 2],
    g: &QualifiedNurbsIntersectionGraph,
    p: &[NurbsFacePartition; 2],
    regions: &[RetainedRegionMaterial],
) -> Result<(
    Model,
    Option<SourceAtlasCertificate>,
    Option<NurbsShellForestCertificate>,
)> {
    let keys = [keys(models, g, &p[0], 0)?, keys(models, g, &p[1], 1)?];
    let mut out = Model::empty(models[0].tolerance_mm.max(models[1].tolerance_mm))?;
    let mut vertices = BTreeMap::<VertexKey, usize>::new();
    let mut edges = BTreeMap::<EdgeKey, usize>::new();
    let mut face_uses = Vec::new();
    let mut face_parents = BTreeMap::<(usize, usize), Vec<usize>>::new();
    for region in regions.iter().filter(|r| r.keep) {
        let side = region.operand;
        let m = &p[side].model;
        let f = &m.faces[region.face];
        let mut loops = Vec::new();
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let mut ring = Vec::new();
            for c in &m.loops[w].coedges {
                let source_edge = &m.edges[c.edge];
                let (key, reverse) = match p[side].edge_sources[c.edge] {
                    RegionEdgeOrigin::Original { edge, range } => (
                        EdgeKey::Original(side, edge, range[0].to_bits(), range[1].to_bits()),
                        false,
                    ),
                    RegionEdgeOrigin::Intersection { arc, reversed } => {
                        (EdgeKey::Cut(arc), reversed)
                    }
                };
                let mut vs = [0; 2];
                for end in 0..2 {
                    let old = source_edge.vertices[if reverse { 1 - end } else { end }];
                    let vk = keys[side][old].clone();
                    let point = m.vertices[old].point;
                    vs[end] = if let Some(&id) = vertices.get(&vk) {
                        if out.vertices[id].point != point {
                            return Err(fail(
                                "exact root merge would ROUND two distinct saved endpoints",
                            ));
                        }
                        id
                    } else {
                        let id = out.vertices.len();
                        out.vertices.push(Vertex { point });
                        vertices.insert(vk, id);
                        id
                    };
                }
                let id = if let Some(&id) = edges.get(&key) {
                    if out.edges[id].vertices != vs {
                        return Err(fail(
                            "shared selected edge has inconsistent source vertices",
                        ));
                    }
                    id
                } else {
                    let id = out.edges.len();
                    let curve = if reverse {
                        source_edge.curve.reverse()?
                    } else {
                        source_edge.curve.clone()
                    };
                    out.edges.push(Edge {
                        curve,
                        vertices: vs,
                        degenerate: false,
                    });
                    edges.insert(key, id);
                    id
                };
                ring.push(Coedge::new(id, c.reversed ^ reverse, c.pcurve.clone()));
            }
            loops.push(out.loops.len());
            out.loops.push(crate::Loop { coedges: ring });
        }
        let id = out.faces.len();
        face_parents
            .entry((side, region.original_face))
            .or_default()
            .push(id);
        out.faces.push(crate::Face {
            surface: f.surface.clone(),
            outer: loops[0],
            holes: loops[1..].to_vec(),
        });
        face_uses.push(FaceUse {
            face: id,
            reversed: exact::owner(m, region.face)? ^ region.reverse,
        });
    }
    if out.faces.is_empty() {
        return Ok((out, None, None));
    }
    let mut incidence = vec![Vec::<(usize, bool)>::new(); out.edges.len()];
    for u in &face_uses {
        for c in exact::uses(&out, u.face) {
            incidence[c.edge].push((u.face, c.reversed ^ u.reversed));
        }
    }
    let mut adjacency = vec![Vec::new(); out.faces.len()];
    for uses in incidence {
        if uses.len() != 2 || uses[0].0 == uses[1].0 || uses[0].1 == uses[1].1 {
            return Err(fail(
                "selected boundary is not a closed oriented TWO-manifold",
            ));
        }
        adjacency[uses[0].0].push(uses[1].0);
        adjacency[uses[1].0].push(uses[0].0);
    }
    let mut seen = BTreeSet::new();
    for start in 0..out.faces.len() {
        if seen.contains(&start) {
            continue;
        }
        let mut pending = vec![start];
        let mut component = Vec::new();
        while let Some(f) = pending.pop() {
            if seen.insert(f) {
                component.push(face_uses[f].clone());
                pending.extend(&adjacency[f]);
            }
        }
        let shell = out.shells.len();
        out.shells.push(Shell {
            faces: component,
            closed: true,
        });
        out.bodies.push(Body {
            outer_shell: shell,
            inner_shells: vec![],
        });
    }
    out.inherit_topology_ids(&models);
    for ((side, original), children) in face_parents {
        let parent = models[side].1.faces[original];
        let children = children
            .into_iter()
            .map(|i| out.1.faces[i])
            .collect::<Vec<_>>();
        if children != vec![parent] {
            out.1.lineage.push(crate::TopologyLineageRecord {
                operation: "nurbs-boolean-retained".into(),
                entity_kind: "face".into(),
                parents: vec![parent],
                children,
            });
        }
    }
    for (key, id) in edges {
        let parents = match key {
            EdgeKey::Original(side, edge, ..) => vec![models[side].1.edges[edge]],
            EdgeKey::Cut(arc) => {
                // Topology lineage is kind-preserving. The two original
                // root-owning edges, rather than carrier-face IDs, are the
                // parents of a newly generated intersection edge.
                g.certificate().arcs[arc]
                    .vertices
                    .iter()
                    .map(|&v| {
                        let root = &g.certificate().vertices[v];
                        models[root.operand].1.edges[root.edge]
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            }
        };
        let child = out.1.edges[id];
        if parents != vec![child] {
            out.1.lineage.push(crate::TopologyLineageRecord {
                operation: "nurbs-boolean-boundary".into(),
                entity_kind: "edge".into(),
                parents,
                children: vec![child],
            });
        }
    }
    out.refresh_change_set(&models);
    out.validate()?;
    // Independently prove complete shell nesting, including negative cavity
    // components, before assigning body roles. Connectivity is not ownership.
    let shells =
        atlas::point_material::assign_bodies(&mut out, g.certificate().limits.source_depth)?;
    let atlas = certify_source_atlas(&out, g.certificate().limits.source_depth)?;
    Ok((out, Some(atlas), Some(shells)))
}
