//! Keep source entity policy when face subdivision rebuilds topology naming.
use crate::Model;
use std::collections::BTreeSet;
pub(super) fn preserve(model: &mut Model, source: &Model) {
    if source.1.local_tolerances_mm.is_empty() && source.1.sewed_edges.is_empty() {
        return;
    }
    let vertices: BTreeSet<_> = model.1.vertices.iter().copied().collect();
    let edges: BTreeSet<_> = model.1.edges.iter().copied().collect();
    for (&id, &tol) in &source.1.local_tolerances_mm {
        if vertices.contains(&id) || edges.contains(&id) {
            model.1.local_tolerances_mm.insert(id, tol);
        }
    }
    model.1.sewed_edges.extend(
        source
            .1
            .sewed_edges
            .iter()
            .filter(|id| edges.contains(id))
            .copied(),
    );
}
