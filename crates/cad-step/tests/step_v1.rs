//! Constructor STEP (v1) keeps the analytic surfaces and round-trips.
#[test]
fn step_roundtrip_not_faceted() {
    let model = brep_core::cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let (text, cert) = cad_step::export_step(&model).unwrap();
    assert!(cert.complete);
    assert!(text.contains("ADVANCED_FACE"));
    assert!(text.contains("PLANE"));
    assert!(text.contains("VERTEX_POINT"));
    assert!(text.contains("EDGE_CURVE"));
    assert!(text.contains("AP242"));
    assert!(!text.contains("FACETED_BREP"));
    let (back, _) = cad_step::import_step(&text).unwrap();
    back.validate().unwrap();
}
