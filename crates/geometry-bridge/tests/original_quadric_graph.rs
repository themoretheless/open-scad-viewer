use geometry_bridge::dispatch;
use value_codec::{json, Value};
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
    q["patch"]["controlPoints"] = json!((0..3)
        .map(|i| (0..3)
            .map(|j| vec![i as f64 / 2., j as f64 / 2., i as f64 * j as f64 / 4.])
            .collect::<Vec<_>>())
        .collect::<Vec<_>>());
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

#[test]
fn multispan_original_graph_serializes_global_natural_coordinates() {
    let mut q = query();
    q["patch"]["knotsU"] = json!([-2., -2., 0., 2., 2.]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 1., 0.]],
        [[0.5, 0., 0.], [0.5, 1., 0.5]],
        [[1., 0., 0.], [1., 1., 1.]]
    ]);
    q["patch"]["weights"] = json!([[1., 1.], [1., 1.], [1., 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetGraphProven"], true);
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(r["targetParameterization"], "polynomial-affine");
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [0., 0.], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn rational_multispan_graph_keeps_target_clock_proof_and_work_ledger() {
    let mut q = query();
    q["patch"]["knotsU"] = json!([-2., -2., -1., 1., 2., 2.]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 1., 0.]],
        [[0.25, 0., 0.], [0.25, 1., 0.25]],
        [[0.75, 0., 0.], [0.75, 1., 0.75]],
        [[1., 0., 0.], [1., 1., 1.]]
    ]);
    q["patch"]["weights"] = json!([[1., 1.], [3., 3.], [5., 5.], [7., 7.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "separable-rational");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [-0.25, -0.25], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn coupled_planar_bilinear_clock_serializes_certified_joint_uv() {
    let mut q = query();
    q["patch"]["controlPoints"] =
        json!([[[0., 0., 0.], [0., 1., 0.]], [[1., 0., 0.], [1., 1., 0.]]]);
    q["patch"]["weights"] = json!([[1., 3.], [2., 2.]]);
    q["quadric"] = json!([
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., 0.5],
        [0., 0., 0.5, 0.]
    ]);
    q["curve"]["controlPoints"] = json!([[0.5, 0.625, -1.], [0.5, 0.625, 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "coupled-bilinear");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    for (i, x) in [4., 0., 12.].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][i];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
    }
    ledger(&r);
}

#[test]
fn coupled_bernstein_planar_target_serializes_joint_uv_enclosures() {
    let mut q = query();
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 14., 14., 14.]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 0.5, 0.], [0., 1., 0.]],
        [[0.5, 0., 0.], [0.5, 0.5, 0.], [0.5, 1., 0.]],
        [[1., 0., 0.], [1., 0.5, 0.], [1., 1., 0.]]
    ]);
    q["patch"]["weights"] = json!([[1., 1., 1.], [1., 1., 1.], [1., 1., 17.]]);
    q["quadric"] = json!([
        [0., 0., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., 0.5],
        [0., 0., 0.5, 0.]
    ]);
    q["curve"]["controlPoints"] = json!([[0.75, 0.75, -1.], [0.75, 0.75, 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "coupled-bernstein");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    for (i, x) in [4., 0., 12.].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][i];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
    }
    ledger(&r);
}

