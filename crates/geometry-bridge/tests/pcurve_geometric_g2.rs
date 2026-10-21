#[path = "../../brep-core/examples/support/face_window_fixture.rs"]
mod fixture;
use value_codec::json;
#[test]
fn actual_trimmed_seam_receipts_preserve_g2_scope_and_reject_curvature_jump() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    for broken in [false, true] {
        let mut target = patch.clone();
        if broken {
            target.control_points[3][2][2] -= 0.1;
        }
        let built =
            brep_core::face_window::replace(&source, face, &target, &boundary, 1e-8, 1000000)
                .unwrap();
        let result=geometry_bridge::dispatch(json!({"op":"brep_geometric_g2_audit","model":built.model,"edges":built.edges,
            "positionTolerance":1e-7,"normalTolerance":1e-7,"curvatureTolerance":1e-5,"maxCells":100000})).unwrap();
        assert_eq!(result["complete"], true);
        assert_eq!(result["withinTolerance"], !broken);
        assert_eq!(result["exactIdentityClaim"], false);
        assert_eq!(result["solidGeometryCertified"], false);
        let work = result["exactWork"].as_u64().unwrap();
        assert!(work > 0 && work <= 1000000);
        assert!(result["cells"].as_u64().unwrap() <= 100000);
        assert!(
            result["seams"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["positionValidated"] == true)
        );
        let wire = value_codec::to_string(&result).unwrap();
        assert!(!wire.contains("Infinity") && !wire.contains("NaN"));
    }
}
