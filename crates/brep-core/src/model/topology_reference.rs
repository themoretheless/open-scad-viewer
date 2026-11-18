//! Durable vertex references over the validated authoritative ChangeSet.
//! Geometry proximity and current array indices are never naming evidence.
use crate::{Error, Model, Result, TopoId, TopoKind};
use std::collections::{BTreeMap, BTreeSet};
fn refused(message: &str) -> Error {
    Error::new("BREP_TOPOLOGY_REFERENCE", message)
}
#[derive(Debug, PartialEq, Eq)]
pub enum VertexResolution {
    Persistent(usize),
    Followed(usize),
    Missing,
    Ambiguous(Vec<usize>),
}
pub struct VertexReferenceIndex {
    current: BTreeMap<TopoId, usize>,
    successors: BTreeMap<TopoId, BTreeSet<TopoId>>,
}
impl VertexReferenceIndex {
    pub fn new(model: &Model) -> Result<Self> {
        model.validate()?;
        if model.1.change_set.nodes.len() > 100000 || model.1.change_set.changes.len() > 100000 {
            return Err(refused(
                "Vertex reference history exceeds 100000 nodes or changes",
            ));
        }
        let current = model
            .1
            .vertices
            .iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        let mut successors: BTreeMap<TopoId, BTreeSet<TopoId>> = BTreeMap::new();
        let mut edges = 0;
        for change in &model.1.change_set.changes {
            if change.topo_kind != TopoKind::Vertex {
                continue;
            }
            for &parent in &change.parents {
                for &child in &change.children {
                    if parent != child && successors.entry(parent).or_default().insert(child) {
                        edges += 1;
                        if edges > 100000 {
                            return Err(refused(
                                "Vertex reference history exceeds 100000 successor links",
                            ));
                        }
                    }
                }
            }
        }
        Ok(Self {
            current,
            successors,
        })
    }
    pub fn resolve(&self, id: TopoId, max_work: usize) -> Result<VertexResolution> {
        if id.kind() != TopoKind::Vertex || !(1..=1000000).contains(&max_work) {
            return Err(refused(
                "Vertex reference needs a vertex ID and 1..1000000 work units",
            ));
        }
        let mut pending = vec![id];
        let mut visited = BTreeSet::new();
        let mut current = BTreeSet::new();
        let mut work = 0;
        while let Some(node) = pending.pop() {
            if !visited.insert(node) {
                continue;
            }
            work += 1;
            if work > max_work {
                return Err(refused("Vertex reference traversal work budget exceeded"));
            }
            if let Some(index) = self.current.get(&node) {
                current.insert(*index);
            }
            if let Some(children) = self.successors.get(&node) {
                work += children.len();
                if work > max_work {
                    return Err(refused("Vertex reference traversal work budget exceeded"));
                }
                pending.extend(children.iter().copied());
            }
        }
        // A split may preserve the parent's ID on one live branch. Checking
        // the direct ID first would silently hide its other live successors.
        Ok(match current.len() {
            0 => VertexResolution::Missing,
            1 => {
                let index = *current.first().unwrap();
                if self.current.get(&id) == Some(&index) {
                    VertexResolution::Persistent(index)
                } else {
                    VertexResolution::Followed(index)
                }
            }
            _ => VertexResolution::Ambiguous(current.into_iter().collect()),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChangeKind, ChangeProvenance, TopologyChange};
    fn change(model: &mut Model, kind: ChangeKind, parents: Vec<TopoId>, children: Vec<TopoId>) {
        for id in parents.iter().chain(&children) {
            model.1.change_set.nodes.insert(*id, TopoKind::Vertex);
        }
        model.1.change_set.changes.push(TopologyChange {
            kind,
            topo_kind: TopoKind::Vertex,
            parents,
            children,
            provenance: ChangeProvenance {
                operation: "reference-test".into(),
                operand: None,
                occurrence: "part".into(),
            },
            role: "vertex".into(),
            anchor: None,
        });
    }
    #[test]
    fn follows_persistent_vertices_after_real_edge_subdivision_and_reordering() {
        let source = crate::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let id = source.1.vertices[0];
        let p = source.vertices[0].point;
        let wire =
            crate::sketch::polygon_wire(vec![[0., 0.], [1., 0.], [2., 0.], [2., 1.], [0., 1.]])
                .unwrap();
        let mut changed = crate::prism::extrude(&[wire], 0., 1.).unwrap();
        changed.inherit_topology_ids(&[&source]);
        assert_eq!(changed.vertices.len(), 10);
        changed.vertices.swap(0, 1);
        changed.1.vertices.swap(0, 1);
        for edge in &mut changed.edges {
            for v in &mut edge.vertices {
                if *v == 0 {
                    *v = 1;
                } else if *v == 1 {
                    *v = 0;
                }
            }
        }
        let VertexResolution::Persistent(index) = VertexReferenceIndex::new(&changed)
            .unwrap()
            .resolve(id, 10000)
            .unwrap()
        else {
            panic!("Lost retained vertex")
        };
        assert_eq!(changed.vertices[index].point, p);
    }
    #[test]
    fn follows_unique_modification_and_refuses_a_split_even_when_parent_survives() {
        let mut model = crate::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let original = model.1.vertices[0];
        let replacement = TopoId::derive(
            TopoKind::Vertex,
            "reference-test",
            "part",
            "replacement",
            b"new",
        );
        model.1.vertices[0] = replacement;
        change(
            &mut model,
            ChangeKind::Modified,
            vec![original],
            vec![replacement],
        );
        assert_eq!(
            VertexReferenceIndex::new(&model)
                .unwrap()
                .resolve(original, 10000)
                .unwrap(),
            VertexResolution::Followed(0)
        );
        let other = model.1.vertices[1];
        change(
            &mut model,
            ChangeKind::Split,
            vec![replacement],
            vec![replacement, other],
        );
        assert_eq!(
            VertexReferenceIndex::new(&model)
                .unwrap()
                .resolve(original, 10000)
                .unwrap(),
            VertexResolution::Ambiguous(vec![0, 1])
        );
        assert!(
            VertexReferenceIndex::new(&model)
                .unwrap()
                .resolve(original, 1)
                .is_err()
        );
    }
    #[test]
    fn unique_merge_successor_is_followed_and_cyclic_history_is_rejected() {
        let mut model = crate::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let a = TopoId::derive(TopoKind::Vertex, "reference-test", "part", "old-a", b"a");
        let b = TopoId::derive(TopoKind::Vertex, "reference-test", "part", "old-b", b"b");
        let target = model.1.vertices[0];
        change(&mut model, ChangeKind::Merge, vec![a, b], vec![target]);
        let index = VertexReferenceIndex::new(&model).unwrap();
        assert_eq!(
            index.resolve(a, 10000).unwrap(),
            VertexResolution::Followed(0)
        );
        assert_eq!(
            index.resolve(b, 10000).unwrap(),
            VertexResolution::Followed(0)
        );
        change(&mut model, ChangeKind::Modified, vec![target], vec![a]);
        assert!(VertexReferenceIndex::new(&model).is_err());
    }
    #[test]
    fn missing_identity_never_falls_back_to_nearby_geometry_or_an_index() {
        let model = crate::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let index = VertexReferenceIndex::new(&model).unwrap();
        let absent = TopoId::derive(
            TopoKind::Vertex,
            "different-occurrence",
            "elsewhere",
            "vertex",
            b"same-coordinate",
        );
        assert_eq!(
            index.resolve(absent, 10000).unwrap(),
            VertexResolution::Missing
        );
        assert!(index.resolve(model.1.edges[0], 10000).is_err());
    }
}
