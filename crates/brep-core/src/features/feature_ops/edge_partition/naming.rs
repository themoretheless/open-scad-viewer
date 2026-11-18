use super::*;
use std::collections::BTreeSet;
pub(super) fn assign(
    source: &Model,
    model: &mut Model,
    parent: usize,
    edges: &[usize],
    vertices: &[usize],
    loops: &BTreeSet<usize>,
    key: &str,
) -> Result<()> {
    let id = source.1.edges[parent];
    let signature = id.to_string();
    let old_local = source.1.local_tolerances_mm.get(&id).copied();
    model.1.local_tolerances_mm.remove(&id);
    model.1.sewed_edges.remove(&id);
    for (i, &vertex) in vertices.iter().enumerate() {
        let child = TopoId::derive(
            TopoKind::Vertex,
            "edge-partition",
            key,
            &format!("cut-{i}"),
            signature.as_bytes(),
        );
        model.1.vertices.push(child);
        if let Some(t) = old_local {
            model.1.local_tolerances_mm.insert(child, t);
        }
        assert_eq!(vertex, model.1.vertices.len() - 1);
    }
    let mut children = Vec::new();
    for (i, &edge) in edges.iter().enumerate() {
        let child = TopoId::derive(
            TopoKind::Edge,
            "edge-partition",
            key,
            &format!("piece-{i}"),
            signature.as_bytes(),
        );
        if i == 0 {
            model.1.edges[edge] = child;
        } else {
            model.1.edges.push(child);
        }
        children.push(child);
        if let Some(t) = old_local {
            model.1.local_tolerances_mm.insert(child, t);
        }
    }
    model.1.lineage.push(crate::TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "edge".into(),
        parents: vec![id],
        children,
    });
    for &wire in loops {
        let parent = source.1.loops[wire];
        let child = TopoId::derive(
            TopoKind::Loop,
            "edge-partition",
            key,
            &parent.to_string(),
            signature.as_bytes(),
        );
        model.1.loops[wire] = child;
        model.1.lineage.push(crate::TopologyLineageRecord {
            operation: "persist".into(),
            entity_kind: "loop".into(),
            parents: vec![parent],
            children: vec![child],
        });
    }
    model.refresh_change_set(&[source]);
    for change in &mut model.1.change_set.changes {
        if change.kind == crate::ChangeKind::Generated
            && change
                .children
                .iter()
                .any(|id| vertices.iter().any(|v| model.1.vertices[*v] == *id))
        {
            change.provenance.operation = "edge-partition".into();
            change.provenance.occurrence = key.into();
            change.role = "original-edge-cut-vertex".into();
        }
    }
    model.validate()?;
    Ok(())
}
