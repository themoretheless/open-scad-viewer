use crate::{MeshView, Result, check, error};
use std::collections::{BTreeMap, BTreeSet};

impl MeshView<'_> {
    /// Ordered boundary loops, each repeating its first vertex at the end.
    /// Ambiguous nonmanifold/branching boundaries are rejected, not guessed.
    pub fn boundary_loops(&self) -> Result<Vec<Vec<usize>>> {
        let (report, edges) = self.inspect_with_edges()?;
        check(
            report.non_manifold_edges == 0
                && report.orientation_conflicts == 0
                && report.degenerate_triangles == 0,
            "Boundary loops require a consistently oriented manifold mesh.",
        )?;
        let mut next = BTreeMap::new();
        let mut incoming = BTreeSet::new();
        for [a, b] in edges.boundary() {
            check(
                next.insert(a, b).is_none() && incoming.insert(b),
                "Boundary loops branch at a vertex.",
            )?;
        }
        drop(edges);
        check(
            next.keys().all(|k| incoming.contains(k)),
            "Boundary chain does not close.",
        )?;
        let mut loops = Vec::new();
        while let Some((&start, _)) = next.first_key_value() {
            let mut path = vec![start];
            let mut current = start;
            loop {
                let target = next
                    .remove(&current)
                    .ok_or_else(|| error("Boundary chain does not close."))?;
                path.push(target);
                if target == start {
                    break;
                }
                current = target;
            }
            loops.push(path);
        }
        Ok(loops)
    }
}
