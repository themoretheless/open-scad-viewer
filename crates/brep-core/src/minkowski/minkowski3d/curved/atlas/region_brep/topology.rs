//! Exact cell incidence and oriented cap sewing; no rounded algebraic nodes.
use super::*;
pub(super) fn assemble(
    p: &domains::DomainPartition,
    selected: &[u32],
    orientation: bool,
    lower_orientation: bool,
) -> Result<(
    Vec<AlgebraicRegionFace>,
    Vec<AlgebraicRegionSeam>,
    Vec<Vec<u32>>,
)> {
    assemble_recipe(p, selected, orientation, lower_orientation, false)
}
pub(super) fn with_side_boundaries(
    p: &domains::DomainPartition,
    selected: &[u32],
    orientation: bool,
    lower_orientation: bool,
) -> Result<(
    Vec<AlgebraicRegionFace>,
    Vec<AlgebraicRegionSeam>,
    Vec<Vec<u32>>,
)> {
    assemble_recipe(p, selected, orientation, lower_orientation, true)
}
fn assemble_recipe(
    p: &domains::DomainPartition,
    selected: &[u32],
    orientation: bool,
    lower_orientation: bool,
    side_boundaries: bool,
) -> Result<(
    Vec<AlgebraicRegionFace>,
    Vec<AlgebraicRegionSeam>,
    Vec<Vec<u32>>,
)> {
    let chosen = selected.iter().copied().collect::<BTreeSet<_>>();
    if chosen.len() != selected.len()
        || chosen.iter().any(|id| {
            p.faces
                .get(*id as usize)
                .is_none_or(|f| !side_boundaries && f.signs[0] != 1)
        })
    {
        return Err(fail(
            "algebraic B-rep selection must contain unique positive cells",
        ));
    }
    let mut incident = vec![vec![]; p.edges.len()];
    for (id, f) in p.faces.iter().enumerate() {
        for edge in &f.boundary {
            incident[edge.unsigned_abs() as usize - 1].push((id as u32, *edge));
        }
    }
    let mut neighbors = vec![vec![]; p.faces.len()];
    let mut boundary = BTreeMap::new();
    for (id, uses) in incident.iter().enumerate() {
        let kept = uses
            .iter()
            .filter(|(f, _)| chosen.contains(f))
            .collect::<Vec<_>>();
        if kept.len() == 2 {
            if kept[0].1 != -kept[1].1 {
                return Err(fail("algebraic region has same-side internal edge uses"));
            }
            neighbors[kept[0].0 as usize].push(kept[1].0);
            neighbors[kept[1].0 as usize].push(kept[0].0);
        } else if kept.len() == 1 {
            // The cap-pair recipe requires opposite signs. The Model pocket
            // recipe instead supplies a separate ruled wall for this edge.
            // Opposite source signs on the two open cells force exact F=0
            // on their complete common arc by continuity. No point sample
            // or root-interval midpoint is used to assert surface equality.
            if uses.len() != 2
                || (!side_boundaries
                    && uses
                        .iter()
                        .filter(|(f, _)| !chosen.contains(f))
                        .any(|(f, _)| p.faces[*f as usize].signs[0] != -1))
            {
                return Err(fail("selected region exposes a nonintersection or triangle boundary; independent side caps are required"));
            }
            boundary.insert(id as u32, (kept[0].0, kept[0].1));
        } else if kept.len() > 2 {
            return Err(fail("algebraic B-rep edge incidence exceeds two"));
        }
    }
    let mut components = vec![];
    let mut assigned = BTreeSet::new();
    for f in selected {
        if assigned.contains(f) {
            continue;
        }
        let mut todo = vec![*f];
        let mut component = vec![];
        while let Some(f) = todo.pop() {
            if assigned.insert(f) {
                component.push(f);
                todo.extend(&neighbors[f as usize]);
            }
        }
        component.sort();
        components.push(component);
    }
    let mut faces = vec![];
    let mut seams = vec![];
    for component in &components {
        let members = component.iter().copied().collect::<BTreeSet<_>>();
        let mut next = BTreeMap::new();
        let mut incoming = BTreeSet::new();
        let mut ids = vec![];
        for (id, (f, signed)) in &boundary {
            if members.contains(f) {
                let e = &p.edges[*id as usize];
                let [a, b] = if *signed > 0 {
                    e.endpoints
                } else {
                    [e.endpoints[1], e.endpoints[0]]
                };
                if next.insert(a, (b, *signed)).is_some() || !incoming.insert(b) {
                    return Err(fail(
                        "algebraic B-rep boundary has a branching or pinched vertex",
                    ));
                }
                ids.push(*id);
            }
        }
        if next.is_empty() || next.keys().copied().collect::<BTreeSet<_>>() != incoming {
            return Err(fail(
                "algebraic B-rep boundary is not a union of closed cycles",
            ));
        }
        let mut cycles = vec![];
        while let Some((&start, _)) = next.iter().next() {
            let mut at = start;
            let mut cycle = vec![];
            loop {
                let (b, e) = next
                    .remove(&at)
                    .ok_or_else(|| fail("algebraic B-rep cycle is open"))?;
                cycle.push(e);
                at = b;
                if at == start {
                    break;
                }
            }
            if cycle.len() < 2 {
                return Err(fail("algebraic B-rep boundary cycle is degenerate"));
            }
            cycles.push(cycle);
        }
        if orientation {
            cycles = cycles
                .iter()
                .map(|c| c.iter().rev().map(|e| -e).collect())
                .collect();
        }
        let upper = faces.len() as u32;
        let lower = upper + 1;
        let opposite = cycles
            .iter()
            .map(|cycle| cycle.iter().rev().map(|e| -e).collect())
            .collect();
        faces.push(AlgebraicRegionFace {
            carrier: 0,
            reversed: orientation,
            regions: component.clone(),
            boundary_cycles: cycles,
        });
        faces.push(AlgebraicRegionFace {
            carrier: 1,
            reversed: !lower_orientation,
            regions: component.clone(),
            boundary_cycles: opposite,
        });
        for id in ids {
            let signed = boundary[&id].1 * if orientation { -1 } else { 1 };
            seams.push(AlgebraicRegionSeam {
                domain_edge: id,
                faces: [upper, lower],
                oriented_uses: [signed, -signed],
            });
        }
    }
    Ok((faces, seams, components))
}
