use super::*;

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

fn utf16_len(text: &str) -> usize {
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

/// ECMAScript Number::toString for a finite binary64 (JSON.stringify number form).
pub(super) fn js_f64(value: f64) -> String {
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
const PATH_SEGMENT_KINDS: [&str; 5] = ["call", "module", "control", "branch", "body"];

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
