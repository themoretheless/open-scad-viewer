use super::*;

fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected a semantic envelope object"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid("Semantic envelope fields do not match the schema"));
    }
    Ok(())
}

/// Exact key sets from validateSemanticProgramV1 / validateCore.
const ENVELOPE_KEYS: [&str; 7] = [
    "schema",
    "schemaVersion",
    "source",
    "core",
    "provenance",
    "tessellationIntents",
    "diagnostics",
];
pub(super) const CORE_KEYS: [&str; 14] = [
    "schema",
    "schemaVersion",
    "requiredFeatures",
    "identityVersion",
    "language",
    "units",
    "operations",
    "occurrences",
    "nodes",
    "execution",
    "result",
    "declaredCapabilities",
    "capabilityClosure",
    "diagnosticTemplates",
];
/// Envelope-level components that also travel as top-level request fields.
pub(super) const COMPONENT_KEYS: [&str; 4] = ["source", "provenance", "tessellationIntents", "diagnostics"];

/// Admits a transported SemanticProgramEnvelopeV1 object: exact envelope and
/// core key sets, the schema literals, and deep-equality between every
/// envelope component and its separately transported duplicate. Returns the
/// request view with envelope-only components adopted for admission.
pub fn admit_envelope_object(request: &Value, envelope: &Value) -> Result<Value> {
    exact(envelope, &ENVELOPE_KEYS)?;
    if envelope["schema"].as_str() != Some("semantic-program-envelope") {
        return Err(invalid("Expected the semantic-program-envelope schema"));
    }
    super::super::brep_envelope::validate_schema_version(&envelope["schemaVersion"])?;
    let core = &envelope["core"];
    exact(core, &CORE_KEYS)?;
    if core["schema"].as_str() != Some("semantic-program-core") {
        return Err(invalid("Expected the semantic-program-core schema"));
    }
    let mut merged = request.clone();
    let object = merged
        .as_object_mut()
        .ok_or_else(|| invalid("Expected a semantic graph request object"))?;
    for key in CORE_KEYS {
        let component = &core[key];
        match object.get(key) {
            Some(duplicate) if duplicate != component => {
                return Err(invalid(
                    "Semantic envelope component does not match its transported duplicate",
                ));
            }
            Some(_) => {}
            None => {
                object.insert(key.to_owned(), component.clone());
            }
        }
    }
    for key in COMPONENT_KEYS {
        let component = &envelope[key];
        match object.get(key) {
            Some(duplicate) if duplicate != component => {
                return Err(invalid(
                    "Semantic envelope component does not match its transported duplicate",
                ));
            }
            Some(_) => {}
            None => {
                object.insert(key.to_owned(), component.clone());
            }
        }
    }
    Ok(merged)
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// The JavaScript regular-expression `\s` set (including U+FEFF).
pub(super) fn js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

pub(super) fn js_trim_start(text: &str) -> &str {
    text.trim_start_matches(js_whitespace)
}

pub(super) fn js_trim(text: &str) -> &str {
    text.trim_matches(js_whitespace)
}

/// String.split(/\r\n|\n|\r/): CR, LF and CRLF are each one separator.
pub(super) fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                lines.push(&text[start..index]);
                index += if bytes.get(index + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = index;
            }
            b'\n' => {
                lines.push(&text[start..index]);
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }
    lines.push(&text[start..]);
    lines
}

#[derive(Default)]
pub(super) struct ScanState {
    in_block_comment: bool,
    in_string: bool,
    escaped: bool,
}

/// Port of sourceOutsideBlockComments: replaces comment/string interiors so
/// directives are recognized only outside block comments and strings.
pub(super) fn source_outside_block_comments(raw_line: &str, state: &mut ScanState) -> String {
    let chars: Vec<char> = raw_line.chars().collect();
    let mut line = if state.in_string {
        String::from("\"")
    } else {
        String::new()
    };
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        let next = chars.get(index + 1).copied();
        if state.in_block_comment {
            if character == '*' && next == Some('/') {
                state.in_block_comment = false;
                index += 1;
            }
            index += 1;
            continue;
        }
        if state.in_string {
            if state.escaped {
                state.escaped = false;
            } else if character == '\\' {
                state.escaped = true;
            } else if character == '"' {
                state.in_string = false;
                line.push('"');
            }
            index += 1;
            continue;
        }
        if character == '"' {
            state.in_string = true;
            line.push(character);
            index += 1;
            continue;
        }
        if character == '/' && next == Some('*') {
            state.in_block_comment = true;
            index += 2;
            continue;
        }
        if character == '/' && next == Some('/') {
            line.extend(&chars[index..]);
            break;
        }
        line.push(character);
        index += 1;
    }
    line
}

/// Strips the `^\s*\/\/\s*` prefix shared by every directive pattern.
pub(super) fn directive_body(directive_line: &str) -> Option<&str> {
    let trimmed = js_trim_start(directive_line);
    let comment = trimmed.strip_prefix("//")?;
    Some(js_trim_start(comment))
}

/// `^\s*\/\/\s*@language\s+(\S+)\s*$`
pub(super) fn match_language(directive_line: &str) -> Option<&str> {
    let body = directive_body(directive_line)?;
    let rest = body.strip_prefix("@language")?;
    if !rest.starts_with(js_whitespace) {
        return None;
    }
    let rest = js_trim_start(rest);
    let token: &str = &rest[..rest.find(js_whitespace).unwrap_or(rest.len())];
    if token.is_empty() || !js_trim(&rest[token.len()..]).is_empty() {
        return None;
    }
    Some(token)
}

/// `^\s*\/\/\s*@requires\s+(.+?)\s*$`
pub(super) fn match_requires(directive_line: &str) -> Option<&str> {
    let body = directive_body(directive_line)?;
    let rest = body.strip_prefix("@requires")?;
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(_)) if js_whitespace(first) => {}
        _ => return None,
    }
    Some(js_trim(rest))
}
