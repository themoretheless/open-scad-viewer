//! Minimal parametric feature history with transactional topology rebinding.
//!
//! Every rebuild starts at the block and executes the complete dependency DAG.
//! A binding is resolved against the model immediately before its consumer:
//! exact persistent ID first, then one unambiguous geometric match. Losing or
//! ambiguously matching a binding is a typed refusal, never an index fallback.
use crate::analysis::surface_classify::{SurfaceClass, classify_surface, face_area};
use crate::direct_edit::{
    DeleteFaceOptions, DeleteFaceStrategy, delete_face, direct_edit_model_hash, face_plane_fit,
    replace_face,
};
use crate::feature_graph::{Feature, FeatureGraph, FeatureSuppressionReport};
use crate::{Error, Model, Result, TopoId, TopoKind, analytic_fillet, cuboid};
use nurbs_core::foundation::guards::Budget;
use std::collections::BTreeSet;

pub const FEATURE_HISTORY_INVALID: &str = "BREP_FEATURE_HISTORY_INVALID";
pub const FEATURE_HISTORY_BINDING_LOST: &str = "BREP_FEATURE_HISTORY_BINDING_LOST";
pub const FEATURE_HISTORY_BINDING_AMBIGUOUS: &str = "BREP_FEATURE_HISTORY_BINDING_AMBIGUOUS";

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RebindPolicy {
    pub linear_tolerance_mm: f64,
    pub area_relative_tolerance: f64,
    pub angle_tolerance_rad: f64,
}

impl Default for RebindPolicy {
    fn default() -> Self {
        Self {
            linear_tolerance_mm: 1e-6,
            area_relative_tolerance: 1e-6,
            angle_tolerance_rad: 1e-6,
        }
    }
}

