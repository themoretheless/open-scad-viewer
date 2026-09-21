//! Native admission of SemanticProgram envelope source identity, core literal
//! fields, declared capabilities and the exact derived capability closure.
//! Operation/occurrence identity digests are admitted in brep_identity.rs;
//! provenance, tessellationIntents, diagnostics and diagnosticTemplates are
//! admitted in brep_provenance.rs; the unified envelope object and source
//! attestation (digest/lengths, routing header, surrogate-pair span
//! endpoints) are admitted in brep_attestation.rs.
use super::{Result, Value};
use std::collections::BTreeSet;

fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

fn exact(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected a semantic contract object"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid(
            "Semantic contract fields do not match the envelope schema",
        ));
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

/// `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$` from the host validator.
pub(crate) fn capability_identifier(text: &str) -> bool {
    let bytes = text.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// Unique, UTF-8-byte sorted identifier list, mirroring validateSortedStrings.
fn sorted_identifiers(value: &Value, maximum: usize) -> Result<Vec<String>> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid("Expected a semantic capability array"))?;
    if items.len() > maximum {
        return Err(invalid("Semantic capability limit exceeded"));
    }
    let mut out: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        let text = item
            .as_str()
            .ok_or_else(|| invalid("Expected a semantic capability string"))?;
        if !capability_identifier(text) {
            return Err(invalid("Invalid semantic capability identifier"));
        }
        if out.last().is_some_and(|last| last.as_str() >= text) {
            return Err(invalid(
                "Semantic capabilities must be unique and sorted by raw UTF-8 bytes",
            ));
        }
        out.push(text.to_owned());
    }
    Ok(out)
}

fn validate_source(source: &Value) -> Result<()> {
    exact(source, &["sha256", "utf16CodeUnitLength", "utf8ByteLength"])?;
    let sha256 = source["sha256"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic source digest"))?;
    let bytes = sha256.as_bytes();
    if bytes.len() != 64
        || !bytes
            .iter()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid(
            "Semantic source digest must be a lowercase SHA-256",
        ));
    }
    bounded_integer(&source["utf8ByteLength"], 4_000_000)?;
    bounded_integer(&source["utf16CodeUnitLength"], 250_000)?;
    Ok(())
}

pub(crate) fn validate_schema_version(version: &Value) -> Result<()> {
    exact(version, &["major", "minor"])?;
    let major = bounded_integer(&version["major"], u64::MAX)?;
    let minor = bounded_integer(&version["minor"], u64::MAX)?;
    if major == 1 && minor <= 1 {
        return Err(invalid(
            "Semantic programs before 1.2 require exact-source re-lowering",
        ));
    }
    if major != 1 || minor != 2 {
        return Err(invalid("Only semantic schema version 1.2 is supported"));
    }
    Ok(())
}

fn validate_language(language: &Value) -> Result<()> {
    exact(
        language,
        &["capabilityGraphVersion", "contract", "semanticsRevision"],
    )?;
    let contract = language["contract"]
        .as_str()
        .ok_or_else(|| invalid("Expected a semantic language contract"))?;
    let expected = match contract {
        "legacy/current" => "1.0.0",
        "openscad-viewer/brep-1" => "brep-1.0.0",
        _ => return Err(invalid("Unknown semantic language contract")),
    };
    if language["semanticsRevision"].as_str() != Some(expected) {
        return Err(invalid(
            "Semantic language revision does not match its contract",
        ));
    }
    if language["capabilityGraphVersion"].as_str() != Some("semantic-capabilities-v1") {
        return Err(invalid("Unknown semantic capability graph"));
    }
    Ok(())
}

