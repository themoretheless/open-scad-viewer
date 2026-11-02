use super::*;
fn signed(edge: PocketEdge, positive: bool) -> PocketEdgeUse {
    PocketEdgeUse { edge, positive }
}
fn original_loops(model: &Model, used: &brep_topology::FaceUse) -> Vec<Vec<PocketEdgeUse>> {
    let face = &model.faces[used.face];
    std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|l| {
            let mut edges = model.loops[l]
                .coedges
                .iter()
                .filter(|c| !model.edges[c.edge].degenerate)
                .map(|c| signed(PocketEdge::Source(c.edge), !(c.reversed ^ used.reversed)))
                .collect::<Vec<_>>();
            if used.reversed {
                edges.reverse();
            }
            edges
        })
        .collect()
}
pub(super) fn assemble(
    model: &Model,
    target: usize,
    partition: &domains::DomainPartition,
    selected: &[u32],
    templates: &[AlgebraicRegionFace],
    components: &[Vec<u32>],
    floor_reversed: bool,
) -> Result<Vec<PocketFace>> {
    let chosen = selected.iter().copied().collect::<BTreeSet<_>>();
    let mut incident = vec![vec![]; partition.edges.len()];
    for (f, face) in partition.faces.iter().enumerate() {
        for edge in &face.boundary {
            incident[edge.unsigned_abs() as usize - 1].push(f as u32);
        }
    }
    let mut neighbors = vec![vec![]; partition.faces.len()];
    for uses in &incident {
        if uses.len() == 2 && uses.iter().all(|f| !chosen.contains(f)) {
            neighbors[uses[0] as usize].push(uses[1]);
            neighbors[uses[1] as usize].push(uses[0]);
        }
    }
    let mut owner = vec![usize::MAX; partition.faces.len()];
    let mut groups = vec![];
    for start in 0..partition.faces.len() {
        if chosen.contains(&(start as u32)) || owner[start] != usize::MAX {
            continue;
        }
        let id = groups.len();
        let mut todo = vec![start as u32];
        let mut group = vec![];
        while let Some(f) = todo.pop() {
            if owner[f as usize] == usize::MAX {
                owner[f as usize] = id;
                group.push(f);
                todo.extend(&neighbors[f as usize]);
            }
        }
        group.sort();
        groups.push(group);
    }
    let outside = groups
        .iter()
        .enumerate()
        .filter(|(_, group)| {
            group.iter().any(|f| {
                partition.faces[*f as usize]
                    .boundary
                    .iter()
                    .any(|e| incident[e.unsigned_abs() as usize - 1].len() == 1)
            })
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if outside.len() != 1 {
        return Err(fail(
            "pocket remainder has no unique original-face exterior component",
        ));
    }
    let outside = outside[0];
    let mut extra_loops = vec![vec![]; groups.len()];
    let mut walls = vec![];
    let mut floors = vec![];
    for (component, cells) in components.iter().enumerate() {
        let cycles = &templates[2 * component].boundary_cycles;
        let mut floor_loops = vec![];
        for cycle in cycles {
            let mut floor_loop = vec![];
            let mut remainder_loop = vec![];
            let mut remainder_owner = None;
            for e in cycle {
                let id = e.unsigned_abs() - 1;
                let positive = *e > 0;
                let edge = &partition.edges[id as usize];
                let other = incident[id as usize]
                    .iter()
                    .find(|f| !chosen.contains(f))
                    .copied()
                    .ok_or_else(|| fail("pocket seam has no retained source region"))?;
                let o = owner[other as usize];
                if remainder_owner.is_some_and(|old| old != o) {
                    return Err(fail("pocket loop changes its retained face component"));
                }
                remainder_owner = Some(o);
                floor_loop.push(signed(PocketEdge::Bottom(id), positive));
                remainder_loop.push(signed(PocketEdge::Top(id), !positive));
                let [a, b] = if positive {
                    edge.endpoints
                } else {
                    [edge.endpoints[1], edge.endpoints[0]]
                };
                walls.push(PocketFace {
                    kind: PocketFaceKind::Wall { domain_edge: id },
                    loops: vec![vec![
                        signed(PocketEdge::Top(id), positive),
                        signed(PocketEdge::Vertical(b), true),
                        signed(PocketEdge::Bottom(id), !positive),
                        signed(PocketEdge::Vertical(a), false),
                    ]],
                });
            }
            remainder_loop.reverse();
            extra_loops[remainder_owner.unwrap()].push(remainder_loop);
            floor_loops.push(floor_loop);
        }
        floors.push(PocketFace {
            kind: PocketFaceKind::Floor {
                component: component as u32,
                regions: cells.clone(),
                reversed: floor_reversed,
            },
            loops: floor_loops,
        });
    }
    let mut result = vec![];
    for used in &model.shells[model.bodies[0].outer_shell].faces {
        if used.face != target {
            result.push(PocketFace {
                kind: PocketFaceKind::Source {
                    face: used.face,
                    reversed: used.reversed,
                },
                loops: original_loops(model, used),
            });
        } else {
            let mut loops = original_loops(model, used);
            loops.extend(extra_loops[outside].clone());
            result.push(PocketFace {
                kind: PocketFaceKind::Remainder {
                    source_face: target,
                    regions: groups[outside].clone(),
                    outside_triangle: true,
                    reversed: used.reversed,
                },
                loops,
            });
        }
    }
    let used = model.shells[model.bodies[0].outer_shell]
        .faces
        .iter()
        .find(|u| u.face == target)
        .unwrap();
    for (id, group) in groups.iter().enumerate() {
        if id != outside {
            result.push(PocketFace {
                kind: PocketFaceKind::Remainder {
                    source_face: target,
                    regions: group.clone(),
                    outside_triangle: false,
                    reversed: used.reversed,
                },
                loops: extra_loops[id].clone(),
            });
        }
    }
    result.extend(floors);
    result.extend(walls);
    // Preserve every original abstract edge and pair every new cap/wall edge.
    let mut census = BTreeMap::<PocketEdge, (usize, i32)>::new();
    for face in &result {
        for cycle in &face.loops {
            for use_ in cycle {
                let count = census.entry(use_.edge.clone()).or_default();
                count.0 += 1;
                count.1 += if use_.positive { 1 } else { -1 };
            }
        }
    }
    for face in &result {
        for cycle in &face.loops {
            let mut first = None;
            let mut previous = None;
            for use_ in cycle {
                let [a, b] = edge_vertices(model, partition, &use_.edge)?;
                let (a, b) = if use_.positive { (a, b) } else { (b, a) };
                if previous.as_ref().is_some_and(|last| last != &a) {
                    return Err(fail("pocket face boundary has disconnected endpoints"));
                }
                if first.is_none() {
                    first = Some(a);
                }
                previous = Some(b);
            }
            if first != previous {
                return Err(fail("pocket face boundary is open"));
            }
        }
    }
    if census
        .values()
        .any(|(count, sign)| *count != 2 || *sign != 0)
    {
        return Err(fail("pocket result has an unpaired or same-direction edge"));
    }
    Ok(result)
}
