use geometry_bridge::dispatch;
use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::json;
#[path = "../../nurbs-core/tests/support/authored_seam_map.rs"]
mod fixture;
#[test]
fn authored_map_native_protocol_keeps_local_and_original_global_evidence_separate() {
    let (a, b, map) = fixture::fixture();
    let request = json!({"op":"nurbs_mapped_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],
        "order":2,"normalScale":1.,"parameterMap":map,"maxExactWork":1000000,
        "continuousBound":true,"originalGlobalCertified":true,"solidCertified":true,"fullBoundaryG2Certified":true,
        "jointContinuousGlobalCertified":true,"jointContinuousGlobalG2Certified":true});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["certified"], true, "{r:?}");
    assert_eq!(r["seamG2Certified"], true);
    assert_eq!(r["regularityCertified"], true);
    for key in [
        "continuousBound",
        "originalGlobalCertified",
        "solidCertified",
        "fullBoundaryG2Certified",
        "jointContinuousGlobalCertified",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(
            r[key], false,
            "{key}: local map must not manufacture full evidence"
        );
    }
    assert!(r["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(r["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut changed = request;
    let mut wrong: Curve = value_codec::from_value(changed["parameterMap"].clone()).unwrap();
    wrong.control_points[1][0] = 0.125f64.next_up();
    changed["parameterMap"] = value_codec::to_value(wrong).unwrap();
    assert_eq!(dispatch(changed).unwrap()["certified"], false);
}

#[test]
fn automatic_map_native_protocol_owns_original_identity_and_never_borrows_global_flags() {
    let (a, b, _) = fixture::fixture();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,
        "certified":true,"continuousBound":true,"originalGlobalCertified":true,"jointContinuousGlobalG2Certified":true});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["certified"], true, "{r:?}");
    assert_eq!(r["mapSource"], "automatic-exact-proof");
    for key in [
        "continuousBound",
        "originalGlobalCertified",
        "solidCertified",
        "fullBoundaryG2Certified",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(r[key], false);
    }
    let mut short = request.clone();
    short["maxExactWork"] = json!(r["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut changed = request;
    let mut patches: [Surface; 2] = value_codec::from_value(changed["patches"].clone()).unwrap();
    patches[1].control_points[4][0][1] = 90.75f64.next_up();
    changed["patches"] = value_codec::to_value(patches).unwrap();
    assert_eq!(dispatch(changed).unwrap()["certified"], false);
}

#[path = "support/rational_closed_material.rs"]
mod material_fixture;
#[test]
fn original_correlated_center_caps_recompute_global_g2_after_step_round_trip() {
    let mut request = material_fixture::request(false, false);
    for piece in request["source"]["motion"].as_array_mut().unwrap() {
        let mut poses: Vec<nurbs_core::affine_volume::Pose> =
            value_codec::from_value(piece["controls"].clone()).unwrap();
        for pose in &mut poses {
            pose[3] = [-0.875 * pose[0][0], -0.875 * pose[0][1], 0.25];
        }
        piece["controls"] = value_codec::to_value(poses).unwrap();
    }
    let full = dispatch(request.clone()).unwrap();
    material_fixture::full(&full, "correlated center construct");
    assert_eq!(full["jointContinuousGlobalG2Certified"], true);
    assert_eq!(
        full["originalMaterialUVBounds"],
        json!([[1., 5.], [-2., 2.]])
    );
    assert!(full["originalProofUVWindow"][0][0].as_f64().unwrap() > 0.);
    assert_eq!(full["proofCoordinates"]["columnFactor"]["dependent"], 3);
    let model: brep_core::Model = value_codec::from_value(full["model"].clone()).unwrap();
    let (text, exported, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(exported.complete);
    let (restored, imported, _) = cad_step::import_step_v9(&text).unwrap();
    assert!(imported.complete);
    request["op"] = json!("brep_rational_closed_material_audit");
    request["model"] = value_codec::to_value(restored).unwrap();
    request["limits"]["constructionRows"] = json!(0);
    let audited = dispatch(request.clone()).unwrap();
    material_fixture::full(&audited, "correlated center STEP reproof");
    assert_eq!(audited["jointContinuousGlobalG2Certified"], true);
    let mut folded = request.clone();
    folded["op"] = json!("brep_rational_closed_material_construct");
    folded["limits"]["constructionRows"] = json!(1000000);
    folded["limits"]["volume"]["maxCells"] = json!(10000);
    folded["originalMaterialUVBounds"] = json!([[3., 5.], [-2., 2.]]);
    folded["originalProofUVWindow"] = json!([[0.6, 0.9], [0.2, 0.8]]);
    folded["jointContinuousGlobalG2Certified"] = json!(true);
    for piece in folded["source"]["motion"].as_array_mut().unwrap() {
        let mut poses: Vec<nurbs_core::affine_volume::Pose> =
            value_codec::from_value(piece["controls"].clone()).unwrap();
        for pose in &mut poses {
            pose[3] = [-2. * pose[0][0], -2. * pose[0][1], 0.25];
        }
        piece["controls"] = value_codec::to_value(poses).unwrap();
    }
    let wrong = dispatch(folded).unwrap();
    assert_eq!(wrong["reason"], "original-rational-filled-volume-unproved");
    assert_eq!(wrong["jointContinuousGlobalG2Certified"], false);
    assert!(wrong["model"].is_null());
    request["limits"]["volume"]["maxCells"] = json!(0);
    request["jointContinuousGlobalG2Certified"] = json!(true);
    let denied = dispatch(request).unwrap();
    assert_eq!(denied["jointContinuousGlobalG2Certified"], false);
    assert!(denied["model"].is_null());
}

#[path = "../../nurbs-core/tests/support/switched_seam_map.rs"]
mod switched_fixture;
#[test]
fn exact_inverse_chart_switch_reaches_native_seam_protocol_without_global_credit() {
    for (a, b) in [
        switched_fixture::fixture(),
        switched_fixture::interior_pole_fixture(),
    ] {
        let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,"jointContinuousGlobalG2Certified":true});
        let r = dispatch(request.clone()).unwrap();
        assert_eq!(r["seamG2Certified"], true, "{r:?}");
        assert_eq!(r["jointContinuousGlobalG2Certified"], false);
        assert!(r["model"].is_null());
        let mut short = request;
        short["maxExactWork"] = json!(r["exactWork"].as_u64().unwrap() - 1);
        assert_eq!(dispatch(short).unwrap()["certified"], false);
    }
}
fn sheared_material_request() -> value_codec::Value {
    let mut r = material_fixture::request(false, false);
    let mut trims: Vec<Vec<Curve>> =
        value_codec::from_value(r["material"]["trims"].clone()).unwrap();
    for c in trims.iter_mut().flatten() {
        for p in &mut c.control_points {
            p[0] += p[1];
        }
    }
    r["material"]["trims"] = value_codec::to_value(trims).unwrap();
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-4., -4., 10., 10.],
        knots_v: vec![-4., -4., 4., 4.],
        control_points: vec![
            vec![vec![0., -4., 0.], vec![-8., 4., 0.]],
            vec![vec![14., -4., 0.], vec![6., 4., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    r["material"]["surface"] = value_codec::to_value(s).unwrap();
    for piece in r["source"]["motion"].as_array_mut().unwrap() {
        let mut poses: Vec<nurbs_core::affine_volume::Pose> =
            value_codec::from_value(piece["controls"].clone()).unwrap();
        for p in &mut poses {
            p[3] = [-0.875 * p[0][0], -0.875 * p[0][1], 0.25];
        }
        piece["controls"] = value_codec::to_value(poses).unwrap();
    }
    r
}
#[test]
fn exact_original_material_plane_chart_owns_step_g2_and_rejects_forged_clip() {
    let mut r = sheared_material_request();
    let full = dispatch(r.clone()).unwrap();
    material_fixture::full(&full, "original affine carrier plane");
    assert_eq!(full["jointContinuousGlobalG2Certified"], true);
    assert!(full["originalProofUVWindow"].is_null());
    assert!(
        full["originalMaterialUVBounds"][0][0].as_f64().unwrap() < 0.875
            && full["originalMaterialUVBounds"][0][1].as_f64().unwrap() > 0.875
    );
    assert!(
        full["originalMaterialPlaneChart"]["bounds"][0][0]
            .as_f64()
            .unwrap()
            > 0.875
    );
    let model: brep_core::Model = value_codec::from_value(full["model"].clone()).unwrap();
    let (step, ex, _) = cad_step::export_step_v9(&model).unwrap();
    assert!(ex.complete);
    let (model, im, _) = cad_step::import_step_v9(&step).unwrap();
    assert!(im.complete);
    r["op"] = json!("brep_rational_closed_material_audit");
    r["model"] = value_codec::to_value(model).unwrap();
    r["limits"]["constructionRows"] = json!(0);
    let audit = dispatch(r.clone()).unwrap();
    material_fixture::full(&audit, "material plane STEP source reproof");
    assert_eq!(audit["jointContinuousGlobalG2Certified"], true);
    r["op"] = json!("brep_rational_closed_material_construct");
    r["limits"]["constructionRows"] = json!(1000000);
    r["limits"]["volume"]["maxCells"] = json!(10000);
    for piece in r["source"]["motion"].as_array_mut().unwrap() {
        let mut poses: Vec<nurbs_core::affine_volume::Pose> =
            value_codec::from_value(piece["controls"].clone()).unwrap();
        for p in &mut poses {
            p[3] = [-2. * p[0][0], -2. * p[0][1], 0.25];
        }
        piece["controls"] = value_codec::to_value(poses).unwrap();
    }
    r["originalMaterialPlaneChart"] = json!({"bounds":[[3.,5.],[-2.,2.]],"determinant":[1.,1.]});
    r["jointContinuousGlobalG2Certified"] = json!(true);
    let wrong = dispatch(r).unwrap();
    assert_eq!(wrong["reason"], "original-rational-filled-volume-unproved");
    assert_eq!(wrong["jointContinuousGlobalG2Certified"], false);
    assert!(wrong["model"].is_null());
}

#[test]
fn implicit_algebraic_seam_reaches_native_protocol_and_never_grants_solid_credit() {
    let make = |degree, xy: &[[f64; 2]], z, dz| Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + dz * j as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; xy.len()],
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(
        4,
        &[[0., 0.], [7.5, 0.], [20., 2.5], [37.5, 15.], [60., 60.]],
        0.,
        0.5,
    );
    let b = make(
        6,
        &[
            [0., 0.],
            [5., 0.],
            [10., 1.],
            [16.5, 3.],
            [26., 8.],
            [40., 20.],
            [60., 60.],
        ],
        1.,
        1.,
    );
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],
        "order":2,"normalScale":2.,"maxExactWork":1000000,"jointContinuousGlobalG2Certified":true,"solidCertified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    assert_eq!(full["jointContinuousGlobalG2Certified"], false);
    assert_eq!(full["solidCertified"], false);
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut changed = request;
    let mut patches: [Surface; 2] = value_codec::from_value(changed["patches"].clone()).unwrap();
    patches[1].control_points[1][0][1] = 0.125;
    changed["patches"] = value_codec::to_value(patches).unwrap();
    assert_eq!(dispatch(changed).unwrap()["certified"], false);
}

