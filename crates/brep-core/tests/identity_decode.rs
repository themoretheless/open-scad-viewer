use brep_core::{Model, cuboid};
use value_codec::{Deserialize, Serialize, json};

#[test]
fn owned_decode_preserves_non_object_refusal() {
    for value in [
        json!(null),
        json!(false),
        json!(1),
        json!("body"),
        json!([]),
    ] {
        assert_eq!(
            Model::from_value(value).unwrap_err().to_string(),
            "Expected B-rep object"
        );
    }
}

#[test]
fn missing_identity_tables_are_generated_only_after_safe_validation() {
    let model = cuboid([0.; 3], [2.; 3]).unwrap();
    let mut legacy = model.to_value();
    legacy.as_object_mut().unwrap().remove("topologyIds");
    let decoded = Model::from_value(legacy.clone()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.1.faces, model.1.faces);
    for path in ["index", "weights", "control", "knots", "edge weights"] {
        let mut broken = legacy.clone();
        match path {
            "index" => broken["shells"][0]["faces"][0]["face"] = json!(999),
            "weights" => broken["faces"][0]["surface"]["weights"] = json!([]),
            "control" => broken["faces"][0]["surface"]["controlPoints"][0][0] = json!([0., 0.]),
            "knots" => broken["edges"][0]["curve"]["knots"] = json!([]),
            _ => broken["edges"][0]["curve"]["weights"] = json!([]),
        }
        assert!(
            Model::from_value(broken).is_err(),
            "malformed {path} accepted"
        );
    }
}

#[test]
fn partial_supplied_identity_tables_are_not_silently_replaced() {
    let mut value = cuboid([0.; 3], [2.; 3]).unwrap().to_value();
    value["topologyIds"]["vertices"] = json!([]);
    assert!(Model::from_value(value).unwrap().validate().is_err());
}

#[test]
fn canonical_ids_and_change_set_round_trip_while_legacy_ids_require_migration() {
    let model = cuboid([0.; 3], [2.; 3]).unwrap();
    let canonical = model.to_value();
    let face = canonical["topologyIds"]["faces"][0].as_str().unwrap();
    assert_eq!(face.len(), 34);
    assert!(face.starts_with("f:"));
    assert_eq!(
        canonical["topologyIds"]["changeSet"]["schema"].as_u64(),
        Some(1)
    );
    let restored = Model::from_value(canonical.clone()).unwrap();
    restored.validate().unwrap();

    let mut rejected = canonical.clone();
    rejected["topologyIds"]["faces"][0] = json!("f:0123456789abcdef");
    assert!(Model::from_value(rejected).is_err());

    let mut legacy = canonical;
    legacy["topologyIds"]
        .as_object_mut()
        .unwrap()
        .remove("changeSet");
    for (field, prefix) in [
        ("vertices", "v"),
        ("edges", "e"),
        ("loops", "l"),
        ("faces", "f"),
        ("shells", "s"),
        ("bodies", "b"),
    ] {
        for (index, value) in legacy["topologyIds"][field]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            *value = json!(format!("{prefix}:{:016x}", index + 1));
        }
    }
    let migrated = Model::from_legacy_topology_value(legacy).unwrap();
    migrated.validate().unwrap();
    assert!(migrated.1.faces.iter().all(|id| id.to_string().len() == 34));
}
