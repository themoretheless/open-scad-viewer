use super::*;
#[test]
fn pole_sphere_json_retains_source_pullbacks_and_uncertified_scope() {
    let sphere =
        crate::dispatch(value_codec::json!({"op":"brep_nurbs_sphere_exact","radius":2.})).unwrap();
    let model: brep_core::Model = value_codec::from_value(sphere.clone()).unwrap();
    let second = brep_core::transform::affine(
        &model,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 2.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let result = dispatch(
        value_codec::json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":second}),
    )
    .unwrap();
    assert_eq!(result["coverage"], "numerically_resolved");
    assert_eq!(result["permitsTopologyChange"], false);
    assert_eq!(result["evidence"], "numerical_uncertified");
    let lifts = result["components"][0]["firstUv"].as_array().unwrap();
    assert_eq!(lifts.len(), 8);
    for (i, lift) in lifts.iter().enumerate() {
        assert_eq!(lift["arcs"], value_codec::json!([]));
        let constraint = &lift["implicitContour"];
        assert_eq!(
            constraint["representation"],
            "implicit-surface-plane-pullback"
        );
        assert_eq!(constraint["arcComponentsEnumerated"], false);
        assert_eq!(constraint["permitsTopologyChange"], false);
        let source: nurbs_core::surface::Surface =
            value_codec::from_value(constraint["sourceSurface"].clone()).unwrap();
        assert_eq!(source, model.faces[i].surface);
    }
}
