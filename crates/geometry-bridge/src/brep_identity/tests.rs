//! Pinned against the host derivers in src/core/semanticProgram.ts; every
//! expected digest below was produced by running the TS functions in node.
use super::*;
use value_codec::json;

fn root_path() -> Vec<Value> {
    json!([{"kind":"module","name":"$root","ordinal":0}])
        .as_array()
        .unwrap()
        .clone()
}
fn cube_path(ordinal: u64) -> Vec<Value> {
    json!([{"kind":"module","name":"$root","ordinal":0},{"kind":"call","name":"cube","ordinal":ordinal}])
        .as_array()
        .unwrap()
        .clone()
}
fn deep_path() -> Vec<Value> {
    json!([
        {"kind":"module","name":"$root","ordinal":0},
        {"kind":"call","name":"cube","ordinal":0},
        {"kind":"body","name":"translate","ordinal":2},
    ])
    .as_array()
    .unwrap()
    .clone()
}
/// A translate body admissible inside an operation forest: dense ordinal 0.
fn body_path() -> Vec<Value> {
    json!([
        {"kind":"module","name":"$root","ordinal":0},
        {"kind":"call","name":"cube","ordinal":0},
        {"kind":"body","name":"translate","ordinal":0},
    ])
    .as_array()
    .unwrap()
    .clone()
}

const OP_ROOT: &str = "opv1:0489bb4c0418f9b1ac1a9f809a7837f40ee01885fd046ea1e48ef8c43d25058c";
const OP_CUBE: &str = "opv1:aa1b69232ea248b7b0d579a78e85d896be804407a124efae5b89da3d029d8444";
const OP_CUBE2: &str = "opv1:9e0e159bcb2b3aedb855201783354a572f3419f2ca753ac23d4523638c6c1e53";
const OP_DEEP: &str = "opv1:b3e3f595ac4fa6bd62bfc2ea7d874ce27dd1cdb8258af0fc25f43c0acb0fe9b6";
const OP_UNICODE: &str = "opv1:2304b13524d1e0ef4ed9be4e946329f11492fce169128c2a5805e47e6d85b14f";
const AMB_CUBE: &str = "ambv1:7091f44fd00b3e56a724bdc2a7f05457111138db8b10edc1fcc10c0a70c29f05";
const AMB_EMPTY: &str = "ambv1:f647a0bc27ea07df386d8d01ef2e8425e3c97a6480a9351cc49c4b14bb8ad05a";
const OCC_ROOT: &str = "occv1:186b39eeb3ac955fa00bcd0f1ef2703e9528a65c543ff4be8534a02c7895c57f";
const OCC_CUBE: &str = "occv1:2933c75a72463a2314409e2d69493581f71ab79578248b09ceab280a89eab398";
const OCC_SLOTS: &str = "occv1:4ce9eed7a80a6f8695a8212bddc1f8a7f990b125fd4fa5f0f975a7c271842fb9";
const OCC_RICH: &str = "occv1:27a00c7630133c2daaab49f5d92f4cd13e9aa422d4b362e764feffdf62f33b5a";
const OCC_DUP0: &str = "occv1:3726fdc4dc75227af761c87d3ad3f5d6edf0c317bb16d501383383f78d7961aa";
const OCC_DUP1: &str = "occv1:979a23a036b7596e1d8f4f479b4ea3d16179b6fc3db8bc73e88183c621c2ac9a";
const ENT0: &str = "entity:v2:7c74d5fe0f63b770b403c0aaafe76a1a61aa555f0c282c51778aadd96da5cf7c";
const ENT1: &str = "entity:v2:3afb017f8f58910dc4d1a0eefd0c6ac56e1b184c0ce517d0be247a2f84d88559";