fn validate_units(units: &Value) -> Result<()> {
    exact(
        units,
        &[
            "angle",
            "composition",
            "handedness",
            "length",
            "matrixLayout",
            "upAxis",
        ],
    )?;
    let fixed = [
        ("length", "millimeter"),
        ("angle", "degree"),
        ("handedness", "right"),
        ("upAxis", "z"),
        ("matrixLayout", "column-major"),
        ("composition", "parent-times-local"),
    ];
    if fixed
        .iter()
        .any(|(key, want)| units[*key].as_str() != Some(*want))
    {
        return Err(invalid(
            "Semantic v1 units/frame/matrix convention is fixed",
        ));
    }
    Ok(())
}

fn capability_by_kind(kind: &str) -> Result<&'static str> {
    Ok(match kind {
        "box" => "construct.box",
        "sphere-analytic" => "construct.sphere.analytic",
        "sphere-polygonal" => "construct.sphere.polygonal",
        "cylinder-analytic" => "construct.cylinder.analytic",
        "cylinder-polygonal" => "construct.cylinder.polygonal",
        "polyhedron" => "construct.polyhedron",
        "rectangle" => "construct.rectangle",
        "circle-analytic" => "construct.circle.analytic",
        "circle-polygonal" => "construct.circle.polygonal",
        "polygon" => "construct.polygon",
        "transform" => "operation.transform",
        "boolean" => "operation.boolean",
        "hull" => "operation.hull",
        "linear-extrude" => "operation.linear-extrude",
        "rotate-extrude-analytic" => "operation.rotate-extrude.analytic",
        "rotate-extrude-polygonal" => "operation.rotate-extrude.polygonal",
        "projection" => "operation.projection",
        "offset" => "operation.offset",
        _ => return Err(invalid("Unknown semantic node kind")),
    })
}

struct Signature<'a> {
    geometry_kind: &'a str,
    space: &'a str,
    representation: &'a str,
    evidence: &'a str,
    certificate_profile: Option<&'a str>,
    certificate_policy_hash: Option<&'a str>,
}

fn signature(node: &Value) -> Result<Signature<'_>> {
    let value_type = node
        .get("valueType")
        .ok_or_else(|| invalid("Semantic node is missing its value type"))?;
    let missing = || invalid("Semantic node value type is incomplete");
    let evidence = value_type["evidence"]["tag"].as_str().ok_or_else(missing)?;
    let (certificate_profile, certificate_policy_hash) = if evidence == "certified-approximation" {
        (
            Some(
                value_type["evidence"]["certificateProfile"]
                    .as_str()
                    .ok_or_else(missing)?,
            ),
            Some(
                value_type["evidence"]["certificatePolicyHash"]
                    .as_str()
                    .ok_or_else(missing)?,
            ),
        )
    } else {
        (None, None)
    };
    Ok(Signature {
        geometry_kind: value_type["geometryKind"].as_str().ok_or_else(missing)?,
        space: value_type["space"].as_str().ok_or_else(missing)?,
        representation: value_type["representation"].as_str().ok_or_else(missing)?,
        evidence,
        certificate_profile,
        certificate_policy_hash,
    })
}

/// semanticNodeInputs: authored order is semantic and is never sorted.
pub(crate) fn inputs(node: &Value, kind: &str) -> Result<Vec<usize>> {
    let index = |value: &Value| {
        value
            .as_u64()
            .and_then(|x| usize::try_from(x).ok())
            .ok_or_else(|| invalid("Semantic node input reference is invalid"))
    };
    Ok(match kind {
        "transform"
        | "linear-extrude"
        | "rotate-extrude-analytic"
        | "rotate-extrude-polygonal"
        | "projection"
        | "offset" => vec![index(&node["input"])?],
        "boolean" | "hull" => node["inputs"]
            .as_array()
            .ok_or_else(|| invalid("Semantic node is missing its inputs"))?
            .iter()
            .map(index)
            .collect::<Result<Vec<_>>>()?,
        _ => Vec::new(),
    })
}

