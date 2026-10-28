//! Resolve literal endpoint topology without assuming the original outer loop.
use crate::Model;
pub(super) fn owned_boundary(
    source: &Model,
    face: usize,
    endpoints: [usize; 2],
) -> Option<(usize, [usize; 2])> {
    let f = source.faces.get(face)?;
    let mut hits = [Vec::new(), Vec::new()];
    for (side, vertex) in endpoints.into_iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (i, c) in source.loops[wire].coedges.iter().enumerate() {
                if source.edges[c.edge].vertices[usize::from(c.reversed)] == vertex {
                    hits[side].push((wire, i));
                }
            }
        }
    }
    if hits.iter().any(|h| h.len() != 1) || hits[0][0].0 != hits[1][0].0 {
        return None;
    }
    Some((hits[0][0].0, [hits[0][0].1, hits[1][0].1]))
}