#[path = "../../nurbs-core/tests/support/branched_seam_map.rs"]
mod branched_fixture;
#[test]
fn branched_algebraic_native_protocol_owns_order_and_refuses_a_second_lap() {
    let (a, b) = branched_fixture::fixture(1);
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],
        "order":2,"normalScale":1.,"maxExactWork":1000000,"jointContinuousGlobalG2Certified":true,"solidCertified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    assert_eq!(full["jointContinuousGlobalG2Certified"], false);
    assert_eq!(full["solidCertified"], false);
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let (_, twice) = branched_fixture::fixture(2);
    let mut wrong = request;
    wrong["patches"][1] = value_codec::to_value(twice).unwrap();
    assert_eq!(dispatch(wrong).unwrap()["certified"], false);
}

#[test]
fn independently_cut_branches_native_keeps_seam_and_solid_evidence_separate() {
    let (a, b) = branched_fixture::independently_cut();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],
        "order":2,"normalScale":1.,"maxExactWork":1000000,"continuousBound":true,"jointContinuousGlobalG2Certified":true,"solidCertified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    for key in [
        "continuousBound",
        "jointContinuousGlobalG2Certified",
        "solidCertified",
    ] {
        assert_eq!(full[key], false);
    }
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut changed = b;
    changed.control_points[2][0][0] = changed.control_points[2][0][0].next_up();
    let mut wrong = request;
    wrong["patches"][1] = value_codec::to_value(changed).unwrap();
    assert_eq!(dispatch(wrong).unwrap()["certified"], false);
}

