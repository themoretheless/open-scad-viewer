//! Native occurrence-production replay tests. Every fixture was lowered by
//! the real TS lowerer (tmp/dump-brep-production-fixtures.mts) and admitted
//! by the host validateSemanticProgramV1 before dumping; mutations are
//! wrong-by-one edits, with surgical variants re-deriving every affected
//! occv1:/entity:v2: digest through the ported derivers so exactly one
//! post-identity rule can refuse.
use super::*;
use value_codec::json;

struct Fixture {
    operations: Vec<Value>,
    occurrences: Vec<Value>,
    nodes: Vec<Value>,
    result: Value,
    execution: Value,
}

fn load(name: &str) -> Fixture {
    let text = match name {
        "difference" => include_str!("fixtures/difference.json"),
        "unionTransform" => include_str!("fixtures/unionTransform.json"),
        "hull" => include_str!("fixtures/hull.json"),
        "children" => include_str!("fixtures/children.json"),
        "module" => include_str!("fixtures/module.json"),
        "nestedTransform" => include_str!("fixtures/nestedTransform.json"),
        "multiOutput" => include_str!("fixtures/multiOutput.json"),
        _ => unreachable!(),
    };
    let parsed: Value = value_codec::from_str(text).expect("fixture JSON parses");
    Fixture {
        operations: parsed["operations"].as_array().unwrap().clone(),
        occurrences: parsed["occurrences"].as_array().unwrap().clone(),
        nodes: parsed["nodes"].as_array().unwrap().clone(),
        result: parsed["result"].clone(),
        execution: parsed["execution"].clone(),
    }
}

/// Re-derives every occurrenceId/sceneEntityId bottom-up (parents always
/// precede children) after a structural mutation, so the identity admission
/// layer admits the tampered rows and only the targeted later rule can fire.
fn rederive_occurrence_digests(occurrences: &mut [Value], operations: &[Value]) {
    let mut ids: Vec<String> = Vec::with_capacity(occurrences.len());
    for index in 0..occurrences.len() {
        let operation = occurrences[index]["operation"].as_u64().unwrap() as usize;
        let parent = occurrences[index]["parent"].as_u64().map(|row| row as usize);
        let static_parent = occurrences[index]["staticParent"]
            .as_u64()
            .map(|row| row as usize);
        let slots = occurrences[index]["dynamicSlots"].as_array().unwrap().clone();
        let id = super::super::brep_identity::derive_occurrence_id(
            parent.map(|row| ids[row].as_str()),
            static_parent.map(|row| ids[row].as_str()),
            operations[operation]["operationId"].as_str().unwrap(),
            &slots,
        );
        if let Some(ordinal) = occurrences[index]["outputOrdinal"].as_u64() {
            occurrences[index]["sceneEntityId"] =
                json!(super::super::brep_identity::derive_scene_entity_id(&id, ordinal));
        }
        occurrences[index]["occurrenceId"] = json!(id.clone());
        ids.push(id);
    }
}

/// The full native admission chain the envelope runs for transported
/// operations/occurrences.
fn admit(fixture: &Fixture) -> Result<()> {
    super::super::brep_identity::validate_operations(&fixture.operations)?;
    super::super::brep_identity::validate_occurrence_identity(
        &fixture.occurrences,
        &fixture.operations,
        fixture.nodes.len(),
    )?;
    validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
}

#[test]
fn replays_real_lowered_programs_successfully() {
    for name in [
        "difference",
        "unionTransform",
        "hull",
        "children",
        "module",
        "nestedTransform",
        "multiOutput",
    ] {
        admit(&load(name)).unwrap_or_else(|error| panic!("{name} must admit: {error:?}"));
    }
    // An empty program (zero operations/occurrences/nodes, empty result)
    // replays trivially, matching the host.
    admit(&Fixture {
        operations: Vec::new(),
        occurrences: Vec::new(),
        nodes: Vec::new(),
        result: json!({"tag":"empty","type":"never"}),
        execution: json!({"version":"semantic-execution-v2","evaluationOrder":[],"discardedEffects":[],"terminal":null}),
    })
    .unwrap();
}

