use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn query() -> Value {
    json!({"op":"nurbs_original_quadric_graph_contacts","curve":{"degree":1,"knots":[2.,2.,6.,6.],"controlPoints":[[0.,0.,-0.25],[1.,1.,0.75]],"weights":[1.,1.],"periodic":false},"patch":{"degreeU":1,"degreeV":1,"knotsU":[-2.,-2.,2.,2.],"knotsV":[10.,10.,14.,14.],"controlPoints":[[[0.,0.,0.],[0.,1.,0.]],[[1.,0.,0.],[1.,1.,1.]]],"weights":[[1.,1.],[1.,1.]],"periodicU":false,"periodicV":false},"quadric":[[0.,0.5,0.,0.],[0.5,0.,0.,0.],[0.,0.,0.,-0.5],[0.,0.,-0.5,0.]],"constantProducts":[],"tolerance":1e-9,"maxSpans":10,"maxExactWork":1000000})
}
fn boundary() -> Value {
    let mut q = query();
    q["curve"]["degree"] = json!(3);
    q["curve"]["knots"] = json!([2., 2., 2., 2., 6., 6., 6., 6.]);
    q["curve"]["controlPoints"] = json!([
        [1., 0.5, -0.5],
        [0., 0.5, 1.],
        [-1., 0.5, -0.5],
        [-2., 0.5, -5.]
    ]);
    q["curve"]["weights"] = json!([1., 1., 1., 1.]);
    q
}
fn ledger(r: &Value) {
    assert_eq!(
        r["exactWork"].as_u64().unwrap(),
        r["targetMembership"]["workUsed"].as_u64().unwrap()
            + r["carrier"]["exactWork"].as_u64().unwrap()
            + r["boundaryExactWork"].as_u64().unwrap()
    );
    assert_eq!(
        r["boundaryExactWork"].as_u64().unwrap(),
        r["boundaryProofs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["workUsed"].as_u64().unwrap())
            .sum()
    );
}
#[test]
fn finite_tangent_contact_has_target_proof_and_source_provenance() {
    let q = query();
    let before = q.clone();
    let r = dispatch(q.clone()).unwrap();
    assert_eq!(q, before);
    assert_eq!(r["schema"], "nurbs-original-quadric-graph-contacts/1");
    assert_eq!(r["status"], "present-on-patch");
    assert_eq!(r["targetGraphProven"], true);
    assert_eq!(r["finiteSurfaceUvAdmission"], true);
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(r["contacts"][0]["uvMethod"], "exact-singleton");
    assert_eq!(r["contacts"][0]["multiplicities"], json!([2]));
    assert_eq!(
        r["carrier"]["roots"][0]["normalizedOriginalSpanParameters"],
        json!([[0.5, 0.5]])
    );
    assert_eq!(
        r["contacts"][0]["normalizedOriginalSpanParameters"],
        r["carrier"]["roots"][0]["normalizedOriginalSpanParameters"]
    );
    assert_eq!(r["carrier"]["finiteSurfaceUvAdmission"], false);
    for key in [
        "proofContextSerialized",
        "allSurfaceBranchesEnumerated",
        "trimAdmission",
        "bodyAdmission",
    ] {
        assert_eq!(r[key], false);
    }
    assert!(r["contacts"][0].get("crossingSign").is_none());
    ledger(&r);
}
#[test]
fn carrier_presence_is_not_finite_patch_presence() {
    let mut q = query();
    q["curve"]["controlPoints"] = json!([[2., 0., 0.5], [2., 1., 0.5]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "absent-on-patch");
    assert_eq!(r["carrier"]["status"], "present-on-carrier");
    assert_eq!(r["finiteSurfaceUvAdmission"], false);
    assert_eq!(r["patchAbsenceProven"], true);
    ledger(&r);
}
#[test]
fn coordinate_proofs_serialize_per_query_cost_and_original_root_box() {
    let r = dispatch(boundary()).unwrap();
    assert_eq!(r["status"], "present-on-patch");
    assert_eq!(r["contacts"][0]["uvMethod"], "quadric-coordinate-sign");
    assert!(r["boundaryExactWork"].as_u64().unwrap() > 0);
    assert_eq!(r["boundaryProofs"][0]["coordinateSign"], 0);
    assert_eq!(
        r["boundaryProofs"][0]["normalizedRoot"],
        r["carrier"]["roots"][0]["normalizedOriginalSpanParameters"][0]
    );
    ledger(&r);
}
#[test]
fn perturbed_target_identity_and_zero_budget_remain_unresolved() {
    let mut q = query();
    q["patch"]["controlPoints"][1][1][2] = json!(1f64.next_up());
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["targetGraphProven"], false);
    assert_eq!(r["carrier"], Value::Null);
    assert_eq!(r["finiteSurfaceUvAdmission"], false);
    let mut q = query();
    q["maxExactWork"] = json!(0);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetMembership"]["status"], "indeterminate");
    assert_eq!(r["targetMembership"]["reason"], "ResourceLimit");
    assert_eq!(r["exactWork"], 0);
}
#[test]
fn exhausted_boundary_allowance_never_promotes_a_carrier_root() {
    let full = dispatch(boundary()).unwrap();
    let base = full["targetMembership"]["workUsed"].as_u64().unwrap()
        + full["carrier"]["exactWork"].as_u64().unwrap();
    let mut q = boundary();
    q["maxExactWork"] = json!(base);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["targetGraphProven"], true);
    assert_eq!(r["carrier"]["curveCarrierCoverageComplete"], true);
    assert_eq!(r["finiteSurfaceUvAdmission"], false);
    assert!(!r["ambiguousParameters"].as_array().unwrap().is_empty());
    ledger(&r);
}
#[test]
fn irrational_interior_roots_retain_original_control_bound_method() {
    let mut q = query();
    q["curve"]["controlPoints"] = json!([[0., 0., -0.125], [1., 1., 0.875]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "present-on-patch");
    assert_eq!(r["contacts"].as_array().unwrap().len(), 2);
    for c in r["contacts"].as_array().unwrap() {
        assert_eq!(c["uvMethod"], "original-affine-control-bounds");
    }
    ledger(&r);
}
#[test]
fn caller_admission_and_excessive_work_are_refused() {
    for key in [
        "bodyAdmission",
        "finiteSurfaceUvAdmission",
        "targetGraphProven",
    ] {
        let mut q = query();
        q[key] = json!(true);
        assert!(dispatch(q).is_err());
    }
    let mut q = query();
    q["maxExactWork"] = json!(1000001);
    assert!(dispatch(q).is_err());
}
#[test]
fn ambiguous_surface_sheets_and_nonseparable_target_weights_refuse() {
    let mut q = query();
    q["patch"]["weights"][1][1] = json!(2.);
    assert!(dispatch(q).is_err());
    let mut q = query();
    q["quadric"] = json!([
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., -1.]
    ]);
    assert!(dispatch(q).is_err());
}

