use super::*;

fn source() -> Value {
    let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
    let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.25)
        .unwrap()
        .reverse()
        .unwrap();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = nurbs_core::progressive_sweep::constant_vector_law([1., 0., 0.]).unwrap();
    let twist = nurbs_core::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    json!({"op":"brep_nurbs_progressive_profile_annular_body","loops":[[outer],[hole]],"path":path,
        "scale":scale,"twist":twist,"normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
        "initial_sections":3,"max_sections":3,"max_deviation":0.01,"smoothness_max_work":2000000})
}

#[test]
fn progressive_annular_wire_owns_caps_full_bound_and_actual_jets() {
    let request = source();
    let before = request.clone();
    let body = dispatch(request.clone()).unwrap();
    assert_eq!(request, before);
    assert_eq!(body["model"]["faces"].as_array().unwrap().len(), 24);
    assert_eq!(body["capRepresentation"], "radial-annular");
    assert_eq!(
        body["capFaceGroups"],
        json!([[16, 17, 18, 19], [20, 21, 22, 23]])
    );
    assert_eq!(body["capFaces"].as_array().unwrap().len(), 8);
    assert_eq!(body["planarCapFaces"], json!([]));
    assert_eq!(body["boundaryContinuousBound"], true);
    assert_eq!(body["boundaryErrorWithinBudget"], true);
    assert!(body["boundaryErrorUpper"].as_f64().unwrap() <= 0.01);
    assert_eq!(body["globalEmbeddingCertified"], false);
    assert!(body["retainedCaps"].is_null() && body["capProjection"].is_null());
    let caps = &body["retainedAnnularFilling"];
    assert_eq!(caps["sourceFillingCovered"], true);
    assert_eq!(caps["withinBudget"], true);
    assert_eq!(caps["solidGeometryCertified"], false);
    assert_eq!(
        caps["exactWork"].as_u64().unwrap(),
        caps["constructionExactWork"].as_u64().unwrap()
            + caps["fillingExactWork"].as_u64().unwrap()
    );
    assert!(caps["exactWork"].as_u64().unwrap() <= 1000000);
    assert_eq!(body["retainedSmoothness"]["capWallContinuity"], "C0");
    assert_eq!(
        body["retainedSmoothness"]["fullBoundarySmoothnessCertified"],
        false
    );
    assert!(
        body["retainedSmoothness"]["totalExactWork"]
            .as_u64()
            .unwrap()
            <= 2000000
    );
    assert_eq!(
        body["retainedSmoothness"]["capInternalSeams"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn progressive_annular_wire_refuses_injection_and_unproved_budgets() {
    for key in [
        "model",
        "sections",
        "boundaryCertificate",
        "retainedAnnularFilling",
        "capFaces",
        "planarCapFaces",
        "boundaryContinuousBound",
        "retainedSmoothness",
        "stationReconstruction",
    ] {
        let mut request = source();
        request[key] = json!({"continuousBound":true,"errorUpper":0});
        assert!(
            dispatch(request).is_err(),
            "Injected {key} must not be accepted as an original source"
        );
    }
    let mut zero = source();
    zero["smoothness_max_work"] = json!(0);
    let report = dispatch(zero).unwrap();
    assert_eq!(report["boundaryContinuousBound"], true);
    assert_eq!(report["retainedSmoothness"]["totalExactWork"], 0);
    assert_eq!(
        report["retainedSmoothness"]["walls"]["profile"]["exactG1G2Certified"],
        false
    );
    let mut no_products = source();
    no_products["annular_cap_max_products"] = json!(0);
    let report = dispatch(no_products).unwrap();
    assert_eq!(report["boundaryContinuousBound"], false);
    assert!(report["filledCapErrorUpper"].is_null());
    let mut no_work = source();
    no_work["annular_cap_max_work"] = json!(0);
    assert!(dispatch(no_work).is_err());
    let mut excessive = source();
    excessive["annular_cap_max_work"] = json!(1000001);
    assert!(dispatch(excessive).is_err());
}

#[test]
fn progressive_annular_wire_reconstruction_keeps_all_caps_and_replays_original_source() {
    let request = source();
    let original = dispatch(request.clone()).unwrap();
    let mut reconstruct = request.clone();
    reconstruct["op"] = json!("brep_nurbs_progressive_profile_annular_body_reconstruct_stations");
    reconstruct["expected_source_model"] = original["model"].clone();
    reconstruct["station_quantum"] = json!(0.125);
    reconstruct["station_tolerance"] = json!(1e-9);
    reconstruct["station_max_work"] = json!(1000000);
    let result = dispatch(reconstruct.clone()).unwrap();
    assert_eq!(result["capFaceGroups"], original["capFaceGroups"]);
    let report = &result["stationReconstruction"];
    assert_eq!(report["boundaryCertificate"]["continuousBound"], true);
    assert_eq!(report["boundaryCertificate"]["withinBudget"], true);
    assert_eq!(report["globalEmbeddingCertified"], false);
    assert_eq!(
        report["profileSmoothness"]["station"]["stationG2Certified"],
        true
    );
    let model: brep_core::Model =
        value_codec::from_value(report["reconstruction"]["model"].clone()).unwrap();
    let before: brep_core::Model = value_codec::from_value(original["model"].clone()).unwrap();
    for cap in original["capFaces"].as_array().unwrap() {
        let index = cap.as_u64().unwrap() as usize;
        assert_eq!(model.faces[index], before.faces[index]);
    }
    let x = reconstruct["expected_source_model"]["faces"][16]["surface"]["controlPoints"][0][0][0]
        .as_f64()
        .unwrap();
    reconstruct["expected_source_model"]["faces"][16]["surface"]["controlPoints"][0][0][0] =
        json!(x.next_up());
    assert!(dispatch(reconstruct).is_err());
}

#[test]
fn progressive_auto_caps_preserve_legacy_model_and_source_replay() {
    let mut request=source();request["op"]=json!("brep_nurbs_progressive_profile_auto_body");
    let mut old=request.clone();old["op"]=json!("brep_nurbs_progressive_profile_body");
    let old=dispatch(old).unwrap();
    let body=dispatch(request.clone()).unwrap();
    assert_eq!(body["model"],old["model"]);
    assert_eq!(body["capRepresentation"],"legacy");
    assert_eq!(body["capFaceGroups"],json!([[16],[17]]));
    assert_eq!(body["planarCapFaces"],json!([16,17]));
    assert_eq!(body["boundaryContinuousBound"],true);
    let selected=body["capSelectionExactWork"].as_u64().unwrap();
    assert!(selected>0 && selected+body["retainedCaps"]["exactWork"].as_u64().unwrap()<=1000000);
    let mut short=request.clone();short["annular_cap_max_work"]=json!(selected);
    let refused=dispatch(short).unwrap();
    assert_eq!(refused["boundaryContinuousBound"],false);
    assert!(refused["boundaryErrorUpper"].is_null());
    request["op"]=json!("brep_nurbs_progressive_profile_auto_body_reconstruct_stations");
    request["expected_source_model"]=body["model"].clone();
    request["station_quantum"]=json!(0.125);request["station_tolerance"]=json!(1e-9);
    request["station_max_work"]=json!(1000000);
    let repaired=dispatch(request.clone()).unwrap();
    assert_eq!(repaired["stationReconstruction"]["boundaryCertificate"]["continuousBound"],true);
    assert_eq!(repaired["stationReconstruction"]["profileSmoothness"]["station"]["stationG2Certified"],true);
    request["expected_source_model"]["vertices"][0]["point"][0]=json!(999);
    assert!(dispatch(request).is_err());
}

#[test]
fn progressive_auto_caps_own_oblique_original_source_and_selection_ledger() {
    let mut request=source();request["op"]=json!("brep_nurbs_progressive_profile_auto_body");
    let outer=nurbs_core::primitives::circle([0.;3],[3.,4.,0.],0.1).unwrap();
    let inner=nurbs_core::primitives::circle([0.;3],[3.,4.,0.],0.05).unwrap().reverse().unwrap();
    request["loops"]=json!([[outer],[inner]]);
    request["path"]=json!(nurbs_core::primitives::line([0.;3],[3.,4.,0.]).unwrap());
    request["normal"]=json!([0.,0.,1.]);request["orientation"]=json!("rmf");
    request["initial_sections"]=json!(2);request["max_sections"]=json!(129);
    request["scale"]["controlPoints"][1][0]=json!(2);
    let before=request.clone();let body=dispatch(request.clone()).unwrap();
    assert_eq!(request,before);
    assert_eq!(body["capRepresentation"],"radial-annular");
    assert_eq!(body["planarCapFaces"],json!([]));
    assert_eq!(body["boundaryContinuousBound"],true);
    assert_eq!(body["boundaryErrorWithinBudget"],true);
    let caps=&body["retainedAnnularFilling"];
    assert_eq!(caps["selectionExactWork"],body["capSelectionExactWork"]);
    assert_eq!(caps["exactWork"].as_u64().unwrap(),caps["selectionExactWork"].as_u64().unwrap()
        +caps["constructionExactWork"].as_u64().unwrap()+caps["fillingExactWork"].as_u64().unwrap());
    assert!(caps["exactWork"].as_u64().unwrap()<=1000000);
    request["annular_cap_max_work"]=body["capSelectionExactWork"].clone();
    assert!(dispatch(request).is_err());
}

#[test]
fn progressive_auto_closed_body_reports_absent_caps_not_sharp_caps() {
    let mut request=source();request["op"]=json!("brep_nurbs_progressive_profile_auto_body");
    request["path"]=json!(nurbs_core::primitives::circle([0.;3],[0.,0.,1.],2.).unwrap());
    request["loops"]=json!([[nurbs_core::primitives::circle([2.,0.,0.],[0.,1.,0.],0.5).unwrap()],
        [nurbs_core::primitives::circle([2.,0.,0.],[0.,1.,0.],0.25).unwrap().reverse().unwrap()]]);
    request["orientation"]=json!("rmf");
    request["normal"]=json!([0.,0.,1.]);request["initial_sections"]=json!(9);request["max_sections"]=json!(9);
    request["max_deviation"]=json!(2);
    let body=dispatch(request.clone()).unwrap();
    assert_eq!(body["capRepresentation"],"closed");
    assert_eq!(body["capFaces"],json!([]));
    assert_eq!(body["capFaceGroups"],json!([[],[]]));
    assert_eq!(body["planarCapFaces"],json!([]));
    assert_eq!(body["capSelectionExactWork"],0);
    assert_eq!(body["retainedSmoothness"]["capWallContinuity"],"absent");
    assert!(body["retainedAnnularFilling"].is_null());
    // Profiles are authored at the path start. Moving this pair to the
    // circle center collapses the sweep axis and must not publish a body.
    request["loops"]=json!([[nurbs_core::primitives::circle([0.;3],[0.,1.,0.],0.5).unwrap()],
        [nurbs_core::primitives::circle([0.;3],[0.,1.,0.],0.25).unwrap().reverse().unwrap()]]);
    assert!(dispatch(request).is_err());
}
