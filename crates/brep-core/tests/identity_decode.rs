use brep_core::{Model, cuboid};
use value_codec::{Deserialize, Serialize, json};

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