/// Exact port of deriveSemanticCapabilityClosure from src/core/semanticProgram.ts.
/// BTreeSet iteration is raw UTF-8 byte order, matching the host utf8Compare sort.
pub fn derive_capability_closure(
    declared: &[String],
    nodes: &[Value],
    result_tag: &str,
    occurrence_count: usize,
    diagnostic_template_count: usize,
) -> Result<Vec<String>> {
    let mut capabilities: BTreeSet<String> = declared.iter().cloned().collect();
    capabilities.insert("semantic.program-v1".into());
    capabilities.insert("semantic.operation-graph".into());
    capabilities.insert("semantic.identity-evidence".into());
    capabilities.insert(format!("semantic.result.{result_tag}"));
    if occurrence_count > 0 {
        capabilities.insert("semantic.occurrences".into());
    }
    if diagnostic_template_count > 0 {
        capabilities.insert("diagnostics.deterministic".into());
    }
    for node in nodes {
        let kind = node["kind"]
            .as_str()
            .ok_or_else(|| invalid("Semantic node is missing its kind"))?;
        let own = signature(node)?;
        capabilities.insert(capability_by_kind(kind)?.into());
        capabilities.insert(format!("geometry.kind.{}", own.geometry_kind));
        capabilities.insert(format!("geometry.space.{}", own.space));
        capabilities.insert(format!("representation.{}", own.representation));
        capabilities.insert(format!("evidence.{}", own.evidence));
        if let (Some(profile), Some(policy)) =
            (own.certificate_profile, own.certificate_policy_hash)
        {
            capabilities.insert(format!("evidence.profile.{profile}"));
            capabilities.insert(format!("evidence.policy.sha256.{policy}"));
        }
        capabilities.insert("geometry.value".into());
        if kind == "boolean" {
            let operation = node["operation"]
                .as_str()
                .ok_or_else(|| invalid("Semantic Boolean node is missing its operation"))?;
            capabilities.insert(format!("operation.boolean.{operation}"));
            capabilities.insert("operation.boolean".into());
        }
        for input_index in inputs(node, kind)? {
            let input = nodes
                .get(input_index)
                .ok_or_else(|| invalid("Semantic node input reference is out of range"))?;
            let source = signature(input)?;
            if source.geometry_kind != own.geometry_kind || source.space != own.space {
                capabilities.insert(format!(
                    "geometry.transition.{}.{}.to.{}.{}",
                    source.geometry_kind, source.space, own.geometry_kind, own.space
                ));
            }
            if source.representation != own.representation {
                capabilities.insert(format!(
                    "representation.transition.{}.to.{}",
                    source.representation, own.representation
                ));
            }
            if source.evidence != own.evidence {
                capabilities.insert(format!(
                    "evidence.transition.{}.to.{}",
                    source.evidence, own.evidence
                ));
            }
        }
    }
    Ok(capabilities.into_iter().collect())
}