#[path = "../../nurbs-core/tests/support/overlap_seam_map.rs"]
mod overlap_fixture;
#[test]
fn algebraic_overlap_native_refuses_forged_global_evidence_and_short_work() {
    let (a, b) = overlap_fixture::fixture();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,"certified":true,"solidCertified":true,"continuousBound":true,"jointContinuousGlobalG2Certified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    for key in [
        "solidCertified",
        "continuousBound",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(full[key], false);
    }
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut changed = b;
    changed.control_points[2][0][0] = changed.control_points[2][0][0].next_up();
    let mut wrong = request;
    wrong["patches"][1] = value_codec::to_value(changed).unwrap();
    assert_eq!(dispatch(wrong).unwrap()["certified"], false);
}

#[path = "../../nurbs-core/tests/support/ramified_seam_map.rs"]
mod ramified_fixture;
#[test]
fn ramified_algebraic_native_never_promotes_projection_or_real_singularity() {
    let (a, b) = ramified_fixture::fixture();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,"certified":true,"regularityCertified":true,"continuousBound":true,"solidCertified":true,"jointContinuousGlobalG2Certified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    for key in [
        "continuousBound",
        "solidCertified",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(full[key], false);
    }
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let (c, d) = ramified_fixture::rank_loss();
    let mut wrong = request;
    wrong["patches"] = json!([c, d]);
    let bad = dispatch(wrong).unwrap();
    assert_eq!(bad["certified"], false);
    assert_eq!(bad["regularityCertified"], false);
    assert!(bad["model"].is_null());
}

#[path = "../../nurbs-core/tests/support/circuit_seam_map.rs"]
mod circuit_fixture;
#[test]
fn rational_circuit_curve_native_stays_local_and_refuses_short_work() {
    let (a, b) = circuit_fixture::fixture();
    let request = json!({"op":"nurbs_surface_seam_audit","patches":[a,b],"boundaries":["vMax","vMin"],"order":2,"normalScale":1.,"maxExactWork":1000000,"certified":true,"solidCertified":true,"continuousBound":true,"jointContinuousGlobalG2Certified":true});
    let full = dispatch(request.clone()).unwrap();
    assert_eq!(full["seamG2Certified"], true, "{full:?}");
    for key in [
        "solidCertified",
        "continuousBound",
        "jointContinuousGlobalG2Certified",
    ] {
        assert_eq!(full[key], false);
    }
    assert!(full["model"].is_null());
    let mut short = request.clone();
    short["maxExactWork"] = json!(full["exactWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["certified"], false);
    let mut bad = b;
    bad.control_points[2][0][0] = bad.control_points[2][0][0].next_up();
    let mut wrong = request;
    wrong["patches"][1] = value_codec::to_value(bad).unwrap();
    assert_eq!(dispatch(wrong).unwrap()["certified"], false);
}
