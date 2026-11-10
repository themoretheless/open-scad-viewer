//! Topological incidence only: positions, embedding and material remain separate.
use std::collections::{BTreeMap, BTreeSet};
pub struct TriangleIncidenceReport {
    pub open_edges: usize,
    pub nonmanifold_edges: usize,
    pub orientation_conflicts: usize,
    pub duplicate_triangles: usize,
    pub degenerate_triangles: usize,
    pub invalid_vertex_links: usize,
}
impl TriangleIncidenceReport {
    pub fn closed_oriented(&self) -> bool {
        self.open_edges
            + self.nonmanifold_edges
            + self.orientation_conflicts
            + self.duplicate_triangles
            + self.degenerate_triangles
            + self.invalid_vertex_links
            == 0
    }
}
pub fn inspect(triangles: &[[usize; 3]]) -> TriangleIncidenceReport {
    let mut out = TriangleIncidenceReport {
        open_edges: 0,
        nonmanifold_edges: 0,
        orientation_conflicts: 0,
        duplicate_triangles: 0,
        degenerate_triangles: usize::from(triangles.is_empty()),
        invalid_vertex_links: 0,
    };
    let mut edges: BTreeMap<(usize, usize), (usize, i32)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut links: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for &t in triangles {
        if t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
            out.degenerate_triangles += 1;
            continue;
        }
        let mut sorted = t;
        sorted.sort();
        if !seen.insert(sorted) {
            out.duplicate_triangles += 1;
        }
        for i in 0..3 {
            let a = t[i];
            let b = t[(i + 1) % 3];
            let e = edges.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
            links.entry(a).or_default().push((b, t[(i + 2) % 3]));
        }
    }
    for (count, orientation) in edges.values() {
        if *count == 1 {
            out.open_edges += 1;
        } else if *count != 2 {
            out.nonmanifold_edges += 1;
        } else if *orientation != 0 {
            out.orientation_conflicts += 1;
        }
    }
    for link in links.values() {
        let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &(a, b) in link {
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }
        let mut visited = BTreeSet::new();
        let mut queue = vec![*adjacency.keys().next().unwrap()];
        while let Some(v) = queue.pop() {
            if visited.insert(v) {
                queue.extend(&adjacency[&v]);
            }
        }
        if adjacency.values().any(|n| n.len() != 2) || visited.len() != adjacency.len() {
            out.invalid_vertex_links += 1;
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tetra() -> Vec<[usize; 3]> {
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
    }
    #[test]
    fn closed_tetra_refuses_open_reversed_duplicate_and_pinched_meshes() {
        let t = tetra();
        assert!(inspect(&t).closed_oriented());
        assert!(!inspect(&t[..3]).closed_oriented());
        let mut reversed = t.clone();
        reversed[0].swap(1, 2);
        assert!(inspect(&reversed).orientation_conflicts > 0);
        let mut duplicate = t.clone();
        duplicate.push(t[0]);
        assert_eq!(inspect(&duplicate).duplicate_triangles, 1);
        let mut pinch = t.clone();
        pinch.extend(t.iter().map(|x| x.map(|v| if v == 0 { 0 } else { v + 3 })));
        let r = inspect(&pinch);
        assert_eq!(r.open_edges, 0);
        assert_eq!(r.invalid_vertex_links, 1);
        assert!(!r.closed_oriented());
    }
}
