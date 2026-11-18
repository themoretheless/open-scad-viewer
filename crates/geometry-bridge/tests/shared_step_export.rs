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
    payload["occurrences"] = json!([{"definition":1,"transform":place(0.)}]);
    assert!(dispatch(payload).is_err());
}
