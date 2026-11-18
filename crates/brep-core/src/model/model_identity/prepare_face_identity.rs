//! model identity operations: prepare_face_identity.
use super::*;
impl Model {
pub(super) fn prepare_face_identity(&self, face: &Face, key: String) -> FaceIdentityRegion {
        let plane = Self::axis_plane(&face.surface);
        let triangles = plane
            .and_then(|(axis, _, _)| self.face_region(face, axis))
            .map(|region| {
                region
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|triangle| {
                        [
                            region.positions[triangle[0] as usize],
                            region.positions[triangle[1] as usize],
                            region.positions[triangle[2] as usize],
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
        for point in triangles.iter().flatten() {
            for axis in 0..2 {
                bounds[0][axis] = bounds[0][axis].min(point[axis]);
                bounds[1][axis] = bounds[1][axis].max(point[axis]);
            }
        }
        FaceIdentityRegion {
            key,
            plane,
            triangles,
            bounds,
        }
    }
pub(super) fn face_relation(target: &FaceIdentityRegion, candidate: &FaceIdentityRegion) -> bool {
        if target.key == candidate.key {
            return true;
        }
        // Conservative planar-region evidence; general correspondence must
        // come from operation provenance, not guessed centroid proximity.
        let (Some((axis, coordinate, _)), Some((other_axis, other_coordinate, _))) =
            (target.plane, candidate.plane)
        else {
            return false;
        };
        if axis != other_axis || coordinate != other_coordinate {
            return false;
        }
        if (0..2).any(|axis| {
            target.bounds[1][axis].min(candidate.bounds[1][axis])
                <= target.bounds[0][axis].max(candidate.bounds[0][axis])
        }) {
            return false;
        }
        target.triangles.iter().any(|a| {
            candidate
                .triangles
                .iter()
                .any(|b| Self::triangles_overlap(*a, *b))
        })
    }
pub(crate) fn record_relations(
        records: &mut Vec<TopologyLineageRecord>,
        entity_kind: &str,
        source_ids: &[TopoId],
        target_ids: &[TopoId],
        relations: &[Vec<usize>],
    ) {
        for (parent_index, parent) in source_ids.iter().enumerate() {
            let children: Vec<_> = relations
                .iter()
                .enumerate()
                .filter(|(_, parents)| parents.contains(&parent_index))
                .map(|(child, _)| target_ids[child])
                .collect();
            if children.len() > 1 {
                records.push(TopologyLineageRecord {
                    operation: "split".into(),
                    entity_kind: entity_kind.into(),
                    parents: vec![*parent],
                    children,
                });
            }
        }
        for (child_index, parents) in relations.iter().enumerate() {
            let parent_ids: Vec<_> = parents.iter().map(|&parent| source_ids[parent]).collect();
            if parent_ids.len() > 1 {
                records.push(TopologyLineageRecord {
                    operation: "merge".into(),
                    entity_kind: entity_kind.into(),
                    parents: parent_ids,
                    children: vec![target_ids[child_index]],
                });
            } else if parent_ids.len() == 1
                && parent_ids[0] != target_ids[child_index]
                && relations
                    .iter()
                    .filter(|relation| relation.contains(&parents[0]))
                    .count()
                    == 1
            {
                records.push(TopologyLineageRecord {
                    operation: "persist".into(),
                    entity_kind: entity_kind.into(),
                    parents: parent_ids,
                    children: vec![target_ids[child_index]],
                });
            }
        }
    }
pub(super) fn preserve_unique_ids<'a>(
        target: &mut [TopoId],
        sources: impl Iterator<Item = (&'a [TopoId], &'a [TopoId])>,
    ) {
        let mut matches = BTreeMap::<TopoId, BTreeSet<TopoId>>::new();
        for (geometry_ids, authored_ids) in sources {
            for (geometry, authored) in geometry_ids.iter().zip(authored_ids) {
                matches.entry(*geometry).or_default().insert(*authored);
            }
        }
        let candidates: Vec<_> = target
            .iter()
            .map(|geometry| {
                matches
                    .get(geometry)
                    .and_then(|parents| (parents.len() == 1).then(|| *parents.first().unwrap()))
            })
            .collect();
        let mut counts = BTreeMap::<TopoId, usize>::new();
        for id in candidates.iter().flatten() {
            *counts.entry(*id).or_default() += 1;
        }
        let mut occupied = target.iter().copied().collect::<BTreeSet<_>>();
        for (id, candidate) in target.iter_mut().zip(candidates) {
            if let Some(candidate) = candidate
                && counts[&candidate] == 1
                && (candidate == *id || !occupied.contains(&candidate))
            {
                occupied.remove(id);
                occupied.insert(candidate);
                *id = candidate;
            }
        }
    }
pub fn inherit_topology_ids(&mut self, sources: &[&Model]) {
        let inherited_lineage: Vec<_> = sources
            .iter()
            .flat_map(|source| source.1.lineage.iter().cloned())
            .collect();
        self.rebuild_topology_ids();
        self.1.lineage = inherited_lineage;
        // Prepare geometric signatures and material regions once, not once per
        // candidate pair. Authored IDs can differ after explicit transforms.
        let target_edge_keys = self.1.edges.clone();
        let target_faces: Vec<_> = self
            .faces
            .iter()
            .zip(&self.1.faces)
            .map(|(face, key)| self.prepare_face_identity(face, key.to_string()))
            .collect();
        let geometries: Vec<_> = sources
            .iter()
            .map(|source| {
                let mut geometry = (*source).clone();
                geometry.rebuild_topology_ids();
                geometry
            })
            .collect();
        macro_rules! preserve {
            ($kind:ident) => {
                Self::preserve_unique_ids(
                    &mut self.1.$kind,
                    sources.iter().zip(&geometries).map(|(source, geometry)| {
                        (geometry.1.$kind.as_slice(), source.1.$kind.as_slice())
                    }),
                );
            };
        }
        preserve!(vertices);
        preserve!(edges);
        preserve!(loops);
        preserve!(faces);
        preserve!(shells);
        preserve!(bodies);
        let mut vertex_parents = vec![BTreeSet::new(); self.vertices.len()];
        let mut edge_parents = vec![BTreeSet::new(); self.edges.len()];
        let mut face_parents = vec![BTreeSet::new(); self.faces.len()];
        for (source, geometry) in sources.iter().zip(&geometries) {
            let source_faces: Vec<_> = source
                .faces
                .iter()
                .zip(&geometry.1.faces)
                .map(|(face, key)| source.prepare_face_identity(face, key.to_string()))
                .collect();
            for (target_index, target) in self.vertices.iter().enumerate() {
                for (source_index, candidate) in source.vertices.iter().enumerate() {
                    if target.point == candidate.point {
                        vertex_parents[target_index].insert(source.1.vertices[source_index]);
                    }
                }
            }
            for (target_index, target) in self.edges.iter().enumerate() {
                for (source_index, candidate) in source.edges.iter().enumerate() {
                    if target_edge_keys[target_index] == geometry.1.edges[source_index]
                        || self.edge_overlap(target, source, candidate)
                    {
                        edge_parents[target_index].insert(source.1.edges[source_index]);
                    }
                }
            }
            for (target_index, target) in target_faces.iter().enumerate() {
                for (source_index, candidate) in source_faces.iter().enumerate() {
                    if Self::face_relation(target, candidate) {
                        face_parents[target_index].insert(source.1.faces[source_index]);
                    }
                }
            }
        }
        for (kind, parents, target_ids) in [
            ("vertex", vertex_parents, &self.1.vertices),
            ("edge", edge_parents, &self.1.edges),
            ("face", face_parents, &self.1.faces),
        ] {
            let parent_ids: Vec<_> = parents
                .iter()
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let parent_indices: BTreeMap<_, _> = parent_ids
                .iter()
                .enumerate()
                .map(|(index, id)| (id, index))
                .collect();
            let relations: Vec<Vec<usize>> = parents
                .iter()
                .map(|set| set.iter().map(|parent| parent_indices[parent]).collect())
                .collect();
            Self::record_relations(
                &mut self.1.lineage,
                kind,
                &parent_ids,
                target_ids,
                &relations,
            );
        }
        let mut unique = Vec::new();
        for record in self.1.lineage.drain(..) {
            if !unique.contains(&record) {
                unique.push(record);
            }
        }
        self.1.lineage = unique;
        self.refresh_change_set(sources);
    }
}
