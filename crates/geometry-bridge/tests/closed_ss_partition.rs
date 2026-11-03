use geometry_bridge::dispatch;
use value_codec::{Value, json};
#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod intersections;
#[allow(dead_code)]
#[path = "../../brep-core/examples/support/fiber_cut_fixture.rs"]
mod sheets;
fn request() -> Value {
    let s = intersections::circle();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-6;
    b.tolerance_mm = 1e-6;
    json!({"op":"brep_partition_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-6,"toleranceUv":1e-6})
}
#[test]
fn native_dispatch_saves_both_closed_world_uv_partitions() {
    let input = request();
    let before = input.clone();
    let result = dispatch(input.clone()).unwrap();
    assert_eq!(input, before);
    assert_eq!(result["originalSourceClosedCoverage"], true);
    assert_eq!(result["materialClassified"], false);
    assert_eq!(result["a"]["faces"].as_array().unwrap().len(), 2);
    assert_eq!(result["b"]["faces"].as_array().unwrap().len(), 2);
    for key in ["a", "b"] {
        let m: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        m.validate().unwrap();
        assert!(m.persistent_naming_complete());
    }
}
#[test]
fn caller_flags_and_invalid_face_or_tolerance_cannot_admit_topology() {
    for (key, value) in [
        ("materialClassified", json!(true)),
        ("faceA", json!(99)),
        ("toleranceMm", json!(0.)),
        ("toleranceUv", json!(-1.)),
    ] {
        let mut input = request();
        input[key] = value;
        assert!(dispatch(input).is_err(), "accepted {key}");
    }
}
#[test]
fn all_closed_components_become_two_holes_and_three_finite_faces() {
    let s = intersections::two_ovals();
    let mut a = sheets::sheet(s.first().clone());
    let mut b = sheets::sheet(s.second().clone());
    a.tolerance_mm = 1e-4;
    b.tolerance_mm = 1e-4;
    let input = json!({"op":"brep_partition_all_closed_ss","a":a,"b":b,"faceA":0,"faceB":0,"toleranceMm":1e-4,"toleranceUv":1e-4,"maxRepairs":32});
    let before = input.clone();
    let result = dispatch(input.clone()).unwrap();
    assert_eq!(input, before);
    assert_eq!(result["branches"], 2);
    assert_eq!(result["materialClassified"], false);
    for key in ["a", "b"] {
        let m: brep_core::Model =
            value_codec::Deserialize::from_value(result[key].clone()).unwrap();
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 3);
        assert_eq!(m.faces[0].holes.len(), 2);
        assert!(m.persistent_naming_complete());
    }
    let mut invalid = input.clone();
    invalid["maxRepairs"] = json!(65);
    assert!(dispatch(invalid).is_err());
    let mut invalid = input;
    invalid["originalSourceClosedCoverage"] = json!(true);
    assert!(dispatch(invalid).is_err());
}
