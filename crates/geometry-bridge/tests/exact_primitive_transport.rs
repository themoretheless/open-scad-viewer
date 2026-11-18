#[path = "../../nurbs-core/tests/support/exact_primitive_transport.rs"]
mod support;
use value_codec::{Value, json};
#[test]
fn all_five_operations_reach_the_same_native_reports_through_the_facade() {
    for request in support::requests() {
        let before = request.clone();
        let expected = nurbs_core::dispatch(request.clone()).unwrap();
        let actual = geometry_bridge::dispatch(request.clone()).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(request, before);
        let wire: Value =
            value_codec::from_str(&geometry_bridge::execute(&request.to_string())).unwrap();
        assert_eq!(wire["ok"], true);
        let expected: Value = value_codec::from_str(&expected.to_string()).unwrap();
        assert_eq!(wire["value"], expected);
    }
}
#[test]
fn facade_preserves_atomic_native_work_refusals() {
    for mut request in support::requests() {
        request["maxWork"] = json!(1u64);
        let wire: Value =
            value_codec::from_str(&geometry_bridge::execute(&request.to_string())).unwrap();
        assert_eq!(wire["ok"], true);
        assert_eq!(wire["value"]["status"], "refused");
        assert_eq!(wire["value"]["refusal"], "resource_limit");
        assert!(wire["value"]["geometry"].is_null());
        assert!(wire["value"]["metadata"].is_null());
        assert_eq!(wire["value"]["topologyAuthority"], false);
    }
}
#[test]
fn facade_does_not_accept_caller_forged_exact_primitive_authority() {
    for mut request in support::requests() {
        request["topologyAuthority"] = json!(true);
        assert!(geometry_bridge::dispatch(request.clone()).is_err());
        let wire: Value =
            value_codec::from_str(&geometry_bridge::execute(&request.to_string())).unwrap();
        assert_eq!(wire["ok"], false);
    }
}
