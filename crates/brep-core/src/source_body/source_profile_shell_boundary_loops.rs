//! Ordered face constraints from shared segment indices; no coordinate welding.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
pub struct BoundaryLoopSegment {
    pub segment: usize,
    pub reversed: bool,
    pub side: usize,
}
pub struct BoundaryFaceLoop {
    pub face: usize,
    pub wire: usize,
    pub segments: Vec<BoundaryLoopSegment>,
    pub pending: Vec<&'static str>,
}
fn chain(edges: &[(usize, [usize; 2])], ends: [usize; 2]) -> Option<Vec<(usize, bool)>> {
    if edges.is_empty() || ends[0] == ends[1] {
        return None;
    }
    let mut adjacency: BTreeMap<usize, Vec<(usize, usize, bool)>> = BTreeMap::new();
    for &(index, [a, b]) in edges {
        if a == b {
            return None;
        }
        adjacency.entry(a).or_default().push((index, b, false));
        adjacency.entry(b).or_default().push((index, a, true));
    }
    for (&vertex, uses) in &adjacency {
        if uses.len() != if ends.contains(&vertex) { 1 } else { 2 } {
            return None;
        }
    }
    let mut used = BTreeSet::new();
    let mut current = ends[0];
    let mut out = Vec::new();
    while current != ends[1] {
        let candidates = adjacency
            .get(&current)?
            .iter()
            .filter(|(id, _, _)| !used.contains(id))
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return None;
        }
        let &(id, next, reversed) = candidates[0];
        used.insert(id);
        out.push((id, reversed));
        current = next;
    }
    (used.len() == edges.len()).then_some(out)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Validate and order a diagnostic boundary graph for face triangulation.
    /// Missing, branched, cyclic or exhausted constraints remain explicit.
    pub fn order_boundary_loops(&self, graph: &BoundaryGraphReport) -> Vec<BoundaryFaceLoop> {
        let mut uses = BTreeMap::new();
        for (pair, p) in self.pairs.iter().enumerate() {
            for (side, address) in p.uses.into_iter().enumerate() {
                uses.insert(key(address), (pair, side));
            }
        }
        let poles = self.poles.iter().map(|a| key(*a)).collect::<BTreeSet<_>>();
        let mut out = Vec::new();
        for (face, f) in self.faces.iter().enumerate() {
            for (wire, w) in f.loops.iter().enumerate() {
                let mut result = BoundaryFaceLoop {
                    face,
                    wire,
                    segments: vec![],
                    pending: vec![],
                };
                for edge in 0..w.len() {
                    let address = Address { face, wire, edge };
                    if poles.contains(&key(address)) {
                        continue;
                    }
                    let &(pair, side) = uses.get(&key(address)).unwrap();
                    if graph.pending.iter().any(|p| p.pair == pair) {
                        result.pending.push("boundary-loop-pair-incomplete");
                        continue;
                    }
                    let edges = graph
                        .segments
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.pair == pair)
                        .map(|(id, s)| (id, s.vertices))
                        .collect::<Vec<_>>();
                    let ends = [
                        self.boundary_vertex(address, 0).unwrap(),
                        self.boundary_vertex(address, 1).unwrap(),
                    ];
                    if let Some(chain) = chain(&edges, ends) {
                        result
                            .segments
                            .extend(chain.into_iter().map(|(segment, reversed)| {
                                BoundaryLoopSegment {
                                    segment,
                                    reversed,
                                    side,
                                }
                            }));
                    } else {
                        result.pending.push("boundary-loop-chain-unproven");
                    }
                }
                if result.pending.is_empty() {
                    let ends = |s: &BoundaryLoopSegment| {
                        let v = graph.segments[s.segment].vertices;
                        if s.reversed { [v[1], v[0]] } else { v }
                    };
                    if result.segments.is_empty()
                        || (0..result.segments.len()).any(|i| {
                            ends(&result.segments[i])[1]
                                != ends(&result.segments[(i + 1) % result.segments.len()])[0]
                        })
                    {
                        result.pending.push("boundary-loop-closure-unproven");
                    }
                }
                out.push(result);
            }
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::chain;
    #[test]
    fn chain_orders_indices_and_refuses_missing_branch_or_detached_cycle() {
        assert_eq!(
            chain(&[(8, [3, 2]), (4, [0, 3]), (9, [2, 1])], [0, 1]),
            Some(vec![(4, false), (8, false), (9, false)])
        );
        assert_eq!(
            chain(&[(8, [3, 2]), (4, [0, 3]), (9, [2, 1])], [1, 0]),
            Some(vec![(9, true), (8, true), (4, true)])
        );
        assert!(chain(&[(0, [0, 2])], [0, 1]).is_none());
        assert!(chain(&[(0, [0, 2]), (1, [2, 1]), (2, [2, 3])], [0, 1]).is_none());
        assert!(
            chain(
                &[(0, [0, 1]), (1, [3, 4]), (2, [4, 5]), (3, [5, 3])],
                [0, 1]
            )
            .is_none()
        );
    }
}