impl RebindPolicy {
    fn validate(self) -> Result<()> {
        if !self.linear_tolerance_mm.is_finite()
            || !self.area_relative_tolerance.is_finite()
            || !self.angle_tolerance_rad.is_finite()
            || self.linear_tolerance_mm < 0.
            || self.area_relative_tolerance < 0.
            || self.angle_tolerance_rad < 0.
        {
            return Err(error(
                FEATURE_HISTORY_INVALID,
                "Rebind tolerances must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingResolutionMethod {
    Exact,
    Fuzzy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceSignature {
    pub class: SurfaceClass,
    pub normal_or_axis: Option<[f64; 3]>,
    pub area_mm2: f64,
    pub centroid: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeSignature {
    pub degree: usize,
    pub direction: [f64; 3],
    pub midpoint: [f64; 3],
    pub length_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TopologySignature {
    Face(FaceSignature),
    Edge(EdgeSignature),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopologyBinding {
    pub id: TopoId,
    pub producer: String,
    pub signature: TopologySignature,
}

impl TopologyBinding {
    pub fn from_face(
        model: &Model,
        face: usize,
        producer: impl Into<String>,
        budget: &Budget,
    ) -> Result<Self> {
        let id = *model.1.faces.get(face).ok_or_else(|| {
            error(
                FEATURE_HISTORY_INVALID,
                format!("Face {face} is out of range while creating a binding"),
            )
        })?;
        Ok(Self {
            id,
            producer: producer.into(),
            signature: TopologySignature::Face(face_signature(model, face, budget)?),
        })
    }

    pub fn from_edge(
        model: &Model,
        edge: usize,
        producer: impl Into<String>,
        _budget: &Budget,
    ) -> Result<Self> {
        let id = *model.1.edges.get(edge).ok_or_else(|| {
            error(
                FEATURE_HISTORY_INVALID,
                format!("Edge {edge} is out of range while creating a binding"),
            )
        })?;
        Ok(Self {
            id,
            producer: producer.into(),
            signature: TopologySignature::Edge(edge_signature(model, edge)?),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum HistoryOperation {
    Block {
        min: [f64; 3],
        max: [f64; 3],
    },
    Fillet {
        edge: TopologyBinding,
        radius_mm: f64,
    },
    MoveFace {
        face: TopologyBinding,
        direction: [f64; 3],
        distance_mm: f64,
    },
    DeleteFace {
        face: TopologyBinding,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryNode {
    pub id: String,
    pub name: String,
    pub depends_on: BTreeSet<String>,
    pub operation: HistoryOperation,
    /// Skip this node during history rebuild without deleting its parameters.
    /// Unrelated to geometric `direct_edit::suppress_feature` removal.
    pub suppressed: bool,
}

impl HistoryNode {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        operation: HistoryOperation,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            depends_on: BTreeSet::new(),
            operation,
            suppressed: false,
        }
    }

    pub fn with_suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    pub fn block(
        id: impl Into<String>,
        name: impl Into<String>,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Self {
        Self::new(id, name, HistoryOperation::Block { min, max })
    }

    pub fn fillet(
        id: impl Into<String>,
        name: impl Into<String>,
        upstream: impl Into<String>,
        edge: TopologyBinding,
        radius_mm: f64,
    ) -> Self {
        Self::new(id, name, HistoryOperation::Fillet { edge, radius_mm })
            .with_dependencies([upstream.into()])
    }

    pub fn move_face(
        id: impl Into<String>,
        name: impl Into<String>,
        upstream: impl Into<String>,
        face: TopologyBinding,
        direction: [f64; 3],
        distance_mm: f64,
    ) -> Self {
        Self::new(
            id,
            name,
            HistoryOperation::MoveFace {
                face,
                direction,
                distance_mm,
            },
        )
        .with_dependencies([upstream.into()])
    }

    pub fn delete_face(
        id: impl Into<String>,
        name: impl Into<String>,
        upstream: impl Into<String>,
        face: TopologyBinding,
    ) -> Self {
        Self::new(id, name, HistoryOperation::DeleteFace { face })
            .with_dependencies([upstream.into()])
    }

    pub fn with_dependencies(mut self, dependencies: impl IntoIterator<Item = String>) -> Self {
        self.depends_on = dependencies.into_iter().collect();
        self
    }

    fn binding(&self) -> Option<&TopologyBinding> {
        match &self.operation {
            HistoryOperation::Block { .. } => None,
            HistoryOperation::Fillet { edge, .. } => Some(edge),
            HistoryOperation::MoveFace { face, .. } | HistoryOperation::DeleteFace { face } => {
                Some(face)
            }
        }
    }

    fn set_binding_id(&mut self, id: TopoId) {
        match &mut self.operation {
            HistoryOperation::Block { .. } => {}
            HistoryOperation::Fillet { edge, .. } => edge.id = id,
            HistoryOperation::MoveFace { face, .. } | HistoryOperation::DeleteFace { face } => {
                face.id = id;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatureHistory {
    nodes: Vec<HistoryNode>,
}

impl FeatureHistory {
    pub fn new(nodes: Vec<HistoryNode>) -> Self {
        Self { nodes }
    }

    pub fn nodes(&self) -> &[HistoryNode] {
        &self.nodes
    }

    fn dependency_graph(nodes: &[HistoryNode]) -> Result<FeatureGraph> {
        FeatureGraph::new(
            nodes
                .iter()
                .map(|node| {
                    Feature::new(&node.id, &node.name)
                        .with_dependencies(node.depends_on.iter().cloned())
                        .with_suppressed(node.suppressed)
                })
                .collect(),
        )
    }

    pub fn suppression_report(&self) -> Result<FeatureSuppressionReport> {
        Self::dependency_graph(&self.nodes)?.suppression_report()
    }

    /// Suppress in the parametric tree, preserving the complete operation.
    /// This does not call geometric `direct_edit::suppress_feature`.
    pub fn set_suppressed(
        &mut self,
        feature_id: &str,
        suppressed: bool,
    ) -> Result<FeatureSuppressionReport> {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node.id == feature_id)
            .ok_or_else(|| {
                error(
                    FEATURE_HISTORY_INVALID,
                    format!("Unknown history feature \"{feature_id}\""),
                )
            })?;
        node.suppressed = suppressed;
        self.suppression_report()
    }

    pub fn set_block_dimensions(
        &mut self,
        feature_id: &str,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Result<()> {
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node.id == feature_id)
            .ok_or_else(|| {
                error(
                    FEATURE_HISTORY_INVALID,
                    format!("Unknown history feature \"{feature_id}\""),
                )
            })?;
        let HistoryOperation::Block {
            min: old_min,
            max: old_max,
        } = &mut node.operation
        else {
            return Err(error(
                FEATURE_HISTORY_INVALID,
                format!("Feature \"{}\" is not a block", node.name),
            ));
        };
        *old_min = min;
        *old_max = max;
        Ok(())
    }

    pub fn rebuild(&self, policy: RebindPolicy, budget: &Budget) -> Result<HistoryRebuildReport> {
        Self::execute(&self.nodes, policy, budget).map(|(report, _)| report)
    }

    /// Rebuild on staged bindings and commit their IDs only after every feature
    /// succeeds. Both the operation sequence and the old bindings survive error.
    pub fn rebuild_and_commit(
        &mut self,
        policy: RebindPolicy,
        budget: &Budget,
    ) -> Result<HistoryRebuildReport> {
        let (report, resolved) = Self::execute(&self.nodes, policy, budget)?;
        let mut staged = self.nodes.clone();
        for (index, id) in resolved {
            staged[index].set_binding_id(id);
        }
        self.nodes = staged;
        Ok(report)
    }

    fn execute(
        nodes: &[HistoryNode],
        policy: RebindPolicy,
        budget: &Budget,
    ) -> Result<(HistoryRebuildReport, Vec<(usize, TopoId)>)> {
        policy.validate()?;
        let by_id = nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<std::collections::BTreeMap<_, _>>();
        let graph = Self::dependency_graph(nodes)?;
        let order = graph.topological_order()?;
        let suppression = graph.suppression_report()?;
        let mut model: Option<Model> = None;
        let mut bindings = Vec::new();
        let mut resolved = Vec::new();
        for id in &order {
            if suppression.is_skipped(id) {
                continue;
            }
            let node = by_id[id.as_str()];
            let index = nodes
                .iter()
                .position(|candidate| candidate.id == *id)
                .expect("graph node");
            match &node.operation {
                HistoryOperation::Block { min, max } => {
                    if model.is_some() {
                        return Err(error(
                            FEATURE_HISTORY_INVALID,
                            "A minimal history accepts exactly one root block",
                        ));
                    }
                    model = Some(cuboid(*min, *max)?);
                }
                operation => {
                    let current = model.as_ref().ok_or_else(|| {
                        error(
                            FEATURE_HISTORY_INVALID,
                            format!("Feature \"{}\" has no root block", node.name),
                        )
                    })?;
                    let binding = node.binding().expect("non-block operations have bindings");
                    let resolution =
                        resolve_binding(current, binding, node, nodes, policy, budget)?;
                    let next = match operation {
                        HistoryOperation::Fillet { radius_mm, .. } => {
                            analytic_fillet(current, resolution.index, *radius_mm)?.0
                        }
                        HistoryOperation::MoveFace {
                            direction,
                            distance_mm,
                            ..
                        } => move_planar_face(
                            current,
                            resolution.index,
                            *direction,
                            *distance_mm,
                            budget,
                        )?,
                        HistoryOperation::DeleteFace { .. } => {
                            delete_face(
                                current,
                                resolution.index,
                                DeleteFaceOptions::with_strategy(DeleteFaceStrategy::Cap),
                                budget,
                            )?
                            .model
                        }
                        HistoryOperation::Block { .. } => unreachable!(),
                    };
                    resolved.push((index, resolution.id));
                    bindings.push(FeatureBindingReport {
                        feature_id: node.id.clone(),
                        feature_name: node.name.clone(),
                        old_id: binding.id,
                        new_id: resolution.id,
                        method: resolution.method,
                    });
                    model = Some(next);
                }
            }
        }
        let model = model
            .ok_or_else(|| error(FEATURE_HISTORY_INVALID, "Feature history has no root block"))?;
        let result_hash = direct_edit_model_hash(&model);
        Ok((
            HistoryRebuildReport {
                model,
                order,
                result_hash,
                bindings,
                suppression,
            },
            resolved,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct HistoryRebuildReport {
    pub model: Model,
    pub order: Vec<String>,
    pub result_hash: [u8; 32],
    pub bindings: Vec<FeatureBindingReport>,
    pub suppression: FeatureSuppressionReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureBindingReport {
    pub feature_id: String,
    pub feature_name: String,
    pub old_id: TopoId,
    pub new_id: TopoId,
    pub method: BindingResolutionMethod,
}

struct BindingResolution {
    index: usize,
    id: TopoId,
    method: BindingResolutionMethod,
}

fn face_signature(model: &Model, face: usize, budget: &Budget) -> Result<FaceSignature> {
    let entity = model.faces.get(face).ok_or_else(|| {
        error(
            FEATURE_HISTORY_INVALID,
            format!("Face {face} is out of range while capturing a signature"),
        )
    })?;
    let classification = classify_surface(&entity.surface, model.tolerance_mm.max(1e-7), budget)?;
    let normal_or_axis = match classification.class {
        SurfaceClass::Plane => Some(face_plane_fit(model, face)?.normal),
        _ => classification.axis.map(|axis| axis.direction),
    };
    let mut vertices = BTreeSet::new();
    for ring in std::iter::once(&entity.outer).chain(&entity.holes) {
        for coedge in &model.loops[*ring].coedges {
            vertices.extend(model.edges[coedge.edge].vertices);
        }
    }
    if vertices.is_empty() {
        return Err(error(
            FEATURE_HISTORY_INVALID,
            format!("Face {face} has no boundary vertices"),
        ));
    }
    let sum = vertices.iter().fold([0.; 3], |sum, vertex| {
        let point = model.vertices[*vertex].point;
        [sum[0] + point[0], sum[1] + point[1], sum[2] + point[2]]
    });
    let centroid = sum.map(|coordinate| coordinate / vertices.len() as f64);
    Ok(FaceSignature {
        class: classification.class,
        normal_or_axis,
        area_mm2: face_area(model, face, budget)?,
        centroid,
    })
}

fn edge_signature(model: &Model, edge: usize) -> Result<EdgeSignature> {
    let entity = model.edges.get(edge).ok_or_else(|| {
        error(
            FEATURE_HISTORY_INVALID,
            format!("Edge {edge} is out of range while capturing a signature"),
        )
    })?;
    if entity.curve.degree != 1
        || entity.curve.control_points.len() != 2
        || entity.curve.weights[0] != entity.curve.weights[1]
    {
        return Err(error(
            FEATURE_HISTORY_INVALID,
            "Minimal history edge bindings accept straight edges only",
        ));
    }
    let a = model.vertices[entity.vertices[0]].point;
    let b = model.vertices[entity.vertices[1]].point;
    let delta = sub(b, a);
    let length_mm = norm(delta);
    if !(length_mm.is_finite() && length_mm > 0.) {
        return Err(error(
            FEATURE_HISTORY_INVALID,
            "Cannot bind a zero-length edge",
        ));
    }
    Ok(EdgeSignature {
        degree: entity.curve.degree,
        direction: delta.map(|coordinate| coordinate / length_mm),
        midpoint: std::array::from_fn(|axis| (a[axis] + b[axis]) * 0.5),
        length_mm,
    })
}

fn directions_match(a: [f64; 3], b: [f64; 3], angle_tolerance_rad: f64) -> bool {
    let cosine = dot(a, b).abs().clamp(-1., 1.);
    cosine.acos() <= angle_tolerance_rad
}

fn relative_difference(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(b.abs()).max(1e-300)
}

fn fuzzy_matches(
    expected: &TopologySignature,
    candidate: &TopologySignature,
    policy: RebindPolicy,
) -> bool {
    match (expected, candidate) {
        (TopologySignature::Face(expected), TopologySignature::Face(candidate)) => {
            expected.class == candidate.class
                && expected.normal_or_axis.is_some() == candidate.normal_or_axis.is_some()
                && expected.normal_or_axis.is_none_or(|a| {
                    directions_match(
                        a,
                        candidate.normal_or_axis.expect("presence checked"),
                        policy.angle_tolerance_rad,
                    )
                })
                && distance(expected.centroid, candidate.centroid) <= policy.linear_tolerance_mm
                && relative_difference(expected.area_mm2, candidate.area_mm2)
                    <= policy.area_relative_tolerance
        }
        (TopologySignature::Edge(expected), TopologySignature::Edge(candidate)) => {
            expected.degree == candidate.degree
                && directions_match(
                    expected.direction,
                    candidate.direction,
                    policy.angle_tolerance_rad,
                )
                && distance(expected.midpoint, candidate.midpoint) <= policy.linear_tolerance_mm
                && relative_difference(expected.length_mm, candidate.length_mm)
                    <= policy.area_relative_tolerance
        }
        _ => false,
    }
}

fn resolve_binding(
    model: &Model,
    binding: &TopologyBinding,
    consumer: &HistoryNode,
    nodes: &[HistoryNode],
    policy: RebindPolicy,
    budget: &Budget,
) -> Result<BindingResolution> {
    let ids = match binding.id.kind() {
        TopoKind::Face => &model.1.faces,
        TopoKind::Edge => &model.1.edges,
        _ => {
            return Err(error(
                FEATURE_HISTORY_INVALID,
                format!(
                    "Feature \"{}\" binds unsupported topology {}",
                    consumer.name, binding.id
                ),
            ));
        }
    };
    if let Some(index) = ids.iter().position(|id| id == &binding.id) {
        return Ok(BindingResolution {
            index,
            id: binding.id,
            method: BindingResolutionMethod::Exact,
        });
    }
    let mut matches = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let signature = match binding.signature {
            TopologySignature::Face(_) if binding.id.kind() == TopoKind::Face => {
                TopologySignature::Face(face_signature(model, index, budget)?)
            }
            TopologySignature::Edge(_) if binding.id.kind() == TopoKind::Edge => {
                match edge_signature(model, index) {
                    Ok(signature) => TopologySignature::Edge(signature),
                    Err(_) => continue,
                }
            }
            _ => {
                return Err(error(
                    FEATURE_HISTORY_INVALID,
                    format!(
                        "Feature \"{}\" has a binding/signature kind mismatch",
                        consumer.name
                    ),
                ));
            }
        };
        if fuzzy_matches(&binding.signature, &signature, policy) {
            matches.push(BindingResolution {
                index,
                id: *id,
                method: BindingResolutionMethod::Fuzzy,
            });
        }
    }
    let upstream = if consumer.depends_on.is_empty() {
        "none".to_string()
    } else {
        consumer
            .depends_on
            .iter()
            .map(|id| {
                let name = &nodes
                    .iter()
                    .find(|node| &node.id == id)
                    .expect("graph validated upstream feature")
                    .name;
                format!("\"{name}\"")
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    match matches.len() {
        1 => Ok(matches.pop().expect("one match")),
        0 => Err(error(
            FEATURE_HISTORY_BINDING_LOST,
            format!(
                "Feature \"{}\" lost {} binding {} produced by \"{}\"; upstream features: {upstream}; no exact or unique fuzzy match",
                consumer.name,
                binding.id.kind().as_str(),
                binding.id,
                binding.producer
            ),
        )),
        _ => Err(error(
            FEATURE_HISTORY_BINDING_AMBIGUOUS,
            format!(
                "Feature \"{}\" has ambiguous {} binding {} produced by \"{}\"; upstream features: {upstream}; {} fuzzy matches",
                consumer.name,
                binding.id.kind().as_str(),
                binding.id,
                binding.producer,
                matches.len()
            ),
        )),
    }
}

fn move_planar_face(
    model: &Model,
    face: usize,
    direction: [f64; 3],
    distance_mm: f64,
    budget: &Budget,
) -> Result<Model> {
    if direction.iter().any(|value| !value.is_finite())
        || !distance_mm.is_finite()
        || norm(direction) == 0.
    {
        return Err(error(
            FEATURE_HISTORY_INVALID,
            "Move-face direction must be finite and nonzero; distance must be finite",
        ));
    }
    let mut target = model
        .faces
        .get(face)
        .ok_or_else(|| {
            error(
                FEATURE_HISTORY_INVALID,
                format!("Face {face} is out of range"),
            )
        })?
        .surface
        .clone();
    if target.degree_u != 1
        || target.degree_v != 1
        || target
            .control_points
            .iter()
            .any(|row| row.len() != target.control_points[0].len())
    {
        return Err(error(
            FEATURE_HISTORY_INVALID,
            "Minimal history move-face accepts bilinear planar faces only",
        ));
    }
    for point in target.control_points.iter_mut().flatten() {
        for axis in 0..3 {
            point[axis] += direction[axis] * distance_mm;
        }
    }
    // Unlike move_face's all-planar-body walk, replace_face only retrims the
    // selected face and its planar neighbors. A remote fillet therefore stays
    // untouched while this remains a real geometry edit with solid auditing.
    Ok(replace_face(model, face, &target, budget)?.model)
}

#[cfg(test)]
#[path = "feature_history/tests/mod.rs"]
mod tests;
