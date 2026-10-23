//! Native admission of the complete SemanticProgramEnvelopeV1 object and
//! native source attestation.
//!
//! Port of the envelope-object subset of validateSemanticProgramV1
//! (src/services/semanticProgramValidator.ts l.2137: exact envelope keys and
//! the semantic-program-envelope schema literal) and of
//! semanticSourceDescriptor / attestSemanticProgramSource (l.2334/2348),
//! including a faithful port of parseGeometrySourceRoutingHeader
//! (src/core/geometryRouting.ts l.88).
//!
//! Envelope consistency semantics: the separately transported graph-begin
//! fields remain the execution inputs; when a component is transported both
//! ways the two copies must be deep-equal ("duplicates must match exactly"),
//! and components carried only inside the envelope are adopted into
//! admission. The envelope never replaces execution inputs.
use super::{Result, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

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
const CORE_KEYS: [&str; 14] = [
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
const COMPONENT_KEYS: [&str; 4] = ["source", "provenance", "tessellationIntents", "diagnostics"];

/// Admits a transported SemanticProgramEnvelopeV1 object: exact envelope and
/// core key sets, the schema literals, and deep-equality between every
/// envelope component and its separately transported duplicate. Returns the
/// request view with envelope-only components adopted for admission.
pub fn admit_envelope_object(request: &Value, envelope: &Value) -> Result<Value> {
    exact(envelope, &ENVELOPE_KEYS)?;
    if envelope["schema"].as_str() != Some("semantic-program-envelope") {
        return Err(invalid("Expected the semantic-program-envelope schema"));
    }
    super::brep_envelope::validate_schema_version(&envelope["schemaVersion"])?;
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// The JavaScript regular-expression `\s` set (including U+FEFF).
fn js_whitespace(c: char) -> bool {
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

fn js_trim_start(text: &str) -> &str {
    text.trim_start_matches(js_whitespace)
}

fn js_trim(text: &str) -> &str {
    text.trim_matches(js_whitespace)
}

/// String.split(/\r\n|\n|\r/): CR, LF and CRLF are each one separator.
fn split_lines(text: &str) -> Vec<&str> {
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
struct ScanState {
    in_block_comment: bool,
    in_string: bool,
    escaped: bool,
}

/// Port of sourceOutsideBlockComments: replaces comment/string interiors so
/// directives are recognized only outside block comments and strings.
fn source_outside_block_comments(raw_line: &str, state: &mut ScanState) -> String {
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
fn directive_body(directive_line: &str) -> Option<&str> {
    let trimmed = js_trim_start(directive_line);
    let comment = trimmed.strip_prefix("//")?;
    Some(js_trim_start(comment))
}

/// `^\s*\/\/\s*@language\s+(\S+)\s*$`
fn match_language(directive_line: &str) -> Option<&str> {
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
fn match_requires(directive_line: &str) -> Option<&str> {
    let body = directive_body(directive_line)?;
    let rest = body.strip_prefix("@requires")?;
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(_)) if js_whitespace(first) => {}
        _ => return None,
    }
    Some(js_trim(rest))
}

/// `^\s*\/\/\s*@engine(?:\s+.*)?$`
fn match_engine(directive_line: &str) -> bool {
    let Some(body) = directive_body(directive_line) else {
        return false;
    };
    let Some(rest) = body.strip_prefix("@engine") else {
        return false;
    };
    rest.is_empty() || rest.starts_with(js_whitespace)
}

fn word_character(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `^\s*\/\/\s*@(language|requires|engine)\b` (case-insensitive).
fn match_malformed_reserved(directive_line: &str) -> Option<&'static str> {
    let body = directive_body(directive_line)?;
    let rest = body.strip_prefix('@')?;
    for name in ["language", "requires", "engine"] {
        if rest.len() >= name.len() && rest[..name.len()].eq_ignore_ascii_case(name) {
            let boundary = rest[name.len()..].chars().next();
            if boundary.is_none() || boundary.is_some_and(|c| !word_character(c)) {
                return Some(name);
            }
        }
    }
    None
}

struct RoutingHeader {
    language_contract: String,
    required_capabilities: Vec<String>,
}

/// Port of parseGeometrySourceRoutingHeader. Well-formedness of the text is
/// guaranteed by the transport (Rust strings cannot carry lone surrogates);
/// the length limit is already enforced by the source descriptor checks.
fn routing_header(source: &str) -> Result<RoutingHeader> {
    let mut language_contract = "legacy/current".to_owned();
    let mut language_directive_line: Option<usize> = None;
    let mut body_started = false;
    let mut state = ScanState::default();
    let mut required_capabilities: BTreeSet<String> = BTreeSet::new();
    for (index, raw_line) in split_lines(source).iter().enumerate() {
        let line_number = index + 1;
        let line = source_outside_block_comments(raw_line, &mut state);
        let line_comment_start = line.find("//");
        let directive_line = match line_comment_start {
            Some(start) => &line[start..],
            None => line.as_str(),
        };
        let code_before_directive = line_comment_start
            .is_some_and(|start| start > 0 && !js_trim(&line[..start]).is_empty());
        if let Some(token) = match_language(directive_line) {
            if body_started || code_before_directive {
                return Err(invalid(
                    "The @language directive must be in the leading source header",
                ));
            }
            if language_directive_line.is_some() {
                return Err(invalid("The @language directive is duplicated"));
            }
            if token != "legacy/current" && token != "openscad-viewer/brep-1" {
                return Err(invalid("Unsupported geometry language contract"));
            }
            language_contract = token.to_owned();
            language_directive_line = Some(line_number);
            continue;
        }
        if let Some(capture) = match_requires(directive_line) {
            if body_started || code_before_directive {
                return Err(invalid(
                    "The @requires directive must be in the leading source header",
                ));
            }
            let identifiers: Vec<&str> = capture
                .split(|c: char| js_whitespace(c) || c == ',')
                .filter(|part| !part.is_empty())
                .collect();
            if identifiers.is_empty()
                || identifiers
                    .iter()
                    .any(|id| !super::brep_envelope::capability_identifier(id))
            {
                return Err(invalid(
                    "The @requires directive contains an invalid capability identifier",
                ));
            }
            for identifier in identifiers {
                required_capabilities.insert(identifier.to_owned());
                if required_capabilities.len() > 32 {
                    return Err(invalid("The source may require at most 32 capabilities"));
                }
            }
            continue;
        }
        if match_engine(directive_line) {
            return Err(invalid(
                "The source cannot select an engine directly; choose a versioned @language contract",
            ));
        }
        if match_malformed_reserved(directive_line).is_some() {
            return Err(invalid("Malformed reserved source directive"));
        }
        if !js_trim(&line).is_empty() && !js_trim_start(&line).starts_with("//") {
            body_started = true;
        }
    }
    Ok(RoutingHeader {
        language_contract,
        required_capabilities: required_capabilities.into_iter().collect(),
    })
}

/// UTF-16 code-unit offsets that fall between a high and a low surrogate.
fn surrogate_split_boundaries(source: &str) -> Vec<bool> {
    let mut split = vec![false; source.encode_utf16().count() + 1];
    let mut offset = 0;
    for character in source.chars() {
        if character.len_utf16() == 2 {
            split[offset + 1] = true;
        }
        offset += character.len_utf16();
    }
    split
}

fn assert_scalar_boundary(split: &[bool], offset: u64) -> Result<()> {
    if split.get(offset as usize).is_some_and(|inside| *inside) {
        return Err(invalid("UTF-16 span endpoint splits a surrogate pair"));
    }
    Ok(())
}

fn check_span_boundaries(span: &Value, split: &[bool]) -> Result<()> {
    for key in ["start", "end"] {
        if let Some(offset) = span[key].as_u64() {
            assert_scalar_boundary(split, offset)?;
        }
    }
    Ok(())
}

/// Optional-but-strict native source attestation: port of
/// attestSemanticProgramSource. Runs only when the request transports the
/// exact source text as `sourceText`; the descriptor-only checks stay in
/// brep_envelope when the text is absent.
pub fn attest_source(request: &Value) -> Result<()> {
    let Some(text_value) = request.get("sourceText") else {
        return Ok(());
    };
    let source = text_value
        .as_str()
        .ok_or_else(|| invalid("Expected the exact semantic source text"))?;
    let descriptor = request
        .get("source")
        .ok_or_else(|| invalid("Semantic source attestation requires the source descriptor"))?;
    let utf8_byte_length = source.len() as u64;
    let utf16_code_unit_length = source.encode_utf16().count() as u64;
    if utf16_code_unit_length > 250_000 || utf8_byte_length > 4_000_000 {
        return Err(invalid("Semantic source length limits exceeded"));
    }
    let digest = sha256_hex(source.as_bytes());
    if descriptor["sha256"].as_str() != Some(digest.as_str())
        || descriptor["utf8ByteLength"].as_u64() != Some(utf8_byte_length)
        || descriptor["utf16CodeUnitLength"].as_u64() != Some(utf16_code_unit_length)
    {
        return Err(invalid(
            "Semantic source bytes do not match the envelope attestation",
        ));
    }
    let route = routing_header(source)?;
    let contract = request
        .get("language")
        .and_then(|language| language["contract"].as_str())
        .ok_or_else(|| invalid("Semantic source attestation requires the language contract"))?;
    if route.language_contract != contract {
        return Err(invalid(
            "Source routing language does not match the semantic core",
        ));
    }
    let declared: Vec<&str> = request
        .get("declaredCapabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Semantic source attestation requires declared capabilities"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| invalid("Expected a semantic capability string"))
        })
        .collect::<Result<Vec<_>>>()?;
    let required: Vec<&str> = route
        .required_capabilities
        .iter()
        .map(String::as_str)
        .collect();
    if declared != required {
        return Err(invalid(
            "Source routing requirements do not match authored semantic capabilities",
        ));
    }
    let split = surrogate_split_boundaries(source);
    if let Some(provenance) = request.get("provenance").and_then(Value::as_array) {
        for record in provenance {
            check_span_boundaries(&record["span"], &split)?;
        }
    }
    if let Some(diagnostics) = request.get("diagnostics").and_then(Value::as_array) {
        for diagnostic in diagnostics {
            if !diagnostic["span"].is_null() {
                check_span_boundaries(&diagnostic["span"], &split)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    /// Source with a leading routing header and an astral character (emoji) in
    /// a comment: the emoji occupies UTF-16 offsets 67 (high) and 68 (low), so
    /// offset 68 splits the pair while 67 and 69 are scalar boundaries.
    fn source_text() -> String {
        "// @language openscad-viewer/brep-1\n// @requires custom.feature\n// 😀 note\ncube(1);"
            .to_owned()
    }

    fn descriptor(source: &str) -> Value {
        json!({
            "sha256": sha256_hex(source.as_bytes()),
            "utf8ByteLength": source.len(),
            "utf16CodeUnitLength": source.encode_utf16().count(),
        })
    }

    fn emoji_offsets(source: &str) -> (u64, u64) {
        let start = source[..source.find('😀').unwrap()].encode_utf16().count() as u64;
        (start, start + 2)
    }

    fn attested_request() -> Value {
        let source = source_text();
        let (emoji_start, emoji_end) = emoji_offsets(&source);
        json!({
            "source": descriptor(&source),
            "language": {"contract":"openscad-viewer/brep-1","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"},
            "declaredCapabilities": ["custom.feature"],
            "provenance": [{"operation":0,"span":{"start":emoji_start,"end":emoji_end},"label":"cube"}],
            "diagnostics": [{"template":0,"message":"note","span":{"start":emoji_start,"end":emoji_end}}],
            "sourceText": source,
        })
    }

    #[test]
    fn admits_a_real_source_text_matching_its_descriptor() {
        attest_source(&attested_request()).unwrap();
        // Descriptor-only admission when the text is absent.
        let mut request = attested_request();
        request.as_object_mut().unwrap().remove("sourceText");
        attest_source(&request).unwrap();
        // Null diagnostic spans skip the boundary check.
        let mut request = attested_request();
        request["diagnostics"][0]["span"] = Value::Null;
        attest_source(&request).unwrap();
    }

    #[test]
    fn refuses_descriptor_mismatches_wrong_by_one() {
        // One flipped hex character in the digest.
        let mut request = attested_request();
        request["source"]["sha256"] = json!(format!(
            "{}{}",
            if &attested_request()["source"]["sha256"].as_str().unwrap()[..1] == "a" {
                "b"
            } else {
                "a"
            },
            &attested_request()["source"]["sha256"].as_str().unwrap()[1..]
        ));
        assert!(attest_source(&request).is_err());
        // UTF-8 and UTF-16 lengths off by one.
        for (key, delta) in [("utf8ByteLength", 1i64), ("utf16CodeUnitLength", 1i64)] {
            for sign in [1i64, -1] {
                let mut request = attested_request();
                let value = request["source"][key].as_u64().unwrap() as i64 + delta * sign;
                request["source"][key] = json!(value.max(0));
                assert!(attest_source(&request).is_err(), "accepted {key} {sign}");
            }
        }
        // Attestation requires the descriptor and a string sourceText.
        let mut request = attested_request();
        request.as_object_mut().unwrap().remove("source");
        assert!(attest_source(&request).is_err());
        let mut request = attested_request();
        request["sourceText"] = json!(42);
        assert!(attest_source(&request).is_err());
    }

    #[test]
    fn refuses_span_endpoints_inside_a_surrogate_pair() {
        let source = source_text();
        let (emoji_start, _) = emoji_offsets(&source);
        let mid = emoji_start + 1;
        // Provenance start or end mid-pair refuses; pair edges admit.
        for (start, end, admitted) in [
            (mid, mid + 1, false),
            (emoji_start, mid, false),
            (emoji_start, mid + 1, true),
            (mid - 1, emoji_start, true),
        ] {
            let mut request = attested_request();
            request["provenance"][0]["span"] = json!({"start":start,"end":end});
            assert_eq!(
                attest_source(&request).is_ok(),
                admitted,
                "span {start}..{end}"
            );
        }
        // Diagnostic spans, including empty ones, check both endpoints.
        let mut request = attested_request();
        request["diagnostics"][0]["span"] = json!({"start":mid,"end":mid});
        assert!(attest_source(&request).is_err());
        let mut request = attested_request();
        request["diagnostics"][0]["span"] = json!({"start":emoji_start,"end":emoji_start});
        attest_source(&request).unwrap();
        // The boundary after the final character is always admissible.
        let length = source.encode_utf16().count() as u64;
        let mut request = attested_request();
        request["provenance"][0]["span"] = json!({"start":length - 1,"end":length});
        attest_source(&request).unwrap();
        let _ = source;
    }

    #[test]
    fn refuses_routing_header_inconsistencies() {
        // Language contract mismatch.
        let mut request = attested_request();
        request["language"] = json!({"contract":"legacy/current","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"});
        assert!(attest_source(&request).is_err());
        // Declared capabilities mismatch (missing and extra).
        let mut request = attested_request();
        request["declaredCapabilities"] = json!([]);
        assert!(attest_source(&request).is_err());
        let mut request = attested_request();
        request["declaredCapabilities"] = json!(["custom.feature", "zzz.extra"]);
        assert!(attest_source(&request).is_err());
        // Missing context.
        let mut request = attested_request();
        request.as_object_mut().unwrap().remove("language");
        assert!(attest_source(&request).is_err());
        let mut request = attested_request();
        request
            .as_object_mut()
            .unwrap()
            .remove("declaredCapabilities");
        assert!(attest_source(&request).is_err());
    }

    #[test]
    fn refuses_malformed_routing_headers() {
        let base = attested_request();
        let valid = base["sourceText"].as_str().unwrap().to_owned();
        for (name, text) in [
            (
                "unsupported contract",
                valid.replace("openscad-viewer/brep-1", "openscad-viewer/brep-2"),
            ),
            (
                "duplicated language",
                valid.replace(
                    "// 😀 note",
                    "// @language openscad-viewer/brep-1\n// 😀 note",
                ),
            ),
            (
                "engine selection",
                valid.replace("// 😀 note", "// @engine manifold\n// 😀 note"),
            ),
            (
                "malformed reserved",
                valid.replace("// 😀 note", "// @Language\n// 😀 note"),
            ),
            (
                "directive after body",
                format!("{valid}\n// @language legacy/current"),
            ),
        ] {
            let mut request = attested_request();
            request["source"] = descriptor(&text);
            request["sourceText"] = json!(text);
            // The duplicated-language and directive-after-body mutations keep
            // the contract consistent so only the header rule can fire.
            assert!(attest_source(&request).is_err(), "accepted {name}");
        }
        // A @requires header matching the declared capabilities admits.
        let mut request = attested_request();
        let text = valid.replace(
            "// 😀 note",
            "// @requires custom.feature, custom.feature\n// 😀 note",
        );
        request["source"] = descriptor(&text);
        request["sourceText"] = json!(text);
        attest_source(&request).unwrap();
    }

    fn box_node() -> Value {
        json!({"id":0,"kind":"box","size":[1,1,1],"center":false,"valueType":{"geometryKind":"solid","space":"d3","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}}})
    }

    /// Minimal valid component set, mirrored as one envelope object.
    fn envelope_request() -> Value {
        json!({
            "source": {"sha256":"a".repeat(64),"utf8ByteLength":16,"utf16CodeUnitLength":16},
            "schema": "semantic-program-core",
            "schemaVersion": {"major":1,"minor":2},
            "requiredFeatures": ["semantic.execution-v2"],
            "identityVersion": "semantic-program-core-v1",
            "language": {"contract":"openscad-viewer/brep-1","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"},
            "units": {"length":"millimeter","angle":"degree","handedness":"right","upAxis":"z","matrixLayout":"column-major","composition":"parent-times-local"},
            "declaredCapabilities": [],
            "capabilityClosure": [
                "construct.box",
                "evidence.representation-preserving",
                "geometry.kind.solid",
                "geometry.space.d3",
                "geometry.value",
                "representation.analytic-brep",
                "semantic.identity-evidence",
                "semantic.operation-graph",
                "semantic.program-v1",
                "semantic.result.single",
            ],
            "result": {"tag":"single","item":{"node":0,"producerOccurrence":null,"identityOccurrence":null,"color":null}},
            "occurrences": [],
            "diagnosticTemplates": [],
            "tessellationIntents": [],
            "diagnostics": [],
            "operations": [],
            "provenance": [],
            "nodes": [box_node()],
        })
    }

    fn envelope_object(request: &Value) -> Value {
        let mut core = json!({});
        for key in CORE_KEYS {
            core[key] = request[key].clone();
        }
        let mut envelope = json!({
            "schema": "semantic-program-envelope",
            "schemaVersion": request["schemaVersion"].clone(),
            "core": core,
        });
        for key in COMPONENT_KEYS {
            envelope[key] = request[key].clone();
        }
        envelope
    }

    fn nodes() -> Vec<Value> {
        vec![box_node()]
    }

    #[test]
    fn admits_a_valid_unified_envelope_object() {
        let mut request = envelope_request();
        request["envelope"] = envelope_object(&request);
        super::super::brep_envelope::validate(&request, &nodes()).unwrap();
        // Envelope-only transport: components carried only inside the
        // envelope are adopted into admission (nodes stay top-level because
        // they are the execution input).
        let mut envelope_only = json!({"nodes": [box_node()]});
        envelope_only["envelope"] = envelope_object(&envelope_request());
        super::super::brep_envelope::validate(&envelope_only, &nodes()).unwrap();
    }

    #[test]
    fn refuses_envelope_key_and_literal_mutations() {
        for (name, mutate) in [
            ("extra key", json!("extra")),
            ("missing diagnostics", json!("missing")),
            ("wrong schema literal", json!("literal")),
            ("wrong schema version", json!("version")),
            ("extra core key", json!("core-extra")),
            ("missing core key", json!("core-missing")),
        ] {
            let mut request = envelope_request();
            let mut envelope = envelope_object(&request);
            match mutate.as_str().unwrap() {
                "extra" => {
                    envelope
                        .as_object_mut()
                        .unwrap()
                        .insert("extra".into(), json!(true));
                }
                "missing" => {
                    envelope.as_object_mut().unwrap().remove("diagnostics");
                }
                "literal" => envelope["schema"] = json!("semantic-program-envelope-v2"),
                "version" => envelope["schemaVersion"] = json!({"major":1,"minor":3}),
                "core-extra" => {
                    envelope["core"]
                        .as_object_mut()
                        .unwrap()
                        .insert("extra".into(), json!(true));
                }
                "core-missing" => {
                    envelope["core"].as_object_mut().unwrap().remove("units");
                }
                _ => unreachable!(),
            }
            request["envelope"] = envelope;
            assert!(
                super::super::brep_envelope::validate(&request, &nodes()).is_err(),
                "accepted {name}"
            );
        }
    }

    #[test]
    fn refuses_envelope_component_mismatches() {
        // A valid-but-different envelope component refuses even when each
        // copy would pass alone: envelope declares an extra capability that
        // the transported duplicate does not.
        let mut request = envelope_request();
        let mut envelope = envelope_object(&request);
        envelope["core"]["declaredCapabilities"] = json!(["custom.feature"]);
        envelope["core"]["capabilityClosure"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("custom.feature"));
        request["envelope"] = envelope;
        assert!(super::super::brep_envelope::validate(&request, &nodes()).is_err());
        // Nodes inside the envelope must match the executing top-level nodes.
        let mut request = envelope_request();
        let mut envelope = envelope_object(&request);
        envelope["core"]["nodes"] = json!([]);
        request["envelope"] = envelope;
        assert!(super::super::brep_envelope::validate(&request, &nodes()).is_err());
        // A tampered envelope-only provenance row is adopted and refused.
        let mut envelope_only = json!({"nodes": [box_node()]});
        let mut envelope = envelope_object(&envelope_request());
        envelope["provenance"] =
            json!([{"operation":0,"span":{"start":0,"end":1},"label":"forged"}]);
        envelope_only["envelope"] = envelope;
        assert!(super::super::brep_envelope::validate(&envelope_only, &nodes()).is_err());
    }

    #[test]
    fn admits_source_text_through_full_envelope_validation() {
        let mut request = envelope_request();
        let source = source_text();
        request["source"] = descriptor(&source);
        request["declaredCapabilities"] = json!(["custom.feature"]);
        request["capabilityClosure"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("custom.feature"));
        request["sourceText"] = json!(source);
        request["envelope"] = envelope_object(&request);
        let nodes = nodes();
        super::super::brep_envelope::validate(&request, &nodes).unwrap();
        // A tampered transported text refuses before any node executes.
        let mut tampered = envelope_request();
        let source = format!("{} ", source_text());
        tampered["source"] = descriptor(&source_text());
        tampered["declaredCapabilities"] = json!(["custom.feature"]);
        tampered["capabilityClosure"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!("custom.feature"));
        tampered["sourceText"] = json!(source);
        tampered["envelope"] = envelope_object(&tampered);
        assert!(super::super::brep_envelope::validate(&tampered, &nodes).is_err());
    }
}