#[test]
fn matches_host_digests_for_hand_built_paths_and_chains() {
    assert_eq!(derive_operation_id(&root_path()), OP_ROOT);
    assert_eq!(derive_operation_id(&cube_path(0)), OP_CUBE);
    assert_eq!(derive_operation_id(&cube_path(1)), OP_CUBE2);
    assert_eq!(derive_operation_id(&deep_path()), OP_DEEP);
    let unicode_path = json!([{"kind":"control","name":"деталь·π","ordinal":3}])
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(derive_operation_id(&unicode_path), OP_UNICODE);
    assert_eq!(
        derive_ambiguity_group_id(&root_path(), "geometry", "cube"),
        AMB_CUBE
    );
    assert_eq!(
        derive_ambiguity_group_id(&[], "transform", "translate"),
        AMB_EMPTY
    );
    assert_eq!(derive_occurrence_id(None, None, OP_ROOT, &[]), OCC_ROOT);
    assert_eq!(
        derive_occurrence_id(Some(OCC_ROOT), Some(OCC_ROOT), OP_CUBE, &[]),
        OCC_CUBE
    );
    let slots = json!([
        {"name":"$fn","value":{"tag":"number","value":0.5},"duplicateOrdinal":0},
        {"name":"$preview","value":{"tag":"boolean","value":true},"duplicateOrdinal":0},
    ]);
    let slots = slots.as_array().unwrap();
    assert_eq!(
        derive_occurrence_id(Some(OCC_ROOT), Some(OCC_ROOT), OP_CUBE, slots),
        OCC_SLOTS
    );
    let rich = json!([
        {"name":"ratio","value":{"tag":"number","value":0.3333333333333333},"duplicateOrdinal":0},
        {"name":"label","value":{"tag":"string","value":"héllo€"},"duplicateOrdinal":0},
        {"name":"gap","value":{"tag":"undefined"},"duplicateOrdinal":0},
        {"name":"vec","value":{"tag":"vector","items":[{"tag":"number","value":1e-7},{"tag":"null"}]},"duplicateOrdinal":0},
    ]);
    let rich = rich.as_array().unwrap();
    assert_eq!(
        derive_occurrence_id(Some(OCC_CUBE), None, OP_DEEP, rich),
        OCC_RICH
    );
    let dup = |ordinal: u64| {
        json!([{"name":"v","value":{"tag":"number","value":7},"duplicateOrdinal":ordinal}])
            .as_array()
            .unwrap()
            .clone()
    };
    assert_eq!(
        derive_occurrence_id(Some(OCC_ROOT), Some(OCC_ROOT), OP_CUBE2, &dup(0)),
        OCC_DUP0
    );
    assert_eq!(
        derive_occurrence_id(Some(OCC_ROOT), Some(OCC_ROOT), OP_CUBE2, &dup(1)),
        OCC_DUP1
    );
    assert_eq!(derive_scene_entity_id(OCC_CUBE, 0), ENT0);
    assert_eq!(derive_scene_entity_id(OCC_CUBE, 1), ENT1);
}

#[test]
fn js_number_formatting_matches_json_stringify() {
    let cases: [(f64, &str); 19] = [
        (0.5, "0.5"),
        (0.1, "0.1"),
        (1.0 / 3.0, "0.3333333333333333"),
        (1e-6, "0.000001"),
        (1e-7, "1e-7"),
        (1e20, "100000000000000000000"),
        (1e21, "1e+21"),
        (123456.789, "123456.789"),
        (-2.5, "-2.5"),
        (1e100, "1e+100"),
        (5e-324, "5e-324"),
        (9007199254740991.0, "9007199254740991"),
        (2.220446049250313e-16, "2.220446049250313e-16"),
        (-0.000123456, "-0.000123456"),
        (42.0, "42"),
        (1.7976931348623157e308, "1.7976931348623157e+308"),
        (0.3, "0.3"),
        (100.0, "100"),
        (0.25, "0.25"),
    ];
    for (value, expected) in cases {
        assert_eq!(js_f64(value), expected, "js_f64({value:e})");
    }
    assert_eq!(js_f64(0.0), "0");
    assert_eq!(js_f64(-0.0), "0");
    // Integers arrive as Unsigned/Signed and print as plain digits.
    assert_eq!(js_number(&Number::Unsigned(42)), "42");
    assert_eq!(js_number(&Number::Signed(-42)), "-42");
    assert_eq!(js_number(&Number::Float(42.0)), "42");
}

