use geometry_bridge::dispatch;
use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::{json, Value};
fn request(order: usize) -> Value {
    let c = Curve {
        degree: order,
        knots: [vec![0.; order + 1], vec![1.; order + 1]].concat(),
        control_points: (0..=order)
            .map(|i| {
                vec![
                    if i % 2 == 0 { 0.1 } else { 1.1 },
                    i as f64 / order as f64,
                    if i % 2 == 0 { 0.1 } else { 0. },
                ]
            })
            .collect(),
        weights: vec![1.; order + 1],
        periodic: false,
    };
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.]],
            vec![vec![1.1, 0., 0.1], vec![1.1, 1., 0.1]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    json!({"op":"nurbs_original_affine_patch_contacts","curve":c,"patch":s,"tolerance":1e-9,"maxSpans":10,"maxExactWork":100000})
}
#[test]
fn exact_contacts_keep_multiplicity_without_a_forged_crossing_sign() {
    for order in [2, 4] {
        let v = request(order);
        let before = v.clone();
        let r = dispatch(v.clone()).unwrap();
        assert_eq!(v, before);
        assert_eq!(r["schema"], "nurbs-original-affine-patch-contacts/1");
        assert_eq!(r["status"], "present");
        assert_eq!(r["curvePatchCoverageComplete"], true);
        assert_eq!(r["proofContextSerialized"], false);
        assert_eq!(r["contacts"].as_array().unwrap().len(), 1);
        let c = &r["contacts"][0];
        assert_eq!(c["multiplicities"], json!([order]));
        assert!(c.get("crossingSign").is_none());
        for key in [
            "allSurfaceBranchesEnumerated",
            "trimAdmission",
            "bodyAdmission",
        ] {
            assert_eq!(r[key], false);
        }
        assert_eq!(
            r["scalarProofs"][0]["normalizedOriginalSpanRoots"][0]["multiplicity"],
            order
        );
        assert!(r["exactWork"].as_u64().unwrap() <= 100000);
    }
}
#[test]
fn zero_exact_budget_keeps_unresolved_source_spans() {
    let mut v = request(2);
    v["maxExactWork"] = json!(0);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["curvePatchCoverageComplete"], false);
    assert_eq!(r["patchAbsenceProven"], false);
    assert_eq!(r["exactWork"], 0);
    assert!(!r["unresolvedSpans"].as_array().unwrap().is_empty());
}
#[test]
fn finite_exclusion_and_exact_boundary_contact_remain_distinct() {
    let mut v = request(2);
    for p in v["curve"]["controlPoints"].as_array_mut().unwrap() {
        p[1] = json!(2);
    }
    let r = dispatch(v.clone()).unwrap();
    assert_eq!(r["status"], "absent-on-patch");
    assert_eq!(r["patchAbsenceProven"], true);
    for p in v["curve"]["controlPoints"].as_array_mut().unwrap() {
        p[1] = json!(0);
    }
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["contacts"][0]["uvMethod"], "exact-boundary-projective");
    assert_eq!(r["contacts"][0]["parameterBox"][2], json!([10., 10.]));
}
#[test]
fn extra_verdicts_and_excessive_exact_work_are_rejected() {
    for (key, value) in [
        ("bodyAdmission", json!(true)),
        ("maxExactWork", json!(cad_predicates::MAX_WORK + 1)),
    ] {
        let mut v = request(2);
        v[key] = value;
        assert!(dispatch(v).is_err());
    }
}

#[test]
fn shared_knot_preserves_the_two_one_sided_orders_in_the_public_report() {
    let mut v = request(2);
    v["curve"] = json!(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: [1., 0., 0., 0.5, 1.]
            .into_iter()
            .enumerate()
            .map(|(i, x)| vec![x, 0.25 + i as f64 * 0.125, 1.])
            .collect(),
        weights: vec![1.; 5],
        periodic: false
    });
    v["patch"] = json!(Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]]
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false
    });
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["contacts"].as_array().unwrap().len(), 1);
    assert_eq!(r["contacts"][0]["sourceSpans"], json!([2, 4]));
    assert_eq!(r["contacts"][0]["multiplicities"], json!([2, 1]));
    assert_eq!(r["contacts"][0]["parameterBox"][0], json!([1., 1.]));
}

