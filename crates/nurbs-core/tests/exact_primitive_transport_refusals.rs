#![cfg(feature = "transport")]
#[path = "support/exact_primitive_transport.rs"]
mod support;
use support::*;
use value_codec::{Number, Value, json};
fn refused(request: &Value, reason: &str) {
    let r = envelope(request);
    assert_eq!(r["ok"], true);
    let v = &r["value"];
    assert_eq!(v["status"], "refused");
    assert_eq!(v["refusal"], reason);
    assert!(v["geometry"].is_null());
    assert!(v["metadata"].is_null());
    assert_eq!(v["topologyAuthority"], false);
    assert!(v["workUsed"].as_u64().unwrap() <= request["maxWork"].as_u64().unwrap());
}
fn invalid(request: &Value) {
    let r = envelope(request);
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["code"], "NURBS_INVALID_INPUT");
    assert!(r["value"].is_null());
}
#[test]
fn all_low_budget_reports_are_atomic_and_keep_work_count() {
    for mut r in requests() {
        r["maxWork"] = json!(1u64);
        refused(&r, "resource_limit");
    }
}
#[test]
fn nonrepresentable_poles_never_cross_the_wire_as_approximate_geometry() {
    for (index, mut r) in requests().into_iter().enumerate() {
        r["center"] = json!([0.3, 0., 0.]);
        if index == 0 {
            r["axisU"] = json!([0.1, 0., 0.]);
        } else {
            r["axes"][0][0] = json!(0.1);
        }
        refused(&r, "not_representable");
    }
}
#[test]
fn dependent_axes_keep_the_native_refusal_in_all_five_operations() {
    for (index, mut r) in requests().into_iter().enumerate() {
        if index == 0 {
            r["axisV"] = r["axisU"].clone();
        } else if index == 4 {
            r["axes"][1] = r["axes"][0].clone();
        } else {
            r["axes"][2] = r["axes"][1].clone();
        }
        refused(&r, "dependent_axes");
    }
}
#[test]
fn unknown_fields_and_caller_asserted_certificates_are_rejected() {
    for r in requests() {
        for name in ["exact", "topologyAuthority", "certificate", "max_work"] {
            let mut bad = r.clone();
            bad[name] = json!(true);
            invalid(&bad);
        }
        let mut missing = r;
        missing.as_object_mut().unwrap().remove("maxWork");
        invalid(&missing);
    }
}
#[test]
fn invalid_budget_types_and_unchanged_numeric_limits_are_rejected() {
    for r in requests() {
        for budget in [
            json!(0u64),
            json!(1000001u64),
            json!(-1i64),
            json!(1.5),
            json!(1.0),
            json!("100"),
            Value::Null,
        ] {
            let mut bad = r.clone();
            bad["maxWork"] = budget;
            invalid(&bad);
        }
    }
}
#[test]
fn malformed_shapes_and_nonfinite_numeric_leaves_are_invalid_before_small_budget_work() {
    for r in requests() {
        let mut bad = r.clone();
        bad["maxWork"] = json!(1u64);
        bad["center"] = json!([0., 0.]);
        invalid(&bad);
        let mut bad = r.clone();
        bad["center"] = json!([0., 0., "x"]);
        invalid(&bad);
        let mut bad = r.clone();
        bad["center"][2] = Value::Number(Number::Float(f64::INFINITY));
        assert!(nurbs_core::dispatch(bad).is_err());
    }
}
#[test]
fn integer_coordinates_that_would_round_to_binary64_are_rejected() {
    for n in [
        Number::Unsigned((1u64 << 53) + 1),
        Number::Unsigned(u64::MAX),
        Number::Signed(i64::MAX),
        Number::Signed(-((1i64 << 53) + 1)),
    ] {
        let mut r = requests()[0].clone();
        r["center"][0] = Value::Number(n);
        let answer = envelope(&r);
        assert_eq!(answer["ok"], false);
        assert!(
            answer["error"]["message"]
                .as_str()
                .unwrap()
                .contains("exactly representable")
        );
    }
}
#[test]
fn horn_spindle_zero_width_and_bad_dimensions_keep_native_input_errors() {
    let mut torus = requests()[2].clone();
    for major in [0.5, 1.] {
        torus["majorRadius"] = json!(major);
        invalid(&torus);
    }
    let mut region = requests()[4].clone();
    for radii in [[0., 0.], [1., 1.], [2., 1.], [-1., 1.]] {
        region["radii"] = json!(radii);
        invalid(&region);
    }
    let mut r = requests()[1].clone();
    r["axes"] = json!([[1., 0., 0.], [0., 1., 0.]]);
    invalid(&r);
    let mut r = requests()[4].clone();
    r["axes"] = json!([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
    invalid(&r);
}
#[test]
fn original_two_mib_request_cap_is_retained() {
    let too_large = " ".repeat(2 * 1024 * 1024 + 1);
    let r: Value = value_codec::from_str(&nurbs_core::execute(&too_large)).unwrap();
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["code"], "NURBS_RESOURCE_LIMIT");
}
#[test]
fn exact_but_out_of_range_coordinate_keeps_native_storage_limit() {
    let mut r = requests()[0].clone();
    r["center"] = json!([1u64 << 60, 0u64, 0u64]);
    r["axisU"] = json!([256., 0., 0.]);
    refused(&r, "invalid_output");
}
#[test]
fn lossy_integer_radius_cannot_silently_create_a_unit_disk() {
    let scale = 2f64.powi(-60);
    let mut r = requests()[4].clone();
    r["axes"] = json!([[scale, 0., 0.], [0., scale, 0.]]);
    r["radii"] = json!([0u64, (1u64 << 60) + 1]);
    invalid(&r);
}
