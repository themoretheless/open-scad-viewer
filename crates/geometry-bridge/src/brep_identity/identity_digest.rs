use super::*;

pub(super) fn invalid(message: &str) -> super::super::Error {
    super::super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

pub(super) fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected a semantic identity object"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid("Semantic identity fields do not match the schema"));
    }
    Ok(())
}

pub(super) fn bounded_integer(value: &Value, maximum: u64) -> Result<u64> {
    let number = value
        .as_u64()
        .ok_or_else(|| invalid("Expected a non-negative semantic integer"))?;
    if number > maximum {
        return Err(invalid("Semantic numeric limit exceeded"));
    }
    Ok(number)
}

pub(super) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(super) fn bounded_string(value: &Value, maximum: usize) -> Result<&str> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic identity string"))?;
    if utf16_len(text) > maximum {
        return Err(invalid("Semantic identity string limit exceeded"));
    }
    Ok(text)
}

// ---- Canonical JSON (byte-exact port of stableJson + JSON.stringify) ----

/// identityDigest: SHA-256 over length-prefixed domain and canonical payload.
pub fn identity_digest(domain: &str, payload: &Value) -> String {
    let mut canonical = String::new();
    stable_json(payload, &mut canonical);
    let mut preimage = Vec::with_capacity(8 + domain.len() + canonical.len());
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
            if value["value"]
                .as_bool()
                .ok_or_else(|| invalid("Expected a boolean identity value"))?
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
                Some(Value::Number(Number::Float(v))) if *v == 0.0 && v.is_sign_negative() => {
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
pub(super) fn slot_value_key(slot: &Value) -> Result<String> {
    identity_value_key(&slot["value"])
}

/// sameDynamicSlots: equal names, duplicate ordinals and identity value keys.
pub(super) fn same_dynamic_slots(left: &[Value], right: &[Value]) -> Result<bool> {
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
pub(super) fn same_dynamic_bindings(left: &[Value], right: &[Value]) -> Result<bool> {
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

pub(super) fn validate_dynamic_slots(value: &Value) -> Result<Vec<Value>> {
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

pub(super) const OPERATION_CATEGORIES: [&str; 7] = [
    "geometry",
    "transform",
    "boolean",
    "control",
    "module",
    "assertion",
    "presentation",
];
pub(super) const PATH_SEGMENT_KINDS: [&str; 5] = ["call", "module", "control", "branch", "body"];

pub(super) fn validate_structural_path(value: &Value) -> Result<Vec<Value>> {
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

pub(super) fn same_path(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left["kind"].as_str() == right["kind"].as_str()
                && left["name"].as_str() == right["name"].as_str()
                && left["ordinal"].as_u64() == right["ordinal"].as_u64()
        })
}

pub(super) fn structural_path_key(path: &[Value]) -> String {
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
        while preorder_stack
            .last()
            .copied()
            .is_some_and(|top| Some(top) != parent)
        {
            closed.insert(preorder_stack.pop().expect("non-empty stack"));
        }
        if let Some(parent) = parent
            && (closed.contains(&parent) || preorder_stack.last().copied() != Some(parent))
        {
            return Err(invalid(
                "Semantic operations must be in deterministic parent-before-child preorder",
            ));
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
        if last["name"].as_str() != Some(name) || last["ordinal"].as_u64() != Some(child_ordinal) {
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
            operation["category"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
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

pub(super) fn optional_row_reference(value: &Value, index: usize) -> Result<Option<usize>> {
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
pub(super) const PROOF_BUDGET_LIMIT: usize = 1_000_000;

pub(super) fn consume_proof_budget(budget: &mut usize, steps: usize) -> Result<()> {
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

pub(super) fn operation_path(operation: &Value) -> Result<&[Value]> {
    operation["structuralPath"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("Expected a semantic structural path"))
}

pub(super) fn operation_category(operation: &Value) -> Result<&str> {
    operation["category"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic operation category"))
}

pub(super) fn operation_name(operation: &Value) -> Result<&str> {
    operation["name"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic operation name"))
}

pub(super) fn segment_field<'a>(segment: &'a Value, key: &str) -> &'a Value {
    &segment[key]
}

/// (kind, name, ordinal) triple of a structural path segment.
pub(super) fn segment_parts(segment: &Value) -> (Option<&str>, Option<&str>, Option<u64>) {
    (
        segment_field(segment, "kind").as_str(),
        segment_field(segment, "name").as_str(),
        segment_field(segment, "ordinal").as_u64(),
    )
}

/// Strict lexical containment: the candidate path is a proper prefix of the
/// children() path, segment by segment (kind, name, ordinal).
pub(super) fn path_strictly_contains(candidate: &[Value], children: &[Value]) -> bool {
    candidate.len() < children.len() && same_path(candidate, &children[..candidate.len()])
}

#[derive(Clone, Copy)]
pub(super) struct ModuleAnchor {
    // All four fields are part of the host anchor record; the continuation
    // check consumes body_occurrence and expansion_operation.
    #[allow(dead_code)]
    pub(super) call_occurrence: usize,
    pub(super) body_occurrence: usize,
    #[allow(dead_code)]
    pub(super) definition_occurrence: usize,
    pub(super) expansion_operation: usize,
}