#[test]
fn refuses_a_node_with_zero_materializer_groups() {
    let mut fixture = load("unionTransform");
    fixture.nodes.push(json!({
        "id": 4,
        "kind": "box",
        "size": [9, 9, 9],
        "center": false,
        "valueType": fixture.nodes[0]["valueType"].clone(),
    }));
    let error = validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .unwrap_err();
    assert!(error.message.contains("materializer group"), "{error:?}");
}

#[test]
fn refuses_a_node_claimed_by_two_materializer_groups() {
    let mut fixture = load("multiOutput");
    // The second cube row claims the already-owned first box node.
    fixture.occurrences[3]["node"] = json!(0);
    let error = validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .unwrap_err();
    assert!(error.message.contains("exactly one"), "{error:?}");
}

#[test]
fn refuses_a_broken_transform_map_chain() {
    let mut fixture = load("difference");
    // The transform node must consume the ordered frontier item (node 0).
    fixture.nodes[2]["input"] = json!(0);
    let error = validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .unwrap_err();
    assert!(error.message.contains("matching ordered frontier"), "{error:?}");
    // A transform output row carrying the wrong node kind is refused too.
    let mut fixture = load("difference");
    fixture.occurrences[3]["node"] = json!(3);
    assert!(admit(&fixture).is_err());
}

#[test]
fn refuses_a_bad_boolean_operand_structure() {
    // N-ary union inputs must equal the ordered occurrence frontier.
    let mut fixture = load("unionTransform");
    fixture.nodes[3]["inputs"] = json!([2, 0]);
    let error = validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .unwrap_err();
    assert!(error.message.contains("ordered occurrence frontier"), "{error:?}");
    // The boolean node operation must match its static operation.
    let mut fixture = load("unionTransform");
    fixture.nodes[3]["operation"] = json!("intersection");
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
    // Difference root must consume exactly reduced base and cutters.
    let mut fixture = load("difference");
    fixture.nodes[3]["operation"] = json!("union");
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
}

#[test]
fn refuses_a_hull_reduction_mismatch() {
    // Hull of two solids must materialize one hull node over the frontier.
    let mut fixture = load("hull");
    fixture.nodes[3]["kind"] = json!("boolean");
    fixture.nodes[3]["operation"] = json!("union");
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
}

#[test]
fn refuses_a_misplaced_output_ordinal() {
    let mut fixture = load("difference");
    // Re-derive the scene identity so only the dense-ordinal rule can refuse.
    fixture.occurrences[3]["outputOrdinal"] = json!(1);
    let occurrence_id = fixture.occurrences[3]["occurrenceId"]
        .as_str()
        .unwrap()
        .to_owned();
    fixture.occurrences[3]["sceneEntityId"] =
        json!(super::super::brep_identity::derive_scene_entity_id(&occurrence_id, 1));
    assert!(admit(&fixture).is_err());
    // A non-producing frame row cannot own an output slot either.
    let mut fixture = load("difference");
    fixture.occurrences[1]["node"] = json!(0);
    fixture.occurrences[1]["outputOrdinal"] = json!(0);
    let occurrence_id = fixture.occurrences[1]["occurrenceId"]
        .as_str()
        .unwrap()
        .to_owned();
    fixture.occurrences[1]["sceneEntityId"] =
        json!(super::super::brep_identity::derive_scene_entity_id(&occurrence_id, 0));
    assert!(admit(&fixture).is_err());
}

