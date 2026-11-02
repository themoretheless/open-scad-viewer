use geometry_bridge::dispatch;
use nurbs_core::surface::Surface;
use value_codec::{Value, json};
fn graph() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 0.5, 1.]
            .into_iter()
            .map(|x| vec![vec![x, 0., x * x], vec![x, 1., x * x]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn request(s: Surface) -> Value {
    json!({"op":"nurbs_surface_self_intersection_audit","surface":s,"maxSpans":100})
}
#[test]
fn public_dispatch_proves_absence_only_from_original_global_proof() {
    let r = dispatch(request(graph())).unwrap();
    assert_eq!(r["schema"], "nurbs-surface-self-intersection/1");
    assert_eq!(r["status"], "absent");
    assert_eq!(r["absenceProven"], true);
    assert_eq!(r["presenceProven"], false);
    assert!(r["exactWitness"].is_null());
    assert_eq!(r["allContactBranchesEnumerated"], false);
}
#[test]
fn dispatch_preserves_non_dyadic_exact_witness_as_decimal_integer_strings() {
    let mut s = graph();
    s.knots_u = vec![0., 0., 0., 1., 1., 3., 3., 3.];
    let a = s.control_points[0].clone();
    let b = s.control_points[1].clone();
    let c = s.control_points[2].clone();
    s.control_points = vec![a.clone(), b, a, c.clone(), c];
    s.weights = vec![vec![1.; 2]; 5];
    let r = dispatch(request(s)).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["presenceProven"], true);
    assert_eq!(r["absenceProven"], false);
    assert!(r["witness"].is_null());
    assert_eq!(
        r["exactWitness"][0][0],
        json!({"numerator":"1","denominator":"12"})
    );
    assert_eq!(
        r["exactWitness"][1][0],
        json!({"numerator":"1","denominator":"4"})
    );
    assert_eq!(r["allContactBranchesEnumerated"], false);
    assert_eq!(r["witnessCoordinates"], "normalized-original-active-domain");
}
#[test]
fn caller_cannot_supply_a_forged_verdict_or_witness() {
    for key in ["presenceProven", "absenceProven", "exactWitness", "report"] {
        let mut v = request(graph());
        v[key] = json!(true);
        assert!(dispatch(v).is_err(), "accepted {key}");
    }
}
#[test]
fn bad_budget_and_invalid_surface_fail_without_a_success_report() {
    let mut v = request(graph());
    v["maxSpans"] = json!(0);
    assert!(dispatch(v).is_err());
    let mut v = request(graph());
    v["surface"]["weights"][0][0] = json!(-1.);
    assert!(dispatch(v).is_err());
    let mut v = request(graph());
    v["maxSpans"] = json!(1.5);
    assert!(dispatch(v).is_err());
}

#[test]
fn insufficient_coverage_stays_unresolved_on_public_dispatch() {
    let mut s = graph();
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    let mut v = request(s);
    v["maxSpans"] = json!(1);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["absenceProven"], false);
    assert_eq!(r["presenceProven"], false);
    assert!(r["witness"].is_null() && r["exactWitness"].is_null());
}

#[test]
fn wire_protocol_preserves_exact_integers_above_javascript_safe_range() {
    let mut s = graph();
    let end = 3.;
    s.knots_u = vec![0., 0., 0., 0.1, 0.1, end, end, end];
    let a = s.control_points[0].clone();
    let b = s.control_points[1].clone();
    let c = s.control_points[2].clone();
    s.control_points = vec![a.clone(), b, a, c.clone(), c];
    s.weights = vec![vec![1.; 2]; 5];
    let request = value_codec::to_string(&request(s)).unwrap();
    let encoded = geometry_bridge::execute(&request);
    let response: Value = value_codec::from_str(&encoded).unwrap();
    assert_eq!(response["ok"], true, "{response:?}");
    let r = &response["value"];
    assert_eq!(r["status"], "present");
    assert_eq!(r["exactWitness"][0][0]["numerator"], "3602879701896397");
    assert_eq!(r["exactWitness"][0][0]["denominator"], "432345564227567616");
    assert!(r["exactWitness"][0][0]["denominator"].is_string());
}

#[test]
fn nonlinear_existence_uses_certified_boxes_without_a_rounded_point() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[-1., -1.], [0., 0.], [1., 1.], [1., -1.], [-1., 1.]]
            .into_iter()
            .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let mut v = request(s);
    v["maxSpans"] = json!(10000);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["allContactBranchesEnumerated"], false);
    assert!(r["witness"].is_null() && r["exactWitness"].is_null());
    assert_eq!(r["certifiedCrossing"]["method"], "original-planar-krawczyk");
    assert!(r["certifiedCrossing"]["cells"].as_u64().unwrap() <= 10000);
    for (i, t) in [5. / 18., 5. / 6.].into_iter().enumerate() {
        let b = &r["certifiedCrossing"]["parameterBoxes"][i][0];
        assert!(b[0].as_f64().unwrap() <= t && t <= b[1].as_f64().unwrap());
    }
}

#[test]
fn bridge_certifies_a_nonrepresentable_rational_iso_without_rounding_controls() {
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [[-1., -1.], [0., 0.], [1., 1.], [1., -1.], [-1., 1.]]
            .into_iter()
            .map(|[x, y]| {
                [0., 1., 2.]
                    .into_iter()
                    .map(|z| vec![x + z, y + z, z])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2., 2.]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let mut v = request(s);
    v["maxSpans"] = json!(10000);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert!(r["witness"].is_null() && r["exactWitness"].is_null());
    assert_eq!(r["allContactBranchesEnumerated"], false);
    for (i, x) in [79. / 63., 79. / 63., 8. / 7.].into_iter().enumerate() {
        let b = &r["certifiedCrossing"]["positionBounds"][i];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
    }
}
