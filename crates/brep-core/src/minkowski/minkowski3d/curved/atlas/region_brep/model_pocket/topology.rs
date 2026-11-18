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

/// Retained (unselected) partition faces grouped into connected components
/// across interior edges whose both incident faces are retained.
struct RetainedGroups {
    /// Partition faces incident to each partition edge.
    incident: Vec<Vec<u32>>,
    /// Group id owning each retained partition face.
    owner: Vec<usize>,
    groups: Vec<Vec<u32>>,
    /// The one group that touches the original face boundary.
    outside: usize,
}

/// Faces created along the pocket seam: floor caps, side walls and the
/// extra remainder loops each retained group receives.
struct SeamFaces {
    extra_loops: Vec<Vec<Vec<PocketEdgeUse>>>,
    walls: Vec<PocketFace>,
    floors: Vec<PocketFace>,
}

fn edge_incidence(partition: &domains::DomainPartition) -> Vec<Vec<u32>> {
    let mut incident = vec![vec![]; partition.edges.len()];
    for (f, face) in partition.faces.iter().enumerate() {
        for edge in &face.boundary {
            incident[edge.unsigned_abs() as usize - 1].push(f as u32);
        }
    }
    incident
}

fn retained_groups(
    partition: &domains::DomainPartition,
    chosen: &BTreeSet<u32>,
) -> Result<RetainedGroups> {
    let incident = edge_incidence(partition);
    let mut neighbors = vec![vec![]; partition.faces.len()];
    for uses in &incident {
        if let [a, b] = uses[..]
            && !chosen.contains(&a)
            && !chosen.contains(&b)
        {
            neighbors[a as usize].push(b);
            neighbors[b as usize].push(a);
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
    let touches_boundary = |group: &Vec<u32>| {
        group.iter().any(|f| {
            partition.faces[*f as usize]
                .boundary
                .iter()
                .any(|e| incident[e.unsigned_abs() as usize - 1].len() == 1)
        })
    };
    let outside = groups
        .iter()
        .enumerate()
        .filter(|(_, group)| touches_boundary(group))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let [outside] = outside[..] else {
        return Err(fail(
            "pocket remainder has no unique original-face exterior component",
        ));
    };
    Ok(RetainedGroups {
        incident,
        owner,
        groups,
        outside,
    })
}

fn wall_face(id: u32, positive: bool, endpoints: [u32; 2]) -> PocketFace {
    let [a, b] = if positive {
        endpoints
    } else {
        [endpoints[1], endpoints[0]]
    };
    PocketFace {
        kind: PocketFaceKind::Wall { domain_edge: id },
        loops: vec![vec![
            signed(PocketEdge::Top(id), positive),
            signed(PocketEdge::Vertical(b), true),
            signed(PocketEdge::Bottom(id), !positive),
            signed(PocketEdge::Vertical(a), false),
        ]],
    }
}

fn seam_faces(
    partition: &domains::DomainPartition,
    chosen: &BTreeSet<u32>,
    retained: &RetainedGroups,
    templates: &[AlgebraicRegionFace],
    components: &[Vec<u32>],
    floor_reversed: bool,
) -> Result<SeamFaces> {
    let mut seam = SeamFaces {
        extra_loops: vec![vec![]; retained.groups.len()],
        walls: vec![],
        floors: vec![],
    };
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
                let other = retained.incident[id as usize]
                    .iter()
                    .find(|f| !chosen.contains(f))
                    .copied()
                    .ok_or_else(|| fail("pocket seam has no retained source region"))?;
                let o = retained.owner[other as usize];
                if remainder_owner.is_some_and(|old| old != o) {
                    return Err(fail("pocket loop changes its retained face component"));
                }
                remainder_owner = Some(o);
                floor_loop.push(signed(PocketEdge::Bottom(id), positive));
                remainder_loop.push(signed(PocketEdge::Top(id), !positive));
                seam.walls.push(wall_face(id, positive, edge.endpoints));
            }
            remainder_loop.reverse();
            let owner = remainder_owner.ok_or_else(|| fail("pocket seam cycle has no edges"))?;
            seam.extra_loops[owner].push(remainder_loop);
            floor_loops.push(floor_loop);
        }
        seam.floors.push(PocketFace {
            kind: PocketFaceKind::Floor {
                component: component as u32,
                regions: cells.clone(),
                reversed: floor_reversed,
            },
            loops: floor_loops,
        });
    }
    Ok(seam)
}

/// Original shell faces (the target face replaced by its exterior remainder)
/// followed by the interior remainder components.
fn source_and_remainder_faces(
    model: &Model,
    target: usize,
    retained: &RetainedGroups,
    extra_loops: &[Vec<Vec<PocketEdgeUse>>],
) -> Result<Vec<PocketFace>> {
    let body = model
        .bodies
        .first()
        .ok_or_else(|| fail("pocket model has no body"))?;
    let shell_faces = &model.shells[body.outer_shell].faces;
    let outside = retained.outside;
    let mut result = vec![];
    for used in shell_faces {
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
                    regions: retained.groups[outside].clone(),
                    outside_triangle: true,
                    reversed: used.reversed,
                },
                loops,
            });
        }
    }
    let used = shell_faces
        .iter()
        .find(|u| u.face == target)
        .ok_or_else(|| fail("pocket target face is not on the outer shell"))?;
    for (id, group) in retained.groups.iter().enumerate() {
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
    Ok(result)
}

fn check_closed_loops(
    model: &Model,
    partition: &domains::DomainPartition,
    result: &[PocketFace],
) -> Result<()> {
    for face in result {
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
    Ok(())
}

/// Preserve every original abstract edge and pair every new cap/wall edge.
fn check_edge_pairing(result: &[PocketFace]) -> Result<()> {
    let mut census = BTreeMap::<PocketEdge, (usize, i32)>::new();
    for face in result {
        for cycle in &face.loops {
            for use_ in cycle {
                let count = census.entry(use_.edge.clone()).or_default();
                count.0 += 1;
                count.1 += if use_.positive { 1 } else { -1 };
            }
        }
    }
    if census
        .values()
        .any(|(count, sign)| *count != 2 || *sign != 0)
    {
        return Err(fail("pocket result has an unpaired or same-direction edge"));
    }
    Ok(())
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
    let retained = retained_groups(partition, &chosen)?;
    let seam = seam_faces(
        partition,
        &chosen,
        &retained,
        templates,
        components,
        floor_reversed,
    )?;
    let mut result = source_and_remainder_faces(model, target, &retained, &seam.extra_loops)?;
    result.extend(seam.floors);
    result.extend(seam.walls);
    check_closed_loops(model, partition, &result)?;
    check_edge_pairing(&result)?;
    Ok(result)
}