#[expect(clippy::too_many_arguments, reason = "test fixture mirrors the operation wire shape")]
fn operation(
    id: u64,
    parent: Value,
    child_ordinal: u64,
    name: &str,
    category: &str,
    path: &[Value],
    evidence: &str,
    group: Value,
) -> Value {
    json!({
        "id": id,
        "operationId": derive_operation_id(path),
        "parent": parent,
        "childOrdinal": child_ordinal,
        "name": name,
        "category": category,
        "structuralPath": path,
        "identityEvidence": evidence,
        "ambiguityGroup": group,
    })
}

/// Root module, first same-name cube call with a translate child, then the
/// second same-name cube call: deterministic parent-before-child preorder.
fn operations() -> Vec<Value> {
    let group = derive_ambiguity_group_id(&root_path(), "geometry", "cube");
    vec![
        operation(
            0,
            Value::Null,
            0,
            "$root",
            "module",
            &root_path(),
            "structural-unique",
            Value::Null,
        ),
        operation(
            1,
            json!(0),
            0,
            "cube",
            "geometry",
            &cube_path(0),
            "same-name-positional",
            json!(group.clone()),
        ),
        operation(
            2,
            json!(1),
            0,
            "translate",
            "transform",
            &body_path(),
            "structural-unique",
            Value::Null,
        ),
        operation(
            3,
            json!(0),
            1,
            "cube",
            "geometry",
            &cube_path(1),
            "same-name-positional",
            json!(group),
        ),
    ]
}

#[test]
fn admits_a_valid_operation_forest() {
    validate_operations(&operations()).unwrap();
    // A lone root is valid; an empty forest is valid (nothing to admit).
    validate_operations(&operations()[..1]).unwrap();
    validate_operations(&[]).unwrap();
    // A child appearing after its parent's later sibling is out of preorder.
    let mut late_child = operations();
    late_child.swap(2, 3);
    assert!(validate_operations(&late_child).is_err());
}

