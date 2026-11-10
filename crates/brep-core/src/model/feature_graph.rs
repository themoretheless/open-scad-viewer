//! Standalone feature dependency graph and transactional reorder planning.
//! This is deliberately not a parametric history: callers supply feature
//! operations and the initial model for every trial rebuild.
use crate::{ChangeSet, Error, Model, Result, TopoId, TopoKind};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const FEATURE_GRAPH_INVALID: &str = "BREP_FEATURE_GRAPH_INVALID";
pub const FEATURE_REORDER_CONFLICT: &str = "BREP_FEATURE_REORDER_CONFLICT";
pub const FEATURE_REBUILD_NAMING: &str = "BREP_FEATURE_REBUILD_NAMING";

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feature {
    pub id: String,
    pub name: String,
    /// Explicit feature-to-feature dependencies.
    pub depends_on: BTreeSet<String>,
    /// Persistent topology consumed by this feature.
    pub inputs: BTreeSet<TopoId>,
    /// Persistent topology declared as produced by this feature.
    pub outputs: BTreeSet<TopoId>,
    /// History-tree suppression. This is not geometric feature removal.
    pub suppressed: bool,
}

impl Feature {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            depends_on: BTreeSet::new(),
            inputs: BTreeSet::new(),
            outputs: BTreeSet::new(),
            suppressed: false,
        }
    }

    pub fn with_suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    pub fn with_dependencies(mut self, dependencies: impl IntoIterator<Item = String>) -> Self {
        self.depends_on = dependencies.into_iter().collect();
        self
    }

    pub fn with_inputs(mut self, inputs: impl IntoIterator<Item = TopoId>) -> Self {
        self.inputs = inputs.into_iter().collect();
        self
    }

    pub fn with_outputs(mut self, outputs: impl IntoIterator<Item = TopoId>) -> Self {
        self.outputs = outputs.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyEdge {
    pub provider: String,
    pub dependent: String,
    pub topology: Option<TopoId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionSearch {
    pub positions: std::ops::RangeInclusive<usize>,
    /// Exactly one scan of the dependency edge list; no reachability search.
    pub checked_edges: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderReport {
    pub previous_position: usize,
    pub new_position: usize,
    pub checked_edges: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub feature_id: String,
    pub feature_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanglingFeature {
    pub feature_id: String,
    pub feature_name: String,
    /// Names of immediate parents already suppressed or dangling.
    pub suppressed_parents: Vec<String>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureSuppressionReport {
    /// Explicitly suppressed roots; parameters remain in the graph.
    pub suppressed: Vec<FeatureDescriptor>,
    /// Transitive dependents skipped by the cascade.
    pub dangling: Vec<DanglingFeature>,
    skipped: BTreeSet<String>,
}

impl FeatureSuppressionReport {
    pub fn is_skipped(&self, feature_id: &str) -> bool {
        self.skipped.contains(feature_id)
    }
}

#[derive(Clone, Debug)]
pub struct FeatureRebuildReport {
    pub model: Model,
    pub order: Vec<String>,
    pub result_hash: [u8; 32],
    pub rebuilt_features: usize,
    pub suppression: FeatureSuppressionReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureGraph {
    features: BTreeMap<String, Feature>,
    providers: HashMap<TopoId, String>,
    order: Vec<String>,
}

impl FeatureGraph {
    pub fn new(features: Vec<Feature>) -> Result<Self> {
        let order = features
            .iter()
            .map(|feature| feature.id.clone())
            .collect::<Vec<_>>();
        let mut by_id = BTreeMap::new();
        let mut providers = HashMap::new();
        for feature in features {
            if feature.id.is_empty() || feature.name.is_empty() {
                return Err(error(
                    FEATURE_GRAPH_INVALID,
                    "Feature id and name must be non-empty",
                ));
            }
            if by_id.insert(feature.id.clone(), feature.clone()).is_some() {
                return Err(error(
                    FEATURE_GRAPH_INVALID,
                    format!("Duplicate feature id \"{}\"", feature.id),
                ));
            }
            for output in &feature.outputs {
                if let Some(previous) = providers.insert(*output, feature.id.clone()) {
                    return Err(error(
                        FEATURE_GRAPH_INVALID,
                        format!(
                            "Topology {output} is produced by both \"{previous}\" and \"{}\"",
                            feature.id
                        ),
                    ));
                }
            }
        }
        for feature in by_id.values() {
            for dependency in &feature.depends_on {
                if !by_id.contains_key(dependency) {
                    return Err(error(
                        FEATURE_GRAPH_INVALID,
                        format!(
                            "Feature \"{}\" depends on unknown feature \"{dependency}\"",
                            feature.name
                        ),
                    ));
                }
            }
            for input in &feature.inputs {
                if !providers.contains_key(input) {
                    return Err(error(
                        FEATURE_GRAPH_INVALID,
                        format!(
                            "Feature \"{}\" consumes topology {input} with no declared producer",
                            feature.name
                        ),
                    ));
                }
            }
        }
        let graph = Self {
            features: by_id,
            providers,
            order,
        };
        graph.topological_order()?;
        Ok(graph)
    }

    /// Derive producer declarations from persistent naming. The operation is
    /// the feature id; unknown operations remain outside this explicit graph.
    pub fn from_change_set(mut features: Vec<Feature>, changes: &ChangeSet) -> Result<Self> {
        for change in &changes.changes {
            if let Some(feature) = features
                .iter_mut()
                .find(|feature| feature.id == change.provenance.operation)
            {
                feature.outputs.extend(change.children.iter().copied());
            }
        }
        Self::new(features)
    }

    pub fn order(&self) -> &[String] {
        &self.order
    }

    pub fn feature(&self, id: &str) -> Option<&Feature> {
        self.features.get(id)
    }

    fn descriptor(&self, id: &str) -> FeatureDescriptor {
        let feature = &self.features[id];
        FeatureDescriptor {
            feature_id: feature.id.clone(),
            feature_name: feature.name.clone(),
        }
    }

    /// Set history-tree suppression on one node and return the resulting
    /// cascade. Dependents remain in the graph with every parameter intact.
    pub fn set_suppressed(
        &mut self,
        feature_id: &str,
        suppressed: bool,
    ) -> Result<FeatureSuppressionReport> {
        let feature = self.features.get_mut(feature_id).ok_or_else(|| {
            error(
                FEATURE_GRAPH_INVALID,
                format!("Unknown feature id \"{feature_id}\""),
            )
        })?;
        feature.suppressed = suppressed;
        self.suppression_report()
    }

    /// Compute explicit suppression and its transitive dangling cascade in
    /// topological order. Each edge is examined once while building incoming
    /// adjacency and once during the cascade pass.
    pub fn suppression_report(&self) -> Result<FeatureSuppressionReport> {
        let edges = self.dependency_edges();
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        for edge in edges {
            incoming.entry(edge.dependent).or_default().push(edge.provider);
        }
        let mut suppressed = Vec::new();
        let mut dangling = Vec::new();
        let mut skipped = BTreeSet::new();
        for id in self.topological_order()? {
            let feature = &self.features[&id];
            if feature.suppressed {
                suppressed.push(self.descriptor(&id));
            }
            let mut blocked_parents = incoming
                .get(&id)
                .into_iter()
                .flatten()
                .filter(|parent| skipped.contains(*parent))
                .map(|parent| self.features[parent].name.clone())
                .collect::<Vec<_>>();
            blocked_parents.sort();
            blocked_parents.dedup();
            if !blocked_parents.is_empty() {
                let reason = format!(
                    "Feature \"{}\" depends on suppressed/dangling parent(s): {}",
                    feature.name,
                    blocked_parents.join(", ")
                );
                dangling.push(DanglingFeature {
                    feature_id: feature.id.clone(),
                    feature_name: feature.name.clone(),
                    suppressed_parents: blocked_parents,
                    reason,
                });
            }
            if feature.suppressed
                || incoming
                    .get(&id)
                    .is_some_and(|parents| parents.iter().any(|parent| skipped.contains(parent)))
            {
                skipped.insert(id);
            }
        }
        Ok(FeatureSuppressionReport {
            suppressed,
            dangling,
            skipped,
        })
    }

    pub fn dependency_edges(&self) -> Vec<DependencyEdge> {
        let mut edges = Vec::new();
        for feature in self.features.values() {
            for dependency in &feature.depends_on {
                edges.push(DependencyEdge {
                    provider: dependency.clone(),
                    dependent: feature.id.clone(),
                    topology: None,
                });
            }
            for input in &feature.inputs {
                edges.push(DependencyEdge {
                    provider: self.providers[input].clone(),
                    dependent: feature.id.clone(),
                    topology: Some(*input),
                });
            }
        }
        edges
    }

    pub fn dependency_edge_count(&self) -> usize {
        self.features
            .values()
            .map(|feature| feature.depends_on.len() + feature.inputs.len())
            .sum()
    }

    fn dependency_error(&self, edge: &DependencyEdge) -> Error {
        let provider = &self.features[&edge.provider];
        let dependent = &self.features[&edge.dependent];
        let detail = edge.topology.map_or_else(
            || "an explicit feature input".into(),
            |topology| format!("topology {topology}"),
        );
        error(
            FEATURE_REORDER_CONFLICT,
            format!(
                "Feature \"{}\" depends on {detail} created by feature \"{}\"",
                dependent.name, provider.name
            ),
        )
    }

    pub fn topological_order(&self) -> Result<Vec<String>> {
        let ids = self.order.clone();
        let index: HashMap<_, _> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let mut indegree = vec![0usize; ids.len()];
        let mut adjacency = vec![Vec::<usize>::new(); ids.len()];
        for edge in self.dependency_edges() {
            let provider = index[edge.provider.as_str()];
            let dependent = index[edge.dependent.as_str()];
            adjacency[provider].push(dependent);
            indegree[dependent] += 1;
        }
        let mut ready: BTreeSet<usize> = indegree
            .iter()
            .enumerate()
            .filter(|(_, degree)| **degree == 0)
            .map(|(index, _)| index)
            .collect();
        let mut result = Vec::with_capacity(ids.len());
        while let Some(index) = ready.pop_first() {
            result.push(ids[index].clone());
            for dependent in &adjacency[index] {
                indegree[*dependent] -= 1;
                if indegree[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }
        if result.len() != ids.len() {
            return Err(error(
                FEATURE_GRAPH_INVALID,
                "Feature dependency graph contains a cycle",
            ));
        }
        Ok(result)
    }

    pub fn validate_order(&self, order: &[String]) -> Result<()> {
        if order.len() != self.features.len() {
            return Err(error(
                FEATURE_GRAPH_INVALID,
                "Reorder must contain every feature exactly once",
            ));
        }
        let indices: HashMap<_, _> = self
            .order
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect();
        let mut position = vec![usize::MAX; self.features.len()];
        for (index, id) in order.iter().enumerate() {
            let Some(&feature_index) = indices.get(id.as_str()) else {
                return Err(error(
                    FEATURE_GRAPH_INVALID,
                    format!("Unknown feature id \"{id}\""),
                ));
            };
            if std::mem::replace(&mut position[feature_index], index) != usize::MAX {
                return Err(error(
                    FEATURE_GRAPH_INVALID,
                    "Reorder contains a duplicate feature",
                ));
            }
        }
        if position.contains(&usize::MAX) {
            return Err(error(
                FEATURE_GRAPH_INVALID,
                "Reorder must contain every feature exactly once",
            ));
        }
        for edge in self.dependency_edges() {
            let provider = indices[edge.provider.as_str()];
            let dependent = indices[edge.dependent.as_str()];
            if position[provider] >= position[dependent] {
                return Err(self.dependency_error(&edge));
            }
        }
        Ok(())
    }

    fn reduced_order(&self, feature: &str) -> Result<(Vec<String>, usize)> {
        let previous = self
            .order
            .iter()
            .position(|id| id == feature)
            .ok_or_else(|| {
                error(
                    FEATURE_GRAPH_INVALID,
                    format!("Unknown feature id \"{feature}\""),
                )
            })?;
        let mut reduced = self.order.clone();
        reduced.remove(previous);
        Ok((reduced, previous))
    }

    /// Find insertion bounds with one dependency-edge scan. Direct incoming
    /// and outgoing edges are sufficient; transitive reachability is unnecessary.
    pub fn allowed_positions(&self, feature: &str) -> Result<PositionSearch> {
        let (reduced, _) = self.reduced_order(feature)?;
        self.position_search(feature, &reduced)
    }

    fn position_search(&self, feature: &str, reduced: &[String]) -> Result<PositionSearch> {
        let position: HashMap<_, _> = reduced
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let mut lower = 0usize;
        let mut upper = reduced.len();
        let edges = self.dependency_edges();
        for edge in &edges {
            if edge.dependent == feature {
                lower = lower.max(position[edge.provider.as_str()] + 1);
            }
            if edge.provider == feature {
                upper = upper.min(position[edge.dependent.as_str()]);
            }
        }
        Ok(PositionSearch {
            positions: lower..=upper,
            checked_edges: edges.len(),
        })
    }

    pub fn candidate_order(
        &self,
        feature: &str,
        new_position: usize,
    ) -> Result<(Vec<String>, usize)> {
        let (reduced, previous) = self.reduced_order(feature)?;
        let search = self.position_search(feature, &reduced)?;
        let mut candidate = reduced.clone();
        candidate.insert(new_position.min(candidate.len()), feature.into());
        if new_position > reduced.len() || !search.positions.contains(&new_position) {
            return Err(self.validate_order(&candidate).err().unwrap_or_else(|| {
                error(
                    FEATURE_GRAPH_INVALID,
                    format!("Insertion position {new_position} is out of range"),
                )
            }));
        }
        Ok((candidate, search.checked_edges))
    }

    pub fn reordered(&self, order: &[String]) -> Result<Vec<String>> {
        self.validate_order(order)?;
        Ok(order.to_vec())
    }

    pub fn reorder(&mut self, feature: &str, new_position: usize) -> Result<ReorderReport> {
        let previous_position =
            self.order
                .iter()
                .position(|id| id == feature)
                .ok_or_else(|| {
                    error(
                        FEATURE_GRAPH_INVALID,
                        format!("Unknown feature id \"{feature}\""),
                    )
                })?;
        let (candidate, checked_edges) = self.candidate_order(feature, new_position)?;
        self.order = candidate;
        Ok(ReorderReport {
            previous_position,
            new_position,
            checked_edges,
        })
    }

    pub fn trial_rebuild<F>(
        &self,
        order: &[String],
        initial: &Model,
        mut operation: F,
    ) -> Result<FeatureRebuildReport>
    where
        F: FnMut(FeatureRebuildContext<'_>) -> Result<Model>,
    {
        self.validate_order(order)?;
        let suppression = self.suppression_report()?;
        let mut model = initial.clone();
        let mut rebuilt_features = 0;
        for id in order {
            if suppression.is_skipped(id) {
                continue;
            }
            let context = FeatureRebuildContext {
                graph: self,
                feature: &self.features[id],
                model: &model,
            };
            for input in &context.feature.inputs {
                context.resolve(*input)?;
            }
            model = operation(context)?;
            rebuilt_features += 1;
        }
        let result_hash = crate::direct_edit::direct_edit_model_hash(&model);
        Ok(FeatureRebuildReport {
            model,
            order: order.to_vec(),
            result_hash,
            rebuilt_features,
            suppression,
        })
    }

    pub fn trial_reorder_rebuild<F>(
        &self,
        feature: &str,
        new_position: usize,
        initial: &Model,
        operation: F,
    ) -> Result<FeatureRebuildReport>
    where
        F: FnMut(FeatureRebuildContext<'_>) -> Result<Model>,
    {
        let (candidate, _) = self.candidate_order(feature, new_position)?;
        self.trial_rebuild(&candidate, initial, operation)
    }

    fn naming_error(&self, feature: &Feature, input: TopoId) -> Error {
        let provider_id = &self.providers[&input];
        let provider = &self.features[provider_id];
        error(
            FEATURE_REBUILD_NAMING,
            format!(
                "Feature \"{}\" depends on topology {input} created by feature \"{}\", but the ID is absent during rebuild",
                feature.name, provider.name
            ),
        )
    }
}

/// The selected feature and all of its transitive dependencies and dependents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencySubgraph {
    pub feature: FeatureDescriptor,
    pub parents: Vec<FeatureDescriptor>,
    pub children: Vec<FeatureDescriptor>,
}

/// Read-only deletion impact: the selected feature, its complete dependent
/// cascade, and all transitive parents whose outputs may become unused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletionPreview {
    pub direct: FeatureDescriptor,
    pub cascade: Vec<FeatureDescriptor>,
    pub parents: Vec<FeatureDescriptor>,
}

struct DependencyReachability {
    selected: usize,
    parents: Vec<bool>,
    children: Vec<bool>,
}

fn reachability(graph: &FeatureGraph, feature: &str) -> Result<DependencyReachability> {
    let index: HashMap<_, _> = graph
        .order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let selected = *index.get(feature).ok_or_else(|| {
        error(
            FEATURE_GRAPH_INVALID,
            format!("Unknown feature id \"{feature}\""),
        )
    })?;
    let mut children = vec![Vec::new(); graph.order.len()];
    let mut parents = vec![Vec::new(); graph.order.len()];
    for edge in graph.dependency_edges() {
        let provider = index[edge.provider.as_str()];
        let dependent = index[edge.dependent.as_str()];
        children[provider].push(dependent);
        parents[dependent].push(provider);
    }

    fn traverse(root: usize, adjacency: &[Vec<usize>]) -> Vec<bool> {
        let mut visited = vec![false; adjacency.len()];
        let mut stack = vec![root];
        visited[root] = true;
        while let Some(current) = stack.pop() {
            for &next in &adjacency[current] {
                if !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        visited[root] = false;
        visited
    }

    Ok(DependencyReachability {
        selected,
        parents: traverse(selected, &parents),
        children: traverse(selected, &children),
    })
}

fn ordered_descriptors(graph: &FeatureGraph, selected: &[bool]) -> Vec<FeatureDescriptor> {
    graph
        .order
        .iter()
        .enumerate()
        .filter(|(index, _)| selected[*index])
        .map(|(_, id)| graph.descriptor(id))
        .collect()
}

/// Collect transitive parents and children in O(V + E), returning descriptors
/// in the graph's current user-facing order. The graph is never mutated.
pub fn dependency_subgraph(
    graph: &FeatureGraph,
    feature: &str,
) -> Result<DependencySubgraph> {
    let reachable = reachability(graph, feature)?;
    Ok(DependencySubgraph {
        feature: graph.descriptor(&graph.order[reachable.selected]),
        parents: ordered_descriptors(graph, &reachable.parents),
        children: ordered_descriptors(graph, &reachable.children),
    })
}

/// Preview exact deletion impact without removing or reordering any graph node.
pub fn deletion_preview(graph: &FeatureGraph, feature: &str) -> Result<DeletionPreview> {
    let subgraph = dependency_subgraph(graph, feature)?;
    Ok(DeletionPreview {
        direct: subgraph.feature,
        cascade: subgraph.children,
        parents: subgraph.parents,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopoAddress {
    pub kind: TopoKind,
    pub index: usize,
}

pub struct FeatureRebuildContext<'a> {
    graph: &'a FeatureGraph,
    feature: &'a Feature,
    model: &'a Model,
}

impl<'a> FeatureRebuildContext<'a> {
    pub fn feature(&self) -> &'a Feature {
        self.feature
    }

    pub fn model(&self) -> &'a Model {
        self.model
    }

    pub fn resolve(&self, id: TopoId) -> Result<TopoAddress> {
        let index = match id.kind() {
            TopoKind::Vertex => self
                .model
                .1
                .vertices
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::Edge => self
                .model
                .1
                .edges
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::Loop => self
                .model
                .1
                .loops
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::Face => self
                .model
                .1
                .faces
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::Shell => self
                .model
                .1
                .shells
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::Body => self
                .model
                .1
                .bodies
                .iter()
                .position(|candidate| candidate == &id),
            TopoKind::ControlPoint => None,
        };
        index
            .map(|index| TopoAddress {
                kind: id.kind(),
                index,
            })
            .ok_or_else(|| self.graph.naming_error(self.feature, id))
    }

    pub fn face_index(&self, id: TopoId) -> Result<usize> {
        let address = self.resolve(id)?;
        if address.kind != TopoKind::Face {
            return Err(error(
                FEATURE_REBUILD_NAMING,
                format!("Topology {id} is not a face"),
            ));
        }
        Ok(address.index)
    }
}

#[cfg(test)]
#[path = "feature_graph/tests/mod.rs"]
mod tests;