#[test]
fn original_projective_target_clocks_are_preserved_in_public_uv_boxes() {
    let mut v = request(2);
    v["patch"]["weights"] = json!([[1, 3], [2, 6]]);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    for (axis, t) in [-2. / 3., 11.].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][axis + 1];
        assert!(b[0].as_f64().unwrap() <= t && t <= b[1].as_f64().unwrap());
    }
}

#[test]
fn coupled_target_uv_boxes_preserve_the_injective_bilinear_proof() {
    let mut v = request(2);
    v["patch"]["weights"] = json!([[1, 1], [1, 2]]);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(r["targetWeightsSeparable"], false);
    assert_eq!(r["targetUvInjectivityProven"], true);
    assert_eq!(r["contacts"][0]["uvMethod"], "injective-bilinear-krawczyk");
    let t = 2f64.sqrt() - 1.;
    for (axis, x) in [-2. + 4. * t, 10. + 4. * t].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][axis + 1];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
    }
}

#[test]
fn nonsingleton_boundary_root_uses_original_coordinate_identity() {
    let mut v = request(2);
    v["curve"]["weights"] = json!([1, 2, 4]);
    for p in v["curve"]["controlPoints"].as_array_mut().unwrap() {
        p[1] = json!(0);
    }
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["uvMethod"],
        "original-affine-coordinate-bounds"
    );
    assert!(r["ambiguousParameters"].as_array().unwrap().is_empty());
}

#[test]
fn nonproportional_boundary_common_root_reports_exact_proof_and_total_work() {
    let mut v = request(3);
    v["curve"]["controlPoints"] = json!([
        [1., 1., 0.5],
        [-1., 0., 0.5],
        [0., -1., 0.5],
        [4., -2., 0.5]
    ]);
    v["patch"]["controlPoints"] =
        json!([[[0., 0., 0.], [0., 0., 1.]], [[0., 1., 0.], [0., 1., 1.]]]);
    v["patch"]["weights"] = json!([[1., 2.], [3., 5.]]);
    let r = dispatch(v).unwrap();
    assert_eq!(r["status"], "present");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["uvMethod"],
        "original-polynomial-common-root"
    );
    assert_eq!(
        r["contacts"][0]["positionBounds"],
        json!([[0., 0.], [0., 0.], [0.5, 0.5]])
    );
    assert_eq!(r["exactWork"], r["scalarProofs"][0]["workUsed"]);
    assert_eq!(r["bodyAdmission"], false);
}

#[test]
fn algebraic_coordinate_sign_distinguishes_interior_and_exterior_reports() {
    for delta in [-2f64.powi(-40), 2f64.powi(-40)] {
        let mut v = request(3);
        let controls: Vec<_> = [1., -1., 0., 4.]
            .into_iter()
            .zip([1., 0., -1., -2.])
            .map(|(x, y)| vec![x, y + delta, 0.5])
            .collect();
        v["curve"]["controlPoints"] = json!(controls);
        v["patch"]["controlPoints"] =
            json!([[[0., 0., 0.], [0., 0., 1.]], [[0., 1., 0.], [0., 1., 1.]]]);
        v["patch"]["weights"] = json!([[1., 2.], [3., 5.]]);
        let r = dispatch(v).unwrap();
        assert_eq!(r["curvePatchCoverageComplete"], true);
        assert_eq!(r["exactWork"], r["scalarProofs"][0]["workUsed"]);
        if delta > 0. {
            assert_eq!(r["status"], "present");
            assert_eq!(
                r["contacts"][0]["uvMethod"],
                "original-polynomial-coordinate-sign"
            );
        } else {
            assert_eq!(r["status"], "absent-on-patch");
            assert_eq!(r["patchAbsenceProven"], true);
        }
    }
}
