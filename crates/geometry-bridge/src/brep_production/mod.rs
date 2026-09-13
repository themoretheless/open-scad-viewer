//! Native occurrence-production replay for the B-rep SemanticProgram.
//!
//! Byte-exact Rust port of validateOccurrenceProduction
//! (src/services/semanticProgramValidator.ts l.1381) for the brep-1 reachable
//! path: every DAG node must have exactly one recomputed materializer group,
//! every occurrence group must close its frozen v1 production rule
//! (primitive, transform/projection/offset maps, preserving-alias,
//! transparent, boolean, hull, difference, linear/rotate extrusion), the
//! authenticated evaluation schedule must equal the node IDs exactly, and the
//! synthetic program frontier must equal the transported result.
//!
//! Narrowing (documented in the slice evidence): the interrupted-evaluation
//! branches (active difference bucket reconstruction, partial maps,
//! completeReduction, terminal prefix equality) require a non-null
//! execution.terminal or non-empty execution.discardedEffects, both of which
//! the brep-1 execution-plan admission (brep_execution_plan.rs) refuses
//! before this replay runs, so they are unreachable on the native path and
//! remain host-side; a transported non-empty effects list or non-null
//! terminal is refused here as well.
use super::{Result, Value};
use std::collections::{HashMap, HashSet};

fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

/// SEMANTIC_PROGRAM_LIMITS.snapshotValues bounds the expanded frontier proof.
const SNAPSHOT_VALUES_LIMIT: usize = 1_000_000;
/// Schedule replay budget: occurrences + nodes * 2 (host limits).
const SCHEDULE_EVENT_LIMIT: usize = 100_000 + 25_000 * 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProductionRule {
    Primitive,
    TransformMap,
    ProjectionMap,
    OffsetMap,
    PreservingAlias,
    Transparent,
    Boolean,
    Hull,
    Difference,
    LinearExtrude,
    RotateExtrude,
}

