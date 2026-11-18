//! Glue only identical ORIGINAL curve/surface roots, never nearby vertices.
use super::*;
pub(in super::super) fn join(
    pairs: &[OriginalFacePairGraph],
) -> Result<(
    Vec<OriginalRootKey>,
    Vec<OriginalIntersectionArc>,
    Vec<OriginalIntersectionCycle>,
)> {
    let mut vertices = Vec::new();
    let mut index = BTreeMap::new();
    let mut arcs = Vec::new();
    for (pair, p) in pairs.iter().enumerate() {
        for (piece, r) in p.pieces.iter().enumerate() {
            if r.inside != [true, true] {
                continue;
            }
            let mut ends = [0; 2];
            for (i, end) in r.endpoints.iter().enumerate() {
                let GraphEndpoint::Boundary(key) = end else {
                    return Err(fail("active graph lost an original root identity"));
                };
                ends[i] = *index.entry(key.clone()).or_insert_with(|| {
                    let id = vertices.len();
                    vertices.push(key.clone());
                    id
                });
            }
            if ends[0] == ends[1] {
                return Err(fail(
                    "one physical arc cannot close by relabelling its endpoints",
                ));
            }
            arcs.push(OriginalIntersectionArc {
                pair,
                piece,
                vertices: ends,
            });
        }
    }
    let mut incidence = vec![Vec::new(); vertices.len()];
    for (i, a) in arcs.iter().enumerate() {
        for &v in &a.vertices {
            incidence[v].push(i);
        }
    }
    if incidence.iter().any(|u| u.len() != 2) {
        return Err(fail(
            "complete closed-source seam graph needs exactly two actual incident arcs per original root",
        ));
    }
    let mut used = BTreeSet::new();
    let mut cycles = Vec::new();
    for first in 0..arcs.len() {
        if used.contains(&first) {
            continue;
        }
        let start = arcs[first].vertices[0];
        let mut at = start;
        let mut edge = first;
        let mut cycle = Vec::new();
        loop {
            if !used.insert(edge) {
                return Err(fail("seam cycle repeated an already owned physical arc"));
            }
            let e = &arcs[edge];
            let forward = e.vertices[0] == at;
            if !forward && e.vertices[1] != at {
                return Err(fail("seam cycle lost endpoint incidence"));
            }
            cycle.push((edge as i32 + 1) * if forward { 1 } else { -1 });
            at = e.vertices[usize::from(forward)];
            if at == start {
                break;
            }
            edge = *incidence[at].iter().find(|&&i| i != edge).unwrap();
        }
        cycles.push(OriginalIntersectionCycle { arcs: cycle });
    }
    Ok((vertices, arcs, cycles))
}