#[test]
fn nonplanar_coupled_common_factor_is_proved_on_original_tensor_and_preserves_exact_uv() {
    let mut q = query();
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 14., 14., 14.]);
    q["patch"]["weights"] = json!([[1., 1., 1.], [1., 1.25, 1.5], [1., 1.5, 2.]]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 7.5, 0.], [0., 15., 0.]],
        [[7.5, 0., 0.], [6., 6., 45.], [5., 15., 75.]],
        [[15., 0., 0.], [15., 5., 75.], [15., 15., 225.]]
    ]);
    q["curve"]["controlPoints"] = json!([[7.5, 7.5, 55.25], [7.5, 7.5, 57.25]]);
    let before = q.clone();
    let r = dispatch(q.clone()).unwrap();
    assert_eq!(q, before);
    assert_eq!(r["targetParameterization"], "homogeneous-affine");
    assert_eq!(r["targetGraphProven"], true);
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [0., 0.], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn original_nonplanar_multispan_coupled_tensor_preserves_global_affine_uv() {
    let mut q = query();
    let l = 1808800.;
    let g = [0., 1., 3., 4.];
    let k = [0., 0., 2., 4.];
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 0., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 12., 14., 14., 14.]);
    let w: Vec<Vec<f64>> = g
        .iter()
        .map(|a| g.iter().map(|b| (16. + a * b) / 16.).collect())
        .collect();
    let p: Vec<Vec<Vec<f64>>> = (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    let d = 16. + g[i] * g[j];
                    vec![
                        l * (4. * g[i] + k[i] * g[j]) / d,
                        l * (4. * g[j] + g[i] * k[j]) / d,
                        l * (g[i] * g[j] + k[i] * k[j]) / d,
                    ]
                })
                .collect()
        })
        .collect();
    q["patch"]["weights"] = json!(w);
    q["patch"]["controlPoints"] = json!(p);
    q["quadric"] = json!([
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.]
    ]);
    q["curve"]["controlPoints"] =
        json!([[l / 2., l / 2., l / 4. - 1.], [l / 2., l / 2., l / 4. + 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "homogeneous-affine");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [0., 0.], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn original_nonlinear_boundary_clocks_serialize_refined_nonplanar_uv() {
    let mut q = query();
    let l = 195.;
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 14., 14., 14.]);
    q["patch"]["weights"] = json!([[4., 3., 2.], [3., 3.25, 2.5], [2., 2.5, 2.]]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 65., 0.], [0., 195., 0.]],
        [[65., 0., 0.], [45., 45., 15.], [39., 195., 39.]],
        [[195., 0., 0.], [195., 39., 39.], [195., 195., 195.]]
    ]);
    q["quadric"] = json!([
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.]
    ]);
    q["curve"]["controlPoints"] =
        json!([[l / 2., l / 2., l / 4. - 1.], [l / 2., l / 2., l / 4. + 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "homogeneous-separable");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    for (axis, x) in [4., 2. / 3., 38. / 3.].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][axis];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
        assert!(b[1].as_f64().unwrap() - b[0].as_f64().unwrap() < 1e-12);
    }
    ledger(&r);
}

#[test]
fn original_multispan_nonlinear_clocks_serialize_global_internal_knot_uv() {
    let mut q = query();
    let l = 117733140.;
    let g = [0., 1., 3., 4.];
    let k = [0., 0., 2., 4.];
    let d = |i: usize, j: usize| {
        64. - 8. * g[i] - 8. * g[j] + 5. * g[i] * g[j] - 2. * k[i] * g[j] - 2. * g[i] * k[j]
            + k[i] * k[j]
    };
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["degreeV"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 0., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 10., 12., 14., 14., 14.]);
    let w: Vec<Vec<f64>> = (0..4)
        .map(|i| (0..4).map(|j| d(i, j) / 16.).collect())
        .collect();
    let p: Vec<Vec<Vec<f64>>> = (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    vec![
                        l * (8. * g[i] - g[i] * g[j] + 2. * k[i] * g[j] - k[i] * k[j]) / d(i, j),
                        l * (8. * g[j] - g[i] * g[j] + 2. * g[i] * k[j] - k[i] * k[j]) / d(i, j),
                        l * (g[i] * g[j] + k[i] * k[j]) / d(i, j),
                    ]
                })
                .collect()
        })
        .collect();
    q["patch"]["weights"] = json!(w);
    q["patch"]["controlPoints"] = json!(p);
    q["quadric"] = json!([
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.]
    ]);
    q["curve"]["controlPoints"] =
        json!([[l / 3., l / 3., l / 9. - 1.], [l / 3., l / 3., l / 9. + 1.]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "homogeneous-separable");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [0., 0.], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn stationary_unordered_clock_has_finite_uv_without_differential_or_body_credit() {
    let mut q = query();
    q["patch"]["degreeU"] = json!(3);
    q["patch"]["knotsU"] = json!([-2., -2., -2., -2., 2., 2., 2., 2.]);
    q["patch"]["weights"] = json!([[1., 1.], [1., 1.], [1., 1.], [1., 1.]]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 1., 0.]],
        [[1., 0., 0.], [1., 1., 1.]],
        [[0., 0., 0.], [0., 1., 0.]],
        [[1., 0., 0.], [1., 1., 1.]]
    ]);
    q["curve"]["controlPoints"] = json!([[0.5, 0.5, -0.75], [0.5, 0.5, 1.25]]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], "homogeneous-separable");
    assert_eq!(r["curvePatchCoverageComplete"], true);
    assert_eq!(r["finiteSurfaceUvAdmission"], true);
    assert_eq!(r["trimAdmission"], false);
    assert_eq!(r["bodyAdmission"], false);
    assert_eq!(
        r["contacts"][0]["parameterBox"],
        json!([[4., 4.], [0., 0.], [12., 12.]])
    );
    ledger(&r);
}

