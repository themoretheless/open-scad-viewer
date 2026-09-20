use geometry_bridge::dispatch;
use value_codec::{Value, json};

fn request() -> Value {
    let mesh = polygon_core::solid::primitives::cube([2., 3., 4.], false).unwrap();
    json!({"op":"mesh_build_surfaces","mesh":mesh,"buildDirection":[0.,0.,1.],"coneDegrees":45.,"planeOffsetMm":0.,"planeToleranceMm":0.})
}

#[test]
fn transports_only_geometric_quantities_with_an_explicit_model_kind() {
    let report = dispatch(request()).unwrap();
    assert_eq!(
        report,
        json!({"modelKind":"signed-triangle-build-surfaces-v1","totalAreaMm2":52.,"downwardAreaMm2":0.,"downwardTriangles":0,"contactAreaMm2":6.,"belowPlaneTriangles":0})
    );
    let mut floating = request();
    floating["planeOffsetMm"] = json!(-1.);
    assert_eq!(dispatch(floating).unwrap()["downwardAreaMm2"], json!(6.));
}

#[test]
fn refuses_missing_unknown_and_malformed_fields() {
    for name in [
        "buildDirection",
        "coneDegrees",
        "planeOffsetMm",
        "planeToleranceMm",
    ] {
        let mut missing = request();
        missing.as_object_mut().unwrap().remove(name);
        assert!(dispatch(missing).is_err());
    }
    let mut extra = request();
    extra["supports"] = json!(true);
    assert!(dispatch(extra).is_err());
    let mut extra_mesh = request();
    extra_mesh["mesh"]["transform"] = json!([]);
    assert!(dispatch(extra_mesh).is_err());
    let mut direction = request();
    direction["buildDirection"] = json!([0., 1.]);
    assert!(dispatch(direction).is_err());
    let mut index = request();
    index["mesh"]["indices"] = json!([0, 1, 999]);
    assert!(dispatch(index).is_err());
    let mut null = request();
    null["coneDegrees"] = Value::Null;
    assert!(dispatch(null).is_err());
}
