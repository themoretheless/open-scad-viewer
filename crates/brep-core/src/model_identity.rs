use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Edge, Face, Loop, Model, TopoId, TopoKind,
    TopologyChange, TopologyIds, TopologyLineageRecord, rational_identity,
};
use nurbs_core::{curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct FaceIdentityRegion {
    key: String,
    plane: Option<(usize, f64, bool)>,
    triangles: Vec<[[f64; 2]; 3]>,
    bounds: [[f64; 2]; 2],
}

impl Model {
    fn hash(parts: impl IntoIterator<Item = String>) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in parts
            .into_iter()
            .flat_map(|part| part.into_bytes().into_iter().chain([0xff]))
        {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }
    fn authored_id(kind: TopoKind, role: &str, signature: &str) -> TopoId {
        TopoId::derive(
            kind,
            "authored-geometry",
            signature,
            role,
            signature.as_bytes(),
        )
    }
    pub(crate) fn identity_groups(&self) -> [(TopoKind, &[TopoId]); 6] {
        [
            (TopoKind::Vertex, &self.1.vertices),
            (TopoKind::Edge, &self.1.edges),
            (TopoKind::Loop, &self.1.loops),
            (TopoKind::Face, &self.1.faces),
            (TopoKind::Shell, &self.1.shells),
            (TopoKind::Body, &self.1.bodies),
        ]
    }

    /// Complete naming evidence requires a valid ChangeSet node and an
    /// authored/generated or lineage-producing change for every current
    /// topology entity. This is derived, never trusted from a wire boolean.
    pub fn persistent_naming_complete(&self) -> bool {
        self.1.change_set.validate().is_ok()
            && self.identity_groups().into_iter().all(|(kind, ids)| {
                ids.iter().all(|id| {
                    self.1.change_set.nodes.get(id) == Some(&kind)
                        && self
                            .1
                            .change_set
                            .changes
                            .iter()
                            .any(|change| change.children.contains(id))
                })
            })
    }

    pub(crate) fn refresh_change_set(&mut self, sources: &[&Model]) {
        let mut change_set = ChangeSet::default();
        for source in sources {
            change_set.nodes.extend(
                source
                    .1
                    .change_set
                    .nodes
                    .iter()
                    .map(|(id, kind)| (*id, *kind)),
            );
            for change in &source.1.change_set.changes {
                if !change_set.changes.contains(change) {
                    change_set.changes.push(change.clone());
                }
            }
        }
        for (kind, ids) in self.identity_groups() {
            for id in ids {
                change_set.nodes.insert(*id, kind);
            }
        }
        let lineage = self.1.lineage.clone();
        let mut accepted_lineage = Vec::new();
        for record in &lineage {
            let Some(topo_kind) = TopoKind::parse(&record.entity_kind) else {
                continue;
            };
            for id in record.parents.iter().chain(&record.children) {
                change_set.nodes.insert(*id, topo_kind);
            }
            let kind = match record.operation.as_str() {
                "split" => ChangeKind::Split,
                "merge" => ChangeKind::Merge,
                "persist" if record.parents == record.children => ChangeKind::Persisted,
                "persist" => ChangeKind::Modified,
                _ => continue,
            };
            let change = TopologyChange {
                kind,
                topo_kind,
                parents: record.parents.clone(),
                children: record.children.clone(),
                provenance: ChangeProvenance {
                    operation: record.operation.clone(),
                    operand: None,
                    occurrence: record
                        .parents
                        .first()
                        .or(record.children.first())
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "unknown".into()),
                },
                role: topo_kind.as_str().into(),
                anchor: None,
            };
            if !change_set.changes.contains(&change) {
                let mut candidate = change_set.clone();
                candidate.changes.push(change.clone());
                match candidate.validate() {
                    Ok(()) => {
                        change_set.changes.push(change);
                        accepted_lineage.push(record.clone());
                    }
                    Err(error) if error.message == "Topology lineage must be acyclic" => {
                        // A rebuild may return to an already persistent authored
                        // identity. Reusing that node is persistence, not a new
                        // backwards lineage edge.
                    }
                    Err(_) => {
                        change_set.changes.push(change);
                        accepted_lineage.push(record.clone());
                    }
                }
            } else {
                accepted_lineage.push(record.clone());
            }
        }
        self.1.lineage = accepted_lineage;
        let referenced_children: BTreeSet<_> = change_set
            .changes
            .iter()
            .flat_map(|change| change.children.iter().copied())
            .collect();
        for (kind, ids) in self.identity_groups() {
            for id in ids {
                if !referenced_children.contains(id)
                    && !change_set.changes.iter().any(|change| {
                        change.kind == ChangeKind::Generated && change.children == [*id]
                    })
                {
                    change_set.changes.push(TopologyChange {
                        kind: ChangeKind::Generated,
                        topo_kind: kind,
                        parents: vec![],
                        children: vec![*id],
                        provenance: ChangeProvenance {
                            operation: "authored-geometry".into(),
                            operand: None,
                            occurrence: id.to_string(),
                        },
                        role: kind.as_str().into(),
                        anchor: None,
                    });
                }
            }
        }
        self.1.change_set = change_set;
    }
    fn vector_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|axis| a[axis] - b[axis])
    }
    fn vector_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn number_key(value: f64) -> String {
        // -0 and +0 are the same geometric coordinate. No tolerance rounding:
        // nearby authored entities must never acquire the same identity.
        format!("{:016x}", if value == 0. { 0 } else { value.to_bits() })
    }
    fn point_key(&self, point: [f64; 3]) -> String {
        point.map(Self::number_key).join(",")
    }
    fn directed_curve_key(curve: &Curve, reversed: bool) -> String {
        let points: Vec<_> = if reversed {
            curve.control_points.iter().rev().collect()
        } else {
            curve.control_points.iter().collect()
        };
        let coordinates = points
            .iter()
            .map(|point| {
                point
                    .iter()
                    .copied()
                    .map(Self::number_key)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(";");
        if curve.degree == 1 && points.len() == 2 && curve.weights[0] == curve.weights[1] {
            return format!("line:{coordinates}");
        }
        let knots: Vec<_> = if reversed {
            let [a, b] = curve.domain();
            curve
                .knots
                .iter()
                .rev()
                .map(|k| Self::number_key(a + b - k))
                .collect()
        } else {
            curve.knots.iter().copied().map(Self::number_key).collect()
        };
        let weights: Vec<_> = if reversed {
            curve
                .weights
                .iter()
                .rev()
                .copied()
                .map(Self::number_key)
                .collect()
        } else {
            curve
                .weights
                .iter()
                .copied()
                .map(Self::number_key)
                .collect()
        };
        format!(
            "nurbs:{}:{}:{}:{}:{}",
            curve.degree,
            curve.periodic,
            knots.join(","),
            coordinates,
            weights.join(",")
        )
    }
    fn curve_key(curve: &Curve) -> String {
        Self::directed_curve_key(curve, false).min(Self::directed_curve_key(curve, true))
    }
    fn cyclic_key(parts: &[String]) -> String {
        let Some(start) = (0..parts.len()).min_by(|&a, &b| {
            (0..parts.len())
                .map(|offset| &parts[(a + offset) % parts.len()])
                .cmp((0..parts.len()).map(|offset| &parts[(b + offset) % parts.len()]))
        }) else {
            return String::new();
        };
        // Compare borrowed rotations, then allocate the winning sequence once.
        parts
            .iter()
            .cycle()
            .skip(start)
            .take(parts.len())
            .cloned()
            .collect::<Vec<_>>()
            .join("|")
    }
    fn loop_key(&self, wire: &Loop) -> String {
        Self::cyclic_key(
            &wire
                .coedges
                .iter()
                .map(|coedge| {
                    Self::directed_curve_key(&self.edges[coedge.edge].curve, coedge.reversed)
                })
                .collect::<Vec<_>>(),
        )
    }
    fn axis_plane(surface: &Surface) -> Option<(usize, f64, bool)> {
        // An affine axis-aligned patch has an unambiguous support plane even
        // when an operation changes its UV bounding rectangle. Other surfaces
        // retain their full definition; do not infer sameness from a sample.
        if surface.degree_u != 1
            || surface.degree_v != 1
            || surface.control_points.len() != 2
            || surface.control_points.iter().any(|row| row.len() != 2)
            || surface
                .weights
                .iter()
                .flatten()
                .any(|w| *w != surface.weights[0][0])
        {
            return None;
        }
        let points = &surface.control_points;
        if (0..3).any(|i| points[1][1][i] - points[0][1][i] != points[1][0][i] - points[0][0][i]) {
            return None;
        }
        let u = std::array::from_fn(|i| points[1][0][i] - points[0][0][i]);
        let v = std::array::from_fn(|i| points[0][1][i] - points[0][0][i]);
        let normal = Self::vector_cross(u, v);
        (0..3).find_map(|axis| {
            let coordinate = points[0][0][axis];
            (normal[axis] != 0. && points.iter().flatten().all(|p| p[axis] == coordinate))
                .then_some((axis, coordinate, normal[axis] > 0.))
        })
    }
    fn surface_key(surface: &Surface) -> String {
        if let Some((axis, offset, positive)) = Self::axis_plane(surface) {
            return format!("plane:{axis}:{}:{positive}", Self::number_key(offset));
        }
        // Every knot, weight, control point, periodic flag and orientation is
        // part of the support identity. A shared boundary is insufficient.
        rational_identity::surface_signature(surface)
    }
    fn face_key(&self, face: &Face) -> String {
        let mut holes = face
            .holes
            .iter()
            .map(|&wire| self.loop_key(&self.loops[wire]))
            .collect::<Vec<_>>();
        holes.sort();
        let mut parts = vec![
            Self::surface_key(&face.surface),
            format!("outer:{}", self.loop_key(&self.loops[face.outer])),
        ];
        parts.extend(holes.into_iter().map(|hole| format!("hole:{hole}")));
        if Self::axis_plane(&face.surface).is_none() {
            // Curved support needs the complete lifted UV trimming definition,
            // including which branch of a periodic surface a boundary uses.
            let uv_key = |wire: usize| {
                Self::cyclic_key(
                    &self.loops[wire]
                        .coedges
                        .iter()
                        .map(|coedge| Self::directed_curve_key(&coedge.pcurve, false))
                        .collect::<Vec<_>>(),
                )
            };
            parts.push(format!("uv-outer:{}", uv_key(face.outer)));
            let mut inner_uv = face
                .holes
                .iter()
                .map(|&wire| uv_key(wire))
                .collect::<Vec<_>>();
            inner_uv.sort();
            parts.extend(inner_uv.into_iter().map(|hole| format!("uv-hole:{hole}")));
        }
        Self::hash(parts)
    }
    pub fn rebuild_topology_ids(&mut self) {
        let mut vertices: Vec<_> = self
            .vertices
            .iter()
            .map(|vertex| Self::hash([self.point_key(vertex.point)]))
            .collect();
        let mut edges: Vec<_> = self
            .edges
            .iter()
            .map(|edge| {
                Self::hash([
                    Self::curve_key(&edge.curve),
                    if edge.degenerate {
                        "deg:1".into()
                    } else {
                        "deg:0".into()
                    },
                ])
            })
            .collect();
        let mut loops: Vec<_> = self
            .loops
            .iter()
            .map(|wire| Self::hash([self.loop_key(wire)]))
            .collect();
        let mut faces: Vec<_> = self.faces.iter().map(|face| self.face_key(face)).collect();
        let shells: Vec<_> = self
            .shells
            .iter()
            .map(|shell| {
                let mut members: Vec<_> = shell
                    .faces
                    .iter()
                    .map(|usage| format!("{}:{}", faces[usage.face], usage.reversed))
                    .collect();
                members.sort();
                Self::hash(members)
            })
            .collect();
        let bodies: Vec<String> = self
            .bodies
            .iter()
            .map(|body| {
                let mut inner = body
                    .inner_shells
                    .iter()
                    .map(|&shell| format!("inner:{}", shells[shell]))
                    .collect::<Vec<_>>();
                inner.sort();
                Self::hash(
                    std::iter::once(format!("outer:{}", shells[body.outer_shell])).chain(inner),
                )
            })
            .collect();
        // Separate bodies may touch while owning distinct vertices/edges.
        // Coordinate or curve identity alone is then insufficient. Qualify
        // only collisions using the owning shell's geometry, never an array
        // index. Indistinguishable duplicate shells remain ambiguous and are
        // rejected by validate rather than arbitrarily numbered.
        if [&vertices, &edges, &loops, &faces]
            .iter()
            .any(|ids| ids.iter().collect::<BTreeSet<_>>().len() != ids.len())
        {
            let mut vertex_owners = vec![BTreeSet::new(); vertices.len()];
            let mut edge_owners = vec![BTreeSet::new(); edges.len()];
            let mut loop_owners = vec![BTreeSet::new(); loops.len()];
            let mut face_owners = vec![BTreeSet::new(); faces.len()];
            for (shell, key) in self.shells.iter().zip(&shells) {
                for use_ in &shell.faces {
                    face_owners[use_.face].insert(key.clone());
                    let face = &self.faces[use_.face];
                    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                        // Two faces on different surfaces may bound the same
                        // curve loop in the same UV sense (a ring hole on a
                        // plane and on a reversed sphere patch): qualify the
                        // loop by its face, which carries the surface.
                        loop_owners[wire].insert(faces[use_.face].clone());
                        for coedge in &self.loops[wire].coedges {
                            // A pole can own several collapsed UV boundaries in
                            // one shell. They are distinct chart entities even
                            // though their world curves and vertices coincide.
                            let owner = if self.edges[coedge.edge].degenerate {
                                Self::hash([
                                    key.clone(),
                                    faces[use_.face].clone(),
                                    Self::directed_curve_key(&coedge.pcurve, false),
                                ])
                            } else {
                                key.clone()
                            };
                            edge_owners[coedge.edge].insert(owner);
                            for &vertex in &self.edges[coedge.edge].vertices {
                                vertex_owners[vertex].insert(key.clone());
                            }
                        }
                    }
                }
            }
            let qualify = |ids: &mut [String], owners: &[BTreeSet<String>]| {
                let mut counts = BTreeMap::new();
                for id in ids.iter() {
                    *counts.entry(id.clone()).or_insert(0) += 1;
                }
                for (id, context) in ids.iter_mut().zip(owners) {
                    if counts[id] > 1 && !context.is_empty() {
                        *id = format!("{id}@{}", Self::hash(context.iter().cloned()));
                    }
                }
            };
            qualify(&mut vertices, &vertex_owners);
            qualify(&mut edges, &edge_owners);
            qualify(&mut loops, &loop_owners);
            qualify(&mut faces, &face_owners);
        }
        self.1 = TopologyIds {
            vertices: vertices
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Vertex, "vertex", signature))
                .collect(),
            edges: edges
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Edge, "edge", signature))
                .collect(),
            loops: loops
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Loop, "boundary", signature))
                .collect(),
            faces: faces
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Face, "face", signature))
                .collect(),
            shells: shells
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Shell, "shell", signature))
                .collect(),
            bodies: bodies
                .iter()
                .map(|signature| Self::authored_id(TopoKind::Body, "body", signature))
                .collect(),
            lineage: vec![],
            change_set: ChangeSet::default(),
            local_tolerances_mm: BTreeMap::new(),
            sewed_edges: BTreeSet::new(),
        };
        self.refresh_change_set(&[]);
    }
    fn edge_overlap(&self, target: &Edge, source: &Model, candidate: &Edge) -> bool {
        // Endpoint chords do not establish overlap between rational curves.
        if [&target.curve, &candidate.curve].iter().any(|curve| {
            curve.degree != 1
                || curve.control_points.len() != 2
                || curve.weights[0] != curve.weights[1]
        }) {
            return false;
        }
        let [a, b] = target.vertices.map(|vertex| self.vertices[vertex].point);
        let [c, d] = candidate
            .vertices
            .map(|vertex| source.vertices[vertex].point);
        let ab = Self::vector_sub(b, a);
        if Self::vector_cross(ab, Self::vector_sub(d, c)) != [0.; 3]
            || Self::vector_cross(ab, Self::vector_sub(c, a)) != [0.; 3]
        {
            return false;
        }
        let axis = (0..3)
            .max_by(|left, right| ab[*left].abs().total_cmp(&ab[*right].abs()))
            .unwrap();
        a[axis].max(b[axis]).min(c[axis].max(d[axis]))
            > a[axis].min(b[axis]).max(c[axis].min(d[axis]))
    }
    fn face_region(
        &self,
        face: &Face,
        axis: usize,
    ) -> Option<planar_geometry::tessellation::FillMesh> {
        let ring = |wire: usize| -> Option<Vec<[f64; 2]>> {
            self.loops[wire]
                .coedges
                .iter()
                .map(|coedge| {
                    let edge = &self.edges[coedge.edge];
                    if edge.curve.degree != 1
                        || edge.curve.control_points.len() != 2
                        || edge.curve.weights[0] != edge.curve.weights[1]
                    {
                        return None;
                    }
                    let point = self.vertices[edge.vertices[usize::from(coedge.reversed)]].point;
                    Some([point[(axis + 1) % 3], point[(axis + 2) % 3]])
                })
                .collect()
        };
        let outer = ring(face.outer)?;
        let holes = face
            .holes
            .iter()
            .map(|&wire| ring(wire))
            .collect::<Option<Vec<_>>>()?;
        planar_geometry::triangulation::triangulate_profile(&outer, &holes).ok()
    }
    fn triangles_overlap(a: [[f64; 2]; 3], b: [[f64; 2]; 3]) -> bool {
        // Strict separating-axis test: only positive-area material overlap
        // establishes a face relation. Shared edges/vertices are not merges.
        for triangle in [a, b] {
            for i in 0..3 {
                let start = triangle[i];
                let end = triangle[(i + 1) % 3];
                let axis = [start[1] - end[1], end[0] - start[0]];
                let project = |point: [f64; 2]| axis[0] * point[0] + axis[1] * point[1];
                let pa = a.map(project);
                let pb = b.map(project);
                let min = |values: [f64; 3]| values.into_iter().fold(f64::INFINITY, f64::min);
                let max = |values: [f64; 3]| values.into_iter().fold(f64::NEG_INFINITY, f64::max);
                if max(pa).min(max(pb)) <= min(pa).max(min(pb)) {
                    return false;
                }
            }
        }
        true
    }
    fn prepare_face_identity(&self, face: &Face, key: String) -> FaceIdentityRegion {
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
    fn face_relation(target: &FaceIdentityRegion, candidate: &FaceIdentityRegion) -> bool {
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
    fn preserve_unique_ids<'a>(
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
