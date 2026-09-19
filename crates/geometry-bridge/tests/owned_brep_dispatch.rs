use brep_core::{Model, cuboid};
use geometry_bridge::{brep, dispatch};
use value_codec::{Value, json, to_value};

const OPS: [&str; 6] = [
    "brep_nurbs_inspect",
    "brep_nurbs_tessellate",
    "brep_nurbs_display",
    "brep_nurbs_certified_tessellate",
    "brep_nurbs_certified_freeform_tessellate",
    "brep_nurbs_to_polygon",
];

#[test]
fn owned_requests_match_typed_inspection_and_tessellation() {
    for model in [
        cuboid([0.; 3], [2., 3., 4.]).unwrap(),
        brep_core::sphere(2.).unwrap(),
    ] {
        let actual = dispatch(json!({"op":OPS[0], "model":model})).unwrap();
        assert_eq!(actual, to_value(model.validate().unwrap()).unwrap());
        for segments in [1, 4] {
            let actual =
                dispatch(json!({"op":OPS[1], "model":model, "segments":segments})).unwrap();
            let expected = brep::nurbs(&model, segments).unwrap();
            assert_eq!(actual, to_value(&expected).unwrap());
            let polygon = polygon_core::solid::brep::from_mesh(
                &expected.built.mesh,
                Some(&expected.face_ids),
            )
            .unwrap();
            assert_eq!(
                dispatch(json!({"op":OPS[5], "model":model, "segments":segments})).unwrap(),
                to_value(polygon).unwrap()
            );
        }
    }
}

#[test]
fn certified_options_survive_model_consumption() {
    let model = cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for tolerance in [0.01, 0.1] {
        for (op, expected) in [
            (
                OPS[3],
                to_value(brep::certified_nurbs(&model, tolerance, 20000).unwrap()).unwrap(),
            ),
            (
                OPS[4],
                to_value(brep::certified_freeform_nurbs(&model, tolerance, 20000).unwrap())
                    .unwrap(),
            ),
        ] {
            assert_eq!(
                dispatch(json!({"op":op,"model":model,"chordToleranceMm":tolerance})).unwrap(),
                expected
            );
        }
    }
    for op in [OPS[3], OPS[4]] {
        for max_triangles in [0, 11, 20001] {
            let error = dispatch(json!({"op":op,"model":model,
                "chordToleranceMm":0.1,"maxTriangles":max_triangles}))
            .unwrap_err();
            assert_eq!(error.code, "BREP_TESSELLATION_OPTIONS_INVALID");
        }
    }
}

#[test]
fn missing_and_malformed_models_retain_decode_errors_before_other_options() {
    for op in OPS {
        for bad in [
            Value::Null,
            json!(false),
            json!([]),
            json!({}),
            json!({"vertices":"invalid"}),
        ] {
            let expected = value_codec::from_value::<Model>(bad.clone()).unwrap_err();
            let error = dispatch(json!({"op":op, "model":bad})).unwrap_err();
            assert_eq!(error.code, "GEOMETRY_INVALID_INPUT");
            assert_eq!(error.message, format!("Invalid model: {expected}"));
        }
        let error = dispatch(json!({"op":op})).unwrap_err();
        assert_eq!(error.message, "Invalid model: Expected B-rep object");
    }
}

#[test]
fn display_still_checks_geometry_and_options() {
    let model = cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let display = dispatch(json!({"op":OPS[2],"model":model,"segments":1})).unwrap();
    assert_eq!(display["surfaceArea"], json!(52.));
    assert_eq!(display["displayVertices"].as_array().unwrap().len(), 216);
    for op in [OPS[1], OPS[2], OPS[5]] {
        let error = dispatch(json!({"op":op,"model":model})).unwrap_err();
        assert!(error.message.starts_with("Invalid segments:"));
    }
    let mut bad = to_value(model).unwrap();
    bad["shells"][0]["faces"][0]["face"] = json!(999);
    for op in OPS {
        assert!(
            dispatch(json!({"op":op,"model":bad,"segments":1,"chordToleranceMm":0.1})).is_err()
        );
    }
}