#[test]
fn refuses_wrong_by_one_operation_mutations() {
    let valid = operations();
    // Parent pointing at or past itself.
    for parent in [json!(1), json!(3)] {
        let mut forward = valid.clone();
        forward[1]["parent"] = parent;
        assert!(validate_operations(&forward).is_err());
    }
    // Sibling ordinals not dense from zero.
    let mut sparse = valid.clone();
    sparse[3]["childOrdinal"] = json!(2);
    sparse[3]["structuralPath"] = json!([
        {"kind":"module","name":"$root","ordinal":0},
        {"kind":"call","name":"cube","ordinal":2},
    ]);
    sparse[3]["operationId"] = json!(derive_operation_id(
        json!([{"kind":"module","name":"$root","ordinal":0},{"kind":"call","name":"cube","ordinal":2}])
            .as_array()
            .unwrap()
    ));
    assert!(validate_operations(&sparse).is_err());
    // Wrong structural path (does not extend the parent exactly once).
    let mut wrong_path = valid.clone();
    wrong_path[2]["structuralPath"] = json!([
        {"kind":"module","name":"$root","ordinal":0},
        {"kind":"body","name":"translate","ordinal":2},
    ]);
    assert!(validate_operations(&wrong_path).is_err());
    // Final segment must carry the operation name and childOrdinal.
    let mut renamed = valid.clone();
    renamed[2]["name"] = json!("scale");
    assert!(validate_operations(&renamed).is_err());
    // Tampered operationId.
    let mut tampered = valid.clone();
    tampered[1]["operationId"] = json!(OP_CUBE2);
    assert!(validate_operations(&tampered).is_err());
    // Unknown digest prefix.
    let mut unknown_prefix = valid.clone();
    unknown_prefix[0]["operationId"] = json!(OP_ROOT.replacen("opv1:", "opv2:", 1));
    assert!(validate_operations(&unknown_prefix).is_err());
    // id != index.
    let mut wrong_id = valid.clone();
    wrong_id[2]["id"] = json!(3);
    assert!(validate_operations(&wrong_id).is_err());
    // First operation with a parent.
    let mut rooted = valid.clone();
    rooted[0]["parent"] = json!(0);
    assert!(validate_operations(&rooted).is_err());
    // Ambiguity-group membership errors.
    let mut missing_group = valid.clone();
    missing_group[1]["ambiguityGroup"] = Value::Null;
    assert!(validate_operations(&missing_group).is_err());
    let mut wrong_group = valid.clone();
    wrong_group[1]["ambiguityGroup"] = json!(AMB_EMPTY);
    assert!(validate_operations(&wrong_group).is_err());
    // Same-name siblings without positional evidence.
    let mut unique_evidence = valid.clone();
    unique_evidence[1]["identityEvidence"] = json!("structural-unique");
    unique_evidence[1]["ambiguityGroup"] = Value::Null;
    assert!(validate_operations(&unique_evidence).is_err());
    // Positional evidence without same-name siblings.
    let mut claimed = valid.clone();
    claimed[2]["identityEvidence"] = json!("same-name-positional");
    claimed[2]["ambiguityGroup"] = json!(derive_ambiguity_group_id(
        &cube_path(0),
        "transform",
        "translate"
    ));
    assert!(validate_operations(&claimed).is_err());
    // structural-unique claiming a group.
    let mut claimed_unique = valid.clone();
    claimed_unique[2]["ambiguityGroup"] = json!(AMB_EMPTY);
    assert!(validate_operations(&claimed_unique).is_err());
    // Duplicate structural path.
    let mut duplicate = valid.clone();
    duplicate[3] = valid[1].clone();
    duplicate[3]["id"] = json!(3);
    assert!(validate_operations(&duplicate).is_err());
    // Root path must have exactly one segment.
    let mut long_root = valid.clone();
    long_root[0]["structuralPath"] = json!([
        {"kind":"module","name":"$root","ordinal":0},
        {"kind":"module","name":"$root","ordinal":0},
    ]);
    assert!(validate_operations(&long_root).is_err());
}

#[expect(clippy::too_many_arguments, reason = "test fixture mirrors the occurrence wire shape")]
fn occurrence(
    id: u64,
    occurrence_id: &str,
    operation: u64,
    parent: Value,
    static_parent: Value,
    slots: Value,
    node: Value,
    output_ordinal: Value,
    scene_entity_id: Value,
) -> Value {
    json!({
        "id": id,
        "occurrenceId": occurrence_id,
        "operation": operation,
        "parent": parent,
        "staticParent": static_parent,
        "dynamicSlots": slots,
        "node": node,
        "outputOrdinal": output_ordinal,
        "sceneEntityId": scene_entity_id,
    })
}