#[test]
fn triangular_joint_original_graph_reports_finite_uv_without_body_credit() {
    let mut q = query();
    q["curve"]["controlPoints"] = json!([[1.5, 1.5, -0.25], [1.5, 1.5, 1.75]]);
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 2., 2., 2.]);
    q["patch"]["controlPoints"] = json!([
        [[0., 0., 0.], [0., 3., 0.]],
        [[1.5, 0., 0.], [1., 3., 1.]],
        [[3., 0., 0.], [3., 3., 3.]]
    ]);
    q["patch"]["weights"] = json!([[1., 1.], [1., 1.5], [1., 2.]]);
    q["quadric"] = json!([
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -1.5],
        [0., 0., -1.5, 0.]
    ]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], json!("triangular-rational"));
    assert_eq!(r["finiteSurfaceUvAdmission"], json!(true));
    assert_eq!(r["trimAdmission"], json!(false));
    assert_eq!(r["bodyAdmission"], json!(false));
    ledger(&r);
}

#[test]
fn multispan_triangular_joint_graph_retains_global_conditional_uv_and_no_body_credit() {
    let mut q = query();
    let l = 1680.;
    let a = [0., 1., 2., 4.];
    let c = [0., 0.25, 0.75, 1.];
    let weights: Vec<Vec<f64>> = (0..4)
        .map(|i| {
            a.iter()
                .map(|a| [1., 1. + a / 4., 1. + 3. * a / 4., 1. + a][i])
                .collect()
        })
        .collect();
    let poles: Vec<Vec<Vec<f64>>> = (0..4)
        .map(|i| {
            a.iter()
                .enumerate()
                .map(|(j, a)| {
                    let x = [0., l / (4. + a), l * (3. + 2. * a) / (4. + 3. * a), l][i];
                    vec![x, l * c[j], x * c[j]]
                })
                .collect()
        })
        .collect();
    q["curve"]["controlPoints"] = json!([[840., 840., 419.], [840., 840., 421.]]);
    q["patch"]["degreeU"] = json!(2);
    q["patch"]["knotsU"] = json!([-2., -2., -2., 0., 2., 2., 2.]);
    q["patch"]["knotsV"] = json!([10., 10., 11., 13., 14., 14.]);
    q["patch"]["controlPoints"] = json!(poles);
    q["patch"]["weights"] = json!(weights);
    q["quadric"] = json!([
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -840.],
        [0., 0., -840., 0.]
    ]);
    let r = dispatch(q).unwrap();
    assert_eq!(r["targetParameterization"], json!("triangular-rational"));
    assert_eq!(r["finiteSurfaceUvAdmission"], json!(true));
    assert_eq!(r["trimAdmission"], json!(false));
    assert_eq!(r["bodyAdmission"], json!(false));
    ledger(&r);
    for (axis, x) in [4., 0., 11. + 6. / 7.].into_iter().enumerate() {
        let b = &r["contacts"][0]["parameterBox"][axis];
        assert!(b[0].as_f64().unwrap() <= x && x <= b[1].as_f64().unwrap());
    }
}
