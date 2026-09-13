//! Native identity-digest admission for semantic operations and occurrences.
//!
//! Byte-exact Rust port of the host identity derivers in
//! src/core/semanticProgram.ts (identityDigest + stableJson) and of the
//! identity subsets of validateOperations / validateOccurrences in
//! src/services/semanticProgramValidator.ts, including the staticParent
//! runtime ancestor-chain walk, moduleActivationAnchor (l.600) and
//! isChildrenExpansionContinuation (l.669). Out of scope (host-side):
//! provenance/tessellationIntents/diagnostics (brep_provenance.rs) and the
//! occurrence-production replay (brep_production.rs).
use super::{Result, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use value_codec::Number;

fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected a semantic identity object"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid("Semantic identity fields do not match the schema"));
    }
    Ok(())
}

fn bounded_integer(value: &Value, maximum: u64) -> Result<u64> {
    let number = value
        .as_u64()
        .ok_or_else(|| invalid("Expected a non-negative semantic integer"))?;
    if number > maximum {
        return Err(invalid("Semantic numeric limit exceeded"));
    }
    Ok(number)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn bounded_string(value: &Value, maximum: usize) -> Result<&str> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic identity string"))?;
    if utf16_len(text) > maximum {
        return Err(invalid("Semantic identity string limit exceeded"));
    }
    Ok(text)
}

// ---- Canonical JSON (byte-exact port of stableJson + JSON.stringify) ----

/// ECMAScript Number::toString for a finite binary64 (JSON.stringify number form).
fn js_f64(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned(); // JSON.stringify(-0) is "0"
    }
    let negative = value.is_sign_negative();
    let absolute = value.abs();
    // Rust LowerExp emits the shortest round-trip decimal, matching the digits
    // ECMAScript selects; only the exponent/decimal-point layout differs.
    let scientific = format!("{absolute:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp always carries an exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digit_count = digits.len() as i32;
    let point = exponent + 1; // n in the spec: decimal point position
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if point > 0 && point <= 21 {
        if digit_count <= point {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', (point - digit_count) as usize));
        } else {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        }
    } else if point > -6 && point <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-point) as usize));
        out.push_str(&digits);
    } else {
        out.push_str(&digits[..1]);
        if digit_count > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let exponent = point - 1;
        out.push(if exponent >= 0 { '+' } else { '-' });
        out.push_str(&exponent.unsigned_abs().to_string());
    }
    out
}

pub(crate) fn js_number(number: &Number) -> String {
    match number {
        // Integers within the exactly-representable range print as digits; a
        // wider wire integer could only have reached the host as a rounded
        // binary64, so it is formatted through the float path for parity.
        Number::Unsigned(v) if *v <= (1u64 << 53) => v.to_string(),
        Number::Signed(v) if v.unsigned_abs() <= (1u64 << 53) => v.to_string(),
        Number::Unsigned(v) => js_f64(*v as f64),
        Number::Signed(v) => js_f64(*v as f64),
        Number::Float(v) => js_f64(*v),
    }
}

/// JSON.stringify string escaping (controls, quote and backslash only).
fn js_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// stableJson from src/core/semanticProgram.ts: keys sorted by raw UTF-8 bytes.
/// BTreeMap<String, _> iteration order is exactly that byte order.
pub(crate) fn stable_json(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(v) => out.push_str(&js_number(v)),
        Value::String(v) => js_string(v, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                stable_json(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                js_string(key, out);
                out.push(':');
                stable_json(item, out);
            }
            out.push('}');
        }
    }
}

