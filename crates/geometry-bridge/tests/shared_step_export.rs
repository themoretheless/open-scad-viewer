use geometry_bridge::dispatch;
use value_codec::json;

#[test]
fn editor_payload_exports_one_local_definition_at_two_distinct_placements() {
    let block = brep_core::freeform_cuboid_solid([0.; 3], [2., 3., 4.]).unwrap();
    let place = |x| [[1., 0., 0., x], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
    let mut payload = json!({
        "op":"brep_nurbs_export_shared_step", "definitions":[block],
        "occurrences":[{"definition":0,"transform":place(0.)},{"definition":0,"transform":place(17.)}]
    });
    let result = dispatch(payload.clone()).unwrap();
    let scene = cad_step::import_step_shared_assembly(result["text"].as_str().unwrap()).unwrap();
    assert_eq!(scene.definitions().len(), 1);
    assert_eq!(scene.occurrences().len(), 2);
    assert_eq!(scene.occurrences()[1].transform, place(17.));
    let mut grouped = payload.clone();
    grouped["groups"] = json!(["Drive", "Spare"]);
    grouped["parents"] = json!([None::<usize>, Some(0usize)]);
    let mut styled = grouped.clone();
    styled["colours"] = json!([{"name":"Blue","rgb":[0.,0.,1.]},{"name":"Red","rgb":[1.,0.,0.]}]);
    let result = dispatch(styled.clone()).unwrap();
    let text = result["text"].as_str().unwrap();
    assert_eq!(text.matches("=PRESENTATION_STYLE_BY_CONTEXT(").count(), 2);
    let scene = cad_step::import_step_shared_assembly(text).unwrap();
    assert_eq!(scene.definitions().len(), 1);
    assert!(scene.occurrences().iter().any(|occ| occ.transform == place(17.)));
    styled["colours"] = json!([{"name":"Blue","rgb":[0.,0.,1.]}]);
    assert!(dispatch(styled.clone()).is_err());
    styled["colours"] = json!([{"name":"Blue","rgb":[0.,0.,1.]},value_codec::Value::Null]);
    let sparse = dispatch(styled).unwrap();
    assert_eq!(sparse["text"].as_str().unwrap().matches("=PRESENTATION_STYLE_BY_CONTEXT(").count(), 1);
    let result = dispatch(grouped).unwrap();
    let flattened = dispatch(json!({"op":"brep_nurbs_import_step_v9","text":result["text"].as_str().unwrap()})).unwrap();
    assert_eq!(flattened["model"]["bodies"].as_array().unwrap().len(), 2);
    let reopened = cad_step::import_step_shared_assembly(result["text"].as_str().unwrap()).unwrap();
    assert_eq!(reopened.definitions().len(), 1);
    assert_eq!(reopened.occurrences().len(), 2);
    assert!(reopened.occurrences().iter().any(|occ| occ.transform == place(17.)));
    payload["occurrences"] = json!([{"definition":1,"transform":place(0.)}]);
    assert!(dispatch(payload).is_err());
}
