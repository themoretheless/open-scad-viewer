//! Native admission of envelope provenance, tessellationIntents, diagnostics
//! and diagnosticTemplates, validated against the already-admitted operations,
//! occurrences, language contract and source descriptor.
//!
//! Port of the envelope-component subsets of validateSemanticProgramV1 and
//! validateDiagnosticTemplates in src/services/semanticProgramValidator.ts.
//! Out of scope (host-side): the UTF-16 span surrogate-pair endpoint check in
//! attestSemanticProgramSource needs the actual source text, which is not
//! transported — only the source descriptor is.
use super::{Result, Value};
use std::collections::HashSet;
use value_codec::Number;

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
        .ok_or_else(|| invalid("Expected a semantic envelope string"))?;
    if utf16_len(text) > maximum {
        return Err(invalid("Semantic envelope string limit exceeded"));
    }
    Ok(text)
}

/// `^[A-Z][A-Z0-9_]{0,63}$` from validateDiagnosticTemplates.
fn diagnostic_code(text: &str) -> bool {
    let bytes = text.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

/// Half-open UTF-16 code-unit span bounded by the source descriptor length.
/// The surrogate-pair endpoint check stays host-side: it needs the source
/// text, and only the descriptor crosses the ABI.
fn span(value: &Value, source_length: u64, empty_allowed: bool) -> Result<()> {
    exact(value, &["start", "end"])?;
    let start = bounded_integer(&value["start"], source_length)?;
    let end = bounded_integer(&value["end"], source_length)?;
    if end < start || (!empty_allowed && end == start) {
        return Err(invalid("Invalid half-open UTF-16 semantic source span"));
    }
    Ok(())
}

/// nullablePositive: null or a finite positive binary64 (TS also refuses -0,
/// which is not positive).
fn positive_or_null(value: &Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    let positive = match value {
        Value::Number(Number::Unsigned(v)) => *v > 0,
        Value::Number(Number::Signed(v)) => *v > 0,
        Value::Number(Number::Float(v)) => v.is_finite() && *v > 0.0,
        _ => false,
    };
    if !positive {
        return Err(invalid("Expected a positive semantic tolerance"));
    }
    Ok(())
}

/// minSegments/maxSegments: null or an integer in [3, 1_000_000].
fn segments(value: &Value) -> Result<Option<u64>> {
    if value.is_null() {
        return Ok(None);
    }
    let count = value
        .as_u64()
        .ok_or_else(|| invalid("Expected a semantic segment count"))?;
    if !(3..=1_000_000).contains(&count) {
        return Err(invalid(
            "Semantic segment counts must be integers in [3, 1000000]",
        ));
    }
    Ok(Some(count))
}

/// validateDiagnosticTemplates: positional IDs, code pattern, severity enum,
/// the v1.2 frozen-error rule, bounded operation references and argument
/// discipline.
fn validate_diagnostic_templates(templates: &[Value], operation_count: usize) -> Result<()> {
    if templates.len() > 10_000 {
        return Err(invalid("Semantic diagnostic template limit exceeded"));
    }
    for (index, template) in templates.iter().enumerate() {
        exact(
            template,
            &["id", "code", "severity", "operation", "arguments"],
        )?;
        if template["id"].as_u64() != Some(index as u64) {
            return Err(invalid(
                "Semantic diagnostic template IDs must equal array positions",
            ));
        }
        let code = bounded_string(&template["code"], 64)?;
        if !diagnostic_code(code) {
            return Err(invalid("Invalid semantic diagnostic code"));
        }
        let severity = template["severity"]
            .as_str()
            .ok_or_else(|| invalid("Expected a semantic diagnostic severity"))?;
        if !matches!(severity, "info" | "warning" | "error") {
            return Err(invalid("Unknown semantic diagnostic severity"));
        }
        if severity == "error" && code != "LEGACY_LANGUAGE_ERROR" {
            return Err(invalid(
                "Semantic v1.2 admits only the frozen legacy terminal error template",
            ));
        }
        if !template["operation"].is_null() {
            let operation = template["operation"]
                .as_u64()
                .ok_or_else(|| invalid("Semantic diagnostic operation reference is invalid"))?;
            if operation_count == 0 {
                return Err(invalid(
                    "Semantic diagnostic references an absent operation",
                ));
            }
            if operation >= operation_count as u64 {
                return Err(invalid(
                    "Semantic diagnostic operation reference is invalid",
                ));
            }
        }
        let arguments = template["arguments"]
            .as_array()
            .ok_or_else(|| invalid("Expected semantic diagnostic arguments"))?;
        if arguments.len() > 128 {
            return Err(invalid("Semantic diagnostic argument limit exceeded"));
        }
        let mut names: HashSet<&str> = HashSet::with_capacity(arguments.len());
        for argument in arguments {
            exact(argument, &["name", "value"])?;
            let name = bounded_string(&argument["name"], 128)?;
            if !names.insert(name) {
                return Err(invalid(
                    "Semantic diagnostic argument names must be unique in authored order",
                ));
            }
            super::brep_identity::validate_identity_value(&argument["value"], 0)?;
        }
    }
    Ok(())
}

/// Envelope provenance: exactly one source record per operation, in operation
/// order, with non-empty half-open spans bounded by the source descriptor.
fn validate_provenance(
    provenance: &[Value],
    operations: Option<&[Value]>,
    source_length: Option<u64>,
) -> Result<()> {
    if provenance.len() > 50_000 {
        return Err(invalid("Semantic provenance limit exceeded"));
    }
    let operations =
        operations.ok_or_else(|| invalid("Semantic provenance requires semantic operations"))?;
    if provenance.len() != operations.len() {
        return Err(invalid(
            "Every static operation needs exactly one source record",
        ));
    }
    let source_length = source_length
        .ok_or_else(|| invalid("Semantic provenance requires the source descriptor"))?;
    for (index, record) in provenance.iter().enumerate() {
        exact(record, &["operation", "span", "label"])?;
        if record["operation"].as_u64() != Some(index as u64) {
            return Err(invalid("Semantic provenance must follow operation order"));
        }
        span(&record["span"], source_length, false)?;
        bounded_string(&record["label"], 512)?;
    }
    Ok(())
}

/// Envelope tessellationIntents: forbidden under legacy/current, unique
/// ascending producing-occurrence references, positive-or-null tolerances and
/// bounded segment counts with max >= min.
fn validate_tessellation_intents(request: &Value, intents: &[Value]) -> Result<()> {
    if intents.len() > 50_000 {
        return Err(invalid("Semantic tessellation intent limit exceeded"));
    }
    if intents.is_empty() {
        return Ok(());
    }
    let contract = request
        .get("language")
        .and_then(|language| language["contract"].as_str())
        .ok_or_else(|| invalid("Semantic tessellation intents require the language contract"))?;
    if contract == "legacy/current" {
        return Err(invalid(
            "legacy/current materializes tessellation in SPC1 and cannot carry TSP1 intents",
        ));
    }
    let occurrences = request
        .get("occurrences")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Semantic tessellation intents require semantic occurrences"))?;
    let mut prior: Option<u64> = None;
    for intent in intents {
        exact(
            intent,
            &[
                "occurrence",
                "chordTolerance",
                "angularToleranceDegrees",
                "minSegments",
                "maxSegments",
            ],
        )?;
        let occurrence = intent["occurrence"]
            .as_u64()
            .ok_or_else(|| invalid("Semantic tessellation occurrence reference is invalid"))?;
        if occurrence >= occurrences.len() as u64 {
            return Err(invalid(
                "Semantic tessellation occurrence reference is invalid",
            ));
        }
        if occurrences[occurrence as usize]["node"].is_null() {
            return Err(invalid(
                "Semantic tessellation intent needs a producing occurrence",
            ));
        }
        if prior.is_some_and(|prior| occurrence <= prior) {
            return Err(invalid(
                "Semantic tessellation intents must be unique and ordered by occurrence",
            ));
        }
        prior = Some(occurrence);
        positive_or_null(&intent["chordTolerance"])?;
        positive_or_null(&intent["angularToleranceDegrees"])?;
        let minimum = segments(&intent["minSegments"])?;
        let maximum = segments(&intent["maxSegments"])?;
        if let (Some(minimum), Some(maximum)) = (minimum, maximum) {
            if maximum < minimum {
                return Err(invalid("maxSegments cannot be below minSegments"));
            }
        }
    }
    Ok(())
}

/// Envelope diagnostics: exactly one presentation per diagnostic template, in
/// template order, with bounded messages and nullable bounded spans.
fn validate_diagnostics(
    request: &Value,
    diagnostics: &[Value],
    source_length: Option<u64>,
) -> Result<()> {
    if diagnostics.len() > 10_000 {
        return Err(invalid("Semantic diagnostic limit exceeded"));
    }
    let templates = request
        .get("diagnosticTemplates")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Semantic diagnostics require diagnostic templates"))?;
    if diagnostics.len() != templates.len() {
        return Err(invalid(
            "Every diagnostic template needs exactly one source-bound presentation",
        ));
    }
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        exact(diagnostic, &["template", "message", "span"])?;
        if diagnostic["template"].as_u64() != Some(index as u64) {
            return Err(invalid("Semantic diagnostics must follow template order"));
        }
        bounded_string(&diagnostic["message"], 1_000_000)?;
        if !diagnostic["span"].is_null() {
            let source_length = source_length
                .ok_or_else(|| invalid("Semantic diagnostics require the source descriptor"))?;
            span(&diagnostic["span"], source_length, true)?;
        }
    }
    Ok(())
}