/// identityDigest: SHA-256 over length-prefixed domain and canonical payload.
pub fn identity_digest(domain: &str, payload: &Value) -> String {
    let mut canonical = String::new();
    stable_json(payload, &mut canonical);
    let mut preimage =
        Vec::with_capacity(8 + domain.len() + canonical.len());
    preimage.extend_from_slice(&(domain.len() as u32).to_be_bytes());
    preimage.extend_from_slice(domain.as_bytes());
    preimage.extend_from_slice(&(canonical.len() as u32).to_be_bytes());
    preimage.extend_from_slice(canonical.as_bytes());
    let digest = Sha256::digest(&preimage);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// deriveSemanticOperationId.
pub fn derive_operation_id(path: &[Value]) -> String {
    format!(
        "opv1:{}",
        identity_digest("semantic-operation-v1", &Value::Array(path.to_vec()))
    )
}

/// deriveSemanticAmbiguityGroupId.
pub fn derive_ambiguity_group_id(parent_path: &[Value], category: &str, name: &str) -> String {
    let payload = value_codec::json!({
        "category": category,
        "name": name,
        "parentPath": parent_path,
    });
    format!(
        "ambv1:{}",
        identity_digest("semantic-ambiguity-group-v1", &payload)
    )
}

/// deriveSemanticOccurrenceId.
pub fn derive_occurrence_id(
    parent_occurrence_id: Option<&str>,
    static_parent_occurrence_id: Option<&str>,
    operation_id: &str,
    dynamic_slots: &[Value],
) -> String {
    let id_or_null = |id: Option<&str>| match id {
        Some(id) => Value::String(id.to_owned()),
        None => Value::Null,
    };
    let payload = value_codec::json!({
        "dynamicSlots": dynamic_slots,
        "operationId": operation_id,
        "parentOccurrenceId": id_or_null(parent_occurrence_id),
        "staticParentOccurrenceId": id_or_null(static_parent_occurrence_id),
    });
    format!(
        "occv1:{}",
        identity_digest("semantic-occurrence-v1", &payload)
    )
}

/// deriveSemanticSceneEntityId.
pub fn derive_scene_entity_id(occurrence_id: &str, output_ordinal: u64) -> String {
    let payload = value_codec::json!({
        "occurrenceId": occurrence_id,
        "outputOrdinal": output_ordinal,
    });
    format!(
        "entity:v2:{}",
        identity_digest("semantic-scene-entity-v1", &payload)
    )
}

// ---- Identity values and dynamic slots ----

/// identityValueKey from the host validator.
pub(crate) fn identity_value_key(value: &Value) -> Result<String> {
    let tag = value["tag"]
        .as_str()
        .ok_or_else(|| invalid("Expected a typed semantic identity value"))?;
    Ok(match tag {
        "undefined" => "u".to_owned(),
        "null" => "n".to_owned(),
        "boolean" => {
            if value["value"].as_bool().ok_or_else(|| invalid("Expected a boolean identity value"))?
            {
                "b1".to_owned()
            } else {
                "b0".to_owned()
            }
        }
        "number" => {
            let number = match value.get("value") {
                Some(Value::Number(number)) => number,
                _ => return Err(invalid("Expected a numeric identity value")),
            };
            format!("d{}", js_number(number))
        }
        "string" => {
            let text = value["value"]
                .as_str()
                .ok_or_else(|| invalid("Expected a string identity value"))?;
            format!("s{}:{}", utf16_len(text), text)
        }
        "vector" => {
            let items = value["items"]
                .as_array()
                .ok_or_else(|| invalid("Expected a vector identity value"))?;
            let keys = items
                .iter()
                .map(identity_value_key)
                .collect::<Result<Vec<_>>>()?;
            format!("v{}[{}]", items.len(), keys.join(","))
        }
        _ => return Err(invalid("Unknown semantic identity value tag")),
    })
}

/// validateIdentityValue: exact keys, bounded strings/items, finite numbers.
pub(crate) fn validate_identity_value(value: &Value, depth: usize) -> Result<()> {
    if depth > 32 {
        return Err(invalid("Semantic identity value nesting limit exceeded"));
    }
    let tag = value["tag"]
        .as_str()
        .ok_or_else(|| invalid("Expected a typed semantic identity value"))?;
    match tag {
        "undefined" | "null" => exact(value, &["tag"]),
        "boolean" => {
            exact(value, &["tag", "value"])?;
            if value["value"].as_bool().is_none() {
                return Err(invalid("Expected a boolean identity value"));
            }
            Ok(())
        }
        "number" => {
            exact(value, &["tag", "value"])?;
            match value.get("value") {
                Some(Value::Number(Number::Float(v)))
                    if *v == 0.0 && v.is_sign_negative() =>
                {
                    Err(invalid("Semantic identity number -0 is not canonical"))
                }
                Some(Value::Number(_)) => Ok(()),
                _ => Err(invalid("Expected a finite semantic identity number")),
            }
        }
        "string" => {
            exact(value, &["tag", "value"])?;
            bounded_string(&value["value"], 4_096)?;
            Ok(())
        }
        "vector" => {
            exact(value, &["items", "tag"])?;
            let items = value["items"]
                .as_array()
                .ok_or_else(|| invalid("Expected a vector identity value"))?;
            if items.len() > 100_000 {
                return Err(invalid("Semantic identity vector limit exceeded"));
            }
            for item in items {
                validate_identity_value(item, depth + 1)?;
            }
            Ok(())
        }
        _ => Err(invalid("Unknown semantic identity value tag")),
    }
}

/// identityValueKey applied to a dynamic slot's typed value (the host
/// compares `slot.value`, never the whole slot object).
fn slot_value_key(slot: &Value) -> Result<String> {
    identity_value_key(&slot["value"])
}

/// sameDynamicSlots: equal names, duplicate ordinals and identity value keys.
fn same_dynamic_slots(left: &[Value], right: &[Value]) -> Result<bool> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        if left["name"].as_str() != right["name"].as_str()
            || left["duplicateOrdinal"].as_u64() != right["duplicateOrdinal"].as_u64()
            || slot_value_key(left)? != slot_value_key(right)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// sameDynamicBindings: equal names and identity value keys; duplicate
/// ordinals are ignored (call and definition bind the same authored values).
fn same_dynamic_bindings(left: &[Value], right: &[Value]) -> Result<bool> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        if left["name"].as_str() != right["name"].as_str()
            || slot_value_key(left)? != slot_value_key(right)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_dynamic_slots(value: &Value) -> Result<Vec<Value>> {
    let slots = value
        .as_array()
        .ok_or_else(|| invalid("Expected semantic dynamic slots"))?;
    if slots.len() > 1_000 {
        return Err(invalid("Semantic dynamic slot limit exceeded"));
    }
    let mut names = HashSet::new();
    for slot in slots {
        exact(slot, &["duplicateOrdinal", "name", "value"])?;
        let name = bounded_string(&slot["name"], 128)?;
        if !names.insert(name.to_owned()) {
            return Err(invalid("Semantic dynamic slot names must be unique"));
        }
        validate_identity_value(&slot["value"], 0)?;
        bounded_integer(&slot["duplicateOrdinal"], 1_000_000)?;
    }
    Ok(slots.clone())
}

// ---- Operations admission ----

const OPERATION_CATEGORIES: [&str; 7] = [
    "geometry",
    "transform",
    "boolean",
    "control",
    "module",
    "assertion",
    "presentation",
];
const PATH_SEGMENT_KINDS: [&str; 5] = ["call", "module", "control", "branch", "body"];

fn validate_structural_path(value: &Value) -> Result<Vec<Value>> {
    let segments = value
        .as_array()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    if segments.is_empty() || segments.len() > 512 {
        return Err(invalid("Semantic structural path length is out of range"));
    }
    for segment in segments {
        exact(segment, &["kind", "name", "ordinal"])?;
        let kind = segment["kind"]
            .as_str()
            .ok_or_else(|| invalid("Expected a structural path segment kind"))?;
        if !PATH_SEGMENT_KINDS.contains(&kind) {
            return Err(invalid("Unknown structural path segment kind"));
        }
        bounded_string(&segment["name"], 256)?;
        bounded_integer(&segment["ordinal"], 1_000_000)?;
    }
    Ok(segments.clone())
}

fn same_path(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left["kind"].as_str() == right["kind"].as_str()
                && left["name"].as_str() == right["name"].as_str()
                && left["ordinal"].as_u64() == right["ordinal"].as_u64()
        })
}

