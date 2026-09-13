use brep_core::{Model, cuboid};
use brep_topology::{TopologySnapshot, TopologyStore};

#[test]
fn retained_kernel_validation_rejects_bad_geometry_without_changing_store() {
    let source = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let ids = source.1.clone();
    let snapshot = TopologySnapshot::new_with_model_validation(
        source.0.clone(),
        |_| Ok(()),
        move |topology| Model(topology.clone(), ids.clone()).validate().map(|_| ()),
    )
    .unwrap();
    let mut store = TopologyStore::new(snapshot);
    let base = store.checkout();
    assert!(
        base.prepare(|m| {
            m.edges[0].curve.weights[0] = 0.;
            Ok(())
        })
        .is_err()
    );
    assert!(
        base.prepare(|m| {
            m.faces[0].surface.control_points[0][0][0] = f64::NAN;
            Ok(())
        })
        .is_err()
    );
    assert!(
        base.prepare(|m| {
            m.edges[0].curve.control_points[0][0] += 10.;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(store.revision(), 0);
    Model(store.snapshot().model().clone(), source.1.clone())
        .validate()
        .unwrap();
    let edit = base
        .prepare(|m| {
            m.tolerance_mm = 2e-6;
            Ok(())
        })
        .unwrap();
    store.commit(edit).unwrap();
    assert!(
        store
            .checkout()
            .prepare(|m| {
                m.faces[0].surface.weights[0][0] = 0.;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(store.revision(), 1);
    assert!(store.undo().unwrap());
    assert!(store.redo().unwrap());
    Model(store.snapshot().model().clone(), source.1)
        .validate()
        .unwrap();
}

#[test]
fn cancelled_publication_preserves_redo_and_skips_checks_for_stale_edits() {
    let source = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let ids = source.1.clone();
    let snapshot = TopologySnapshot::new_with_model_validation(
        source.0.clone(),
        |_| Ok(()),
        move |m| Model(m.clone(), ids.clone()).validate().map(|_| ()),
    )
    .unwrap();
    let mut store = TopologyStore::new(snapshot);
    let tx = store
        .checkout()
        .prepare(|m| {
            m.tolerance_mm = 2e-6;
            Ok(())
        })
        .unwrap();
    store.commit(tx).unwrap();
    store.undo().unwrap();
    let original = store.snapshot().model().tolerance_mm;
    let rev = store.revision();
    let history = store.history_lengths();
    let tx = store
        .checkout()
        .prepare(|m| {
            m.tolerance_mm = 3e-6;
            Ok(())
        })
        .unwrap();
    let error = store
        .commit_with_check(tx, |m| {
            assert_eq!(m.tolerance_mm, 3e-6);
            Err(nurbs_core::Error::new(
                "OPERATION_CANCELLED",
                "Cancelled after preparation",
            ))
        })
        .unwrap_err();
    assert_eq!(error.code, "OPERATION_CANCELLED");
    assert_eq!(store.revision(), rev);
    assert_eq!(store.history_lengths(), history);
    assert_eq!(store.snapshot().model().tolerance_mm, original);
    let stale = store.checkout().prepare(|_| Ok(())).unwrap();
    assert!(store.redo().unwrap());
    let called = std::cell::Cell::new(false);
    assert_eq!(
        store
            .commit_with_check(stale, |_| {
                called.set(true);
                Ok(())
            })
            .unwrap_err()
            .code,
        "BREP_STALE_TRANSACTION"
    );
    assert!(!called.get());
    let tx = store.checkout().prepare(|_| Ok(())).unwrap();
    let expected = store.revision() + 1;
    assert_eq!(
        store
            .commit_with_check(tx, |m| {
                called.set(true);
                Model(m.clone(), source.1.clone()).validate().map(|_| ())
            })
            .unwrap(),
        expected
    );
    assert!(called.get());
}

#[test]
fn full_model_boolean_transaction_preserves_geometry_and_identity_through_history() {
    use brep_core::{
        boolean,
        transactions::{ModelSnapshot, ModelStore},
    };
    let original = cuboid([0., 0., 0.], [6., 6., 6.]).unwrap();
    let original_ids = original.1.faces.clone();
    let mut store = ModelStore::new(ModelSnapshot::new(original).unwrap());
    let cutter = cuboid([2., 2., -1.], [4., 4., 7.]).unwrap();
    let tx = store
        .checkout()
        .prepare(|model| {
            *model = boolean(model, &cutter, "difference")?;
            Ok(())
        })
        .unwrap();
    store.commit(tx).unwrap();
    let changed = store.snapshot().model();
    changed.validate().unwrap();
    assert!(changed.faces.len() > 6);
    assert_eq!(changed.faces.len(), changed.1.faces.len());
    let changed_ids = changed.1.faces.clone();
    assert_ne!(changed_ids, original_ids);
    let stale = store.checkout().prepare(|_| Ok(())).unwrap();
    assert!(store.undo().unwrap());
    assert_eq!(store.snapshot().model().1.faces, original_ids);
    assert!(store.redo().unwrap());
    assert_eq!(store.snapshot().model().1.faces, changed_ids);
    assert_eq!(
        store.commit(stale).unwrap_err().code,
        "BREP_STALE_TRANSACTION"
    );
    let revision = store.revision();
    assert!(
        store
            .checkout()
            .prepare(|model| {
                model.1.faces.clear();
                Ok(())
            })
            .is_err()
    );
    assert_eq!(store.revision(), revision);
    assert_eq!(store.snapshot().model().1.faces, changed_ids);
}

#[test]
fn versioned_snapshot_roundtrip_preserves_identity_and_rejects_corruption() {
    use brep_core::transactions::{MAX_MODEL_SNAPSHOT_BYTES, ModelSnapshot, ModelStore};
    use value_codec::json;
    let source = brep_core::boolean(
        &cuboid([0.; 3], [6.; 3]).unwrap(),
        &cuboid([2., 2., -1.], [4., 4., 7.]).unwrap(),
        "difference",
    )
    .unwrap();
    let snapshot = ModelSnapshot::new(source).unwrap();
    let encoded = snapshot.to_json().unwrap();
    let restored = ModelSnapshot::from_json(&encoded).unwrap();
    let duplicated = encoded.replacen("{", "{\"schemaVersion\":999,", 1);
    assert!(ModelSnapshot::from_json(&duplicated).is_err());

    assert_eq!(restored.to_json().unwrap(), encoded);
    assert_eq!(restored.model().1.faces, snapshot.model().1.faces);
    let mut store = ModelStore::new(restored);
    let edit = store
        .checkout()
        .prepare(|m| {
            m.tolerance_mm = 2e-6;
            Ok(())
        })
        .unwrap();
    store.commit(edit).unwrap();
    store.undo().unwrap();
    assert_eq!(store.snapshot().to_json().unwrap(), encoded);
    for mutation in 0..5 {
        let mut value: value_codec::Value = value_codec::from_str(&encoded).unwrap();
        match mutation {
            0 => value["schemaVersion"] = json!(999),
            1 => {
                value
                    .as_object_mut()
                    .unwrap()
                    .insert("unexpected".into(), json!(true));
            }
            2 => {
                value["model"]
                    .as_object_mut()
                    .unwrap()
                    .remove("topologyIds");
            }
            3 => value["model"]["topologyIds"]["faces"] = json!([]),
            _ => value["model"]["edges"][0]["curve"]["weights"][0] = json!(0),
        }
        assert!(ModelSnapshot::from_json(&value_codec::to_string(&value).unwrap()).is_err());
    }
    assert_eq!(
        ModelSnapshot::from_json(&" ".repeat(MAX_MODEL_SNAPSHOT_BYTES + 1))
            .unwrap_err()
            .code,
        "BREP_SNAPSHOT_RESOURCE_LIMIT"
    );
    assert_eq!(store.snapshot().to_json().unwrap(), encoded);
}

#[test]
fn model_history_roundtrip_restores_both_stacks_with_fresh_transaction_identity() {
    use brep_core::transactions::*;
    let mut store = ModelStore::new(ModelSnapshot::new(cuboid([0.; 3], [2.; 3]).unwrap()).unwrap());
    for tolerance in [2e-6, 3e-6] {
        let tx = store
            .checkout()
            .prepare(|m| {
                m.tolerance_mm = tolerance;
                Ok(())
            })
            .unwrap();
        store.commit(tx).unwrap();
    }
    store.undo().unwrap();
    let pending = store.checkout().prepare(|_| Ok(())).unwrap();
    let encoded = history_to_json(&store).unwrap();
    let mut restored = history_from_json(&encoded).unwrap();
    assert_eq!(restored.revision(), 0);
    assert_eq!(restored.history_lengths(), (1, 1));
    assert_eq!(
        restored.commit(pending).unwrap_err().code,
        "BREP_FOREIGN_TRANSACTION"
    );
    assert_eq!(history_to_json(&restored).unwrap(), encoded);
    restored.redo().unwrap();
    assert_eq!(restored.snapshot().model().tolerance_mm, 3e-6);
    restored.undo().unwrap();
    restored.undo().unwrap();
    assert_eq!(
        restored.snapshot().model().tolerance_mm,
        store.history_snapshots().0[0].model().tolerance_mm
    );
    let mut corrupt: value_codec::Value = value_codec::from_str(&encoded).unwrap();
    corrupt["redo"][0]["model"]["topologyIds"]["faces"] = value_codec::json!([]);
    assert!(history_from_json(&value_codec::to_string(&corrupt).unwrap()).is_err());
    let mut excess: value_codec::Value = value_codec::from_str(&encoded).unwrap();
    excess["undo"] = value_codec::Value::Array(vec![excess["current"].clone(); 33]);
    assert!(history_from_json(&value_codec::to_string(&excess).unwrap()).is_err());
    assert_eq!(history_to_json(&store).unwrap(), encoded);
}

#[test]
fn analytic_history_preserves_rational_weights_poles_and_identity_bits() {
    use brep_core::transactions::*;
    use value_codec::Serialize;
    let sphere = brep_core::sphere(3.25).unwrap();
    let torus = brep_core::torus(5.5, 1.25).unwrap();
    assert_eq!(
        sphere
            .vertices
            .iter()
            .filter(|v| v.point[0] == 0. && v.point[1] == 0. && v.point[2].abs() == 3.25)
            .count(),
        2
    );
    assert!(
        torus
            .faces
            .iter()
            .flat_map(|f| f.surface.weights.iter().flatten())
            .any(|w| *w != 1.)
    );
    let sphere_value = sphere.to_value();
    let torus_value = torus.to_value();
    let mut store = ModelStore::new(ModelSnapshot::new(sphere).unwrap());
    let replace = store
        .checkout()
        .prepare(|m| {
            *m = torus;
            Ok(())
        })
        .unwrap();
    store.commit(replace).unwrap();
    store.undo().unwrap();
    let archive = history_to_json(&store).unwrap();
    let mut restored = history_from_json(&archive).unwrap();
    assert_eq!(restored.snapshot().model().to_value(), sphere_value);
    assert_eq!(
        restored
            .snapshot()
            .model()
            .vertices
            .iter()
            .filter(|v| v.point[0] == 0. && v.point[1] == 0. && v.point[2].abs() == 3.25)
            .count(),
        2
    );
    restored.redo().unwrap();
    assert_eq!(restored.snapshot().model().to_value(), torus_value);
    let restored_torus = restored.snapshot().model();
    restored_torus.validate().unwrap();
    // Codec values compare full geometry and identity; additionally inspect
    // raw binary64 bits of non-unit weights after decoding.
    let expected = <Model as value_codec::Deserialize>::from_value(torus_value).unwrap();
    let actual_weights: Vec<_> = restored_torus
        .faces
        .iter()
        .flat_map(|f| f.surface.weights.iter().flatten())
        .map(|w| w.to_bits())
        .collect();
    let expected_weights: Vec<_> = expected
        .faces
        .iter()
        .flat_map(|f| f.surface.weights.iter().flatten())
        .map(|w| w.to_bits())
        .collect();
    assert_eq!(actual_weights, expected_weights);
    restored.undo().unwrap();
    assert_eq!(restored.snapshot().model().to_value(), sphere_value);
}

#[test]
fn canonical_empty_survives_transaction_persistence_and_history_without_stale_ids() {
    use brep_core::{boolean, transactions::*};
    use value_codec::Serialize;
    let original = cuboid([0.; 3], [3.; 3]).unwrap();
    let original_value = original.to_value();
    let mut store = ModelStore::new(ModelSnapshot::new(original).unwrap());
    let remove = store
        .checkout()
        .prepare(|m| {
            *m = boolean(m, m, "difference")?;
            Ok(())
        })
        .unwrap();
    store.commit(remove).unwrap();
    let empty = store.snapshot().model();
    assert!(empty.is_empty());
    assert!(
        empty.vertices.is_empty()
            && empty.edges.is_empty()
            && empty.faces.is_empty()
            && empty.bodies.is_empty()
    );
    assert!(empty.1.vertices.is_empty() && empty.1.edges.is_empty() && empty.1.faces.is_empty());
    empty.validate().unwrap();
    let encoded = history_to_json(&store).unwrap();
    let mut loaded = history_from_json(&encoded).unwrap();
    assert!(loaded.snapshot().model().is_empty());
    loaded.undo().unwrap();
    assert_eq!(loaded.snapshot().model().to_value(), original_value);
    loaded.redo().unwrap();
    assert!(loaded.snapshot().model().is_empty());
    let tool = cuboid([1.; 3], [2.; 3]).unwrap();
    let tool_value = tool.to_value();
    let add = loaded
        .checkout()
        .prepare(|m| {
            *m = boolean(m, &tool, "union")?;
            Ok(())
        })
        .unwrap();
    loaded.commit(add).unwrap();
    assert_eq!(loaded.snapshot().model().to_value(), tool_value);
    loaded.undo().unwrap();
    assert!(loaded.snapshot().model().is_empty());
}
