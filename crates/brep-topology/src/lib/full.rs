//! lib operations: full.
use super::*;
impl ValidationPlan {
pub(super) fn full() -> Self {
        Self {
            full: true,
            vertices: FxHashSet::default(),
            faces: FxHashSet::default(),
            shells: FxHashSet::default(),
        }
    }
/// Expand a declared scope to the closure of entities whose checks read
    /// changed data. Any out-of-range reference or shrunk array means the
    /// edit was structural beyond the scope's description: fall back to full.
    pub(super) fn for_edit<C, S, P, V>(
        old: &Model<C, S, P, V>,
        scope: &EditScope,
        new: &Model<C, S, P, V>,
    ) -> Self {
        let shrunk = new.vertices.len() < old.vertices.len()
            || new.edges.len() < old.edges.len()
            || new.loops.len() < old.loops.len()
            || new.faces.len() < old.faces.len()
            || new.shells.len() < old.shells.len()
            || new.bodies.len() < old.bodies.len();
        let in_scope = scope.vertices.iter().all(|&i| i < old.vertices.len())
            && scope.edges.iter().all(|&i| i < old.edges.len())
            && scope.loops.iter().all(|&i| i < old.loops.len())
            && scope.faces.iter().all(|&i| i < old.faces.len())
            && scope.shells.iter().all(|&i| i < old.shells.len());
        if scope.full || shrunk || !in_scope {
            return Self::full();
        }
        let vertices: FxHashSet<usize> = scope
            .vertices
            .iter()
            .copied()
            .chain(old.vertices.len()..new.vertices.len())
            .collect();
        let edges: FxHashSet<usize> = scope
            .edges
            .iter()
            .copied()
            .chain(old.edges.len()..new.edges.len())
            .collect();
        // A loop's chain check reads its coedges and the referenced edges.
        let mut loops: FxHashSet<usize> = scope
            .loops
            .iter()
            .copied()
            .chain(old.loops.len()..new.loops.len())
            .collect();
        for (li, l) in new.loops.iter().enumerate() {
            if l.coedges.iter().any(|c| edges.contains(&c.edge)) {
                loops.insert(li);
            }
        }
        // A face's checks read its loops and their coedges/edges.
        let mut faces: FxHashSet<usize> = scope
            .faces
            .iter()
            .copied()
            .chain(old.faces.len()..new.faces.len())
            .collect();
        for (fi, f) in new.faces.iter().enumerate() {
            if std::iter::once(&f.outer)
                .chain(&f.holes)
                .any(|l| loops.contains(l))
            {
                faces.insert(fi);
            }
        }
        // A shell's checks read its face uses, faces, loops, coedges and
        // edges (including degenerate flags and vertex references).
        let mut shells: FxHashSet<usize> = scope
            .shells
            .iter()
            .copied()
            .chain(old.shells.len()..new.shells.len())
            .collect();
        for (si, s) in new.shells.iter().enumerate() {
            let hit = s.faces.iter().any(|u| {
                if faces.contains(&u.face) {
                    return true;
                }
                let Some(f) = new.faces.get(u.face) else {
                    return true; // Unknown reference: revalidate this shell.
                };
                std::iter::once(&f.outer).chain(&f.holes).any(|l| {
                    let Some(l) = new.loops.get(*l) else {
                        return true;
                    };
                    l.coedges.iter().any(|c| edges.contains(&c.edge))
                })
            });
            if hit {
                shells.insert(si);
            }
        }
        Self {
            full: false,
            vertices,
            faces,
            shells,
        }
    }
}
