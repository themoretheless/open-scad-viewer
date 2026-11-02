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

fn off_center_surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 3., 3.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[0., 0.], [1., 1.], [0., 1.], [1., 0.]]
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| {
                let z = if i == 3 { 1. } else { 0. };
                vec![vec![x, y, -0.25 * z], vec![x, y, 1. + 0.75 * z]]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
fn iso_request(surface: Surface, parameter: f64) -> Value {
    json!({"op":"nurbs_surface_iso_self_intersection_audit","surface":surface,"axis":0,"parameter":parameter,"tolerance":1e-7,"maxSpans":1000})
}
#[test]
fn off_center_iso_contact_is_not_a_whole_surface_coverage_claim() {
    let s = off_center_surface();
    let mid = dispatch(iso_request(s.clone(), 0.5)).unwrap();
    assert_eq!(mid["status"], "unresolved");
    let r = dispatch(iso_request(s, 0.25)).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["presenceProven"], true);
    assert_eq!(r["surfaceAbsenceProven"], false);
    assert_eq!(r["allContactBranchesEnumerated"], false);
    for i in 0..2 {
        assert_eq!(
            r["crossings"][0]["parameterBoxes"][i][1],
            json!([0.25, 0.25])
        );
    }
}
#[test]
fn simple_iso_does_not_prove_surface_absence() {
    let r = dispatch(iso_request(graph(), 0.25)).unwrap();
    assert_eq!(r["status"], "iso-simple");
    assert_eq!(r["isoAbsenceProven"], true);
    assert_eq!(r["surfaceAbsenceProven"], false);
    assert_eq!(r["isoCoverageComplete"], true);
}
#[test]
fn iso_probe_requests_cannot_supply_a_verdict_or_invalid_parameter() {
    let mut v = iso_request(off_center_surface(), 0.25);
    v["surfaceAbsenceProven"] = json!(true);
    assert!(dispatch(v).is_err());
    for t in [0., 1., -0.1, 1.1] {
        assert!(dispatch(iso_request(off_center_surface(), t)).is_err());
    }
}

#[test]
fn iso_simple_never_hides_a_proven_contact_on_another_fiber() {
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [
            [9. / 16., -9. / 32.],
            [-7. / 16., 13. / 32.],
            [-7. / 16., -13. / 32.],
            [9. / 16., 9. / 32.],
        ]
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| vec![vec![x, y, 0.], vec![i as f64, 0., 1.]])
        .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let mut v = iso_request(s.clone(), 0.125);
    v["maxSpans"] = json!(10000);
    let contact = dispatch(v).unwrap();
    assert_eq!(contact["status"], "present");
    let simple = dispatch(iso_request(s, 0.75)).unwrap();
    assert_eq!(simple["status"], "iso-simple");
    assert_eq!(simple["isoAbsenceProven"], true);
    assert_eq!(simple["surfaceAbsenceProven"], false);
}

#[test]
fn default_surface_audit_reports_automatic_off_center_search_without_full_coverage() {
    let r = dispatch(request(off_center_surface())).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["allContactBranchesEnumerated"], false);
    let search = &r["probeSearch"];
    assert_eq!(search["surfaceCoverageComplete"], false);
    assert!(search["chargedCells"].as_u64().unwrap() <= 100);
    assert!(
        search["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["axis"] == 0 && p["parameter"] == 0.25)
    );
    for i in 0..2 {
        assert_eq!(
            r["certifiedCrossing"]["parameterBoxes"][i][1],
            json!([0.25, 0.25])
        );
    }
}
