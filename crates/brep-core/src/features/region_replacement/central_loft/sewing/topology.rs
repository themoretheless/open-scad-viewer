//! Incidence is explicit. No distance welding or inferred shared vertices.
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub(super) fn vertex(m: &mut Model, p: [f64; 3]) -> usize {
    let i = m.vertices.len();
    m.vertices.push(crate::Vertex { point: p });
    i
}
pub(super) fn edge(m: &mut Model, c: Curve, v: [usize; 2], degenerate: bool) -> usize {
    let i = m.edges.len();
    m.edges.push(crate::Edge {
        curve: c,
        vertices: v,
        degenerate,
    });
    i
}
pub(super) fn endpoint(c: &Curve, end: usize) -> [f64; 3] {
    let p = &c.control_points[if end == 0 {
        0
    } else {
        c.control_points.len() - 1
    }];
    [p[0], p[1], p[2]]
}
pub(super) fn loop_from_edges(m: &Model, uses: Vec<(usize, Curve)>) -> Result<crate::Loop> {
    require(
        uses.len() >= 3,
        "Corner wire needs at least three boundary uses",
    )?;
    let mut pending: BTreeMap<usize, Curve> = uses.into_iter().collect();
    let first = *pending.keys().next().unwrap();
    let e = &m.edges[first];
    let start = e.vertices[0];
    let mut at = start;
    let mut out = Vec::new();
    while !pending.is_empty() {
        let candidates: Vec<_> = pending
            .keys()
            .copied()
            .filter(|&i| m.edges[i].vertices.contains(&at))
            .collect();
        let id = if out.is_empty() {
            first
        } else {
            require(
                candidates.len() == 1,
                "Boundary identity graph is branched or disconnected",
            )?;
            candidates[0]
        };
        let e = &m.edges[id];
        let reversed = e.vertices[1] == at;
        let pc = pending.remove(&id).unwrap();
        out.push(crate::Coedge::new(
            id,
            reversed,
            if reversed { pc.reverse()? } else { pc },
        ));
        at = e.vertices[usize::from(!reversed)];
    }
    require(at == start, "Corner boundary identity graph is open")?;
    Ok(crate::Loop { coedges: out })
}
pub(super) fn positive_loop(
    wire: &mut crate::Loop,
    tol: f64,
    cells: &mut usize,
    limits: CornerSewLimits,
) -> Result<()> {
    for attempt in 0..2 {
        require(*cells < limits.cells, "Corner trim work exhausted")?;
        let pc = wire
            .coedges
            .iter()
            .map(|c| c.pcurve.clone())
            .collect::<Vec<_>>();
        let allowance = (limits.cells - *cells) / 2;
        require(allowance > 0, "Corner trim work exhausted")?;
        let r = nurbs_core::trim_region_audit::inspect(
            &[pc],
            tol,
            limits.trim_pairs,
            allowance,
            allowance.min(limits.domain_cells),
        )?;
        *cells += r.cells + r.domain_cells;
        require(*cells <= limits.cells, "Corner trim exceeded shared work")?;
        require(
            r.valid == Some(true),
            &format!(
                "Corner wire simplicity remains unproven: edges={:?} reason={} outcomes={:?}",
                wire.coedges.iter().map(|c| c.edge).collect::<Vec<_>>(),
                r.reason,
                r.loops.iter().map(|r| r.outcome()).collect::<Vec<_>>()
            ),
        )?;
        if r.winding[0] == Some(1) {
            return Ok(());
        }
        require(
            attempt == 0 && r.winding[0] == Some(-1),
            "Corner wire winding remains unproven",
        )?;
        wire.coedges.reverse();
        for c in &mut wire.coedges {
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse()?;
        }
    }
    Err(refuse("Corner trim orientation remains unproven"))
}
pub(super) fn orient(m: &mut Model, seed: bool) -> Result<()> {
    let mut uses = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for (f, face) in m.faces.iter().enumerate() {
        for c in &m.loops[face.outer].coedges {
            if !m.edges[c.edge].degenerate {
                uses.entry(c.edge).or_default().push((f, c.reversed));
            }
        }
    }
    require(
        uses.values().all(|u| u.len() == 2 && u[0].0 != u[1].0),
        "Corner shell has an open or multiply used edge",
    )?;
    let mut orientation = vec![None; m.faces.len()];
    orientation[0] = Some(seed);
    let mut queue = VecDeque::from([0]);
    while let Some(f) = queue.pop_front() {
        for pair in uses.values() {
            if let Some(slot) = pair.iter().position(|&(a, _)| a == f) {
                let (other, reverse) = pair[1 - slot];
                let proposed = orientation[f].unwrap() ^ pair[slot].1 ^ reverse ^ true;
                if let Some(old) = orientation[other] {
                    require(old == proposed, "Corner shell orientation conflicts")?;
                } else {
                    orientation[other] = Some(proposed);
                    queue.push_back(other);
                }
            }
        }
    }
    require(
        orientation.iter().all(Option::is_some),
        "Corner shell has disconnected face components",
    )?;
    m.shells = vec![crate::Shell {
        faces: orientation
            .into_iter()
            .enumerate()
            .map(|(face, reversed)| crate::FaceUse {
                face,
                reversed: reversed.unwrap(),
            })
            .collect(),
        closed: false,
    }];
    Ok(())
}
pub(super) fn compact_vertices(m: &mut Model) -> Vec<Option<usize>> {
    let used = m
        .edges
        .iter()
        .flat_map(|e| e.vertices)
        .collect::<BTreeSet<_>>();
    let mut map = vec![None; m.vertices.len()];
    let mut vertices = Vec::new();
    for i in used {
        map[i] = Some(vertices.len());
        vertices.push(m.vertices[i].clone());
    }
    for e in &mut m.edges {
        e.vertices = e.vertices.map(|v| map[v].unwrap());
    }
    m.vertices = vertices;
    map
}