fn operation_category(operation: &Value) -> Result<&str> {
    operation["category"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic operation category"))
}

fn operation_name(operation: &Value) -> Result<&str> {
    operation["name"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic operation name"))
}

fn operation_parent(operation: &Value) -> Option<usize> {
    operation["parent"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
}

fn operation_path(operation: &Value) -> Result<&[Value]> {
    operation["structuralPath"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("Expected a semantic structural path"))
}

/// semanticProductionRule (allowInterruptedUnsupported is always false: the
/// interrupted group cannot exist without a terminal plan).
fn production_rule(operation: &Value) -> Result<ProductionRule> {
    let category = operation_category(operation)?;
    let name = operation_name(operation)?;
    let path = operation_path(operation)?;
    let last = path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let prior = path.len().checked_sub(2).map(|index| &path[index]);
    let last_kind = last["kind"].as_str();
    let last_ordinal = last["ordinal"].as_u64();
    let prior_kind = prior.and_then(|segment| segment["kind"].as_str());
    let prior_name = prior.and_then(|segment| segment["name"].as_str());
    let authored_call = last_kind == Some("call");
    let pair = (category, name);
    if category == "geometry"
        && authored_call
        && [
            "cube",
            "sphere",
            "cylinder",
            "polyhedron",
            "square",
            "circle",
            "polygon",
        ]
        .contains(&name)
    {
        return Ok(ProductionRule::Primitive);
    }
    if category == "transform"
        && authored_call
        && ["translate", "rotate", "scale", "mirror", "multmatrix"].contains(&name)
    {
        return Ok(ProductionRule::TransformMap);
    }
    if pair == ("geometry", "projection") && authored_call {
        return Ok(ProductionRule::ProjectionMap);
    }
    if pair == ("geometry", "offset") && authored_call {
        return Ok(ProductionRule::OffsetMap);
    }
    if (pair == ("presentation", "color") || pair == ("assertion", "assert")) && authored_call {
        return Ok(ProductionRule::PreservingAlias);
    }
    if category == "control" {
        let parented = operation["parent"].as_u64().is_some();
        if name == "$assign" && last_kind == Some("control") {
            return Ok(ProductionRule::Transparent);
        }
        if name == "$body"
            && parented
            && last_kind == Some("body")
            && last_ordinal == Some(0)
            && matches!(prior_kind, Some("call" | "module" | "control"))
        {
            return Ok(ProductionRule::Transparent);
        }
        if (name == "$then" || name == "$else")
            && parented
            && last_kind == Some("branch")
            && last_ordinal == Some(0)
            && prior_name == Some("if")
            && prior_kind == Some("control")
        {
            return Ok(ProductionRule::Transparent);
        }
        if name == "$expansion"
            && parented
            && last_kind == Some("control")
            && last_ordinal == Some(0)
            && (prior_name == Some("$body")
                || (prior_name == Some("children") && prior_kind == Some("control")))
        {
            return Ok(ProductionRule::Transparent);
        }
        if !name.starts_with('$')
            && last_kind == Some("control")
            && ["if", "let", "for", "children", "group", "render"].contains(&name)
        {
            return Ok(ProductionRule::Transparent);
        }
    }
    if category == "module"
        && !name.starts_with('$')
        && (last_kind == Some("module") || last_kind == Some("call"))
    {
        return Ok(ProductionRule::Transparent);
    }
    if (pair == ("boolean", "union") || pair == ("boolean", "intersection")) && authored_call {
        return Ok(ProductionRule::Boolean);
    }
    if pair == ("boolean", "hull") && authored_call {
        return Ok(ProductionRule::Hull);
    }
    if pair == ("boolean", "difference") && authored_call {
        return Ok(ProductionRule::Difference);
    }
    if pair == ("geometry", "linear_extrude") && authored_call {
        return Ok(ProductionRule::LinearExtrude);
    }
    if pair == ("geometry", "rotate_extrude") && authored_call {
        return Ok(ProductionRule::RotateExtrude);
    }
    Err(invalid(
        "Semantic operation pair has no frozen v1 production rule",
    ))
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ProductionItem {
    node: usize,
    row: usize,
    identity_owner: usize,
    group: usize,
    direct_child: usize,
}

#[derive(Clone, Copy)]
enum ScheduleEvent {
    Group(usize),
    Node(usize),
}

#[derive(Default)]
struct ProductionGroup {
    output_rows: Vec<usize>,
    output_items: Vec<ProductionItem>,
    frontier: Vec<ProductionItem>,
    terminal_items: Vec<ProductionItem>,
    schedule: Vec<ScheduleEvent>,
    transparent: bool,
}

/// Per-row occurrence facts the replay reads (already identity-admitted).
struct RowFacts {
    operation: usize,
    parent: Option<usize>,
    node: Option<usize>,
    output_ordinal: Option<u64>,
    has_scene_entity: bool,
}

struct Replay<'a> {
    operations: &'a [Value],
    nodes: &'a [Value],
    rows: &'a [RowFacts],
    rows_by_canonical: HashMap<usize, Vec<usize>>,
    children: HashMap<usize, Vec<usize>>,
    production_by_canonical: HashMap<usize, ProductionGroup>,
    owner_by_node: HashMap<usize, usize>,
    expanded_frontier_items: usize,
}

impl<'a> Replay<'a> {
    fn append_frontier(
        &mut self,
        target: &mut Vec<ProductionItem>,
        source: &[ProductionItem],
        direct_child: Option<usize>,
    ) -> Result<()> {
        self.expanded_frontier_items += source.len();
        if self.expanded_frontier_items > SNAPSHOT_VALUES_LIMIT {
            return Err(invalid(
                "Expanded occurrence frontier exceeds its bounded proof budget",
            ));
        }
        for item in source {
            target.push(match direct_child {
                None => *item,
                Some(child) => ProductionItem {
                    direct_child: child,
                    ..*item
                },
            });
        }
        Ok(())
    }

    fn claim_node(&mut self, node: usize, canonical: usize) -> Result<()> {
        if self.owner_by_node.insert(node, canonical).is_some() {
            return Err(invalid(
                "Each DAG node must have exactly one recomputed materializer group",
            ));
        }
        Ok(())
    }

    /// semanticNodeInputs (brep_envelope::inputs): authored order is semantic.
    fn node_inputs(&self, node: usize) -> Result<Vec<usize>> {
        let value = &self.nodes[node];
        let kind = value["kind"]
            .as_str()
            .ok_or_else(|| invalid("Semantic node is missing its kind"))?;
        super::brep_envelope::inputs(value, kind)
    }

    fn node_kind(&self, node: usize) -> Result<&'a str> {
        self.nodes[node]["kind"]
            .as_str()
            .ok_or_else(|| invalid("Semantic node is missing its kind"))
    }

    fn require_node_kind(&self, row: usize, expected: &[&str]) -> Result<usize> {
        let reference = self.rows[row]
            .node
            .ok_or_else(|| invalid("Semantic operation cannot materialize a null node"))?;
        if !expected.contains(&self.node_kind(reference)?) {
            return Err(invalid(
                "Semantic operation cannot materialize this node kind",
            ));
        }
        Ok(reference)
    }

    fn require_output_count(&self, rows: &[usize], expected: usize) -> Result<()> {
        if rows.len() != expected {
            return Err(invalid(
                "Semantic production rule requires an exact output row count",
            ));
        }
        Ok(())
    }

    fn node_space(&self, node: usize) -> Result<&'a str> {
        self.nodes[node]["valueType"]["space"]
            .as_str()
            .ok_or_else(|| invalid("Semantic node value type is incomplete"))
    }

    /// JSON.stringify(matrix) / String(cut) / String(distance) common-parameter
    /// strings from the host map rules.
    fn map_parameters(&self, node: usize, kind: &str) -> Result<String> {
        Ok(match kind {
            "transform" => {
                let mut out = String::new();
                super::brep_identity::stable_json(&self.nodes[node]["matrix"], &mut out);
                out
            }
            "projection" => match &self.nodes[node]["cut"] {
                Value::Bool(value) => value.to_string(),
                Value::Number(number) => super::brep_identity::js_number(number),
                _ => return Err(invalid("Semantic projection node is missing its cut")),
            },
            "offset" => match &self.nodes[node]["distance"] {
                Value::Number(number) => super::brep_identity::js_number(number),
                _ => return Err(invalid("Semantic offset node is missing its distance")),
            },
            _ => return Err(invalid("Unexpected semantic map node kind")),
        })
    }

    fn boolean_union_inputs(&self, node: usize) -> Result<Option<Vec<usize>>> {
        if self.node_kind(node)? != "boolean"
            || self.nodes[node]["operation"].as_str() != Some("union")
        {
            return Ok(None);
        }
        Ok(Some(self.node_inputs(node)?))
    }

    fn evaluate(&mut self, canonical: usize) -> Result<()> {
        let operations: &'a [Value] = self.operations;
        let group_rows = self
            .rows_by_canonical
            .get(&canonical)
            .ok_or_else(|| invalid("Semantic occurrence group reference is invalid"))?
            .clone();
        let output_rows: Vec<usize> = group_rows
            .iter()
            .copied()
            .filter(|row| self.rows[*row].node.is_some())
            .collect();
        let operation = &operations[self.rows[canonical].operation];
        let rule = production_rule(operation)?;
        let category = operation_category(operation)?;
        let name = operation_name(operation)?;
        let is_compiler_frontier_frame = category == "control"
            && ["$body", "$then", "$else", "$expansion"].contains(&name);
        let mut frontier: Vec<ProductionItem> = Vec::new();
        let mut has_body = false;
        let mut schedule: Vec<ScheduleEvent> = Vec::new();
        for child in self.children.get(&canonical).cloned().unwrap_or_default() {
            let visible = self
                .production_by_canonical
                .get(&child)
                .ok_or_else(|| {
                    invalid("Runtime occurrence groups must follow parent-before-child order")
                })?
                .terminal_items
                .clone();
            schedule.push(ScheduleEvent::Group(child));
            if is_compiler_frontier_frame {
                let mut bucket: Vec<ProductionItem> = Vec::new();
                self.append_frontier(&mut bucket, &visible, Some(child))?;
                self.append_frontier(&mut frontier, &bucket, None)?;
            } else {
                self.append_frontier(&mut frontier, &visible, None)?;
            }
            let child_operation = &operations[self.rows[child].operation];
            if operation_category(child_operation)? == "control"
                && operation_name(child_operation)? == "$body"
            {
                if has_body {
                    return Err(invalid(
                        "Semantic operation has more than one compiler-owned body frontier",
                    ));
                }
                has_body = true;
            }
        }
        let mut frontier_nodes: HashSet<usize> = HashSet::new();
        for item in &frontier {
            if !frontier_nodes.insert(item.node) {
                return Err(invalid(
                    "Cross-branch or duplicate frontier node reuse is forbidden in v1",
                ));
            }
        }
        let mut output_items: Vec<ProductionItem> = Vec::new();
        let own = |replay: &mut Self,
                   schedule: &mut Vec<ScheduleEvent>,
                   output_items: &mut Vec<ProductionItem>,
                   row: usize,
                   node: usize|
         -> Result<()> {
            replay.claim_node(node, canonical)?;
            schedule.push(ScheduleEvent::Node(node));
            output_items.push(ProductionItem {
                node,
                row,
                identity_owner: row,
                group: canonical,
                direct_child: canonical,
            });
            Ok(())
        };
        match rule {
            ProductionRule::Transparent => {
                self.require_output_count(&output_rows, 0)?;
            }
            ProductionRule::Primitive => {
                if !frontier.is_empty() {
                    return Err(invalid(
                        "Primitive production frontier must be empty",
                    ));
                }
                self.require_output_count(&output_rows, 1)?;
                let expected: &[&str] = match name {
                    "cube" => &["box"],
                    "sphere" => &["sphere-analytic", "sphere-polygonal"],
                    "cylinder" => &["cylinder-analytic", "cylinder-polygonal"],
                    "polyhedron" => &["polyhedron"],
                    "square" => &["rectangle"],
                    "circle" => &["circle-analytic", "circle-polygonal"],
                    "polygon" => &["polygon"],
                    _ => return Err(invalid("Unknown semantic primitive operation")),
                };
                let produced = self.require_node_kind(output_rows[0], expected)?;
                own(self, &mut schedule, &mut output_items, output_rows[0], produced)?;
            }
            ProductionRule::TransformMap
            | ProductionRule::ProjectionMap
            | ProductionRule::OffsetMap => {
                self.require_output_count(&output_rows, frontier.len())?;
                let mut common_parameters: HashMap<&str, String> = HashMap::new();
                for (index, row) in output_rows.iter().copied().enumerate() {
                    let expected: &[&str] = match rule {
                        ProductionRule::TransformMap => &["transform"],
                        ProductionRule::ProjectionMap => &["projection"],
                        _ => &["offset"],
                    };
                    let produced = self.require_node_kind(row, expected)?;
                    let kind = self.node_kind(produced)?;
                    let input = self.node_inputs(produced)?;
                    if input.len() != 1 || input[0] != frontier[index].node {
                        return Err(invalid(
                            "Map output must consume its matching ordered frontier item",
                        ));
                    }
                    let parameters = self.map_parameters(produced, kind)?;
                    let bucket = if rule == ProductionRule::TransformMap && name == "rotate" {
                        self.node_space(frontier[index].node)?
                    } else {
                        "all"
                    };
                    if let Some(prior) = common_parameters.get(bucket) {
                        if *prior != parameters {
                            return Err(invalid(
                                "One map occurrence must use field-equal common parameters",
                            ));
                        }
                    } else {
                        common_parameters.insert(bucket, parameters);
                    }
                    if rule == ProductionRule::TransformMap {
                        self.claim_node(produced, canonical)?;
                        schedule.push(ScheduleEvent::Node(produced));
                        output_items.push(ProductionItem {
                            node: produced,
                            row,
                            identity_owner: frontier[index].identity_owner,
                            group: canonical,
                            direct_child: canonical,
                        });
                    } else {
                        own(self, &mut schedule, &mut output_items, row, produced)?;
                    }
                }
            }
            ProductionRule::PreservingAlias => {
                self.require_output_count(&output_rows, frontier.len())?;
                for (index, row) in output_rows.iter().copied().enumerate() {
                    if self.rows[row].node != Some(frontier[index].node) {
                        return Err(invalid(
                            "Alias output must equal its matching ordered frontier node",
                        ));
                    }
                    output_items.push(ProductionItem {
                        node: frontier[index].node,
                        row,
                        identity_owner: frontier[index].identity_owner,
                        group: canonical,
                        direct_child: canonical,
                    });
                }
            }
            ProductionRule::Boolean | ProductionRule::Hull => {
                if frontier.is_empty() {
                    self.require_output_count(&output_rows, 0)?;
                } else if frontier.len() == 1 {
                    self.require_output_count(&output_rows, 1)?;
                    if self.rows[output_rows[0]].node != Some(frontier[0].node) {
                        return Err(invalid(
                            "One-item reduction must alias its sole frontier node",
                        ));
                    }
                    if rule == ProductionRule::Hull {
                        output_items.push(ProductionItem {
                            node: frontier[0].node,
                            row: output_rows[0],
                            identity_owner: output_rows[0],
                            group: canonical,
                            direct_child: canonical,
                        });
                    } else {
                        output_items.push(ProductionItem {
                            node: frontier[0].node,
                            row: output_rows[0],
                            identity_owner: frontier[0].identity_owner,
                            group: canonical,
                            direct_child: canonical,
                        });
                    }
                } else {
                    self.require_output_count(&output_rows, 1)?;
                    let expected: &[&str] = if rule == ProductionRule::Hull {
                        &["hull"]
                    } else {
                        &["boolean"]
                    };
                    let produced = self.require_node_kind(output_rows[0], expected)?;
                    if self.node_kind(produced)? == "boolean"
                        && self.nodes[produced]["operation"].as_str() != Some(name)
                    {
                        return Err(invalid(
                            "Boolean node operation does not match its static operation",
                        ));
                    }
                    let inputs = self.node_inputs(produced)?;
                    let expected_inputs: Vec<usize> =
                        frontier.iter().map(|item| item.node).collect();
                    if inputs != expected_inputs {
                        return Err(invalid(
                            "N-ary reduction inputs must exactly equal the ordered occurrence frontier",
                        ));
                    }
                    own(self, &mut schedule, &mut output_items, output_rows[0], produced)?;
                }
            }
            ProductionRule::Difference => {
                let direct_children =
                    self.children.get(&canonical).cloned().unwrap_or_default();
                let bodies: Vec<usize> = direct_children
                    .iter()
                    .copied()
                    .filter(|child| {
                        let child_operation = &operations[self.rows[*child].operation];
                        let path = operation_path(child_operation).unwrap_or(&[]);
                        let last = path.last();
                        operation_parent(child_operation) == Some(self.rows[canonical].operation)
                            && operation_category(child_operation) == Ok("control")
                            && operation_name(child_operation) == Ok("$body")
                            && last.is_some_and(|segment| {
                                segment["kind"].as_str() == Some("body")
                                    && segment["name"].as_str() == Some("$body")
                            })
                    })
                    .collect();
                if bodies.len() > 1
                    || (bodies.len() == 1 && direct_children.len() != 1)
                    || (bodies.is_empty() && !direct_children.is_empty())
                {
                    return Err(invalid(
                        "Difference runtime frontier requires at most one exact compiler $body child and no direct authored children",
                    ));
                }
                let body = bodies.first().copied();
                let body_operation = body.map(|row| self.rows[row].operation);
                let authored_bucket_operations: Vec<usize> = match body_operation {
                    None => Vec::new(),
                    Some(body_operation) => self
                        .operations
                        .iter()
                        .filter(|candidate| operation_parent(candidate) == Some(body_operation))
                        .map(|candidate| {
                            candidate["id"]
                                .as_u64()
                                .and_then(|value| usize::try_from(value).ok())
                                .ok_or_else(|| invalid("Expected a semantic operation ID"))
                        })
                        .collect::<Result<Vec<_>>>()?,
                };
                let runtime_bucket_roots: Vec<usize> = match body {
                    None => Vec::new(),
                    Some(body) => self.children.get(&body).cloned().unwrap_or_default(),
                };
                let authored_bucket_set: HashSet<usize> =
                    authored_bucket_operations.iter().copied().collect();
                if runtime_bucket_roots
                    .iter()
                    .any(|child| !authored_bucket_set.contains(&self.rows[*child].operation))
                {
                    return Err(invalid(
                        "Difference runtime activation is not owned by a direct authored body statement",
                    ));
                }
                let mut buckets: Vec<Vec<ProductionItem>> = Vec::new();
                for bucket_operation in &authored_bucket_operations {
                    let mut bucket: Vec<ProductionItem> = Vec::new();
                    for child in &runtime_bucket_roots {
                        if self.rows[*child].operation != *bucket_operation {
                            continue;
                        }
                        let child_production = self
                            .production_by_canonical
                            .get(child)
                            .ok_or_else(|| {
                                invalid(
                                    "Runtime occurrence groups must follow parent-before-child order",
                                )
                            })?;
                        let visible: Vec<ProductionItem> = if !child_production
                            .output_items
                            .is_empty()
                        {
                            child_production.output_items.clone()
                        } else if child_production.transparent {
                            child_production.frontier.clone()
                        } else {
                            Vec::new()
                        };
                        self.append_frontier(&mut bucket, &visible, Some(*child))?;
                    }
                    buckets.push(bucket);
                }
                let bucket_frontier_rows: Vec<usize> = buckets
                    .iter()
                    .flat_map(|bucket| bucket.iter().map(|item| item.row))
                    .collect();
                let frontier_rows: Vec<usize> = frontier.iter().map(|item| item.row).collect();
                if bucket_frontier_rows != frontier_rows {
                    return Err(invalid(
                        "Difference frontier buckets must exactly partition its ordered frontier",
                    ));
                }
                let base: Vec<ProductionItem> = buckets.first().cloned().unwrap_or_default();
                let mut cutters: Vec<ProductionItem> = Vec::new();
                for bucket in buckets.iter().skip(1) {
                    self.append_frontier(&mut cutters, bucket, None)?;
                }
                let bucket_schedules: Vec<Vec<ScheduleEvent>> = authored_bucket_operations
                    .iter()
                    .map(|bucket_operation| {
                        runtime_bucket_roots
                            .iter()
                            .copied()
                            .filter(|child| self.rows[*child].operation == *bucket_operation)
                            .map(ScheduleEvent::Group)
                            .collect()
                    })
                    .collect();
                let mut base_reducer: Option<usize> = None;
                let mut cutter_reducer: Option<usize> = None;
                let mut difference_root: Option<usize> = None;
                // Discarded cutter effects are refused before this replay on
                // the brep-1 path, so the host `effect !== undefined` branch
                // is unreachable here.
                if base.is_empty() {
                    self.require_output_count(&output_rows, 0)?;
                    if !cutters.is_empty() {
                        return Err(invalid(
                            "Empty-base difference must retain its eager cutter effect",
                        ));
                    }
                } else if cutters.is_empty() {
                    self.require_output_count(&output_rows, 1)?;
                    let output_node = self.rows[output_rows[0]]
                        .node
                        .ok_or_else(|| invalid("Semantic difference output row must carry a node"))?;
                    if base.len() == 1 {
                        if output_node != base[0].node {
                            return Err(invalid(
                                "Single-item difference base must alias its source node",
                            ));
                        }
                        output_items.push(ProductionItem {
                            node: base[0].node,
                            row: output_rows[0],
                            identity_owner: base[0].identity_owner,
                            group: canonical,
                            direct_child: canonical,
                        });
                    } else {
                        let reducer_inputs = self
                            .boolean_union_inputs(output_node)?
                            .ok_or_else(|| {
                                invalid(
                                    "Multi-item difference base requires one canonical union reducer",
                                )
                            })?;
                        if reducer_inputs != base.iter().map(|item| item.node).collect::<Vec<_>>()
                        {
                            return Err(invalid(
                                "Multi-item difference base requires one canonical union reducer",
                            ));
                        }
                        base_reducer = Some(output_node);
                        own(
                            self,
                            &mut schedule,
                            &mut output_items,
                            output_rows[0],
                            output_node,
                        )?;
                    }
                } else {
                    self.require_output_count(&output_rows, 1)?;
                    let produced = self.require_node_kind(output_rows[0], &["boolean"])?;
                    let produced_inputs = self.node_inputs(produced)?;
                    if self.nodes[produced]["operation"].as_str() != Some("difference")
                        || produced_inputs.len() != 2
                    {
                        return Err(invalid(
                            "Difference root must consume exactly reduced base and cutters",
                        ));
                    }
                    let mut require_reduction = |reference: usize,
                                                 items: &[ProductionItem],
                                                 base_side: bool|
                     -> Result<()> {
                        if items.len() == 1 {
                            if reference != items[0].node {
                                return Err(invalid(
                                    "Difference reduction must preserve its sole frontier node",
                                ));
                            }
                            return Ok(());
                        }
                        let reducer_inputs = self
                            .boolean_union_inputs(reference)?
                            .ok_or_else(|| {
                                invalid("Difference requires one canonical ordered union reducer")
                            })?;
                        if reducer_inputs != items.iter().map(|item| item.node).collect::<Vec<_>>()
                        {
                            return Err(invalid(
                                "Difference requires one canonical ordered union reducer",
                            ));
                        }
                        self.claim_node(reference, canonical)?;
                        if base_side {
                            base_reducer = Some(reference);
                        } else {
                            cutter_reducer = Some(reference);
                        }
                        Ok(())
                    };
                    require_reduction(produced_inputs[0], &base, true)?;
                    require_reduction(produced_inputs[1], &cutters, false)?;
                    difference_root = Some(produced);
                    own(
                        self,
                        &mut schedule,
                        &mut output_items,
                        output_rows[0],
                        produced,
                    )?;
                }
                // The compiler-owned difference reductions interleave with
                // child evaluation: rebuild the schedule from authored
                // statement buckets instead of generic child postorder.
                schedule.clear();
                if let Some(first) = bucket_schedules.first() {
                    schedule.extend(first.iter().copied());
                }
                if let Some(reducer) = base_reducer {
                    schedule.push(ScheduleEvent::Node(reducer));
                }
                for bucket in bucket_schedules.iter().skip(1) {
                    schedule.extend(bucket.iter().copied());
                }
                if let Some(reducer) = cutter_reducer {
                    schedule.push(ScheduleEvent::Node(reducer));
                }
                if let Some(root) = difference_root {
                    schedule.push(ScheduleEvent::Node(root));
                }
            }
            ProductionRule::LinearExtrude | ProductionRule::RotateExtrude => {
                if frontier.is_empty() {
                    self.require_output_count(&output_rows, 0)?;
                } else {
                    self.require_output_count(&output_rows, 1)?;
                    let expected: &[&str] = if rule == ProductionRule::LinearExtrude {
                        &["linear-extrude"]
                    } else {
                        &["rotate-extrude-analytic", "rotate-extrude-polygonal"]
                    };
                    let produced = self.require_node_kind(output_rows[0], expected)?;
                    let direct_input = self
                        .node_inputs(produced)?
                        .first()
                        .copied()
                        .ok_or_else(|| invalid("Semantic extrusion node is missing its input"))?;
                    if frontier.len() == 1 {
                        if direct_input != frontier[0].node {
                            return Err(invalid(
                                "Extrusion must consume its sole profile frontier node",
                            ));
                        }
                    } else {
                        let reducer_inputs = self
                            .boolean_union_inputs(direct_input)?
                            .ok_or_else(|| {
                                invalid(
                                    "Multi-profile extrusion requires one canonical internal union reduction",
                                )
                            })?;
                        if reducer_inputs
                            != frontier.iter().map(|item| item.node).collect::<Vec<_>>()
                        {
                            return Err(invalid(
                                "Multi-profile extrusion requires one canonical internal union reduction",
                            ));
                        }
                        self.claim_node(direct_input, canonical)?;
                        schedule.push(ScheduleEvent::Node(direct_input));
                    }
                    own(
                        self,
                        &mut schedule,
                        &mut output_items,
                        output_rows[0],
                        produced,
                    )?;
                }
            }
        }
        let terminal_items = if !output_items.is_empty() {
            output_items.clone()
        } else if rule == ProductionRule::Transparent {
            frontier.clone()
        } else {
            Vec::new()
        };
        self.production_by_canonical.insert(
            canonical,
            ProductionGroup {
                output_rows,
                output_items,
                frontier,
                terminal_items,
                schedule,
                transparent: rule == ProductionRule::Transparent,
            },
        );
        Ok(())
    }
}

/// semanticResultItems: the result output references in authored order.
fn result_items(result: &Value) -> Result<Vec<Value>> {
    Ok(match result["tag"].as_str() {
        Some("empty") => Vec::new(),
        Some("single") => vec![result["item"].clone()],
        Some("multi") => result["items"]
            .as_array()
            .ok_or_else(|| invalid("Expected semantic result items"))?
            .clone(),
        _ => return Err(invalid("Unknown semantic result tag")),
    })
}

fn result_reference(item: &Value, key: &str) -> Result<usize> {
    item[key]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| invalid("Semantic result reference is invalid"))
}