#[test]
fn refuses_an_expansion_row_without_an_anchor() {
    // Surgical: the module-definition occurrence is re-parented onto the call
    // row instead of the call $body, with every digest re-derived so the
    // identity layer admits it and moduleActivationAnchor must refuse.
    let mut fixture = load("children");
    fixture.occurrences[3]["parent"] = json!(1);
    rederive_occurrence_digests(&mut fixture.occurrences, &fixture.operations);
    super::super::brep_identity::validate_occurrence_identity(
        &fixture.occurrences,
        &fixture.operations,
        fixture.nodes.len(),
    )
    .expect_err("unanchored module-definition activation must refuse");
    // The unmodified real fixture anchors cleanly.
    let fixture = load("children");
    super::super::brep_identity::validate_occurrence_identity(
        &fixture.occurrences,
        &fixture.operations,
        fixture.nodes.len(),
    )
    .unwrap();
}

#[test]
fn refuses_an_anchored_row_missing_its_continuation() {
    // Surgical: the first $expansion row hangs under the definition $body
    // instead of its consuming children() call; digests re-derived so only
    // isChildrenExpansionContinuation can refuse.
    let mut fixture = load("children");
    fixture.occurrences[6]["parent"] = json!(4);
    rederive_occurrence_digests(&mut fixture.occurrences, &fixture.operations);
    let error = super::super::brep_identity::validate_occurrence_identity(
        &fixture.occurrences,
        &fixture.operations,
        fixture.nodes.len(),
    )
    .unwrap_err();
    assert!(error.message.contains("continuation"), "{error:?}");
}

#[test]
fn refuses_structural_expansion_mutations() {
    // Identity-level tampering with the $expansion continuation rows.
    let mut fixture = load("children");
    fixture.occurrences[6]["staticParent"] = json!(0);
    assert!(admit(&fixture).is_err());
    // Dropping the second $expansion row breaks the positional row IDs.
    let mut fixture = load("children");
    fixture.occurrences.remove(9);
    assert!(admit(&fixture).is_err());
}

#[test]
fn refuses_a_broken_result_frontier_match() {
    // The transported result must equal the synthetic program frontier.
    let mut fixture = load("difference");
    fixture.result["item"]["producerOccurrence"] = json!(1);
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
    let mut fixture = load("difference");
    fixture.result["item"]["identityOccurrence"] = json!(2);
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
    // A multi-result children() program: dropping one item unbalances it.
    let mut fixture = load("children");
    fixture.result["items"].as_array_mut().unwrap().remove(1);
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
}

#[test]
fn refuses_interrupted_legacy_evaluation_artifacts() {
    // Unreachable on the brep-1 path (execution-plan admission refuses them
    // first); the replay refuses them defensively as host-side territory.
    let mut fixture = load("difference");
    fixture.execution["terminal"] = json!({"tag":"legacy-language-error"});
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
    let mut fixture = load("difference");
    fixture.execution["discardedEffects"] =
        json!([{"tag":"legacy-difference-cutters","root":0,"ownerOccurrence":0}]);
    assert!(validate_occurrence_production(
        &fixture.operations,
        &fixture.occurrences,
        &fixture.nodes,
        &fixture.result,
        &fixture.execution,
    )
    .is_err());
}

#[test]
fn replays_through_the_full_envelope_admission() {
    // The replay is wired into brep_envelope::validate after the identity
    // admission, gated on non-empty transported occurrences.
    let fixture = load("difference");
    let request = json!({
        "operations": fixture.operations.clone(),
        "occurrences": fixture.occurrences.clone(),
        "result": fixture.result.clone(),
        "execution": fixture.execution.clone(),
    });
    super::super::brep_envelope::validate(&request, &fixture.nodes).unwrap();
    // Non-empty occurrences without their result/execution context refuse.
    let mut missing = json!({
        "operations": fixture.operations.clone(),
        "occurrences": fixture.occurrences.clone(),
    });
    assert!(super::super::brep_envelope::validate(&missing, &fixture.nodes).is_err());
    missing["result"] = fixture.result.clone();
    assert!(super::super::brep_envelope::validate(&missing, &fixture.nodes).is_err());
    // A tampered production structure refuses through the envelope path.
    let mut tampered_nodes = fixture.nodes.clone();
    tampered_nodes[2]["input"] = json!(0);
    assert!(super::super::brep_envelope::validate(&request, &tampered_nodes).is_err());
}
