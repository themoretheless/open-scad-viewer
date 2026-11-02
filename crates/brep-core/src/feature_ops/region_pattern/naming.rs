//! Explicit original-to-occurrence IDs, separate from geometric coincidence.
use super::*;
pub(super) fn ids(m: &Model) -> BTreeSet<TopoId> {
    m.identity_groups()
        .into_iter()
        .flat_map(|(_, v)| v.iter().copied())
        .collect()
}
pub(super) fn modified(
    m: &mut Model,
    parent: TopoId,
    child: TopoId,
    operation: &str,
    occurrence: &str,
) -> Result<()> {
    if parent.kind() != child.kind() || parent == child {
        return Err(error("Invalid feature naming relation"));
    }
    m.1.change_set.nodes.insert(parent, parent.kind());
    m.1.change_set.nodes.insert(child, child.kind());
    m.1.change_set.changes.push(crate::TopologyChange {
        kind: crate::ChangeKind::Modified,
        topo_kind: child.kind(),
        parents: vec![parent],
        children: vec![child],
        provenance: crate::ChangeProvenance {
            operation: operation.into(),
            operand: None,
            occurrence: occurrence.into(),
        },
        role: "original-feature-entity".into(),
        anchor: None,
    });
    m.1.change_set.validate().map_err(|e| error(e.to_string()))
}
pub(super) fn instance(
    mut moved: Model,
    source: &Model,
    pattern: &str,
    key: &str,
) -> Result<(Model, BTreeMap<TopoId, TopoId>)> {
    let mut map = BTreeMap::new();
    for (kind, old) in source.identity_groups() {
        for &id in old {
            map.insert(
                id,
                TopoId::derive(
                    kind,
                    "feature-region-instance",
                    pattern,
                    key,
                    id.to_string().as_bytes(),
                ),
            );
        }
    }
    for (kind, old) in source.identity_groups() {
        let output = match kind {
            TopoKind::Vertex => &mut moved.1.vertices,
            TopoKind::Edge => &mut moved.1.edges,
            TopoKind::Loop => &mut moved.1.loops,
            TopoKind::Face => &mut moved.1.faces,
            TopoKind::Shell => &mut moved.1.shells,
            TopoKind::Body => &mut moved.1.bodies,
            TopoKind::ControlPoint => {
                return Err(error(
                    "Unexpected control-point ID in BRep occurrence topology",
                ));
            }
        };
        *output = old.iter().map(|id| map[id]).collect();
    }
    moved.1.local_tolerances_mm = source
        .1
        .local_tolerances_mm
        .iter()
        .map(|(id, t)| (map[id], *t))
        .collect();
    moved.1.sewed_edges = source.1.sewed_edges.iter().map(|id| map[id]).collect();
    moved.refresh_change_set(&[source]);
    for (&parent, &child) in &map {
        modified(
            &mut moved,
            parent,
            child,
            "feature-region-instance",
            &format!("{pattern}/{key}"),
        )?;
    }
    moved.validate()?;
    Ok((moved, map))
}
pub(super) fn descendants(m: &Model, id: TopoId) -> Vec<TopoId> {
    let active = ids(m);
    let mut found = BTreeSet::new();
    let mut todo = vec![id];
    while let Some(parent) = todo.pop() {
        if found.insert(parent) {
            for c in &m.1.change_set.changes {
                if c.parents.contains(&parent) {
                    todo.extend(c.children.iter().copied());
                }
            }
        }
    }
    found.intersection(&active).copied().collect()
}

pub(super) fn derived(
    m: &mut Model,
    mut parents: Vec<TopoId>,
    index: usize,
    is_loop: bool,
) -> Result<()> {
    parents.sort();
    parents.dedup();
    let child = if is_loop {
        m.1.loops[index]
    } else {
        m.1.faces[index]
    };
    if parents.len() == 1 {
        return modified(
            m,
            parents[0],
            child,
            "feature-region-close",
            "original-support-rim",
        );
    }
    m.1.change_set.changes.push(crate::TopologyChange {
        kind: crate::ChangeKind::Merge,
        topo_kind: child.kind(),
        parents,
        children: vec![child],
        provenance: crate::ChangeProvenance {
            operation: "feature-region-carrier-closure".into(),
            operand: None,
            occurrence: child.to_string(),
        },
        role: "derived-support-carrier".into(),
        anchor: None,
    });
    m.1.change_set.validate().map_err(|e| error(e.to_string()))
}
