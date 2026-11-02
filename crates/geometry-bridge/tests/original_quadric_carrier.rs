use geometry_bridge::dispatch;
use value_codec::{Value, json};
fn query(y: f64) -> Value {
    json!({"op":"nurbs_original_quadric_carrier_roots",
 "curve":{"degree":1,"knots":[-2.,-2.,5.,5.],"controlPoints":[[-2.,y,0.],[2.,y,0.]],"weights":[1.,1.],"periodic":false},
 "quadric":[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,-1.]],"constantProducts":[],"tolerance":1e-9,"maxSpans":10,"maxExactWork":100000})
}
#[test]
fn transverse_and_tangent_roots_are_carrier_only() {
    for (y, count, order) in [(0., 2, 1), (1., 1, 2)] {
        let q = query(y);
        let before = q.clone();
        let r = dispatch(q.clone()).unwrap();
        assert_eq!(q, before);
        assert_eq!(r["schema"], "nurbs-original-quadric-carrier-roots/1");
        assert_eq!(r["status"], "present-on-carrier");
        assert_eq!(r["roots"].as_array().unwrap().len(), count);
        assert_eq!(r["curveCarrierCoverageComplete"], true);
        assert_eq!(r["roots"][0]["multiplicities"], json!([order]));
        for key in [
            "finiteSurfaceUvAdmission",
            "allSurfaceBranchesEnumerated",
            "trimAdmission",
            "bodyAdmission",
            "proofContextSerialized",
        ] {
            assert_eq!(r[key], false);
        }
        assert!(r["roots"][0].get("crossingSign").is_none());
        assert_eq!(r["exactWork"], r["scalarProofs"][0]["workUsed"]);
    }
}
#[test]
fn complete_absence_and_zero_budget_are_distinct() {
    let r = dispatch(query(2.)).unwrap();
    assert_eq!(r["status"], "absent-on-carrier");
    assert_eq!(r["carrierAbsenceProven"], true);
    let mut q = query(0.);
    q["maxExactWork"] = json!(0);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["exactWork"], 0);
    assert_eq!(r["unresolvedSpans"], json!([1]));
}
#[test]
fn original_constant_products_preserve_exact_coincidence() {
    let mut q = query(0.);
    q["curve"]["controlPoints"] = json!([[1.1, 0., 0.], [1.1, 0., 0.]]);
    q["quadric"][3][3] = json!(0.);
    q["constantProducts"] = json!([[1.1, -1.1]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "unresolved");
    assert_eq!(r["scalarProofs"][0]["status"], "coincident");
    assert_eq!(r["curveCarrierCoverageComplete"], false);
}
#[test]
fn degree_50_multiplicity_is_not_truncated_by_the_bridge() {
    let mut q = query(0.);
    q["curve"]["degree"] = json!(25);
    q["curve"]["knots"] = json!([vec![0.; 26], vec![1.; 26]].concat());
    q["curve"]["controlPoints"] = json!(
        (0..26)
            .map(|i| vec![if i == 25 { 1. } else { 0. }, 0., 0.])
            .collect::<Vec<_>>()
    );
    q["curve"]["weights"] = json!(vec![1.; 26]);
    q["quadric"] = json!([
        [1., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., 0.]
    ]);
    q["maxExactWork"] = json!(1000000);
    let r = dispatch(q).unwrap();
    assert_eq!(r["status"], "present-on-carrier");
    assert_eq!(r["roots"][0]["multiplicities"], json!([50]));
    assert_eq!(
        r["scalarProofs"][0]["normalizedOriginalSpanRoots"][0]["multiplicity"],
        50
    );
}
#[test]
fn unknown_admission_fields_and_over_limit_work_are_refused() {
    for key in ["bodyAdmission", "finiteSurfaceUvAdmission", "targetSurface"] {
        let mut q = query(0.);
        q[key] = json!(true);
        assert!(dispatch(q).is_err());
    }
    let mut q = query(0.);
    q["maxExactWork"] = json!(1000001);
    assert!(dispatch(q).is_err());
}
#[test]
fn malformed_quadric_products_and_nonpositive_weights_are_refused() {
    let mut q = query(0.);
    q["quadric"] = json!([[1., 0., 0.]]);
    assert!(dispatch(q).is_err());
    let mut q = query(0.);
    q["constantProducts"] = json!([[1., 2., 3.]]);
    assert!(dispatch(q).is_err());
    let mut q = query(0.);
    q["curve"]["weights"] = json!([0., 1.]);
    assert!(dispatch(q).is_err());
}