/// validateOccurrenceProduction (brep-1 reachable subset; see module docs).
/// Runs against the admitted operations, occurrences and nodes as
/// transported, after identity/provenance admission and before any node
/// executes.
pub fn validate_occurrence_production(
    operations: &[Value],
    occurrences: &[Value],
    nodes: &[Value],
    result: &Value,
    execution: &Value,
) -> Result<()> {
    if !execution["discardedEffects"]
        .as_array()
        .is_some_and(Vec::is_empty)
    {
        return Err(invalid(
            "Interrupted legacy cutter effects remain host-side",
        ));
    }
    if !execution["terminal"].is_null() {
        return Err(invalid(
            "Interrupted terminal production replay remains host-side",
        ));
    }
    let mut rows: Vec<RowFacts> = Vec::with_capacity(occurrences.len());
    let mut indexes_by_id: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, occurrence) in occurrences.iter().enumerate() {
        let operation = occurrence["operation"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < operations.len())
            .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
        let parent = if occurrence["parent"].is_null() {
            None
        } else {
            Some(
                occurrence["parent"]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .filter(|value| *value < index)
                    .ok_or_else(|| invalid("Semantic occurrence reference is invalid"))?,
            )
        };
        let node = if occurrence["node"].is_null() {
            None
        } else {
            Some(
                occurrence["node"]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .filter(|value| *value < nodes.len())
                    .ok_or_else(|| invalid("Semantic occurrence node reference is invalid"))?,
            )
        };
        let output_ordinal = occurrence["outputOrdinal"].as_u64();
        rows.push(RowFacts {
            operation,
            parent,
            node,
            output_ordinal,
            has_scene_entity: !occurrence["sceneEntityId"].is_null(),
        });
        let occurrence_id = occurrence["occurrenceId"]
            .as_str()
            .ok_or_else(|| invalid("Expected a semantic occurrence ID"))?;
        indexes_by_id
            .entry(occurrence_id.to_owned())
            .or_default()
            .push(index);
    }
    let mut canonical_by_row = vec![usize::MAX; occurrences.len()];
    let mut rows_by_canonical: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut canonical_groups: Vec<usize> = Vec::new();
    for indexes in indexes_by_id.values() {
        let canonical = indexes[0];
        canonical_groups.push(canonical);
        rows_by_canonical.insert(canonical, indexes.clone());
        for index in indexes {
            canonical_by_row[*index] = canonical;
        }
        // A logical occurrence is exactly one zero/frame row or dense output
        // rows whose ordinals equal their position inside the group.
        let zero = indexes.len() == 1
            && rows[indexes[0]].node.is_none()
            && rows[indexes[0]].output_ordinal.is_none()
            && !rows[indexes[0]].has_scene_entity;
        let output = indexes.iter().enumerate().all(|(ordinal, index)| {
            rows[*index].node.is_some()
                && rows[*index].output_ordinal == Some(ordinal as u64)
                && rows[*index].has_scene_entity
        });
        if !zero && !output {
            return Err(invalid(
                "A logical occurrence must be exactly one zero/frame row or dense output rows",
            ));
        }
    }
    canonical_groups.sort_unstable();
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for canonical in &canonical_groups {
        let Some(parent) = rows[*canonical].parent else {
            continue;
        };
        children
            .entry(canonical_by_row[parent])
            .or_default()
            .push(*canonical);
    }
    let mut replay = Replay {
        operations,
        nodes,
        rows: &rows,
        rows_by_canonical,
        children,
        production_by_canonical: HashMap::new(),
        owner_by_node: HashMap::new(),
        expanded_frontier_items: 0,
    };
    for canonical in canonical_groups.iter().rev() {
        replay.evaluate(*canonical)?;
    }
    if replay.owner_by_node.len() != nodes.len() {
        return Err(invalid(
            "Every reachable DAG node needs exactly one recomputed materializer group",
        ));
    }

    // The authenticated schedule: root groups in authored order, expanding
    // each group's recorded schedule events depth-first.
    let mut schedule_stack: Vec<ScheduleEvent> = Vec::new();
    for canonical in canonical_groups.iter().rev() {
        if rows[*canonical].parent.is_none() {
            schedule_stack.push(ScheduleEvent::Group(*canonical));
        }
    }
    let mut authenticated_schedule: Vec<usize> = Vec::new();
    let mut schedule_events = 0usize;
    while let Some(event) = schedule_stack.pop() {
        schedule_events += 1;
        if schedule_events > SCHEDULE_EVENT_LIMIT {
            return Err(invalid(
                "Occurrence-production schedule exceeds its bounded replay budget",
            ));
        }
        match event {
            ScheduleEvent::Node(node) => {
                authenticated_schedule.push(node);
                if authenticated_schedule.len() > nodes.len() {
                    return Err(invalid(
                        "Occurrence-production schedule creates a node more than once",
                    ));
                }
            }
            ScheduleEvent::Group(group) => {
                let production = replay
                    .production_by_canonical
                    .get(&group)
                    .ok_or_else(|| {
                        invalid("Occurrence-production schedule references an absent group")
                    })?;
                for child in production.schedule.iter().rev() {
                    schedule_stack.push(*child);
                }
            }
        }
    }
    if authenticated_schedule.len() != nodes.len()
        || authenticated_schedule
            .iter()
            .enumerate()
            .any(|(index, node)| *node != index)
    {
        return Err(invalid(
            "Node IDs must equal the exact occurrence-production schedule",
        ));
    }

    // The synthetic program frontier must exactly equal the transported
    // result, row by row (terminal prefixes are refused above).
    let mut program_frontier: Vec<ProductionItem> = Vec::new();
    for canonical in &canonical_groups {
        if rows[*canonical].parent.is_some() {
            continue;
        }
        let terminal_items = replay
            .production_by_canonical
            .get(canonical)
            .ok_or_else(|| invalid("Occurrence-production group reference is invalid"))?
            .terminal_items
            .clone();
        replay.append_frontier(&mut program_frontier, &terminal_items, None)?;
    }
    let root_outputs = result_items(result)?;
    if root_outputs.len() != program_frontier.len() {
        return Err(invalid(
            "Root result must exactly equal the synthetic program frontier",
        ));
    }
    let mut root_groups: Vec<usize> = Vec::new();
    for (output, slot) in root_outputs.iter().zip(program_frontier.iter()) {
        if slot.row != result_reference(output, "producerOccurrence")?
            || slot.node != result_reference(output, "node")?
        {
            return Err(invalid(
                "Root producer must be an exact output row derived by its occurrence production rule",
            ));
        }
        if slot.identity_owner != result_reference(output, "identityOccurrence")? {
            return Err(invalid(
                "Root identity occurrence does not match the recursively derived production owner",
            ));
        }
        root_groups.push(slot.group);
    }
    let mut reachable_output_groups: HashSet<usize> = HashSet::new();
    while let Some(canonical) = root_groups.pop() {
        if !reachable_output_groups.insert(canonical) {
            continue;
        }
        let frontier = replay
            .production_by_canonical
            .get(&canonical)
            .ok_or_else(|| invalid("Occurrence-production group reference is invalid"))?
            .frontier
            .clone();
        for item in frontier {
            root_groups.push(item.group);
        }
    }
    for canonical in &canonical_groups {
        let production = replay
            .production_by_canonical
            .get(canonical)
            .ok_or_else(|| invalid("Occurrence-production group reference is invalid"))?;
        if !production.output_rows.is_empty() && !reachable_output_groups.contains(canonical) {
            return Err(invalid(
                "Node-bearing occurrence group is not consumed by a root production path",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