/// Optional-but-strict admission of envelope components on a graph request.
/// Requests without these fields execute unchanged; each present field is
/// validated before any graph node executes. A present `envelope` object is
/// admitted first (exact SemanticProgramEnvelopeV1 keys, the
/// semantic-program-envelope schema literal and exact duplicate consistency
/// with the separately transported components; envelope-only components are
/// adopted into admission) and a present `sourceText` attests the source
/// descriptor, the routing header and the UTF-16 span endpoints natively.
pub fn validate(request: &Value, nodes: &[Value]) -> Result<()> {
    let merged: Value;
    let request: &Value = match request.get("envelope") {
        Some(envelope) => {
            merged = super::brep_attestation::admit_envelope_object(request, envelope)?;
            &merged
        }
        None => request,
    };
    if let Some(source) = request.get("source") {
        validate_source(source)?;
    }
    if let Some(schema) = request.get("schema")
        && schema.as_str() != Some("semantic-program-core") {
            return Err(invalid("Expected the semantic-program-core schema"));
        }
    if let Some(version) = request.get("schemaVersion") {
        validate_schema_version(version)?;
    }
    if let Some(features) = request.get("requiredFeatures") {
        let features = sorted_identifiers(features, 64)?;
        if features.len() != 1 || features[0] != "semantic.execution-v2" {
            return Err(invalid(
                "Semantic v1.2 requires exactly semantic.execution-v2",
            ));
        }
    }
    if let Some(identity) = request.get("identityVersion")
        && identity.as_str() != Some("semantic-program-core-v1") {
            return Err(invalid("Unknown semantic core identity version"));
        }
    if let Some(language) = request.get("language") {
        validate_language(language)?;
    }
    if let Some(units) = request.get("units") {
        validate_units(units)?;
    }
    let declared = request
        .get("declaredCapabilities")
        .map(|value| sorted_identifiers(value, 32))
        .transpose()?;
    if let Some(closure) = request.get("capabilityClosure") {
        let closure = sorted_identifiers(closure, 128)?;
        let declared = declared
            .ok_or_else(|| invalid("Semantic capability closure requires declared capabilities"))?;
        let result_tag = request
            .get("result")
            .and_then(|result| result["tag"].as_str())
            .ok_or_else(|| invalid("Semantic capability closure requires the semantic result"))?;
        let occurrence_count = request
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("Semantic capability closure requires semantic occurrences"))?
            .len();
        let diagnostic_template_count = request
            .get("diagnosticTemplates")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let expected = derive_capability_closure(
            &declared,
            nodes,
            result_tag,
            occurrence_count,
            diagnostic_template_count,
        )?;
        if closure != expected {
            return Err(invalid(
                "Semantic capability closure does not exactly match authored and inferred capabilities",
            ));
        }
    }
    // Optional-but-strict operation/occurrence identity admission. Absent
    // operations skip both checks; occurrence rows cannot be re-derived
    // without their operation IDs.
    let operations: Option<&[Value]> = request
        .get("operations")
        .map(|value| {
            value
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| invalid("Expected a semantic operation array"))
        })
        .transpose()?;
    if let Some(operations) = operations {
        super::brep_identity::validate_operations(operations)?;
        if let Some(occurrences) = request.get("occurrences") {
            let occurrences = occurrences
                .as_array()
                .ok_or_else(|| invalid("Expected a semantic occurrence array"))?;
            super::brep_identity::validate_occurrence_identity(
                occurrences,
                operations,
                nodes.len(),
            )?;
            // Optional-but-strict occurrence-production replay: with
            // occurrences present the semantic result and execution plan must
            // be transported too (they always are on the executor path).
            // Empty-occurrence programs carry no production proof (an empty
            // program lowers to zero operations, occurrences and nodes).
            if !occurrences.is_empty() {
                let result = request
                    .get("result")
                    .filter(|value| value.is_object())
                    .ok_or_else(|| {
                        invalid(
                            "Semantic occurrence production replay requires the semantic result",
                        )
                    })?;
                let execution = request
                    .get("execution")
                    .filter(|value| value.is_object())
                    .ok_or_else(|| {
                        invalid(
                            "Semantic occurrence production replay requires the semantic execution plan",
                        )
                    })?;
                super::brep_production::validate_occurrence_production(
                    operations,
                    occurrences,
                    nodes,
                    result,
                    execution,
                )?;
            }
        }
    }
    // Optional-but-strict admission of the transported envelope components
    // (provenance, tessellationIntents, diagnostics, diagnosticTemplates)
    // against the admitted operations, occurrences and source descriptor.
    super::brep_provenance::validate(request, operations)?;
    // Optional-but-strict native source attestation: a present sourceText is
    // hashed and measured against the admitted source descriptor, its routing
    // header is parsed and matched, and UTF-16 span endpoints are checked
    // against surrogate-pair boundaries.
    super::brep_attestation::attest_source(request)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    fn solid_type() -> Value {
        json!({"geometryKind":"solid","space":"d3","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}})
    }
    fn region_type() -> Value {
        json!({"geometryKind":"region","space":"d2","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}})
    }
    fn program_nodes() -> Value {
        json!([
            {"id":0,"kind":"box","size":[1,1,1],"center":false,"valueType":solid_type()},
            {"id":1,"kind":"transform","input":0,"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,2,0,0,1],"valueType":solid_type()},
            {"id":2,"kind":"boolean","operation":"union","inputs":[0,1],"valueType":solid_type()},
        ])
    }
    /// String-for-string expectation for box + transform + Boolean union with
    /// one declared capability, a single result and three occurrences.
    fn program_closure() -> Vec<&'static str> {
        vec![
            "construct.box",
            "custom.feature",
            "evidence.representation-preserving",
            "geometry.kind.solid",
            "geometry.space.d3",
            "geometry.value",
            "operation.boolean",
            "operation.boolean.union",
            "operation.transform",
            "representation.analytic-brep",
            "semantic.identity-evidence",
            "semantic.occurrences",
            "semantic.operation-graph",
            "semantic.program-v1",
            "semantic.result.single",
        ]
    }
    fn envelope() -> Value {
        json!({
            "source":{"sha256":"a".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":64},
            "schema":"semantic-program-core",
            "schemaVersion":{"major":1,"minor":2},
            "requiredFeatures":["semantic.execution-v2"],
            "identityVersion":"semantic-program-core-v1",
            "language":{"contract":"openscad-viewer/brep-1","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"},
            "units":{"length":"millimeter","angle":"degree","handedness":"right","upAxis":"z","matrixLayout":"column-major","composition":"parent-times-local"},
            "declaredCapabilities":["custom.feature"],
            "capabilityClosure":program_closure(),
            "result":{"tag":"single","item":{"node":2,"producerOccurrence":0,"identityOccurrence":0,"color":[1,1,1,1]}},
            "occurrences":[{"id":0},{"id":1},{"id":2}],
            "diagnosticTemplates":[],
        })
    }
    fn nodes() -> Vec<Value> {
        program_nodes().as_array().unwrap().clone()
    }

    #[test]
    fn admits_a_complete_valid_brep1_envelope_component_set() {
        validate(&envelope(), &nodes()).unwrap();
        let mut legacy = envelope();
        legacy["language"] = json!({"contract":"legacy/current","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"});
        validate(&legacy, &nodes()).unwrap();
        // Requests without envelope components keep working unchanged.
        validate(&json!({"nodes":[]}), &[]).unwrap();
    }

    #[test]
    fn derives_the_multi_node_closure_string_for_string() {
        let derived =
            derive_capability_closure(&["custom.feature".to_owned()], &nodes(), "single", 3, 0)
                .unwrap();
        assert_eq!(derived, program_closure());
        // 2D profile extrusion crosses a geometry kind/space transition.
        let extrusion = json!([
            {"id":0,"kind":"rectangle","size":[2,3],"center":false,"valueType":region_type()},
            {"id":1,"kind":"linear-extrude","input":0,"height":4,"twistDegrees":0,"slices":1,"scale":[1,1],"center":false,"valueType":solid_type()},
        ]);
        let derived =
            derive_capability_closure(&[], extrusion.as_array().unwrap(), "single", 2, 0).unwrap();
        assert_eq!(
            derived,
            vec![
                "construct.rectangle",
                "evidence.representation-preserving",
                "geometry.kind.region",
                "geometry.kind.solid",
                "geometry.space.d2",
                "geometry.space.d3",
                "geometry.transition.region.d2.to.solid.d3",
                "geometry.value",
                "operation.linear-extrude",
                "representation.analytic-brep",
                "semantic.identity-evidence",
                "semantic.occurrences",
                "semantic.operation-graph",
                "semantic.program-v1",
                "semantic.result.single",
            ]
        );
    }

    #[test]
    fn refuses_source_descriptor_violations() {
        for mutate in [
            json!({"sha256":"A".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":64}),
            json!({"sha256":"a".repeat(63),"utf8ByteLength":128,"utf16CodeUnitLength":64}),
            json!({"sha256":"a".repeat(65),"utf8ByteLength":128,"utf16CodeUnitLength":64}),
            json!({"sha256":"g".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":64}),
            json!({"sha256":"a".repeat(64),"utf8ByteLength":4_000_001,"utf16CodeUnitLength":64}),
            json!({"sha256":"a".repeat(64),"utf8ByteLength":-1,"utf16CodeUnitLength":64}),
            json!({"sha256":"a".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":250_001}),
            json!({"sha256":"a".repeat(64),"utf8ByteLength":128,"utf16CodeUnitLength":64,"extra":true}),
            json!({"sha256":"a".repeat(64),"utf8ByteLength":128}),
        ] {
            let mut request = envelope();
            request["source"] = mutate;
            assert!(validate(&request, &nodes()).is_err());
        }
        let mut boundaries = envelope();
        boundaries["source"] = json!({"sha256":"f".repeat(64),"utf8ByteLength":4_000_000,"utf16CodeUnitLength":250_000});
        validate(&boundaries, &nodes()).unwrap();
        boundaries["source"] =
            json!({"sha256":"0".repeat(64),"utf8ByteLength":0,"utf16CodeUnitLength":0});
        validate(&boundaries, &nodes()).unwrap();
    }

    #[test]
    fn refuses_wrong_core_literals_and_schema_versions() {
        for (key, value) in [
            ("schema", json!("semantic-program-core-v2")),
            ("identityVersion", json!("semantic-program-core-v0")),
            ("requiredFeatures", json!([])),
            ("requiredFeatures", json!(["semantic.execution-v1"])),
            (
                "requiredFeatures",
                json!(["semantic.execution-v2", "semantic.other"]),
            ),
            ("schemaVersion", json!({"major":1,"minor":3})),
            ("schemaVersion", json!({"major":2,"minor":0})),
            ("schemaVersion", json!({"major":1})),
            (
                "language",
                json!({"contract":"unknown","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"}),
            ),
            (
                "language",
                json!({"contract":"legacy/current","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"}),
            ),
            (
                "language",
                json!({"contract":"openscad-viewer/brep-1","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"}),
            ),
            (
                "language",
                json!({"contract":"openscad-viewer/brep-1","semanticsRevision":"brep-1.0.0","capabilityGraphVersion":"semantic-capabilities-v0"}),
            ),
        ] {
            let mut request = envelope();
            request[key] = value;
            assert!(validate(&request, &nodes()).is_err(), "accepted {key}");
        }
        // Schema 1.0/1.1 refuse with the relowering-style message.
        for minor in [0, 1] {
            let mut request = envelope();
            request["schemaVersion"] = json!({"major":1,"minor":minor});
            let error = validate(&request, &nodes()).unwrap_err();
            assert!(error.message.contains("re-lowering"), "{error:?}");
        }
    }

    #[test]
    fn refuses_units_outside_the_fixed_sextet() {
        let mut wrong = envelope();
        wrong["units"]["length"] = json!("meter");
        assert!(validate(&wrong, &nodes()).is_err());
        let mut missing = envelope();
        missing["units"]
            .as_object_mut()
            .unwrap()
            .remove("composition");
        assert!(validate(&missing, &nodes()).is_err());
        let mut extra = envelope();
        extra["units"]
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(validate(&extra, &nodes()).is_err());
    }

    #[test]
    fn refuses_unsorted_duplicate_oversized_and_malformed_capabilities() {
        for declared in [
            json!(["b.cap", "a.cap"]),
            json!(["a.cap", "a.cap"]),
            json!(["bad cap"]),
            json!([".leading"]),
            json!(["x".repeat(129)]),
            json!(
                (0..33)
                    .map(|index| format!("cap{index:02}"))
                    .collect::<Vec<_>>()
            ),
        ] {
            let mut request = envelope();
            request["declaredCapabilities"] = declared;
            assert!(validate(&request, &nodes()).is_err());
        }
        // An added declared capability must also appear in the closure.
        let mut request = envelope();
        request["declaredCapabilities"] = json!(["custom.feature", "zzz.declared"]);
        assert!(validate(&request, &nodes()).is_err());
        // Closure order and limit rules apply before equality.
        for closure in [
            json!(["b.cap", "a.cap"]),
            json!(["a.cap", "a.cap"]),
            json!(
                (0..129)
                    .map(|index| format!("cap{index:03}"))
                    .collect::<Vec<_>>()
            ),
        ] {
            let mut request = envelope();
            request["capabilityClosure"] = closure;
            assert!(validate(&request, &nodes()).is_err());
        }
    }

    #[test]
    fn refuses_every_wrong_by_one_closure_variant() {
        let closure = program_closure();
        // One entry removed.
        for index in [0, 7, closure.len() - 1] {
            let mut shortened: Vec<&str> = closure.clone();
            shortened.remove(index);
            let mut request = envelope();
            request["capabilityClosure"] = json!(shortened);
            assert!(validate(&request, &nodes()).is_err());
        }
        // One entry added at its sorted position.
        let mut extended: Vec<&str> = closure.clone();
        extended.insert(2, "geometry.extra");
        let mut request = envelope();
        request["capabilityClosure"] = json!(extended);
        assert!(validate(&request, &nodes()).is_err());
        // One entry changed in place without disturbing the sort.
        let mut changed = closure.clone();
        changed[1] = "custom.other";
        let mut request = envelope();
        request["capabilityClosure"] = json!(changed);
        assert!(validate(&request, &nodes()).is_err());
        // Result/occurrence context changes the derived closure.
        let mut request = envelope();
        request["result"] = json!({"tag":"empty","type":"never"});
        assert!(validate(&request, &nodes()).is_err());
        let mut request = envelope();
        request["occurrences"] = json!([]);
        assert!(validate(&request, &nodes()).is_err());
        // Closure admission requires its declared/result/occurrence context.
        let mut request = envelope();
        request
            .as_object_mut()
            .unwrap()
            .remove("declaredCapabilities");
        assert!(validate(&request, &nodes()).is_err());
        let mut request = envelope();
        request.as_object_mut().unwrap().remove("result");
        assert!(validate(&request, &nodes()).is_err());
    }

    #[test]
    fn admits_envelope_provenance_components_through_validate() {
        // Empty intents and diagnostics matching the empty templates admit.
        let mut request = envelope();
        request["tessellationIntents"] = json!([]);
        request["diagnostics"] = json!([]);
        validate(&request, &nodes()).unwrap();
        // Provenance requires its operation context.
        let mut request = envelope();
        request["provenance"] = json!([]);
        assert!(validate(&request, &nodes()).is_err());
        // Diagnostics must match the transported templates one-to-one.
        let mut request = envelope();
        request["diagnostics"] = json!([{"template":0,"message":"x","span":null}]);
        assert!(validate(&request, &nodes()).is_err());
        // legacy/current refuses any tessellation intent.
        let mut request = envelope();
        request["language"] = json!({"contract":"legacy/current","semanticsRevision":"1.0.0","capabilityGraphVersion":"semantic-capabilities-v1"});
        request["tessellationIntents"] = json!([{"occurrence":0,"chordTolerance":0.1,"angularToleranceDegrees":null,"minSegments":3,"maxSegments":64}]);
        assert!(validate(&request, &nodes()).is_err());
        // A malformed transported template is refused before nodes execute.
        let mut request = envelope();
        request["diagnosticTemplates"] =
            json!([{"id":0,"code":"bad-code","severity":"info","operation":null,"arguments":[]}]);
        assert!(validate(&request, &nodes()).is_err());
    }
}