/// Root frame occurrence plus one producing cube occurrence with two output
/// rows (dense ordinals 0 and 1), all digests re-derived.
fn occurrences() -> (Vec<Value>, Vec<Value>) {
    let ops = operations();
    let frame = occurrence(
        0,
        OCC_ROOT,
        0,
        Value::Null,
        Value::Null,
        json!([]),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    let row0 = occurrence(
        1,
        OCC_CUBE,
        1,
        json!(0),
        json!(0),
        json!([]),
        json!(0),
        json!(0),
        json!(ENT0),
    );
    let row1 = occurrence(
        2,
        OCC_CUBE,
        1,
        json!(0),
        json!(0),
        json!([]),
        json!(0),
        json!(1),
        json!(ENT1),
    );
    (vec![frame, row0, row1], ops)
}

#[test]
fn admits_a_valid_occurrence_identity_chain() {
    let (occurrences, ops) = occurrences();
    validate_occurrence_identity(&occurrences, &ops, 1).unwrap();
    // No occurrences is valid.
    validate_occurrence_identity(&[], &ops, 1).unwrap();
    // Dynamic slots enter the digest.
    let slots = json!([
        {"name":"$fn","value":{"tag":"number","value":0.5},"duplicateOrdinal":0},
        {"name":"$preview","value":{"tag":"boolean","value":true},"duplicateOrdinal":0},
    ]);
    let mut slotted = occurrence(
        1,
        OCC_SLOTS,
        1,
        json!(0),
        json!(0),
        slots,
        json!(0),
        json!(0),
        Value::Null,
    );
    // Scene identity for the slotted occurrence derives from OCC_SLOTS.
    slotted["sceneEntityId"] = json!(derive_scene_entity_id(OCC_SLOTS, 0));
    let occurrences = vec![occurrences[0].clone(), slotted];
    validate_occurrence_identity(&occurrences, &ops, 1).unwrap();
}

#[test]
fn refuses_wrong_by_one_occurrence_mutations() {
    let (occurrences, ops) = occurrences();
    // Tampered occurrenceId.
    let mut tampered = occurrences.clone();
    tampered[1]["occurrenceId"] = json!(OCC_ROOT);
    assert!(validate_occurrence_identity(&tampered, &ops, 1).is_err());
    // Unknown occurrenceId prefix.
    let mut unknown_prefix = occurrences.clone();
    unknown_prefix[1]["occurrenceId"] = json!(OCC_CUBE.replacen("occv1:", "occv2:", 1));
    assert!(validate_occurrence_identity(&unknown_prefix, &ops, 1).is_err());
    // Tampered sceneEntityId.
    let mut scene = occurrences.clone();
    scene[1]["sceneEntityId"] = json!(ENT1);
    assert!(validate_occurrence_identity(&scene, &ops, 1).is_err());
    // Unknown sceneEntityId prefix.
    let mut scene_prefix = occurrences.clone();
    scene_prefix[1]["sceneEntityId"] = json!(ENT0.replacen("entity:v2:", "entity:v3:", 1));
    assert!(validate_occurrence_identity(&scene_prefix, &ops, 1).is_err());
    // Duplicate output ordinals within one occurrence ID.
    let mut duplicate_ordinal = occurrences.clone();
    duplicate_ordinal[2]["outputOrdinal"] = json!(0);
    duplicate_ordinal[2]["sceneEntityId"] = json!(ENT0);
    assert!(validate_occurrence_identity(&duplicate_ordinal, &ops, 1).is_err());
    // Output rows must be dense from zero.
    let mut gap = occurrences.clone();
    gap[2]["outputOrdinal"] = json!(2);
    gap[2]["sceneEntityId"] = json!(derive_scene_entity_id(OCC_CUBE, 2));
    assert!(validate_occurrence_identity(&gap, &ops, 1).is_err());
    // A repeated ID may only repeat for output slots.
    let mut mixed = occurrences.clone();
    mixed[2]["outputOrdinal"] = Value::Null;
    mixed[2]["sceneEntityId"] = Value::Null;
    assert!(validate_occurrence_identity(&mixed, &ops, 1).is_err());
    // parent referencing a non-canonical later row of the same ID: derive the
    // otherwise-correct ID so only the canonical-row rule can refuse.
    let late_id = derive_occurrence_id(Some(OCC_CUBE), Some(OCC_ROOT), OP_CUBE, &[]);
    let extra = occurrence(
        3,
        &late_id,
        1,
        json!(2),
        json!(0),
        json!([]),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    let mut late_reference_rows = occurrences.clone();
    late_reference_rows.push(extra);
    assert!(validate_occurrence_identity(&late_reference_rows, &ops, 1).is_err());
    // parent pointing at or past the row itself.
    let mut self_parent = occurrences.clone();
    self_parent[1]["parent"] = json!(1);
    assert!(validate_occurrence_identity(&self_parent, &ops, 1).is_err());
    // staticParent not instantiating the static parent operation.
    let mut wrong_static = occurrences.clone();
    wrong_static[1]["staticParent"] = json!(1);
    assert!(validate_occurrence_identity(&wrong_static, &ops, 1).is_err());
    // staticParent on a static-root operation row.
    let mut root_static = occurrences.clone();
    root_static[0]["staticParent"] = json!(0);
    assert!(validate_occurrence_identity(&root_static, &ops, 1).is_err());
    // Rows sharing an ID must describe the same logical evaluation.
    let mut diverged = occurrences.clone();
    diverged[2]["dynamicSlots"] =
        json!([{"name":"v","value":{"tag":"number","value":7},"duplicateOrdinal":0}]);
    diverged[2]["occurrenceId"] = json!(OCC_CUBE);
    assert!(validate_occurrence_identity(&diverged, &ops, 1).is_err());
    // Non-producing occurrence owning an output identity.
    let mut producing_frame = occurrences.clone();
    producing_frame[0]["outputOrdinal"] = json!(0);
    producing_frame[0]["sceneEntityId"] = json!(derive_scene_entity_id(OCC_ROOT, 0));
    assert!(validate_occurrence_identity(&producing_frame, &ops, 1).is_err());
    // Producer-only occurrence claiming scene identity.
    let mut claimed_scene = occurrences.clone();
    claimed_scene[1]["outputOrdinal"] = Value::Null;
    claimed_scene[1]["sceneEntityId"] = json!(ENT0);
    assert!(validate_occurrence_identity(&claimed_scene, &ops, 1).is_err());
    // Duplicate dynamic slot names.
    let mut duplicate_names = occurrences.clone();
    duplicate_names[1]["dynamicSlots"] = json!([
        {"name":"v","value":{"tag":"number","value":1},"duplicateOrdinal":0},
        {"name":"v","value":{"tag":"number","value":2},"duplicateOrdinal":0},
    ]);
    assert!(validate_occurrence_identity(&duplicate_names, &ops, 1).is_err());
    // -0 identity values are not canonical.
    let mut negative_zero = occurrences.clone();
    negative_zero[1]["dynamicSlots"] =
        json!([{"name":"v","value":{"tag":"number","value":-0.0},"duplicateOrdinal":0}]);
    assert!(validate_occurrence_identity(&negative_zero, &ops, 1).is_err());
}

#[test]
fn refuses_duplicate_ordinal_disorder_across_rows() {
    let ops = operations();
    let frame = occurrence(
        0,
        OCC_ROOT,
        0,
        Value::Null,
        Value::Null,
        json!([]),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    let slot = |ordinal: u64| json!([{"name":"v","value":{"tag":"number","value":7},"duplicateOrdinal":ordinal}]);
    // Two rows with equal dynamic values need dense ordinals 0 then 1.
    let first = occurrence(
        1,
        OCC_DUP0,
        3,
        json!(0),
        json!(0),
        slot(0),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    let second = occurrence(
        2,
        OCC_DUP1,
        3,
        json!(0),
        json!(0),
        slot(1),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    validate_occurrence_identity(&[frame.clone(), first.clone(), second.clone()], &ops, 1).unwrap();
    // Skipping ordinal 0 is refused.
    let skipped = occurrence(
        1,
        OCC_DUP1,
        3,
        json!(0),
        json!(0),
        slot(1),
        Value::Null,
        Value::Null,
        Value::Null,
    );
    assert!(validate_occurrence_identity(&[frame.clone(), skipped], &ops, 1).is_err());
    // Two rows with the same ordinal must carry the same occurrence ID; here
    // the second row repeats the first row's exact identity as a frame row.
    let mut repeated = vec![frame, first.clone(), first.clone()];
    repeated[2]["id"] = json!(2);
    assert!(validate_occurrence_identity(&repeated, &ops, 1).is_err());
}