/// Optional-but-strict admission of the transported envelope components.
/// Absent components skip their checks; each present component is validated
/// against the admitted operations/occurrences/templates and the source
/// descriptor before any graph node executes.
pub fn validate(request: &Value, operations: Option<&[Value]>) -> Result<()> {
    let source_length: Option<u64> = request
        .get("source")
        .and_then(|source| source["utf16CodeUnitLength"].as_u64());
    if let Some(templates) = request.get("diagnosticTemplates") {
        let templates = templates
            .as_array()
            .ok_or_else(|| invalid("Expected a semantic diagnostic template array"))?;
        validate_diagnostic_templates(templates, operations.map_or(0, <[Value]>::len))?;
    }
    if let Some(provenance) = request.get("provenance") {
        let provenance = provenance
            .as_array()
            .ok_or_else(|| invalid("Expected a semantic provenance array"))?;
        validate_provenance(provenance, operations, source_length)?;
    }
    if let Some(intents) = request.get("tessellationIntents") {
        let intents = intents
            .as_array()
            .ok_or_else(|| invalid("Expected a semantic tessellation intent array"))?;
        validate_tessellation_intents(request, intents)?;
    }
    if let Some(diagnostics) = request.get("diagnostics") {
        let diagnostics = diagnostics
            .as_array()
            .ok_or_else(|| invalid("Expected a semantic diagnostic array"))?;
        validate_diagnostics(request, diagnostics, source_length)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    /// Placeholder operation rows: this module reads only the operation count.
    fn operations() -> Vec<Value> {
        json!([{"id":0},{"id":1},{"id":2}])
            .as_array()
            .unwrap()
            .clone()
    }
    fn program_request() -> Value {
        json!({
            "source":{"sha256":"a".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":64},
            "language":{"contract":"openscad-viewer/brep-1","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"},
            "occurrences":[{"id":0,"node":0},{"id":1,"node":1},{"id":2,"node":2}],
            "diagnosticTemplates":[
                {"id":0,"code":"LEGACY_LANGUAGE_ERROR","severity":"error","operation":null,"arguments":[{"name":"detail","value":{"tag":"string","value":"boom"}}]},
                {"id":1,"code":"AMBIGUOUS_GEOMETRY","severity":"warning","operation":2,"arguments":[]},
            ],
            "provenance":[
                {"operation":0,"span":{"start":0,"end":8},"label":"cube"},
                {"operation":1,"span":{"start":9,"end":30},"label":"translate"},
                {"operation":2,"span":{"start":31,"end":64},"label":"difference"},
            ],
            "tessellationIntents":[
                {"occurrence":0,"chordTolerance":0.1,"angularToleranceDegrees":null,"minSegments":3,"maxSegments":64},
                {"occurrence":2,"chordTolerance":null,"angularToleranceDegrees":2.5,"minSegments":null,"maxSegments":null},
            ],
            "diagnostics":[
                {"template":0,"message":"legacy failure","span":null},
                {"template":1,"message":"ambiguous","span":{"start":9,"end":30}},
            ],
        })
    }
    fn admit(request: &Value) -> Result<()> {
        validate(request, Some(&operations()))
    }

    #[test]
    fn admits_a_complete_valid_component_set() {
        admit(&program_request()).unwrap();
        // Absent components skip admission entirely.
        validate(&json!({"nodes":[]}), None).unwrap();
        // Empty intents are admissible even without language/occurrence context.
        admit(&json!({"tessellationIntents":[]})).unwrap();
        // Diagnostic spans are nullable and may be empty half-open intervals.
        let mut empty_span = program_request();
        empty_span["diagnostics"][1]["span"] = json!({"start":9,"end":9});
        admit(&empty_span).unwrap();
        // Segment-count boundaries are inclusive.
        let mut boundaries = program_request();
        boundaries["tessellationIntents"][0]["minSegments"] = json!(3);
        boundaries["tessellationIntents"][0]["maxSegments"] = json!(1_000_000);
        admit(&boundaries).unwrap();
    }

    #[test]
    fn refuses_provenance_violations() {
        // Wrong-by-one counts.
        let mut dropped = program_request();
        dropped["provenance"].as_array_mut().unwrap().remove(0);
        assert!(admit(&dropped).is_err());
        let mut extended = program_request();
        extended["provenance"]
            .as_array_mut()
            .unwrap()
            .push(json!({"operation":3,"span":{"start":0,"end":1},"label":"extra"}));
        assert!(admit(&extended).is_err());
        // Operation order.
        let mut reordered = program_request();
        reordered["provenance"][0]["operation"] = json!(1);
        assert!(admit(&reordered).is_err());
        // Empty, reversed and out-of-range spans.
        for span in [
            json!({"start":8,"end":8}),
            json!({"start":8,"end":7}),
            json!({"start":0,"end":65}),
            json!({"start":65,"end":64}),
            json!({"start":-1,"end":8}),
        ] {
            let mut request = program_request();
            request["provenance"][0]["span"] = span;
            assert!(admit(&request).is_err());
        }
        // Overlong label and unknown record key.
        let mut request = program_request();
        request["provenance"][0]["label"] = json!("x".repeat(513));
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request["provenance"][0]
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(admit(&request).is_err());
        // Provenance requires the source descriptor and the operations.
        let mut request = program_request();
        request.as_object_mut().unwrap().remove("source");
        assert!(admit(&request).is_err());
        assert!(validate(&program_request(), None).is_err());
    }

    #[test]
    fn refuses_tessellation_intent_violations() {
        // Non-ascending and duplicate occurrence references.
        let mut swapped = program_request();
        swapped["tessellationIntents"] = json!([
            {"occurrence":2,"chordTolerance":null,"angularToleranceDegrees":null,"minSegments":null,"maxSegments":null},
            {"occurrence":0,"chordTolerance":null,"angularToleranceDegrees":null,"minSegments":null,"maxSegments":null},
        ]);
        assert!(admit(&swapped).is_err());
        let mut duplicated = program_request();
        duplicated["tessellationIntents"][1]["occurrence"] = json!(0);
        assert!(admit(&duplicated).is_err());
        // Out-of-range and non-producing occurrence references.
        let mut request = program_request();
        request["tessellationIntents"][1]["occurrence"] = json!(3);
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request["occurrences"][2]["node"] = json!(null);
        assert!(admit(&request).is_err());
        // Non-positive tolerances.
        for (key, value) in [
            ("chordTolerance", json!(0)),
            ("chordTolerance", json!(-0.5)),
            ("angularToleranceDegrees", json!(0)),
            ("angularToleranceDegrees", json!("wide")),
        ] {
            let mut request = program_request();
            request["tessellationIntents"][0][key] = value;
            assert!(admit(&request).is_err(), "accepted {key}");
        }
        // Segment bounds and max < min.
        for (key, value) in [
            ("minSegments", json!(2)),
            ("minSegments", json!(1_000_001)),
            ("maxSegments", json!(2.5)),
            ("maxSegments", json!(-3)),
        ] {
            let mut request = program_request();
            request["tessellationIntents"][0][key] = value;
            assert!(admit(&request).is_err(), "accepted {key}");
        }
        let mut request = program_request();
        request["tessellationIntents"][0]["minSegments"] = json!(10);
        request["tessellationIntents"][0]["maxSegments"] = json!(9);
        assert!(admit(&request).is_err());
        // Unknown intent key.
        let mut request = program_request();
        request["tessellationIntents"][0]
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(admit(&request).is_err());
        // legacy/current cannot carry intents.
        let mut legacy = program_request();
        legacy["language"] = json!({"contract":"legacy/current","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"});
        assert!(admit(&legacy).is_err());
        // Non-empty intents require language and occurrence context.
        let mut request = program_request();
        request.as_object_mut().unwrap().remove("language");
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request.as_object_mut().unwrap().remove("occurrences");
        assert!(admit(&request).is_err());
    }

    #[test]
    fn refuses_diagnostics_violations() {
        // Wrong-by-one counts.
        let mut dropped = program_request();
        dropped["diagnostics"].as_array_mut().unwrap().remove(0);
        assert!(admit(&dropped).is_err());
        let mut extended = program_request();
        extended["diagnostics"]
            .as_array_mut()
            .unwrap()
            .push(json!({"template":2,"message":"extra","span":null}));
        assert!(admit(&extended).is_err());
        // Template order.
        let mut reordered = program_request();
        reordered["diagnostics"][0]["template"] = json!(1);
        assert!(admit(&reordered).is_err());
        // Message budget.
        let mut request = program_request();
        request["diagnostics"][0]["message"] = json!("x".repeat(1_000_001));
        assert!(admit(&request).is_err());
        // Out-of-range and reversed spans (empty spans stay admissible).
        for span in [json!({"start":9,"end":65}), json!({"start":30,"end":29})] {
            let mut request = program_request();
            request["diagnostics"][1]["span"] = span;
            assert!(admit(&request).is_err());
        }
        // Diagnostics require the transported templates, and a present span
        // requires the source descriptor.
        let mut request = program_request();
        request
            .as_object_mut()
            .unwrap()
            .remove("diagnosticTemplates");
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request.as_object_mut().unwrap().remove("source");
        assert!(admit(&request).is_err());
    }

    #[test]
    fn refuses_diagnostic_template_violations() {
        // Non-index ID.
        let mut request = program_request();
        request["diagnosticTemplates"][1]["id"] = json!(0);
        assert!(admit(&request).is_err());
        // Code pattern violations.
        for code in ["abc", "1ABC", "A-B", "A B", &"A".repeat(65)] {
            let mut request = program_request();
            request["diagnosticTemplates"][1]["code"] = json!(code);
            assert!(admit(&request).is_err(), "accepted {code}");
        }
        // Severity outside the enum and a non-legacy error template under v1.2.
        let mut request = program_request();
        request["diagnosticTemplates"][1]["severity"] = json!("fatal");
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request["diagnosticTemplates"][1]["severity"] = json!("error");
        assert!(admit(&request).is_err());
        // Operation reference out of range, and any reference without operations.
        let mut request = program_request();
        request["diagnosticTemplates"][1]["operation"] = json!(3);
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request.as_object_mut().unwrap().remove("provenance");
        request["diagnosticTemplates"][1]["operation"] = json!(0);
        assert!(validate(&request, None).is_err());
        // Argument discipline: count, unique names, name length, value tags.
        let mut request = program_request();
        let arguments: Vec<Value> = (0..129)
            .map(|index| json!({"name":format!("a{index}"),"value":{"tag":"null"}}))
            .collect();
        request["diagnosticTemplates"][1]["arguments"] = json!(arguments);
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request["diagnosticTemplates"][0]["arguments"] = json!([
            {"name":"detail","value":{"tag":"null"}},
            {"name":"detail","value":{"tag":"null"}},
        ]);
        assert!(admit(&request).is_err());
        let mut request = program_request();
        request["diagnosticTemplates"][0]["arguments"] = json!([
            {"name":"x".repeat(129),"value":{"tag":"null"}},
        ]);
        assert!(admit(&request).is_err());
        for value in [
            json!({"tag":"bogus"}),
            json!({"tag":"number","value":-0.0}),
            json!({"tag":"string","value":"x".repeat(4_097)}),
        ] {
            let mut request = program_request();
            request["diagnosticTemplates"][0]["arguments"] =
                json!([{"name":"detail","value":value}]);
            assert!(admit(&request).is_err());
        }
        // Unknown template key.
        let mut request = program_request();
        request["diagnosticTemplates"][0]
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(admit(&request).is_err());
    }
}
