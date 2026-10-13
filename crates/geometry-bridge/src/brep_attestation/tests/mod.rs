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
