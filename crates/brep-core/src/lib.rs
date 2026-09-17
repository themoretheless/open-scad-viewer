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

pub mod analysis;
pub mod analytic;
pub mod analytic_boolean;
pub mod analytic_features;
pub mod analytic_ss;
mod boolean_support;
pub mod close_topology;
pub mod coverage_verifier;
pub mod imprint_pipeline;
pub mod intersections;
pub mod iges_interchange_v2;
pub mod nurbs_ss_g6;
pub mod nurbs_step_interchange;
mod nurbs_step_shared;
pub mod nurbs_step_solid;
pub mod nurbs_step_trimmed;
pub mod operations;
pub mod planar_trim;
pub mod predicate_evidence;
pub mod prism;
pub mod prism_frame;
mod profile_imprint;
pub mod sketch;
pub mod solid_audit;
mod sphere_boolean;
pub mod step_interchange;
pub mod step_interchange_v3;
mod stepped_prism;
pub mod transactions;
pub mod transform;
pub mod trim_sew;
pub mod uv_arrangement;
pub use analytic::{
    cylinder, frustum, revolve, revolve_angle, revolve_region, revolve_region_angle, revolve_wire,
    revolve_wire_angle, ruled_loft, sphere, torus, tube,
};
pub use analytic_boolean::{BooleanCertificate, analytic_boolean, analytic_boolean_audited};
pub use close_topology::{
    AuditedTopologyComplex, BodyRole, CLOSE_TOPOLOGY_CAPABILITY,
    CLOSE_TOPOLOGY_IGES_CAPABILITY, CLOSE_TOPOLOGY_STEP_CAPABILITY, ComplexHealCertificate,
    ComplexHealPlan, ComplexInterchangeCertificate, ComplexPart, CorrespondenceKind,
    EdgeRadialRing, EdgeUseRef, ExactParameterPartition, FaceRef, LocalCorrespondence,
    MixedDimensionalBrep, SharedFace, TOLERANT_COMPLEX_HEAL_CAPABILITY,
    TopologyComplexCertificate, VertexFan, VertexUseRef, certify_complex_heal,
    complex_relation_id, export_complex_iges, export_complex_step, import_complex_iges,
    import_complex_step,
};
pub use analytic_features::{
    AUDITED_MULTI_EDGE_FILLET_CAPABILITY, AuditedFeatureResult,
    EXACT_ANALYTIC_SHELL_CAPABILITY, EXACT_BENT_RMF_SWEEP_CAPABILITY, EXACT_CONVEX_CHAMFER_CAPABILITY,
    EXACT_CONVEX_PRISM_FILLET_CAPABILITY, EXACT_MULTI_SECTION_LOFT_CAPABILITY,
    EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY, FeatureCertificate, analytic_chamfer, analytic_fillet,
    analytic_fillet_chain, analytic_shell, analytic_solid_loft, audited_multi_edge_fillet,
    audited_bent_rmf_sweep, audited_multi_section_loft, audited_parallel_frame_sweep,
    exact_analytic_shell, exact_convex_chamfer, exact_convex_prism_fillet, export_iges,
    exact_variable_radius_fillet, frame_law_ruled_sweep, import_iges,
};
pub use iges_interchange_v2::{
    IGES_INTERCHANGE_V2_CAPABILITY, IgesV2Report, export_iges_v2, import_iges_v2,
};
pub use nurbs_ss_g6::{
    G6_CAPABILITY, G6_MATURITY, G6Component, G6Maturity, NURBS_BOOLEAN_CAPABILITY,
    NURBS_BOOLEAN_CAPABILITY_V1, NURBS_BOOLEAN_CAPABILITY_V3, NURBS_BOOLEAN_CAPABILITY_V4,
    NURBS_BOOLEAN_CAPABILITY_V5, NURBS_BOOLEAN_CAPABILITY_V7, NURBS_BOOLEAN_V1_MATURITY,
    ContainedGraphBooleanCertificate,
    BranchCompletenessCertificate, BranchComponent, BranchGraph, BranchOrientation,
    CertifiedBranchFragment, CurvedGraphBooleanCertificate, NurbsBooleanImprintCertificate,
    GeneralNurbsBooleanCertificate, GeneralNurbsBooleanNaming, GENERAL_NURBS_BOOLEAN_AUTHORITY,
    RationalBezierDecomposition, RationalBezierPatchSpan, TensorSpanId, TransverseSpanEvidence,
    author_general_nurbs_boolean, canonical_bezier_graph_solid,
    canonical_multispan_graph_solid, canonical_rational_graph_solid, certify_multispan_ss,
    decompose_rational_bezier_spans, join_certified_multispan_fragments,
    narrow_transverse_bezier_le3, narrow_transverse_bicubic, nurbs_boolean_graph_containment_v4,
    nurbs_boolean_graph_patch_unequal_v4, nurbs_boolean_graph_patch_v3,
    nurbs_boolean_imprint_solids, nurbs_boolean_rational_graph_patch_v5,
    nurbs_boolean_transverse_bicubic,
};
pub use nurbs_step_interchange::{
    NURBS_STEP_BICUBIC_FACE_CAPABILITY, bicubic_open_face, export_nurbs_step, import_nurbs_step,
};
pub use nurbs_step_solid::{
    NURBS_STEP_SOLID_CAPABILITY, NURBS_STEP_SOLID_V2_CAPABILITY, export_nurbs_step_solid,
    export_nurbs_step_solid_v2, freeform_cuboid_solid, freeform_cuboid_with_bump_face,
    import_nurbs_step_solid, import_nurbs_step_solid_v2,
};
pub use nurbs_step_trimmed::{
    NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY, bicubic_trimmed_face, export_nurbs_step_trimmed,
    import_nurbs_step_trimmed,
};
pub use operations::{
    boolean, chamfer, chamfer_edges, extrude_polygon, extrude_polygon_with_holes, faceted_cylinder,
    faceted_loft, faceted_revolve, faceted_sphere, faceted_sweep, fillet, fillet_edges,
};
pub use step_interchange::{
    STEP_INTERCHANGE_V2_CAPABILITY, StepIdentityReport, export_step, export_step_v2, import_step,
    import_step_v2,
};
pub use step_interchange_v3::{
    STEP_INTERCHANGE_V3_CAPABILITY, STEP_INTERCHANGE_V4_CAPABILITY, STEP_INTERCHANGE_V5_CAPABILITY,
    STEP_INTERCHANGE_V6_CAPABILITY, STEP_INTERCHANGE_V7_CAPABILITY, STEP_INTERCHANGE_V8_CAPABILITY,
    STEP_INTERCHANGE_V9_CAPABILITY, STEP_INTERCHANGE_V10_CAPABILITY,
    StepRegularityEvidence, StepV3Report, StepV8Certificate, StepV10Document, compose_step_v7_occurrences,
    compose_step_v8_occurrences, compose_step_v9_occurrences,
    export_step_v3, export_step_v4, export_step_v5, export_step_v6, export_step_v7, export_step_v8,
    export_step_v9, export_step_v10, import_step_v3, import_step_v4, import_step_v5, import_step_v6,
    import_step_v7, import_step_v8, import_step_v9, import_step_v10,
};

