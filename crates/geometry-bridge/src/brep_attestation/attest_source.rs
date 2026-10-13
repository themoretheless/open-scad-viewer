use super::*;

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
                    .any(|id| !super::super::brep_envelope::capability_identifier(id))
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
