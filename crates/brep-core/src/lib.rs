//! Indexed boundary topology over exact rational curve/surface definitions.
//! Validation certifies combinatorial incidence, not geometric solid validity.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub mod operations;
pub use operations::{
    boolean, chamfer, chamfer_edges, extrude_polygon, extrude_polygon_with_holes, faceted_cylinder,
    faceted_loft, faceted_revolve, faceted_sphere, faceted_sweep, fillet, fillet_edges,
};

pub use brep_topology::{Body, FaceUse, Shell, Vertex};
pub type Edge = brep_topology::Edge<Curve>;
pub type Coedge = brep_topology::Coedge<Curve>;
pub type Loop = brep_topology::Loop<Curve>;
pub type Face = brep_topology::Face<Surface>;
#[derive(Clone, Debug, Default)]
pub struct TopologyIds {
    pub vertices: Vec<String>,
    pub edges: Vec<String>,
    pub loops: Vec<String>,
    pub faces: Vec<String>,
    pub shells: Vec<String>,
    pub bodies: Vec<String>,
    pub lineage: Vec<TopologyLineageRecord>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyLineageRecord {
    pub operation: String,
    pub entity_kind: String,
    pub parents: Vec<String>,
    pub children: Vec<String>,
}
impl value_codec::Serialize for TopologyLineageRecord {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "operation".into(),
            value_codec::Serialize::to_value(&self.operation),
        );
        object.insert(
            "entityKind".into(),
            value_codec::Serialize::to_value(&self.entity_kind),
        );
        object.insert(
            "parents".into(),
            value_codec::Serialize::to_value(&self.parents),
        );
        object.insert(
            "children".into(),
            value_codec::Serialize::to_value(&self.children),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for TopologyLineageRecord {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected topology lineage record"))?;
        let field = |key: &str| {
            object
                .get(key)
                .cloned()
                .ok_or_else(|| value_codec::error(format!("Missing field {key}")))
        };
        Ok(Self {
            operation: value_codec::Deserialize::from_value(field("operation")?)?,
            entity_kind: value_codec::Deserialize::from_value(field("entityKind")?)?,
            parents: value_codec::Deserialize::from_value(field("parents")?)?,
            children: value_codec::Deserialize::from_value(field("children")?)?,
        })
    }
}
impl value_codec::Serialize for TopologyIds {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "edges".into(),
            value_codec::Serialize::to_value(&self.edges),
        );
        object.insert(
            "loops".into(),
            value_codec::Serialize::to_value(&self.loops),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "shells".into(),
            value_codec::Serialize::to_value(&self.shells),
        );
        object.insert(
            "bodies".into(),
            value_codec::Serialize::to_value(&self.bodies),
        );
        object.insert(
            "lineage".into(),
            value_codec::Serialize::to_value(&self.lineage),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for TopologyIds {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected topologyIds object"))?;
        let read = |key: &str| {
            object
                .get(key)
                .cloned()
                .map(value_codec::Deserialize::from_value)
                .transpose()
                .map(|value| value.unwrap_or_default())
        };
        Ok(Self {
            vertices: read("vertices")?,
            edges: read("edges")?,
            loops: read("loops")?,
            faces: read("faces")?,
            shells: read("shells")?,
            bodies: read("bodies")?,
            lineage: object
                .get("lineage")
                .cloned()
                .map(value_codec::Deserialize::from_value)
                .transpose()?
                .unwrap_or_default(),
        })
    }
}
#[derive(Clone, Debug)]
pub struct Model(
    pub brep_topology::Model<Curve, Surface, Curve>,
    pub TopologyIds,
);
impl value_codec::Serialize for Model {
    fn to_value(&self) -> value_codec::Value {
        let mut value = value_codec::Serialize::to_value(&self.0);
        value.as_object_mut().unwrap().insert(
            "topologyIds".into(),
            value_codec::Serialize::to_value(&self.1),
        );
        value
    }
}
impl<'de> value_codec::Deserialize<'de> for Model {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected B-rep object"))?
            .clone();
        let ids = object
            .remove("topologyIds")
            .map(TopologyIds::from_value)
            .transpose()?;
        let topology =
            <brep_topology::Model<Curve, Surface, Curve> as value_codec::Deserialize>::from_value(
                value_codec::Value::Object(object),
            )?;
        let mut model = Self(topology, ids.unwrap_or_default());
        if model.1.vertices.is_empty() {
            model.rebuild_topology_ids();
        }
        Ok(model)
    }
}
impl std::ops::Deref for Model {
    type Target = brep_topology::Model<Curve, Surface, Curve>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Model {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[derive(Debug)]
pub struct Report {
    pub vertex_count: usize,
    pub edge_count: usize,
    pub loop_count: usize,
    pub face_count: usize,
    pub shell_count: usize,
    pub body_count: usize,
    pub boundary_edge_count: usize,
    pub topology_valid: bool,
    pub geometry_agreement: &'static str,
    pub solid_geometry_status: &'static str,
}
impl value_codec::Serialize for Report {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertexCount".into(),
            value_codec::Serialize::to_value(&self.vertex_count),
        );
        object.insert(
            "edgeCount".into(),
            value_codec::Serialize::to_value(&self.edge_count),
        );
        object.insert(
            "loopCount".into(),
            value_codec::Serialize::to_value(&self.loop_count),
        );
        object.insert(
            "faceCount".into(),
            value_codec::Serialize::to_value(&self.face_count),
        );
        object.insert(
            "shellCount".into(),
            value_codec::Serialize::to_value(&self.shell_count),
        );
        object.insert(
            "bodyCount".into(),
            value_codec::Serialize::to_value(&self.body_count),
        );
        object.insert(
            "boundaryEdgeCount".into(),
            value_codec::Serialize::to_value(&self.boundary_edge_count),
        );
        object.insert(
            "topologyValid".into(),
            value_codec::Serialize::to_value(&self.topology_valid),
        );
        object.insert(
            "geometryAgreement".into(),
            value_codec::Serialize::to_value(&self.geometry_agreement),
        );
        object.insert(
            "solidGeometryStatus".into(),
            value_codec::Serialize::to_value(&self.solid_geometry_status),
        );
        value_codec::Value::Object(object)
    }
}
fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: "BREP_INVALID_TOPOLOGY",
        message: message.into(),
    }
}
fn require(ok: bool, message: &str) -> Result<()> {
    ok.ok_or_else(|| invalid(message))
}
fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn close_points(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    distance(&a, &b) <= tolerance
}
fn curve_point(c: &Curve, t: f64) -> Result<Vec<f64>> {
    let d = c.domain();
    Ok(c.evaluate(d[0] + t * (d[1] - d[0]))?.point)
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
    fn point_key(&self, point: [f64; 3]) -> String {
        let quantum = self.tolerance_mm.max(1e-10);
        point
            .map(|coordinate| (coordinate / quantum).round() as i64)
            .map(|coordinate| coordinate.to_string())
            .join(",")
    }
    pub fn rebuild_topology_ids(&mut self) {
        let vertices: Vec<_> = self
            .vertices
            .iter()
            .map(|vertex| format!("v:{}", Self::hash([self.point_key(vertex.point)])))
            .collect();
        let edges: Vec<_> = self
            .edges
            .iter()
            .map(|edge| {
                let mut ends = edge.vertices.map(|vertex| vertices[vertex].clone());
                ends.sort();
                format!("e:{}", Self::hash(ends))
            })
            .collect();
        let loops: Vec<_> = self
            .loops
            .iter()
            .map(|wire| {
                let mut boundary: Vec<_> = wire
                    .coedges
                    .iter()
                    .map(|coedge| edges[coedge.edge].clone())
                    .collect();
                boundary.sort();
                format!("l:{}", Self::hash(boundary))
            })
            .collect();
        let faces: Vec<_> = self
            .faces
            .iter()
            .map(|face| {
                let mut boundaries: Vec<_> = std::iter::once(&face.outer)
                    .chain(&face.holes)
                    .map(|&wire| loops[wire].clone())
                    .collect();
                boundaries.sort();
                format!("f:{}", Self::hash(boundaries))
            })
            .collect();
        let shells: Vec<_> = self
            .shells
            .iter()
            .map(|shell| {
                let mut members: Vec<_> = shell
                    .faces
                    .iter()
                    .map(|face| faces[face.face].clone())
                    .collect();
                members.sort();
                format!("s:{}", Self::hash(members))
            })
            .collect();
        let bodies = self
            .bodies
            .iter()
            .map(|body| {
                let mut members: Vec<_> = std::iter::once(&body.outer_shell)
                    .chain(&body.inner_shells)
                    .map(|&shell| shells[shell].clone())
                    .collect();
                members.sort();
                format!("b:{}", Self::hash(members))
            })
            .collect();
        self.1 = TopologyIds {
            vertices,
            edges,
            loops,
            faces,
            shells,
            bodies,
            lineage: vec![],
        };
    }
    fn edge_overlap(&self, target: &Edge, source: &Model, candidate: &Edge) -> bool {
        let [a, b] = target.vertices.map(|vertex| self.0.vertices[vertex].point);
        let [c, d] = candidate
            .vertices
            .map(|vertex| source.0.vertices[vertex].point);
        let ab = std::array::from_fn::<_, 3, _>(|axis| b[axis] - a[axis]);
        let cd = std::array::from_fn::<_, 3, _>(|axis| d[axis] - c[axis]);
        let cross = [
            ab[1] * cd[2] - ab[2] * cd[1],
            ab[2] * cd[0] - ab[0] * cd[2],
            ab[0] * cd[1] - ab[1] * cd[0],
        ];
        let length = distance(&a, &b);
        if length <= self.tolerance_mm
            || distance(&[0., 0., 0.], &cross) > length * distance(&c, &d) * 1e-8
        {
            return false;
        }
        let ac = std::array::from_fn::<_, 3, _>(|axis| c[axis] - a[axis]);
        let line_cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        if distance(&[0., 0., 0.], &line_cross) > length * self.tolerance_mm * 8. {
            return false;
        }
        let axis = (0..3)
            .max_by(|left, right| ab[*left].abs().total_cmp(&ab[*right].abs()))
            .unwrap();
        let (a0, a1) = if a[axis] < b[axis] {
            (a[axis], b[axis])
        } else {
            (b[axis], a[axis])
        };
        let (b0, b1) = if c[axis] < d[axis] {
            (c[axis], d[axis])
        } else {
            (d[axis], c[axis])
        };
        a1.min(b1) - a0.max(b0) > self.tolerance_mm * 4.
    }
    fn face_relation(&self, target: &Face, source: &Model, candidate: &Face) -> bool {
        let target_points: Vec<_> = self.0.loops[target.outer]
            .coedges
            .iter()
            .map(|coedge| {
                self.0.vertices[self.0.edges[coedge.edge].vertices[usize::from(coedge.reversed)]]
                    .point
            })
            .collect();
        let source_points: Vec<_> = source.0.loops[candidate.outer]
            .coedges
            .iter()
            .map(|coedge| {
                source.0.vertices
                    [source.0.edges[coedge.edge].vertices[usize::from(coedge.reversed)]]
                .point
            })
            .collect();
        if target_points.len() < 3 || source_points.len() < 3 {
            return false;
        }
        let ab = std::array::from_fn::<_, 3, _>(|i| source_points[1][i] - source_points[0][i]);
        let ac = std::array::from_fn::<_, 3, _>(|i| source_points[2][i] - source_points[0][i]);
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let length = distance(&normal, &[0., 0., 0.]);
        if length <= self.tolerance_mm
            || !target_points.iter().all(|point| {
                let delta = std::array::from_fn::<_, 3, _>(|i| point[i] - source_points[0][i]);
                (normal.iter().zip(delta).map(|(a, b)| a * b).sum::<f64>() / length).abs()
                    <= self.tolerance_mm * 8.
            })
        {
            return false;
        }
        let drop_axis = (0..3)
            .max_by(|left, right| normal[*left].abs().total_cmp(&normal[*right].abs()))
            .unwrap();
        let project = |point: [f64; 3]| {
            let kept: Vec<_> = (0..3)
                .filter(|axis| *axis != drop_axis)
                .map(|axis| point[axis])
                .collect();
            [kept[0], kept[1]]
        };
        let center = target_points
            .iter()
            .copied()
            .fold([0.; 3], |sum, point| {
                std::array::from_fn(|axis| sum[axis] + point[axis])
            })
            .map(|coordinate| coordinate / target_points.len() as f64);
        std::iter::once(center).any(|point| {
            let point = project(point);
            let polygon: Vec<_> = source_points.iter().copied().map(project).collect();
            let mut inside = false;
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                let cross = (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]);
                if cross.abs() <= self.tolerance_mm * 8.
                    && point[0] >= a[0].min(b[0]) - self.tolerance_mm
                    && point[0] <= a[0].max(b[0]) + self.tolerance_mm
                    && point[1] >= a[1].min(b[1]) - self.tolerance_mm
                    && point[1] <= a[1].max(b[1]) + self.tolerance_mm
                {
                    return true;
                }
                if (a[1] > point[1]) != (b[1] > point[1])
                    && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
                {
                    inside = !inside;
                }
            }
            inside
        })
    }
    fn record_relations(
        records: &mut Vec<TopologyLineageRecord>,
        entity_kind: &str,
        source_ids: &[String],
        target_ids: &[String],
        relations: &[Vec<usize>],
    ) {
        for (parent_index, parent) in source_ids.iter().enumerate() {
            let children: Vec<_> = relations
                .iter()
                .enumerate()
                .filter(|(_, parents)| parents.contains(&parent_index))
                .map(|(child, _)| target_ids[child].clone())
                .collect();
            if children.len() > 1 {
                records.push(TopologyLineageRecord {
                    operation: "split".into(),
                    entity_kind: entity_kind.into(),
                    parents: vec![parent.clone()],
                    children,
                });
            }
        }
        for (child_index, parents) in relations.iter().enumerate() {
            let parent_ids: Vec<_> = parents
                .iter()
                .map(|&parent| source_ids[parent].clone())
                .collect();
            if parent_ids.len() > 1 {
                records.push(TopologyLineageRecord {
                    operation: "merge".into(),
                    entity_kind: entity_kind.into(),
                    parents: parent_ids,
                    children: vec![target_ids[child_index].clone()],
                });
            } else if parent_ids.len() == 1 && parent_ids[0] != target_ids[child_index] {
                records.push(TopologyLineageRecord {
                    operation: "persist".into(),
                    entity_kind: entity_kind.into(),
                    parents: parent_ids,
                    children: vec![target_ids[child_index].clone()],
                });
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
        for source in sources {
            for (target, vertex) in self.0.vertices.iter().enumerate() {
                if let Some(source_index) = source.0.vertices.iter().position(|candidate| {
                    close_points(candidate.point, vertex.point, self.tolerance_mm * 4.)
                }) {
                    self.1.vertices[target] = source.1.vertices[source_index].clone();
                }
            }
            for (target, edge) in self.0.edges.iter().enumerate() {
                let endpoints = edge.vertices.map(|vertex| self.0.vertices[vertex].point);
                if let Some(source_index) = source.0.edges.iter().position(|candidate| {
                    let other = candidate
                        .vertices
                        .map(|vertex| source.0.vertices[vertex].point);
                    (close_points(endpoints[0], other[0], self.tolerance_mm * 4.)
                        && close_points(endpoints[1], other[1], self.tolerance_mm * 4.))
                        || (close_points(endpoints[0], other[1], self.tolerance_mm * 4.)
                            && close_points(endpoints[1], other[0], self.tolerance_mm * 4.))
                }) {
                    self.1.edges[target] = source.1.edges[source_index].clone();
                }
            }
            for (target, face) in self.0.faces.iter().enumerate() {
                let target_vertices: BTreeSet<_> = self.0.loops[face.outer]
                    .coedges
                    .iter()
                    .map(|coedge| self.1.vertices[self.0.edges[coedge.edge].vertices[0]].clone())
                    .collect();
                if let Some(source_index) = source.0.faces.iter().position(|candidate| {
                    let source_vertices: BTreeSet<_> = source.0.loops[candidate.outer]
                        .coedges
                        .iter()
                        .map(|coedge| {
                            source.1.vertices[source.0.edges[coedge.edge].vertices[0]].clone()
                        })
                        .collect();
                    target_vertices == source_vertices
                }) {
                    self.1.faces[target] = source.1.faces[source_index].clone();
                }
            }
            for (target, wire) in self.0.loops.iter().enumerate() {
                let target_edges: BTreeSet<_> = wire
                    .coedges
                    .iter()
                    .map(|coedge| self.1.edges[coedge.edge].clone())
                    .collect();
                if let Some(source_index) = source.0.loops.iter().position(|candidate| {
                    candidate
                        .coedges
                        .iter()
                        .map(|coedge| source.1.edges[coedge.edge].clone())
                        .collect::<BTreeSet<_>>()
                        == target_edges
                }) {
                    self.1.loops[target] = source.1.loops[source_index].clone();
                }
            }
            for (target, shell) in self.0.shells.iter().enumerate() {
                let target_faces: BTreeSet<_> = shell
                    .faces
                    .iter()
                    .map(|face| self.1.faces[face.face].clone())
                    .collect();
                if let Some(source_index) = source.0.shells.iter().position(|candidate| {
                    candidate
                        .faces
                        .iter()
                        .map(|face| source.1.faces[face.face].clone())
                        .collect::<BTreeSet<_>>()
                        == target_faces
                }) {
                    self.1.shells[target] = source.1.shells[source_index].clone();
                }
            }
            for (target, body) in self.0.bodies.iter().enumerate() {
                let target_shells: BTreeSet<_> = std::iter::once(&body.outer_shell)
                    .chain(&body.inner_shells)
                    .map(|&shell| self.1.shells[shell].clone())
                    .collect();
                if let Some(source_index) = source.0.bodies.iter().position(|candidate| {
                    std::iter::once(&candidate.outer_shell)
                        .chain(&candidate.inner_shells)
                        .map(|&shell| source.1.shells[shell].clone())
                        .collect::<BTreeSet<_>>()
                        == target_shells
                }) {
                    self.1.bodies[target] = source.1.bodies[source_index].clone();
                }
            }
            let vertex_relations: Vec<Vec<usize>> = self
                .vertices
                .iter()
                .map(|vertex| {
                    source
                        .vertices
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| {
                            close_points(candidate.point, vertex.point, self.tolerance_mm * 4.)
                        })
                        .map(|(index, _)| index)
                        .collect()
                })
                .collect();
            let edge_relations: Vec<Vec<usize>> = self
                .edges
                .iter()
                .map(|edge| {
                    source
                        .edges
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| self.edge_overlap(edge, source, candidate))
                        .map(|(index, _)| index)
                        .collect()
                })
                .collect();
            let face_relations: Vec<Vec<usize>> = self
                .faces
                .iter()
                .map(|face| {
                    source
                        .faces
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| {
                            self.face_relation(face, source, candidate)
                                || source.face_relation(candidate, self, face)
                        })
                        .map(|(index, _)| index)
                        .collect()
                })
                .collect();
            Self::record_relations(
                &mut self.1.lineage,
                "vertex",
                &source.1.vertices,
                &self.1.vertices,
                &vertex_relations,
            );
            Self::record_relations(
                &mut self.1.lineage,
                "edge",
                &source.1.edges,
                &self.1.edges,
                &edge_relations,
            );
            Self::record_relations(
                &mut self.1.lineage,
                "face",
                &source.1.faces,
                &self.1.faces,
                &face_relations,
            );
        }
        let mut cross_source_merges = vec![];
        for (target_index, target) in self.vertices.iter().enumerate() {
            let parents: Vec<_> = sources
                .iter()
                .flat_map(|source| {
                    source
                        .vertices
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| {
                            close_points(candidate.point, target.point, self.tolerance_mm * 4.)
                        })
                        .map(|(index, _)| source.1.vertices[index].clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if parents.len() > 1 {
                cross_source_merges.push(TopologyLineageRecord {
                    operation: "merge".into(),
                    entity_kind: "vertex".into(),
                    parents,
                    children: vec![self.1.vertices[target_index].clone()],
                });
            }
        }
        for (target_index, target) in self.edges.iter().enumerate() {
            let parents: Vec<_> = sources
                .iter()
                .flat_map(|source| {
                    source
                        .edges
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| self.edge_overlap(target, source, candidate))
                        .map(|(index, _)| source.1.edges[index].clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if parents.len() > 1 {
                cross_source_merges.push(TopologyLineageRecord {
                    operation: "merge".into(),
                    entity_kind: "edge".into(),
                    parents,
                    children: vec![self.1.edges[target_index].clone()],
                });
            }
        }
        for (target_index, target) in self.faces.iter().enumerate() {
            let parents: Vec<_> = sources
                .iter()
                .flat_map(|source| {
                    source
                        .faces
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| {
                            self.face_relation(target, source, candidate)
                                || source.face_relation(candidate, self, target)
                        })
                        .map(|(index, _)| source.1.faces[index].clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if parents.len() > 1 {
                cross_source_merges.push(TopologyLineageRecord {
                    operation: "merge".into(),
                    entity_kind: "face".into(),
                    parents,
                    children: vec![self.1.faces[target_index].clone()],
                });
            }
        }
        self.1.lineage.extend(cross_source_merges);
        let mut unique = Vec::new();
        for record in self.1.lineage.drain(..) {
            if !unique.contains(&record) {
                unique.push(record);
            }
        }
        self.1.lineage = unique;
    }
    pub fn loop_uv(&self, id: usize, segments: usize) -> Result<Vec<[f64; 2]>> {
        require((1..=64).contains(&segments), "Edge sampling must be 1..64")?;
        let wire = self.loops.get(id).ok_or_else(|| invalid("Unknown loop"))?;
        let mut uv = Vec::new();
        for c in &wire.coedges {
            for i in 0..segments {
                let p = curve_point(&c.pcurve, i as f64 / segments as f64)?;
                require(p.len() == 2, "pcurve must be 2D")?;
                uv.push([p[0], p[1]]);
            }
        }
        Ok(uv)
    }
    pub fn validate(&self) -> Result<Report> {
        self.0.validate_topology().map_err(|e| Error {
            code: e.code,
            message: e.message,
        })?;
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        let uses = self.loops.iter().map(|l| l.coedges.len()).sum::<usize>();
        if count > 4096 || uses > 8192 || self.faces.len() > 256 {
            return Err(Error {
                code: "BREP_RESOURCE_LIMIT",
                message: "B-rep exceeds 4096 entities, 256 faces or 8192 coedges".into(),
            });
        }
        require(
            self.tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&self.tolerance_mm),
            "Invalid B-rep tolerance (1e-10..1e-2 mm)",
        )?;
        let tol = self.tolerance_mm;
        require(
            self.1.vertices.len() == self.vertices.len()
                && self.1.edges.len() == self.edges.len()
                && self.1.loops.len() == self.loops.len()
                && self.1.faces.len() == self.faces.len()
                && self.1.shells.len() == self.shells.len()
                && self.1.bodies.len() == self.bodies.len(),
            "Topology ID table does not match entity counts",
        )?;
        let all_ids = self
            .1
            .vertices
            .iter()
            .chain(&self.1.edges)
            .chain(&self.1.loops)
            .chain(&self.1.faces)
            .chain(&self.1.shells)
            .chain(&self.1.bodies);
        let ids: BTreeSet<_> = all_ids.clone().collect();
        require(ids.len() == all_ids.count(), "Topology IDs must be unique")?;
        let mut vertex_used = vec![false; self.vertices.len()];
        for v in &self.vertices {
            require(
                v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )?;
        }
        for e in &self.edges {
            e.curve.validate()?;
            require(
                e.curve.control_points[0].len() == 3,
                "Edge curve must be 3D",
            )?;
            for (i, &v) in e.vertices.iter().enumerate() {
                let point = &self
                    .vertices
                    .get(v)
                    .ok_or_else(|| invalid("Edge references unknown vertex"))?
                    .point;
                vertex_used[v] = true;
                require(
                    distance(&curve_point(&e.curve, i as f64)?, point) <= tol,
                    "Edge endpoint does not match vertex",
                )?;
            }
        }
        require(vertex_used.iter().all(|v| *v), "Unused vertex")?;
        let mut loop_owner = vec![None; self.loops.len()];
        let mut edge_used = vec![false; self.edges.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            f.surface.validate()?;
            for (li, &l) in std::iter::once(&f.outer).chain(&f.holes).enumerate() {
                let wire = self
                    .loops
                    .get(l)
                    .ok_or_else(|| invalid("Face references unknown loop"))?;
                require(
                    loop_owner[l].replace(fi).is_none(),
                    "Loop belongs to multiple faces or is repeated",
                )?;
                require(!wire.coedges.is_empty(), "Empty face loop")?;
                let mut previous: Option<usize> = None;
                let mut first = None;
                let mut previous_uv: Option<Vec<f64>> = None;
                let mut first_uv = None;
                for c in &wire.coedges {
                    let edge = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Coedge references unknown edge"))?;
                    edge_used[c.edge] = true;
                    c.pcurve.validate()?;
                    require(c.pcurve.control_points[0].len() == 2, "pcurve must be 2D")?;
                    let start = edge.vertices[usize::from(c.reversed)];
                    let end = edge.vertices[usize::from(!c.reversed)];
                    if let Some(p) = previous {
                        require(p == start, "Loop vertex chain is disconnected")?;
                    } else {
                        first = Some(start);
                    }
                    previous = Some(end);
                    let uv0 = curve_point(&c.pcurve, 0.)?;
                    let uv1 = curve_point(&c.pcurve, 1.)?;
                    if let Some(p) = &previous_uv {
                        require(distance(p, &uv0) <= 1e-9, "Loop UV chain is disconnected")?;
                    } else {
                        first_uv = Some(uv0);
                    }
                    previous_uv = Some(uv1);
                    // Endpoint agreement is exact to tolerance; interiors are explicitly sampled.
                    for i in 0..=8 {
                        let t = i as f64 / 8.;
                        let uv = curve_point(&c.pcurve, t)?;
                        let p = f.surface.evaluate(uv[0], uv[1])?.point;
                        let q = curve_point(&edge.curve, if c.reversed { 1. - t } else { t })?;
                        require(
                            distance(&p, &q) <= tol,
                            "pcurve/surface and 3D edge disagree at sampled parameters",
                        )?;
                    }
                }
                require(previous == first, "Loop is not closed")?;
                require(
                    distance(previous_uv.as_ref().unwrap(), first_uv.as_ref().unwrap()) <= 1e-9,
                    "UV loop is not closed",
                )?;
                let uv = self.loop_uv(l, 8)?;
                let area = (0..uv.len())
                    .map(|i| {
                        let a = uv[i];
                        let b = uv[(i + 1) % uv.len()];
                        (a[0] - uv[0][0]) * (b[1] - uv[0][1])
                            - (b[0] - uv[0][0]) * (a[1] - uv[0][1])
                    })
                    .sum::<f64>();
                require(
                    if li == 0 { area > 0. } else { area < 0. },
                    "Outer UV loop must be CCW and hole loops CW",
                )?;
            }
        }
        require(loop_owner.iter().all(Option::is_some), "Unused loop")?;
        require(edge_used.iter().all(|v| *v), "Unused edge")?;
        let boundary = self
            .shells
            .iter()
            .map(|s| {
                let mut uses = BTreeMap::<usize, usize>::new();
                for u in &s.faces {
                    let f = &self.faces[u.face];
                    for &l in std::iter::once(&f.outer).chain(&f.holes) {
                        for c in &self.loops[l].coedges {
                            *uses.entry(c.edge).or_default() += 1;
                        }
                    }
                }
                uses.values().filter(|n| **n == 1).count()
            })
            .sum();
        Ok(Report {
            vertex_count: self.vertices.len(),
            edge_count: self.edges.len(),
            loop_count: self.loops.len(),
            face_count: self.faces.len(),
            shell_count: self.shells.len(),
            body_count: self.bodies.len(),
            boundary_edge_count: boundary,
            topology_valid: true,
            geometry_agreement: "sampled_with_tolerance",
            solid_geometry_status: "not_certified",
        })
    }
}
fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    }
}
/// Construct an exact six-face NURBS box with shared edges, not six loose patches.
pub fn cuboid(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    require(
        (0..3).all(|i| {
            min[i].is_finite()
                && max[i].is_finite()
                && max[i] > min[i]
                && min[i].abs() <= 1e6
                && max[i].abs() <= 1e6
        }),
        "Invalid box bounds",
    )?;
    let vertices = [
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 1],
        [1, 1, 1],
        [0, 1, 1],
    ]
    .map(|p| Vertex {
        point: std::array::from_fn(|i| if p[i] == 0 { min[i] } else { max[i] }),
    })
    .to_vec();
    let mut m = Model(
        brep_topology::Model {
            vertices,
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    let mut edges = BTreeMap::new();
    for corners in [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ] {
        let mut coedges = vec![];
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let key = (a.min(b), a.max(b));
            let edge = *edges.entry(key).or_insert_with(|| {
                let id = m.0.edges.len();
                m.0.edges.push(Edge {
                    vertices: [key.0, key.1],
                    curve: line(
                        m.0.vertices[key.0].point.to_vec(),
                        m.0.vertices[key.1].point.to_vec(),
                    ),
                });
                id
            });
            coedges.push(Coedge {
                edge,
                reversed: a > b,
                pcurve: line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
            });
        }
        let outer = m.0.loops.len();
        m.0.loops.push(Loop { coedges });
        m.0.faces.push(Face {
            surface: Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![
                    vec![
                        m.0.vertices[corners[0]].point.to_vec(),
                        m.0.vertices[corners[3]].point.to_vec(),
                    ],
                    vec![
                        m.0.vertices[corners[1]].point.to_vec(),
                        m.0.vertices[corners[2]].point.to_vec(),
                    ],
                ],
                weights: vec![vec![1., 1.], vec![1., 1.]],
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes: vec![],
        });
    }
    m.0.shells.push(Shell {
        faces: (0..6)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    m.0.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    m.rebuild_topology_ids();
    m.validate()?;
    Ok(m)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn box_incidence_and_serialization() {
        let m = cuboid([0.; 3], [2., 3., 4.]).unwrap();
        let r = m.validate().unwrap();
        assert_eq!(
            (r.vertex_count, r.edge_count, r.face_count, r.body_count),
            (8, 12, 6, 1)
        );
        let restored: Model = value_codec::from_str(&value_codec::to_string(&m).unwrap()).unwrap();
        restored.validate().unwrap();
    }
    #[test]
    fn rejects_broken_incidence_geometry_and_orientation() {
        let a = cuboid([0.; 3], [1.; 3]).unwrap();
        for kind in 0..6 {
            let mut m = a.clone();
            match kind {
                0 => m.0.edges[0].vertices[0] = 999,
                1 => m.0.loops[0].coedges[0].reversed ^= true,
                2 => m.0.shells[0].faces[0].reversed = true,
                3 => m.0.faces[0].outer = m.0.faces[1].outer,
                4 => m.0.vertices[0].point[0] = 0.2,
                _ => m.0.bodies[0].inner_shells.push(0),
            }
            assert!(m.validate().is_err(), "mutation {kind}");
        }
    }
    #[test]
    fn open_shell_is_not_a_body() {
        let mut m = cuboid([0.; 3], [1.; 3]).unwrap();
        m.0.shells[0].closed = false;
        assert!(m.validate().is_err());
        m.0.bodies.clear();
        m.rebuild_topology_ids();
        m.validate().unwrap();
    }
}