pub use brep_topology::{
    Body, ChangeKind, ChangeProvenance, ChangeSet, FaceUse, Shell, TopoId, TopoKind,
    TopologyChange, Vertex,
};
pub type Edge = brep_topology::Edge<Curve>;
pub type Coedge = brep_topology::Coedge<Curve>;
pub type Loop = brep_topology::Loop<Curve>;
pub type Face = brep_topology::Face<Surface>;
#[derive(Clone, Debug, Default)]
pub struct TopologyIds {
    pub vertices: Vec<TopoId>,
    pub edges: Vec<TopoId>,
    pub loops: Vec<TopoId>,
    pub faces: Vec<TopoId>,
    pub shells: Vec<TopoId>,
    pub bodies: Vec<TopoId>,
    pub lineage: Vec<TopologyLineageRecord>,
    pub change_set: ChangeSet,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyLineageRecord {
    pub operation: String,
    pub entity_kind: String,
    pub parents: Vec<TopoId>,
    pub children: Vec<TopoId>,
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
        if object.len() != 4
            || ["operation", "entityKind", "parents", "children"]
                .iter()
                .any(|key| !object.contains_key(*key))
        {
            return Err(value_codec::error(
                "Topology lineage fields do not match the schema",
            ));
        }
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
        object.insert(
            "changeSet".into(),
            value_codec::Serialize::to_value(&self.change_set),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for TopologyIds {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected topologyIds object"))?;
        if object.keys().any(|key| {
            ![
                "vertices",
                "edges",
                "loops",
                "faces",
                "shells",
                "bodies",
                "lineage",
                "changeSet",
            ]
            .contains(&key.as_str())
        }) {
            return Err(value_codec::error("Unknown topologyIds field"));
        }
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
            change_set: object
                .get("changeSet")
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
        let generate_ids = ids.is_none();
        let mut model = Self(topology, ids.unwrap_or_default());
        if generate_ids {
            // Legacy documents omit the optional identity tables. Validate
            // every index and rational definition before deriving signatures;
            // malformed documents must return errors rather than index traps.
            let decode_error = |e: Error| value_codec::error(format!("{}: {}", e.code, e.message));
            model.0.validate_topology().map_err(decode_error)?;
            for edge in &model.edges {
                edge.curve.validate().map_err(decode_error)?;
            }
            for wire in &model.loops {
                for use_ in &wire.coedges {
                    use_.pcurve.validate().map_err(decode_error)?;
                }
            }
            for face in &model.faces {
                face.surface.validate().map_err(decode_error)?;
            }
            model.rebuild_topology_ids();
        } else if model.1.change_set.nodes.is_empty() && model.1.change_set.changes.is_empty() {
            // Compatibility for canonical-ID snapshots written before the
            // authoritative changeSet field was introduced.
            model.refresh_change_set(&[]);
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
fn curve_point(c: &Curve, t: f64) -> Result<Vec<f64>> {
    let d = c.domain();
    Ok(c.evaluate(d[0] + t * (d[1] - d[0]))?.point)
}
struct FaceIdentityRegion {
    key: String,
    plane: Option<(usize, f64, bool)>,
    triangles: Vec<[[f64; 2]; 3]>,
    bounds: [[f64; 2]; 2],
}
impl Model {
    /// Explicit migration entry point for old `<prefix>:<16hex>` identity
    /// tables. Normal deserialization intentionally rejects those values.
    pub fn from_legacy_topology_value(mut value: value_codec::Value) -> value_codec::Result<Self> {
        let ids = value
            .get_mut("topologyIds")
            .and_then(value_codec::Value::as_object_mut)
            .ok_or_else(|| value_codec::error("Missing legacy topologyIds object"))?;
        let mut migrated = BTreeMap::<String, String>::new();
        for field in ["vertices", "edges", "loops", "faces", "shells", "bodies"] {
            let values = ids
                .get_mut(field)
                .and_then(value_codec::Value::as_array_mut)
                .ok_or_else(|| value_codec::error(format!("Missing legacy field {field}")))?;
            for value in values {
                let old = value
                    .as_str()
                    .ok_or_else(|| value_codec::error("Legacy topology ID must be a string"))?;
                let id = TopoId::migrate_legacy(old).map_err(value_codec::error)?;
                migrated.insert(old.into(), id.to_string());
                *value = value_codec::Value::String(id.to_string());
            }
        }
        if let Some(records) = ids
            .get_mut("lineage")
            .and_then(value_codec::Value::as_array_mut)
        {
            for record in records {
                for endpoint in ["parents", "children"] {
                    let values = record
                        .get_mut(endpoint)
                        .and_then(value_codec::Value::as_array_mut)
                        .ok_or_else(|| value_codec::error("Invalid legacy lineage endpoints"))?;
                    for value in values {
                        let old = value.as_str().ok_or_else(|| {
                            value_codec::error("Legacy lineage ID must be a string")
                        })?;
                        let replacement = match migrated.get(old) {
                            Some(id) => id.clone(),
                            None => TopoId::migrate_legacy(old)
                                .map_err(value_codec::error)?
                                .to_string(),
                        };
                        *value = value_codec::Value::String(replacement);
                    }
                }
            }
        }
        ids.remove("changeSet");
        <Self as value_codec::Deserialize>::from_value(value)
    }

    /// Canonical regularized empty solid. It has no placeholder shell or body.
    pub fn empty(tolerance_mm: f64) -> Result<Self> {
        let model = Self(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm,
            },
            TopologyIds::default(),
        );
        model.validate()?;
        Ok(model)
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
            && self.edges.is_empty()
            && self.loops.is_empty()
            && self.faces.is_empty()
            && self.shells.is_empty()
            && self.bodies.is_empty()
    }

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
    fn identity_groups(&self) -> [(TopoKind, &[TopoId]); 6] {
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

    fn refresh_change_set(&mut self, sources: &[&Model]) {
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
        value_codec::to_string(surface).unwrap()
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
                        loop_owners[wire].insert(key.clone());
                        for coedge in &self.loops[wire].coedges {
                            edge_owners[coedge.edge].insert(key.clone());
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
                    .chunks_exact(3)
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
    fn record_relations(
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
                matches
                    .entry(geometry.clone())
                    .or_default()
                    .insert(authored.clone());
            }
        }
        let candidates: Vec<_> = target
            .iter()
            .map(|geometry| {
                matches.get(geometry).and_then(|parents| {
                    (parents.len() == 1).then(|| parents.first().unwrap().clone())
                })
            })
            .collect();
        let mut counts = BTreeMap::<TopoId, usize>::new();
        for id in candidates.iter().flatten() {
            *counts.entry(id.clone()).or_default() += 1;
        }
        let mut occupied = target.iter().copied().collect::<BTreeSet<_>>();
        for (id, candidate) in target.iter_mut().zip(candidates) {
            if let Some(candidate) = candidate {
                if counts[&candidate] == 1 && (candidate == *id || !occupied.contains(&candidate)) {
                    occupied.remove(id);
                    occupied.insert(candidate);
                    *id = candidate;
                }
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
                        vertex_parents[target_index]
                            .insert(source.1.vertices[source_index].clone());
                    }
                }
            }
            for (target_index, target) in self.edges.iter().enumerate() {
                for (source_index, candidate) in source.edges.iter().enumerate() {
                    if target_edge_keys[target_index] == geometry.1.edges[source_index]
                        || self.edge_overlap(target, source, candidate)
                    {
                        edge_parents[target_index].insert(source.1.edges[source_index].clone());
                    }
                }
            }
            for (target_index, target) in target_faces.iter().enumerate() {
                for (source_index, candidate) in source_faces.iter().enumerate() {
                    if Self::face_relation(target, candidate) {
                        face_parents[target_index].insert(source.1.faces[source_index].clone());
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
        for (kind, ids) in self.identity_groups() {
            require(
                ids.iter().all(|id| id.kind() == kind),
                "Topology ID prefix does not match its entity kind",
            )?;
            require(
                ids.iter()
                    .all(|id| self.1.change_set.nodes.get(id) == Some(&kind)),
                "Authoritative change set is missing a topology entity",
            )?;
        }
        self.1.change_set.validate().map_err(|error| Error {
            code: error.code,
            message: error.message,
        })?;
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
            if e.degenerate {
                let pole = self.vertices[e.vertices[0]].point;
                require(
                    e.curve
                        .control_points
                        .iter()
                        .all(|p| p.as_slice() == pole.as_slice()),
                    "Collapsed edge curve must be identically its pole vertex",
                )?;
            } else {
                require(
                    e.curve
                        .control_points
                        .iter()
                        .any(|p| *p != e.curve.control_points[0]),
                    "Constant edge requires an explicit collapsed-boundary marker",
                )?;
            }
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
                    if edge.degenerate {
                        validate_pole_boundary(
                            &f.surface,
                            &c.pcurve,
                            self.vertices[edge.vertices[0]].point,
                        )?;
                    }
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
                            if !self.edges[c.edge].degenerate {
                                *uses.entry(c.edge).or_default() += 1;
                            }
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
/// Certify collapsed boundaries from control nets, not a finite sample set.
/// Currently supported pole charts collapse one complete rectangular boundary.
fn validate_pole_boundary(surface: &Surface, pcurve: &Curve, pole: [f64; 3]) -> Result<()> {
    require(
        pcurve.degree == 1 && pcurve.control_points.len() == 2,
        "Pole pcurve must be a straight surface boundary",
    )?;
    let u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let a = &pcurve.control_points[0];
    let b = &pcurve.control_points[1];
    let lies_in = |a: f64, b: f64, domain: [f64; 2]| {
        a != b && a >= domain[0] && a <= domain[1] && b >= domain[0] && b <= domain[1]
    };
    let same = |p: &Vec<f64>| p.as_slice() == pole.as_slice();
    let collapsed = if a[0] == b[0] && lies_in(a[1], b[1], v) {
        if a[0] == u[0] {
            surface.control_points[0].iter().all(same)
        } else if a[0] == u[1] {
            surface.control_points.last().unwrap().iter().all(same)
        } else {
            false
        }
    } else if a[1] == b[1] && lies_in(a[0], b[0], u) {
        if a[1] == v[0] {
            surface.control_points.iter().all(|r| same(&r[0]))
        } else if a[1] == v[1] {
            surface
                .control_points
                .iter()
                .all(|r| same(r.last().unwrap()))
        } else {
            false
        }
    } else {
        false
    };
    require(
        collapsed,
        "Pole boundary is not identically collapsed in the surface control net",
    )
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
                    degenerate: false,
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
