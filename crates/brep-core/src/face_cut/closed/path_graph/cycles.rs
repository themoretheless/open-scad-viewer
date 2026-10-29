use super::*;
pub(super) struct Cycles {
    pub rings: Vec<Vec<Coedge>>,
    pub path_cycles: Vec<[usize; 2]>,
}
pub(super) fn build(
    source: &Model,
    face: usize,
    paths: &[BridgePath],
    ports: &[[(usize, usize); 2]],
    offsets: &[usize],
) -> Result<Cycles> {
    let f = &source.faces[face];
    let mut edges = Vec::new();
    let mut next = Vec::new();
    let mut starts = std::collections::BTreeMap::new();
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        let start = edges.len();
        starts.insert(wire, start);
        let cs = &source.loops[wire].coedges;
        for (i, c) in cs.iter().enumerate() {
            edges.push(c.clone());
            next.push(start + (i + 1) % cs.len());
        }
    }
    let mut directed = Vec::new();
    for (j, p) in paths.iter().enumerate() {
        let n = p.segments.len();
        let forward = edges.len();
        for (i, s) in p.segments.iter().enumerate() {
            edges.push(Coedge::new(offsets[j] + i, false, s.pcurve.clone()));
            next.push(forward + i + 1);
        }
        let backward = edges.len();
        for (i, s) in p.segments.iter().enumerate().rev() {
            let pc = s.pcurve.reverse()?;
            if !reference::exact_reversal(&pc, &s.pcurve) {
                return Err(unproven(
                    "Path graph cancellation requires exact saved UV reversal",
                ));
            }
            edges.push(Coedge::new(offsets[j] + i, true, pc));
            next.push(edges.len());
        }
        for (side, start) in [(0, forward), (1, backward)] {
            let (wire, index) = ports[j][side];
            let base = starts[&wire];
            let previous = base
                + (index + source.loops[wire].coedges.len() - 1) % source.loops[wire].coedges.len();
            next[previous] = start;
            // Traversing into the opposite boundary follows its material arc.
            let (target_wire, target_index) = ports[j][1 - side];
            next[start + n - 1] = starts[&target_wire] + target_index;
        }
        directed.push([forward, backward]);
    }
    let mut incoming = vec![0; next.len()];
    for &to in &next {
        if to >= next.len() {
            return Err(unproven(
                "Path graph successor escaped its authored halfedges",
            ));
        }
        incoming[to] += 1;
    }
    if incoming.iter().any(|&n| n != 1) {
        return Err(unproven(
            "Path graph successors are not a closed permutation",
        ));
    }
    let mut owner = vec![usize::MAX; edges.len()];
    let mut rings = Vec::new();
    for start in 0..edges.len() {
        if owner[start] != usize::MAX {
            continue;
        }
        let mut ring = Vec::new();
        let mut at = start;
        loop {
            if owner[at] != usize::MAX {
                if at != start {
                    return Err(unproven("Path graph walked into another cycle"));
                }
                break;
            }
            owner[at] = rings.len();
            ring.push(edges[at].clone());
            at = next[at];
        }
        rings.push(ring);
    }
    let path_cycles = directed
        .into_iter()
        .map(|p| [owner[p[0]], owner[p[1]]])
        .collect();
    Ok(Cycles { rings, path_cycles })
}
