use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn root() -> Value {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let artifact =
        dispatch(json!({"op":"cad_full_round_exact","source":source,"faces":[5,4,3]})).unwrap();
    dispatch(json!({"op":"cad_full_round_exact_history","action":"create","artifact":artifact}))
        .unwrap()
}
fn step(state: &Value, action: &str) -> Value {
    dispatch(
        json!({"op":"cad_full_round_exact_history","action":action,"history":state["document"]}),
    )
    .unwrap()
}
fn rebase(state: &Value, scale: f64) -> Value {
    let cursor = state["cursor"].as_u64().unwrap() as usize;
    let source: brep_core::Model = value_codec::from_value(
        state["document"]["revisions"][cursor]["artifact"]["source"].clone(),
    )
    .unwrap();
    let source = brep_core::transform::affine(
        &source,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., scale, 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    dispatch(json!({"op":"cad_full_round_exact_history","action":"rebase-source","history":state["document"],"source":source})).unwrap()
}
#[test]
fn public_history_transport_rebuilds_undoes_redoes_and_forks_exact_bodies() {
    let original = root();
    let edited = rebase(&original, 1.25);
    assert_eq!(edited["revisionCount"].as_u64(), Some(2));
    assert_eq!(
        edited["selectedSourceFaceIds"],
        original["selectedSourceFaceIds"]
    );
    assert!(
        edited["work"]["pairProjections"].as_u64().unwrap()
            > original["work"]["pairProjections"].as_u64().unwrap()
    );
    let undone = step(&edited, "undo");
    assert_eq!(undone["cursor"].as_u64(), Some(0));
    assert_eq!(step(&undone, "redo")["document"], edited["document"]);
    let fork = step(&undone, "branch");
    assert_eq!(fork["revisionCount"].as_u64(), Some(1));
    let branch = rebase(&fork, 1.5);
    assert_ne!(branch["document"], edited["document"]);
    assert_eq!(step(&edited, "restore")["document"], edited["document"]);
}
#[test]
fn selection_edit_uses_authored_ids_and_preserves_input_document() {
    let state = root();
    let before = state.clone();
    let source: brep_core::Model =
        value_codec::from_value(state["document"]["revisions"][0]["artifact"]["source"].clone())
            .unwrap();
    let ids = [3, 2, 5].map(|i| source.1.faces[i]);
    let next=dispatch(json!({"op":"cad_full_round_exact_history","action":"reselect","history":state["document"],"faceIds":ids})).unwrap();
    assert_eq!(next["revisionCount"].as_u64(), Some(2));
    assert_eq!(state, before);
    assert_eq!(
        next["document"]["revisions"][1]["edit"].as_str(),
        Some("reselect")
    );
}
#[test]
fn external_credit_unknown_fields_and_forged_redo_are_refused() {
    let state = root();
    let artifact = &state["document"]["revisions"][0]["artifact"];
    for budget in [
        json!({"maxBytes":1}),
        json!({"maxRevisions":0}),
        json!({"material":{"maxPairProjections":0}}),
        json!({"credit":999}),
    ] {
        assert!(dispatch(json!({"op":"cad_full_round_exact_history","action":"create","artifact":artifact,"budget":budget})).is_err());
    }
    assert!(dispatch(json!({"op":"cad_full_round_exact_history","action":"restore","history":state["document"],"work":{"pairs":0}})).is_err());
    let mut edited = rebase(&state, 1.25);
    edited["document"]["cursor"] = json!(0);
    edited["document"]["revisions"][1]["artifact"]["geometry"]["vertices"][0]["point"][0] =
        json!(["999", "1"]);
    assert!(dispatch(json!({"op":"cad_full_round_exact_history","action":"restore","history":edited["document"]})).is_err());
}
