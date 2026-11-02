use geometry_bridge::{brep_graph, brep_session::Session};
use value_codec::{Value, json};
fn carrier(exact: bool) -> Value {
    json!({"space":"d3","geometryKind":"solid","representation":if exact {"rational-brep"} else {"analytic-brep"},
        "evidence":{"tag":if exact {"representation-preserving"} else {"representation-preserving"}}})
}
fn cube() -> Value {
    json!({"id":0,"kind":"box","valueType":carrier(false),"size":[4.,8.,6.],"center":false})
}
fn selected() -> [usize; 3] {
    let m = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let face = |axis: usize, value: f64| {
        m.faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[axis] == value)
            })
            .unwrap()
    };
    [face(0, 0.), face(1, 8.), face(0, 4.)]
}
fn round(exact: bool) -> Value {
    json!({"id":1,"kind":"full-round-exact","input":0,"faces":selected(),"valueType":carrier(exact)})
}
#[test]
fn exact_node_commits_rational_geometry_and_full_frozen_owner() {
    let bundle = brep_graph::execute(
        vec![cube(), round(true)],
        &[1],
        Session::new(4, 4 << 20).unwrap(),
    )
    .unwrap();
    let value = bundle.results().next().unwrap();
    assert_eq!(value["representation"].as_str(), Some("rational-brep"));
    assert_eq!(value["model"], value["artifact"]["geometry"]);
    // This allowance fits the source + rational model, but not the complete
    // frozen source/recipe/evidence that must actually remain owned.
    let source_chars = value_codec::to_string(&value["artifact"]["source"])
        .unwrap()
        .len();
    let model_chars = value_codec::to_string(&value["model"]).unwrap().len();
    let mut constrained = Session::new(4, 4 << 20)
        .unwrap()
        .with_character_limit(source_chars + model_chars + 1024)
        .unwrap();
    let input = constrained.evaluate(cube(), &[]).unwrap();
    let prior = constrained.retained_bytes();
    assert!(constrained.evaluate(round(true), &[input.clone()]).is_err());
    assert_eq!(constrained.retained_bytes(), prior);
    assert!(constrained.commit(&[input]).is_err());
    assert_eq!(constrained.retained_bytes(), 0);
    assert!(value["model"]["vertices"][0]["point"][0][0].is_string());
    assert!(
        value["model"]["topologyIds"]["faces"]
            .as_array()
            .unwrap()
            .len()
            > 0
    );
}
#[test]
fn exact_carrier_is_not_a_cast_for_legacy_primitives() {
    let mut node = cube();
    node["valueType"] = carrier(true);
    let mut session = Session::new(4, 4 << 20).unwrap();
    assert!(session.evaluate(node, &[]).is_err());
    assert_eq!(session.retained_bytes(), 0);
    assert!(session.commit(&[]).is_err());
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn material_exhaustion_and_wrong_result_carrier_abort_graph() {
    let mut node = round(true);
    node["limits"] = json!({"maxPairProjections":0});
    assert!(
        brep_graph::execute(vec![cube(), node], &[1], Session::new(4, 4 << 20).unwrap()).is_err()
    );
    assert!(
        brep_graph::execute(
            vec![cube(), round(false)],
            &[1],
            Session::new(4, 4 << 20).unwrap()
        )
        .is_err()
    );
}

#[test]
fn public_graph_router_commits_exact_output_and_refuses_legacy_consumption() {
    let result=geometry_bridge::dispatch(json!({"op":"brep_graph","nodes":[cube(),round(true)],"outputs":[1],"maxNodes":4,"maxBytes":4194304})).unwrap();
    assert_eq!(
        result["outcomes"][0]["geometry"]["representation"].as_str(),
        Some("rational-brep")
    );
    let mut session = Session::new(4, 4 << 20).unwrap();
    let base = session.evaluate(cube(), &[]).unwrap();
    let exact = session.evaluate(round(true), &[base]).unwrap();
    let node = json!({"id":2,"kind":"transform","input":1,"valueType":carrier(false),"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]});
    let prior = session.retained_bytes();
    assert_eq!(
        session.evaluate(node, &[exact]).unwrap_err().code,
        "BREP_UNSUPPORTED_SEMANTIC"
    );
    assert_eq!(session.retained_bytes(), prior);
    assert!(session.commit(&[]).is_err());
    assert_eq!(session.retained_bytes(), 0);
}

#[test]
fn exact_graph_requires_complete_representation_transition_closure() {
    let nodes = vec![cube(), round(true)];
    let closure =
        geometry_bridge::brep_envelope::derive_capability_closure(&[], &nodes, "single", 1, 0)
            .unwrap();
    assert!(closure.iter().any(|s| s == "operation.full-round-exact"));
    let transition = "representation.transition.analytic-brep.to.rational-brep";
    assert!(closure.iter().any(|s| s == transition));
    let request = json!({
        "op":"brep_graph", "nodes":nodes,
        "maxNodes":4, "maxBytes":4194304,
        "declaredCapabilities":[], "capabilityClosure":closure,
        "occurrences":[{"id":0,"parent":null,"node":1,"occurrenceId":"exact-result-test","sceneEntityId":"exact-result-entity"}],
        "result":{"tag":"single","item":{"node":1,"producerOccurrence":0,"identityOccurrence":0,"color":[1,1,1,1]}}
    });
    let result = geometry_bridge::dispatch(request.clone()).unwrap();
    assert_eq!(
        result["outcomes"][0]["geometry"]["representation"].as_str(),
        Some("rational-brep")
    );
    for missing in [
        "operation.full-round-exact",
        transition,
        "representation.rational-brep",
    ] {
        let mut bad = request.clone();
        bad["capabilityClosure"] = json!(
            closure
                .iter()
                .filter(|s| s.as_str() != missing)
                .collect::<Vec<_>>()
        );
        let error = geometry_bridge::dispatch(bad).unwrap_err();
        assert_eq!(
            error.code, "BREP_SEMANTIC_CONTRACT",
            "accepted missing {missing}"
        );
        assert!(error.message.contains("closure"));
    }
    let mut invalid_reference = nodes.clone();
    invalid_reference[1]["input"] = json!(9);
    assert!(
        geometry_bridge::brep_envelope::derive_capability_closure(
            &[],
            &invalid_reference,
            "single",
            0,
            0,
        )
        .is_err()
    );
}

#[test]
fn real_host_lowered_source_passes_native_source_identity_and_geometry_admission() {
    // Generated by scripts/qualify-full-round-source-contract.mts through the
    // public trusted lowerer and codec, not a hand-authored semantic fixture.
    let request: Value = value_codec::from_str(include_str!(
        "../../../docs/qualification/full-round-host-contract-2026-11-02/native-request.json"
    ))
    .unwrap();
    let output = geometry_bridge::dispatch(request.clone()).unwrap();
    assert_eq!(
        output["outcomes"][0]["geometry"]["representation"].as_str(),
        Some("rational-brep")
    );
    assert_eq!(
        output["resultItems"][0]["reference"]["identityOccurrence"].as_u64(),
        Some(0)
    );
    for key in ["sourceText", "capabilityClosure"] {
        let mut bad = request.clone();
        if key == "sourceText" {
            bad[key] = json!("// changed source\n");
        } else {
            let mut closure = bad[key].as_array().unwrap().clone();
            closure.retain(|entry| entry.as_str() != Some("operation.full-round-exact"));
            bad[key] = json!(closure);
            // Keep the duplicated envelope internally consistent so admission
            // must reject the actual missing capability, not duplication.
            bad["envelope"]["core"][key] = bad[key].clone();
        }
        assert!(geometry_bridge::dispatch(bad).is_err());
    }
}