#[test]
fn rational_target_projective_clocks_have_explicit_parameterization_metadata() {
    let mut q = query();
    q["patch"]["weights"] = json!([[1., 3.], [2., 6.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "separable-projective");
    assert_eq!(r["status"], "present-on-patch");
    let uv = &r["contacts"][0]["parameterBox"];
    let u = -2. + 4. / 3.;
    assert!(uv[1][0].as_f64().unwrap() <= u && u <= uv[1][1].as_f64().unwrap());
    assert_eq!(uv[2], json!([11., 11.]));
    ledger(&r);
}
#[test]
fn arbitrary_separable_bernstein_clocks_produce_refined_finite_uv_boxes() {
    let mut q = query();
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 14., 14., 14.]);
    q["patch"]["controlPoints"] = json!(
        (0..3)
            .map(|i| (0..3)
                .map(|j| vec![i as f64 / 2., j as f64 / 2., i as f64 * j as f64 / 4.])
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
    q["patch"]["weights"] = json!([[1., 3., 7.], [2., 6., 14.], [5., 15., 35.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "separable-rational");
    assert_eq!(r["finiteSurfaceUvAdmission"], true);
    assert_eq!(r["curvePatchCoverageComplete"], true);
    let b = &r["contacts"][0]["parameterBox"][1];
    assert!(b[0].as_f64().unwrap() < b[1].as_f64().unwrap());
    assert!(b[1].as_f64().unwrap() - b[0].as_f64().unwrap() < 1e-12);
    ledger(&r);
}