fn structural_path_key(path: &[Value]) -> String {
    path.iter()
        .map(|segment| {
            format!(
                "{}:{}{}:{}{}",
                segment["kind"].as_str().map_or(0, str::len),
                segment["kind"].as_str().unwrap_or_default(),
                segment["name"].as_str().map_or(0, str::len),
                segment["name"].as_str().unwrap_or_default(),
                segment["ordinal"].as_u64().unwrap_or(u64::MAX),
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Identity subset of validateOperations: positional IDs, deterministic
/// parent-before-child preorder, exact path extension, re-derived opv1:
/// digests, dense sibling ordinals and ambiguity-group membership.
pub fn validate_operations(operations: &[Value]) -> Result<()> {
    if operations.len() > 50_000 {
        return Err(invalid("Semantic operation limit exceeded"));
    }
    let mut ambiguity_counts: HashMap<String, usize> = HashMap::new();
    let mut sibling_counts: HashMap<(Option<usize>, String, String), usize> = HashMap::new();
    let mut sibling_ordinals: HashMap<(Option<usize>, String, String), Vec<u64>> = HashMap::new();
    let mut operation_ids: HashSet<String> = HashSet::new();
    let mut structural_paths: HashSet<String> = HashSet::new();
    let mut preorder_stack: Vec<usize> = Vec::new();
    let mut closed: HashSet<usize> = HashSet::new();
    let mut paths: Vec<Vec<Value>> = Vec::with_capacity(operations.len());
    let mut evidence: Vec<&str> = Vec::with_capacity(operations.len());
    for (index, operation) in operations.iter().enumerate() {
        exact(
            operation,
            &[
                "id",
                "operationId",
                "parent",
                "childOrdinal",
                "name",
                "category",
                "structuralPath",
                "identityEvidence",
                "ambiguityGroup",
            ],
        )?;
        if operation["id"].as_u64() != Some(index as u64) {
            return Err(invalid("Semantic operation IDs must equal array positions"));
        }
        let parent = if operation["parent"].is_null() {
            None
        } else {
            let parent = operation["parent"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value < index)
                .ok_or_else(|| invalid("Semantic operation parent reference is invalid"))?;
            Some(parent)
        };
        if index == 0 && parent.is_some() {
            return Err(invalid("The first semantic operation cannot have a parent"));
        }
        while preorder_stack.last().copied().is_some_and(|top| Some(top) != parent) {
            closed.insert(preorder_stack.pop().expect("non-empty stack"));
        }
        if let Some(parent) = parent {
            if closed.contains(&parent) || preorder_stack.last().copied() != Some(parent) {
                return Err(invalid(
                    "Semantic operations must be in deterministic parent-before-child preorder",
                ));
            }
        }
        let child_ordinal = bounded_integer(&operation["childOrdinal"], 1_000_000)?;
        let name = bounded_string(&operation["name"], 256)?;
        let category = operation["category"]
            .as_str()
            .ok_or_else(|| invalid("Expected a semantic operation category"))?;
        if !OPERATION_CATEGORIES.contains(&category) {
            return Err(invalid("Unknown semantic operation category"));
        }
        let path = validate_structural_path(&operation["structuralPath"])?;
        let last = path.last().expect("non-empty structural path");
        if last["name"].as_str() != Some(name)
            || last["ordinal"].as_u64() != Some(child_ordinal)
        {
            return Err(invalid(
                "Semantic operation name and childOrdinal must match the final path segment",
            ));
        }
        match parent {
            None => {
                if path.len() != 1 {
                    return Err(invalid("A root semantic operation path has one segment"));
                }
            }
            Some(parent) => {
                let parent_path = &paths[parent];
                if path.len() != parent_path.len() + 1
                    || !same_path(&path[..path.len() - 1], parent_path)
                {
                    return Err(invalid(
                        "A child structural path must extend its parent path exactly once",
                    ));
                }
            }
        }
        let expected_id = derive_operation_id(&path);
        if operation["operationId"].as_str() != Some(expected_id.as_str()) {
            return Err(invalid(
                "Semantic operation ID does not match its structural path",
            ));
        }
        if !structural_paths.insert(structural_path_key(&path))
            || !operation_ids.insert(expected_id.clone())
        {
            return Err(invalid(
                "Semantic operation paths and operation IDs must be unique",
            ));
        }
        let sibling_key = (parent, category.to_owned(), name.to_owned());
        *sibling_counts.entry(sibling_key.clone()).or_insert(0) += 1;
        sibling_ordinals
            .entry(sibling_key)
            .or_default()
            .push(child_ordinal);
        let identity_evidence = operation["identityEvidence"]
            .as_str()
            .ok_or_else(|| invalid("Unknown semantic identity evidence"))?;
        match identity_evidence {
            "structural-unique" => {
                if !operation["ambiguityGroup"].is_null() {
                    return Err(invalid(
                        "A structural-unique operation cannot claim an ambiguity group",
                    ));
                }
            }
            "same-name-positional" => {
                let empty: Vec<Value> = Vec::new();
                let parent_path = match parent {
                    None => &empty,
                    Some(parent) => &paths[parent],
                };
                let expected_group = derive_ambiguity_group_id(parent_path, category, name);
                if operation["ambiguityGroup"].as_str() != Some(expected_group.as_str()) {
                    return Err(invalid(
                        "Semantic ambiguity group does not match sibling identity",
                    ));
                }
                *ambiguity_counts.entry(expected_group).or_insert(0) += 1;
            }
            _ => return Err(invalid("Unknown semantic identity evidence")),
        }
        evidence.push(identity_evidence);
        paths.push(path);
        preorder_stack.push(index);
    }
    for (index, operation) in operations.iter().enumerate() {
        let parent = operation["parent"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok());
        let key = (
            parent,
            operation["category"].as_str().unwrap_or_default().to_owned(),
            operation["name"].as_str().unwrap_or_default().to_owned(),
        );
        let count = sibling_counts.get(&key).copied().unwrap_or(0);
        if (count > 1) != (evidence[index] == "same-name-positional") {
            return Err(invalid(
                "Same-name siblings require explicit positional ambiguity evidence",
            ));
        }
    }
    if ambiguity_counts.values().any(|count| *count < 2) {
        return Err(invalid("A semantic ambiguity group requires two members"));
    }
    for ordinals in sibling_ordinals.values() {
        if ordinals
            .iter()
            .enumerate()
            .any(|(index, ordinal)| *ordinal != index as u64)
        {
            return Err(invalid(
                "Sibling child ordinals must be unique and dense from zero",
            ));
        }
    }
    Ok(())
}

// ---- Occurrence identity admission ----

fn optional_row_reference(value: &Value, index: usize) -> Result<Option<usize>> {
    if value.is_null() {
        return Ok(None);
    }
    let reference = value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < index)
        .ok_or_else(|| invalid("Semantic occurrence reference is invalid"))?;
    Ok(Some(reference))
}

// ---- $expansion continuation and module-activation anchoring ----
// Exact port of moduleActivationAnchor (semanticProgramValidator.ts l.600)
// and isChildrenExpansionContinuation (l.669) plus the staticParent runtime
// ancestor-chain walk in validateOccurrences.

/// SEMANTIC_PROGRAM_LIMITS.snapshotValues bounds the ancestry/continuation proof.
const PROOF_BUDGET_LIMIT: usize = 1_000_000;

fn consume_proof_budget(budget: &mut usize, steps: usize) -> Result<()> {
    *budget += steps;
    if *budget > PROOF_BUDGET_LIMIT {
        return Err(invalid(
            "Occurrence ancestry/continuation proof exceeds its bounded work budget",
        ));
    }
    Ok(())
}

/// Per-occurrence row facts gathered by the identity loop.
pub(crate) struct OccurrenceRow {
    pub operation: usize,
    pub parent: Option<usize>,
    pub static_parent: Option<usize>,
    pub slots: Vec<Value>,
}

fn operation_path(operation: &Value) -> Result<&[Value]> {
    operation["structuralPath"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("Expected a semantic structural path"))
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

fn segment_field<'a>(segment: &'a Value, key: &str) -> &'a Value {
    &segment[key]
}

/// (kind, name, ordinal) triple of a structural path segment.
fn segment_parts(segment: &Value) -> (Option<&str>, Option<&str>, Option<u64>) {
    (
        segment_field(segment, "kind").as_str(),
        segment_field(segment, "name").as_str(),
        segment_field(segment, "ordinal").as_u64(),
    )
}

/// Strict lexical containment: the candidate path is a proper prefix of the
/// children() path, segment by segment (kind, name, ordinal).
fn path_strictly_contains(candidate: &[Value], children: &[Value]) -> bool {
    candidate.len() < children.len() && same_path(candidate, &children[..candidate.len()])
}

#[derive(Clone, Copy)]
struct ModuleAnchor {
    // All four fields are part of the host anchor record; the continuation
    // check consumes body_occurrence and expansion_operation.
    #[allow(dead_code)]
    call_occurrence: usize,
    body_occurrence: usize,
    #[allow(dead_code)]
    definition_occurrence: usize,
    expansion_operation: usize,
}

/// Shared indexes built after the identity loop (SemanticOccurrenceValidationIndex).
struct OccurrenceIndex<'a> {
    operations: &'a [Value],
    rows: &'a [OccurrenceRow],
    operation_children: Vec<Vec<usize>>,
    runtime_children: Vec<Vec<usize>>,
    activation_by_definition: HashMap<usize, Option<ModuleAnchor>>,
    proof_budget: usize,
}

/// moduleActivationAnchor: proves a re-parented module-definition occurrence
/// is anchored to its exact matching call body (cached per definition row).
fn module_activation_anchor(
    definition_occurrence_index: usize,
    index: &mut OccurrenceIndex,
) -> Result<Option<ModuleAnchor>> {
    if let Some(cached) = index.activation_by_definition.get(&definition_occurrence_index) {
        return Ok(*cached);
    }
    let operations = index.operations;
    let rows = index.rows;
    let reject = |index: &mut OccurrenceIndex| {
        index
            .activation_by_definition
            .insert(definition_occurrence_index, None);
        None
    };
    let definition_occurrence = &rows[definition_occurrence_index];
    let definition_operation = operations
        .get(definition_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let definition_path = operation_path(definition_operation)?;
    let definition_last = definition_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    if definition_operation["parent"].as_u64().is_some()
        || operation_category(definition_operation)? != "module"
        || operation_name(definition_operation)?.starts_with('$')
        || segment_parts(definition_last).0 != Some("module")
    {
        return Ok(reject(index));
    }
    let Some(body_occurrence_index) = definition_occurrence.parent else {
        return Ok(reject(index));
    };
    let body_occurrence = &rows[body_occurrence_index];
    let body_operation = operations
        .get(body_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let body_path = operation_path(body_operation)?;
    let body_segment = body_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let body_segment_parts = segment_parts(body_segment);
    if operation_category(body_operation)? != "control"
        || operation_name(body_operation)? != "$body"
        || body_segment_parts.0 != Some("body")
        || body_segment_parts.1 != Some("$body")
        || body_segment_parts.2 != Some(0)
        || body_operation["parent"].as_u64().is_none()
    {
        return Ok(reject(index));
    }
    let Some(call_occurrence_index) = body_occurrence.parent else {
        return Ok(reject(index));
    };
    let call_occurrence = &rows[call_occurrence_index];
    let call_operation = operations
        .get(call_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let call_path = operation_path(call_operation)?;
    let call_last = call_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    if body_operation["parent"].as_u64().and_then(|v| usize::try_from(v).ok())
        != Some(call_occurrence.operation)
        || operation_category(call_operation)? != "module"
        || operation_name(call_operation)?.starts_with('$')
        || segment_parts(call_last).0 != Some("call")
        || operation_name(call_operation)? != operation_name(definition_operation)?
        || !same_dynamic_bindings(&call_occurrence.slots, &definition_occurrence.slots)?
    {
        return Ok(reject(index));
    }

    let static_bodies = index
        .operation_children
        .get(call_occurrence.operation)
        .cloned()
        .unwrap_or_default();
    let static_expansions = index
        .operation_children
        .get(body_occurrence.operation)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        static_bodies.len() + static_expansions.len(),
    )?;
    if static_bodies.len() != 1
        || static_bodies[0] != body_occurrence.operation
        || static_expansions.len() != 1
    {
        return Ok(reject(index));
    }
    let sole_static_expansion = static_expansions[0];
    let expansion_operation = operations
        .get(sole_static_expansion)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let expansion_path = operation_path(expansion_operation)?;
    let expansion_segment = expansion_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let expansion_segment_parts = segment_parts(expansion_segment);
    if operation_category(expansion_operation)? != "control"
        || operation_name(expansion_operation)? != "$expansion"
        || expansion_segment_parts.0 != Some("control")
        || expansion_segment_parts.1 != Some("$expansion")
        || expansion_segment_parts.2 != Some(0)
    {
        return Ok(reject(index));
    }

    let call_runtime_children = index
        .runtime_children
        .get(call_occurrence_index)
        .cloned()
        .unwrap_or_default();
    let body_runtime_children = index
        .runtime_children
        .get(body_occurrence_index)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        call_runtime_children.len() + body_runtime_children.len(),
    )?;
    if call_runtime_children.len() != 1
        || call_runtime_children[0] != body_occurrence_index
        || body_runtime_children.len() != 1
        || body_runtime_children[0] != definition_occurrence_index
    {
        return Ok(reject(index));
    }
    let anchor = ModuleAnchor {
        call_occurrence: call_occurrence_index,
        body_occurrence: body_occurrence_index,
        definition_occurrence: definition_occurrence_index,
        expansion_operation: sole_static_expansion,
    };
    index
        .activation_by_definition
        .insert(definition_occurrence_index, Some(anchor));
    Ok(Some(anchor))
}

/// isChildrenExpansionContinuation: an $expansion occurrence row is the exact
/// bounded continuation of its consuming children() call.
fn is_children_expansion_continuation(
    occurrence_index: usize,
    operation: usize,
    parent: Option<usize>,
    static_parent: usize,
    slots: &[Value],
    index: &mut OccurrenceIndex,
) -> Result<bool> {
    let Some(parent) = parent else {
        return Ok(false);
    };
    let operations = index.operations;
    let rows = index.rows;
    let expansion_operation = operations
        .get(operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let children_occurrence = &rows[parent];
    let children_operation = operations
        .get(children_occurrence.operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let caller_body_occurrence = &rows[static_parent];
    let expansion_path = operation_path(expansion_operation)?;
    let children_path = operation_path(children_operation)?.to_vec();
    let expansion_segment = expansion_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let children_segment = children_path
        .last()
        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
    let expansion_segment_parts = segment_parts(expansion_segment);
    let children_segment_parts = segment_parts(children_segment);
    if operation_name(expansion_operation)? != "$expansion"
        || operation_category(expansion_operation)? != "control"
        || expansion_segment_parts.0 != Some("control")
        || expansion_segment_parts.1 != Some("$expansion")
        || expansion_segment_parts.2 != Some(0)
        || expansion_operation["parent"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            != Some(caller_body_occurrence.operation)
        || operation_name(children_operation)? != "children"
        || operation_category(children_operation)? != "control"
        || children_segment_parts.0 != Some("control")
        || children_segment_parts.1 != Some("children")
    {
        return Ok(false);
    }

    let children_slots = &children_occurrence.slots;
    if children_slots.len() != 1
        || slots.len() != 1
        || children_slots[0]["name"].as_str() != Some("$index")
        || slots[0]["name"].as_str() != Some("$index")
        || slot_value_key(&children_slots[0])? != slot_value_key(&slots[0])?
    {
        return Ok(false);
    }

    // A selected `!children()` continuation hangs directly beneath its
    // consuming children() call; all expanded statements must stay in that
    // exact static subtree.
    if static_parent == parent && caller_body_occurrence.operation == children_occurrence.operation
    {
        let static_children = index
            .operation_children
            .get(children_occurrence.operation)
            .cloned()
            .unwrap_or_default();
        let runtime_children = index
            .runtime_children
            .get(parent)
            .cloned()
            .unwrap_or_default();
        let expanded_children = index
            .runtime_children
            .get(occurrence_index)
            .cloned()
            .unwrap_or_default();
        consume_proof_budget(
            &mut index.proof_budget,
            static_children.len() + runtime_children.len() + expanded_children.len(),
        )?;
        let expanded_stay_in_subtree = expanded_children.iter().all(|candidate| {
            let candidate_operation = &operations[rows[*candidate].operation];
            candidate_operation["parent"]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                == Some(operation)
        });
        return Ok(static_children.len() == 1
            && static_children[0] == operation
            && runtime_children.len() == 1
            && runtime_children[0] == occurrence_index
            && expanded_stay_in_subtree);
    }

    let mut cursor = children_occurrence.parent;
    let mut definition_occurrence: Option<usize> = None;
    while let Some(candidate_index) = cursor {
        if candidate_index == static_parent {
            break;
        }
        let candidate = &rows[candidate_index];
        let candidate_operation = operations
            .get(candidate.operation)
            .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
        let candidate_path = operation_path(candidate_operation)?;
        consume_proof_budget(&mut index.proof_budget, candidate_path.len())?;
        let candidate_last = candidate_path
            .last()
            .ok_or_else(|| invalid("Expected a semantic structural path"))?;
        if segment_parts(candidate_last).0 == Some("module")
            && path_strictly_contains(candidate_path, &children_path)
        {
            definition_occurrence = Some(candidate_index);
            break;
        }
        consume_proof_budget(&mut index.proof_budget, 1)?;
        cursor = candidate.parent;
    }
    let Some(definition_index) = definition_occurrence else {
        return Ok(false);
    };
    if rows[definition_index].parent != Some(static_parent) {
        return Ok(false);
    }
    let definition_operation = operations
        .get(rows[definition_index].operation)
        .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
    let activation = module_activation_anchor(definition_index, index)?;
    let Some(activation) = activation else {
        return Ok(false);
    };
    if activation.body_occurrence != static_parent || activation.expansion_operation != operation {
        return Ok(false);
    }
    let definition_path = operation_path(definition_operation)?.to_vec();
    consume_proof_budget(&mut index.proof_budget, definition_path.len())?;
    let lexically_contains_children = path_strictly_contains(&definition_path, &children_path);
    let expansion_runtime_children = index
        .runtime_children
        .get(parent)
        .cloned()
        .unwrap_or_default();
    let expanded_children = index
        .runtime_children
        .get(occurrence_index)
        .cloned()
        .unwrap_or_default();
    consume_proof_budget(
        &mut index.proof_budget,
        expansion_runtime_children.len() + expanded_children.len(),
    )?;
    let expanded_children_stay_in_static_subtree = expanded_children.iter().all(|candidate| {
        let candidate_operation = &operations[rows[*candidate].operation];
        candidate_operation["parent"]
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            == Some(operation)
    });
    Ok(lexically_contains_children
        && expansion_runtime_children.len() == 1
        && expansion_runtime_children[0] == occurrence_index
        && expanded_children_stay_in_static_subtree)
}

/// Identity subset of validateOccurrences: positional IDs, canonical-row
/// parent/staticParent references, re-derived occv1:/entity:v2: digests,
/// per-ID output-slot density, dynamic-slot duplicate-ordinal discipline, the
/// staticParent runtime ancestor-chain walk, and $expansion continuation /
/// module-activation anchoring (ports of isChildrenExpansionContinuation and
/// moduleActivationAnchor).
pub fn validate_occurrence_identity(
    occurrences: &[Value],
    operations: &[Value],
    node_count: usize,
) -> Result<()> {
    if occurrences.len() > 100_000 {
        return Err(invalid("Semantic occurrence limit exceeded"));
    }
    if operations.is_empty() && !occurrences.is_empty() {
        return Err(invalid("A semantic occurrence requires an operation"));
    }
    let operation_parents: Vec<Option<usize>> = operations
        .iter()
        .map(|operation| {
            operation["parent"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
        })
        .collect();
    let operation_ids: Vec<&str> = operations
        .iter()
        .map(|operation| {
            operation["operationId"]
                .as_str()
                .ok_or_else(|| invalid("Expected a semantic operation ID"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut scene_ids: HashSet<String> = HashSet::new();
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut group_order: Vec<String> = Vec::new();
    let mut duplicate_slots: HashMap<(Option<usize>, usize, usize, String, String), Vec<(u64, String)>> =
        HashMap::new();
    let mut occurrence_ids: Vec<String> = Vec::with_capacity(occurrences.len());
    let mut rows: Vec<OccurrenceRow> = Vec::with_capacity(occurrences.len());
    let mut reparented_static_roots: Vec<usize> = Vec::new();
    let mut pending_continuations: Vec<(usize, usize, Option<usize>, usize, Vec<Value>)> =
        Vec::new();
    let mut proof_budget: usize = 0;
    for (index, occurrence) in occurrences.iter().enumerate() {
        exact(
            occurrence,
            &[
                "id",
                "occurrenceId",
                "operation",
                "parent",
                "staticParent",
                "dynamicSlots",
                "node",
                "outputOrdinal",
                "sceneEntityId",
            ],
        )?;
        if occurrence["id"].as_u64() != Some(index as u64) {
            return Err(invalid("Semantic occurrence IDs must equal array positions"));
        }
        let operation = occurrence["operation"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value < operations.len())
            .ok_or_else(|| invalid("Semantic occurrence operation reference is invalid"))?;
        let parent = optional_row_reference(&occurrence["parent"], index)?;
        let static_parent = optional_row_reference(&occurrence["staticParent"], index)?;
        let slots = validate_dynamic_slots(&occurrence["dynamicSlots"])?;
        match operation_parents[operation] {
            None => {
                if static_parent.is_some() {
                    return Err(invalid(
                        "A static-root operation requires a null staticParent",
                    ));
                }
                if parent.is_some() {
                    let root_operation = &operations[operation];
                    let root_path = operation_path(root_operation)?;
                    let last = root_path
                        .last()
                        .ok_or_else(|| invalid("Expected a semantic structural path"))?;
                    if operation_category(root_operation)? != "module"
                        || segment_parts(last).0 != Some("module")
                    {
                        return Err(invalid(
                            "A static-root operation cannot have a runtime parent outside a module-definition activation",
                        ));
                    }
                    reparented_static_roots.push(index);
                }
            }
            Some(static_parent_operation) => {
                let referenced = static_parent.ok_or_else(|| {
                    invalid("staticParent must instantiate the static parent operation")
                })?;
                if rows[referenced].operation != static_parent_operation {
                    return Err(invalid(
                        "staticParent must instantiate the static parent operation",
                    ));
                }
                // Walk the runtime ancestor chain from parent up to the
                // staticParent row: only control/module frames may separate
                // them, and the static parent must be the nearest match.
                let mut cursor = parent;
                let mut crossed_non_expansion_frame = false;
                while cursor != Some(referenced) {
                    consume_proof_budget(&mut proof_budget, 1)?;
                    let Some(skipped_index) = cursor else {
                        return Err(invalid(
                            "staticParent is not on the runtime ancestor chain",
                        ));
                    };
                    let skipped = &rows[skipped_index];
                    if skipped.operation == static_parent_operation {
                        return Err(invalid(
                            "staticParent must be the nearest matching ancestor",
                        ));
                    }
                    let category = operation_category(&operations[skipped.operation])?;
                    if category != "control" && category != "module" {
                        crossed_non_expansion_frame = true;
                    }
                    cursor = skipped.parent;
                }
                if operation_name(&operations[operation])? == "$expansion" {
                    pending_continuations.push((
                        index,
                        operation,
                        parent,
                        referenced,
                        slots.clone(),
                    ));
                } else if crossed_non_expansion_frame {
                    return Err(invalid(
                        "Only control/module frames may separate parent from staticParent",
                    ));
                }
            }
        }
        let parent_occurrence_id = parent.map(|row| occurrence_ids[row].as_str());
        let static_parent_occurrence_id = static_parent.map(|row| occurrence_ids[row].as_str());
        let expected_id = derive_occurrence_id(
            parent_occurrence_id,
            static_parent_occurrence_id,
            operation_ids[operation],
            &slots,
        );
        if occurrence["occurrenceId"].as_str() != Some(expected_id.as_str()) {
            return Err(invalid(
                "Semantic occurrence ID does not match operation, parent, and dynamic slots",
            ));
        }
        if let Some(group) = groups.get(&expected_id) {
            let first = group[0];
            let first_row = &rows[first];
            if first_row.operation != operation
                || first_row.parent != parent
                || first_row.static_parent != static_parent
                || !same_dynamic_slots(&first_row.slots, &slots)?
            {
                return Err(invalid(
                    "Rows sharing an occurrence ID must describe the same logical evaluation",
                ));
            }
        } else {
            group_order.push(expected_id.clone());
        }
        groups.entry(expected_id.clone()).or_default().push(index);
        for (slot_index, slot) in slots.iter().enumerate() {
            let key = (
                parent,
                operation,
                slot_index,
                slot["name"].as_str().unwrap_or_default().to_owned(),
                identity_value_key(&slot["value"])?,
            );
            let ordinal = slot["duplicateOrdinal"]
                .as_u64()
                .ok_or_else(|| invalid("Expected a semantic duplicate ordinal"))?;
            let ordinals = duplicate_slots.entry(key).or_default();
            if let Some((_, existing)) = ordinals.iter().find(|(known, _)| *known == ordinal) {
                if *existing != expected_id {
                    return Err(invalid(
                        "Duplicate ordinals must distinguish equal dynamic values",
                    ));
                }
            } else {
                if ordinal != ordinals.len() as u64 {
                    return Err(invalid(
                        "Duplicate ordinals must be dense in language evaluation order",
                    ));
                }
                ordinals.push((ordinal, expected_id.clone()));
            }
        }
        let node = if occurrence["node"].is_null() {
            None
        } else {
            let node = occurrence["node"]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value < node_count)
                .ok_or_else(|| invalid("Semantic occurrence node reference is invalid"))?;
            Some(node)
        };
        match node {
            None => {
                if !occurrence["outputOrdinal"].is_null() || !occurrence["sceneEntityId"].is_null()
                {
                    return Err(invalid(
                        "A non-producing occurrence cannot own an output identity",
                    ));
                }
            }
            Some(_) => {
                if occurrence["outputOrdinal"].is_null() {
                    if !occurrence["sceneEntityId"].is_null() {
                        return Err(invalid(
                            "A producer-only occurrence cannot claim scene identity",
                        ));
                    }
                } else {
                    let ordinal = bounded_integer(&occurrence["outputOrdinal"], 1_000_000)?;
                    let expected_scene = derive_scene_entity_id(&expected_id, ordinal);
                    if occurrence["sceneEntityId"].as_str() != Some(expected_scene.as_str()) {
                        return Err(invalid(
                            "Scene entity ID does not match occurrence and output ordinal",
                        ));
                    }
                    if !scene_ids.insert(expected_scene) {
                        return Err(invalid("Scene entity IDs must be unique"));
                    }
                }
            }
        }
        occurrence_ids.push(expected_id);
        rows.push(OccurrenceRow {
            operation,
            parent,
            static_parent,
            slots,
        });
    }
    // $expansion continuation and module-activation anchoring (host
    // moduleActivationAnchor / isChildrenExpansionContinuation ports).
    let mut operation_children: Vec<Vec<usize>> = vec![Vec::new(); operations.len()];
    for operation in operations.iter() {
        if let Some(parent) = operation["parent"]
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
        {
            operation_children[parent].push(
                operation["id"]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| invalid("Expected a semantic operation ID"))?,
            );
        }
    }
    let mut runtime_children: Vec<Vec<usize>> = vec![Vec::new(); occurrences.len()];
    for (index, row) in rows.iter().enumerate() {
        if let Some(parent) = row.parent {
            runtime_children[parent].push(index);
        }
    }
    let mut occurrence_index = OccurrenceIndex {
        operations,
        rows: &rows,
        operation_children,
        runtime_children,
        activation_by_definition: HashMap::new(),
        proof_budget,
    };
    for definition_occurrence in reparented_static_roots {
        if module_activation_anchor(definition_occurrence, &mut occurrence_index)?.is_none() {
            return Err(invalid(
                "Module-definition activation is not anchored to its exact matching call body",
            ));
        }
    }
    for (occurrence, operation, parent, static_parent, slots) in pending_continuations {
        if !is_children_expansion_continuation(
            occurrence,
            operation,
            parent,
            static_parent,
            &slots,
            &mut occurrence_index,
        )? {
            return Err(invalid(
                "$expansion must be the exact children() continuation of its matching caller",
            ));
        }
    }
    // parent/staticParent must reference the canonical first row of its ID.
    for (index, occurrence) in occurrences.iter().enumerate() {
        for field_name in ["parent", "staticParent"] {
            let Some(reference) = optional_row_reference(&occurrence[field_name], index)? else {
                continue;
            };
            let referenced_id = &occurrence_ids[reference];
            if groups
                .get(referenced_id)
                .and_then(|group| group.first())
                .copied()
                != Some(reference)
            {
                return Err(invalid(
                    "Occurrence references must target the canonical first row of a logical occurrence",
                ));
            }
        }
    }
    // Rows sharing an ID: one zero/frame row, or dense output rows 0..k.
    for id in &group_order {
        let group = &groups[id];
        let output_rows: Vec<usize> = group
            .iter()
            .copied()
            .filter(|index| !occurrences[*index]["outputOrdinal"].is_null())
            .collect();
        if group.len() > 1 && output_rows.len() != group.len() {
            return Err(invalid(
                "An occurrence ID may repeat only for its output slots",
            ));
        }
        for (ordinal, index) in output_rows.iter().enumerate() {
            if occurrences[*index]["outputOrdinal"].as_u64() != Some(ordinal as u64) {
                return Err(invalid(
                    "Output ordinals must be unique, ordered, and dense from zero per occurrence ID",
                ));
            }
        }
    }
    for ordinals in duplicate_slots.values() {
        let mut ordered: Vec<u64> = ordinals.iter().map(|(ordinal, _)| *ordinal).collect();
        ordered.sort_unstable();
        if ordered
            .iter()
            .enumerate()
            .any(|(index, ordinal)| *ordinal != index as u64)
        {
            return Err(invalid(
                "Duplicate ordinals for equal dynamic slot values must be dense from zero",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
